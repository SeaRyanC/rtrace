use crate::scene::{Color, Vec3};

/// Configuration parameters for outline detection
#[derive(Debug, Clone)]
pub struct OutlineConfig {
    /// Weight for depth differences (w_d)
    pub depth_weight: f64,
    /// Weight for normal differences (w_n)
    pub normal_weight: f64,
    /// Threshold for edge detection (T)
    pub threshold: f64,
    /// Color for outline edges
    pub edge_color: Color,
    /// Whether to use 8-neighbor (true) or 4-neighbor (false) sampling
    pub use_8_neighbors: bool,
    /// Line thickness factor (1.0 = no thickening, >1.0 = thicker lines)
    pub line_thickness: f64,
}

impl Default for OutlineConfig {
    fn default() -> Self {
        Self {
            depth_weight: 1.0,
            normal_weight: 1.0,
            threshold: 0.1,
            edge_color: Color::new(0.0, 0.0, 0.0), // Black edges
            use_8_neighbors: false, // 4-neighbor by default for performance
            line_thickness: 1.0,
        }
    }
}

/// Buffers containing depth and normal data for outline detection
pub struct OutlineBuffers {
    pub width: u32,
    pub height: u32,
    /// Camera-space depth values (None for background pixels)
    pub depth_buffer: Vec<Option<f64>>,
    /// World-space normal vectors (None for background pixels)
    pub normal_buffer: Vec<Option<Vec3>>,
}

impl OutlineBuffers {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            depth_buffer: vec![None; size],
            normal_buffer: vec![None; size],
        }
    }

    fn get_index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn set_depth(&mut self, x: u32, y: u32, depth: f64) {
        let index = self.get_index(x, y);
        self.depth_buffer[index] = Some(depth);
    }

    pub fn set_normal(&mut self, x: u32, y: u32, normal: Vec3) {
        let index = self.get_index(x, y);
        self.normal_buffer[index] = Some(normal);
    }

    pub fn get_depth(&self, x: u32, y: u32) -> Option<f64> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = self.get_index(x, y);
        self.depth_buffer[index]
    }

    pub fn get_normal(&self, x: u32, y: u32) -> Option<Vec3> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = self.get_index(x, y);
        self.normal_buffer[index]
    }
}

/// Apply outline detection to a color image using depth and normal buffers
pub fn apply_outline_detection(
    image_data: &mut Vec<(u32, u32, Color)>,
    buffers: &OutlineBuffers,
    config: &OutlineConfig,
) {
    // Create edge mask
    let edge_mask = detect_edges(buffers, config);
    
    // Apply line thickness if requested
    let final_mask = if config.line_thickness > 1.0 {
        dilate_edges(&edge_mask, buffers.width, buffers.height, config.line_thickness)
    } else {
        edge_mask
    };

    // Apply edges to image data
    for (x, y, color) in image_data.iter_mut() {
        let index = (*y * buffers.width + *x) as usize;
        if index < final_mask.len() && final_mask[index] > 0.0 {
            // Blend edge color based on edge strength
            let edge_strength = final_mask[index].min(1.0);
            *color = blend_colors(*color, config.edge_color, edge_strength);
        }
    }
}

/// Detect edges using depth and normal discontinuities
fn detect_edges(buffers: &OutlineBuffers, config: &OutlineConfig) -> Vec<f64> {
    let size = (buffers.width * buffers.height) as usize;
    let mut edge_mask = vec![0.0; size];

    for y in 0..buffers.height {
        for x in 0..buffers.width {
            let edge_strength = compute_edge_strength(buffers, x, y, config);
            let index = (y * buffers.width + x) as usize;
            edge_mask[index] = if edge_strength > config.threshold {
                (edge_strength - config.threshold) / (1.0 - config.threshold)
            } else {
                0.0
            };
        }
    }

    edge_mask
}

/// Compute edge strength for a pixel using neighboring pixels
fn compute_edge_strength(
    buffers: &OutlineBuffers,
    x: u32,
    y: u32,
    config: &OutlineConfig,
) -> f64 {
    let current_depth = buffers.get_depth(x, y);
    let current_normal = buffers.get_normal(x, y);

    // Skip background pixels
    if current_depth.is_none() || current_normal.is_none() {
        return 0.0;
    }

    let current_depth = current_depth.unwrap();
    let current_normal = current_normal.unwrap();

    let neighbors = if config.use_8_neighbors {
        get_8_neighbors(x, y)
    } else {
        get_4_neighbors(x, y)
    };

    let mut max_depth_diff: f64 = 0.0;
    let mut max_normal_diff: f64 = 0.0;

    for (nx, ny) in neighbors {
        if let (Some(neighbor_depth), Some(neighbor_normal)) = 
            (buffers.get_depth(nx, ny), buffers.get_normal(nx, ny)) {
            
            // Compute depth difference
            let depth_diff = (current_depth - neighbor_depth).abs();
            max_depth_diff = max_depth_diff.max(depth_diff);

            // Compute normal difference: n_diff = 1 - dot(n_i, n_j)
            let dot_product = current_normal.dot(&neighbor_normal).clamp(-1.0, 1.0);
            let normal_diff = 1.0 - dot_product;
            max_normal_diff = max_normal_diff.max(normal_diff);
        }
    }

    // Normalize depth differences by a reasonable scale
    // Use a heuristic depth scale based on the current depth
    let depth_scale = (current_depth * 0.1).max(0.1);
    let normalized_depth_diff = max_depth_diff / depth_scale;

    // Combine using weights: E = w_d * z_diff + w_n * n_diff
    config.depth_weight * normalized_depth_diff + config.normal_weight * max_normal_diff
}

/// Get 4-connected neighbors (up, down, left, right)
fn get_4_neighbors(x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::new();
    
    // Left
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    // Right
    neighbors.push((x + 1, y));
    // Up
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    // Down
    neighbors.push((x, y + 1));
    
    neighbors
}

/// Get 8-connected neighbors (including diagonals)
fn get_8_neighbors(x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::new();
    
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue; // Skip center pixel
            }
            
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            
            if nx >= 0 && ny >= 0 {
                neighbors.push((nx as u32, ny as u32));
            }
        }
    }
    
    neighbors
}

/// Dilate edge mask for thicker lines
fn dilate_edges(edge_mask: &[f64], width: u32, height: u32, thickness: f64) -> Vec<f64> {
    let mut dilated = edge_mask.to_vec();
    let radius = (thickness - 1.0).ceil() as i32;
    
    if radius <= 0 {
        return dilated;
    }
    
    let original = edge_mask.to_vec();
    
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            let mut max_strength = original[index];
            
            // Check neighborhood for maximum edge strength
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    
                    if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                        let ni = (ny * width as i32 + nx) as usize;
                        if ni < original.len() {
                            let distance = ((dx * dx + dy * dy) as f64).sqrt();
                            if distance <= thickness {
                                // Apply distance-based falloff
                                let falloff = 1.0 - (distance / thickness);
                                let strength = original[ni] * falloff;
                                max_strength = max_strength.max(strength);
                            }
                        }
                    }
                }
            }
            
            dilated[index] = max_strength;
        }
    }
    
    dilated
}

/// Blend two colors with given weight (0.0 = color1, 1.0 = color2)
fn blend_colors(color1: Color, color2: Color, weight: f64) -> Color {
    let weight = weight.clamp(0.0, 1.0);
    Color::new(
        color1.x * (1.0 - weight) + color2.x * weight,
        color1.y * (1.0 - weight) + color2.y * weight,
        color1.z * (1.0 - weight) + color2.z * weight,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outline_config_default() {
        let config = OutlineConfig::default();
        assert_eq!(config.depth_weight, 1.0);
        assert_eq!(config.normal_weight, 1.0);
        assert_eq!(config.threshold, 0.1);
        assert_eq!(config.edge_color, Color::new(0.0, 0.0, 0.0));
        assert!(!config.use_8_neighbors);
        assert_eq!(config.line_thickness, 1.0);
    }

    #[test]
    fn test_outline_buffers_creation() {
        let buffers = OutlineBuffers::new(10, 20);
        assert_eq!(buffers.width, 10);
        assert_eq!(buffers.height, 20);
        assert_eq!(buffers.depth_buffer.len(), 200);
        assert_eq!(buffers.normal_buffer.len(), 200);
    }

    #[test]
    fn test_outline_buffers_set_get() {
        let mut buffers = OutlineBuffers::new(10, 10);
        
        buffers.set_depth(5, 7, 10.5);
        buffers.set_normal(5, 7, Vec3::new(0.0, 0.0, 1.0));
        
        assert_eq!(buffers.get_depth(5, 7), Some(10.5));
        assert_eq!(buffers.get_normal(5, 7), Some(Vec3::new(0.0, 0.0, 1.0)));
        
        // Test bounds checking
        assert_eq!(buffers.get_depth(10, 10), None);
        assert_eq!(buffers.get_normal(10, 10), None);
    }

    #[test]
    fn test_get_4_neighbors() {
        let neighbors = get_4_neighbors(5, 5);
        assert_eq!(neighbors.len(), 4);
        assert!(neighbors.contains(&(4, 5))); // Left
        assert!(neighbors.contains(&(6, 5))); // Right
        assert!(neighbors.contains(&(5, 4))); // Up
        assert!(neighbors.contains(&(5, 6))); // Down
        
        // Test edge case (corner)
        let neighbors = get_4_neighbors(0, 0);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&(1, 0))); // Right
        assert!(neighbors.contains(&(0, 1))); // Down
    }

    #[test]
    fn test_get_8_neighbors() {
        let neighbors = get_8_neighbors(5, 5);
        assert_eq!(neighbors.len(), 8);
        
        // Test edge case (corner)
        let neighbors = get_8_neighbors(0, 0);
        assert_eq!(neighbors.len(), 3);
        assert!(neighbors.contains(&(1, 0)));
        assert!(neighbors.contains(&(0, 1)));
        assert!(neighbors.contains(&(1, 1)));
    }

    #[test]
    fn test_blend_colors() {
        let color1 = Color::new(1.0, 0.0, 0.0); // Red
        let color2 = Color::new(0.0, 1.0, 0.0); // Green
        
        let blended = blend_colors(color1, color2, 0.5);
        assert_eq!(blended, Color::new(0.5, 0.5, 0.0));
        
        let blended = blend_colors(color1, color2, 0.0);
        assert_eq!(blended, color1);
        
        let blended = blend_colors(color1, color2, 1.0);
        assert_eq!(blended, color2);
    }

    #[test]
    fn test_edge_detection_simple() {
        let mut buffers = OutlineBuffers::new(3, 3);
        let config = OutlineConfig::default();
        
        // Create a simple depth discontinuity in the middle
        for y in 0..3 {
            for x in 0..3 {
                let depth = if x == 1 { 10.0 } else { 1.0 };
                let normal = Vec3::new(0.0, 0.0, 1.0);
                buffers.set_depth(x, y, depth);
                buffers.set_normal(x, y, normal);
            }
        }
        
        let edge_mask = detect_edges(&buffers, &config);
        
        // The middle column should have edges due to depth discontinuity
        let center_index = (1 * buffers.width + 1) as usize;
        assert!(edge_mask[center_index] > 0.0, "Center pixel should have an edge");
    }

    #[test]
    fn test_outline_detection_integration() {
        use crate::scene::Color;
        
        let mut buffers = OutlineBuffers::new(3, 3);
        let config = OutlineConfig {
            depth_weight: 1.0,
            normal_weight: 1.0,
            threshold: 0.05,
            edge_color: Color::new(1.0, 0.0, 0.0), // Red edges
            use_8_neighbors: false,
            line_thickness: 1.0,
        };
        
        // Create test data with depth and normal discontinuities
        for y in 0..3 {
            for x in 0..3 {
                let depth = if x == 1 { 5.0 } else { 1.0 };
                let normal = if x == 1 { 
                    Vec3::new(1.0, 0.0, 0.0) 
                } else { 
                    Vec3::new(0.0, 0.0, 1.0) 
                };
                buffers.set_depth(x, y, depth);
                buffers.set_normal(x, y, normal);
            }
        }
        
        // Create initial image data
        let mut image_data = Vec::new();
        for y in 0..3 {
            for x in 0..3 {
                image_data.push((x, y, Color::new(0.5, 0.5, 0.5))); // Gray background
            }
        }
        
        // Apply outline detection
        apply_outline_detection(&mut image_data, &buffers, &config);
        
        // Check that edges were applied
        let center_pixel = image_data.iter().find(|(x, y, _)| *x == 1 && *y == 1).unwrap();
        
        // The center pixel should have some red component from edge detection
        assert!(center_pixel.2.x > 0.5, "Center pixel should have red edge contribution");
    }
}