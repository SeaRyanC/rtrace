use nalgebra::{Vector3, Point3};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// 3D point type alias
pub type Point = Point3<f64>;

/// 3D vector type alias  
pub type Vec3 = Vector3<f64>;

/// Triangle defined by three vertices and a normal
#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Point; 3],
    pub normal: Vec3,
}

impl Triangle {
    /// Get the center point of the triangle
    pub fn center(&self) -> Point {
        (self.vertices[0] + self.vertices[1].coords + self.vertices[2].coords) / 3.0
    }

    /// Get the bounding box of the triangle
    pub fn bounds(&self) -> (Point, Point) {
        let mut min = self.vertices[0];
        let mut max = self.vertices[0];
        
        for vertex in &self.vertices[1..] {
            min.coords = min.coords.inf(&vertex.coords);
            max.coords = max.coords.sup(&vertex.coords);
        }
        
        (min, max)
    }
}

/// K-d tree node for accelerating ray-triangle intersections
#[derive(Debug, Clone)]
enum KdNode {
    /// Internal node with splitting plane
    Internal {
        axis: usize,          // 0=x, 1=y, 2=z
        split_pos: f64,       // position along axis
        left: Box<KdNode>,    // left child (values <= split_pos)
        right: Box<KdNode>,   // right child (values > split_pos)
        bounds: (Point, Point), // bounding box of this node
    },
    /// Leaf node containing triangles
    Leaf {
        triangles: Vec<usize>, // indices into mesh triangle array
        bounds: (Point, Point), // bounding box of this node
    },
}

/// K-d tree for accelerating ray-triangle intersections
/// 
/// A k-dimensional tree that recursively subdivides 3D space to enable
/// fast ray-triangle intersection queries. Instead of testing every triangle
/// in a mesh (O(n) complexity), the k-d tree allows logarithmic search time
/// O(log n) by only testing triangles in leaf nodes that the ray intersects.
/// 
/// For the 35,628 triangle Espresso Tray STL file, this provides significant
/// performance improvement over brute force intersection testing.
#[derive(Debug, Clone)]
pub struct KdTree {
    root: Option<KdNode>,
    max_depth: usize,
    max_triangles_per_leaf: usize,
}

impl KdTree {
    /// Create a new k-d tree for the given triangles
    pub fn new(triangles: &[Triangle], max_depth: usize, max_triangles_per_leaf: usize) -> Self {
        let mut tree = Self {
            root: None,
            max_depth,
            max_triangles_per_leaf,
        };

        if !triangles.is_empty() {
            // Create list of all triangle indices
            let triangle_indices: Vec<usize> = (0..triangles.len()).collect();
            
            // Compute overall bounds
            let bounds = Self::compute_bounds(triangles, &triangle_indices);
            
            // Build the tree recursively
            tree.root = Some(tree.build_recursive(triangles, triangle_indices, bounds, 0));
        }

        tree
    }

    /// Recursively build the k-d tree
    fn build_recursive(
        &self,
        triangles: &[Triangle],
        triangle_indices: Vec<usize>,
        bounds: (Point, Point),
        depth: usize,
    ) -> KdNode {
        // Create leaf if we've reached maximum depth or have few enough triangles
        if depth >= self.max_depth || triangle_indices.len() <= self.max_triangles_per_leaf {
            return KdNode::Leaf {
                triangles: triangle_indices,
                bounds,
            };
        }

        // Choose splitting axis (cycle through x, y, z)
        let axis = depth % 3;
        
        // Find median position along the axis
        let mut positions: Vec<(f64, usize)> = triangle_indices
            .iter()
            .map(|&idx| (triangles[idx].center()[axis], idx))
            .collect();
        
        positions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        let median_idx = positions.len() / 2;
        let split_pos = positions[median_idx].0;

        // Split triangles into left and right
        let mut left_triangles = Vec::new();
        let mut right_triangles = Vec::new();
        
        for (pos, triangle_idx) in positions {
            if pos <= split_pos {
                left_triangles.push(triangle_idx);
            } else {
                right_triangles.push(triangle_idx);
            }
        }

        // Ensure we don't create empty splits
        if left_triangles.is_empty() {
            left_triangles.push(right_triangles.pop().unwrap());
        } else if right_triangles.is_empty() {
            right_triangles.push(left_triangles.pop().unwrap());
        }

        // Compute bounds for left and right children
        let left_bounds = Self::compute_bounds(triangles, &left_triangles);
        let right_bounds = Self::compute_bounds(triangles, &right_triangles);

        // Recursively build left and right subtrees
        let left = Box::new(self.build_recursive(triangles, left_triangles, left_bounds, depth + 1));
        let right = Box::new(self.build_recursive(triangles, right_triangles, right_bounds, depth + 1));

        KdNode::Internal {
            axis,
            split_pos,
            left,
            right,
            bounds,
        }
    }

    /// Compute bounding box for a set of triangles
    fn compute_bounds(triangles: &[Triangle], triangle_indices: &[usize]) -> (Point, Point) {
        if triangle_indices.is_empty() {
            return (Point::origin(), Point::origin());
        }

        let mut min = Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for &idx in triangle_indices {
            let (tri_min, tri_max) = triangles[idx].bounds();
            min.coords = min.coords.inf(&tri_min.coords);
            max.coords = max.coords.sup(&tri_max.coords);
        }

        (min, max)
    }

    /// Check if a ray intersects a bounding box
    fn ray_intersects_bounds(ray_origin: &Point, ray_direction: &Vec3, bounds: &(Point, Point)) -> bool {
        let (min, max) = bounds;
        
        let mut t_min = f64::NEG_INFINITY;
        let mut t_max = f64::INFINITY;
        
        for axis in 0..3 {
            if ray_direction[axis].abs() < 1e-9 {
                // Ray is parallel to the slab
                if ray_origin[axis] < min[axis] || ray_origin[axis] > max[axis] {
                    return false;
                }
            } else {
                let inv_dir = 1.0 / ray_direction[axis];
                let mut t0 = (min[axis] - ray_origin[axis]) * inv_dir;
                let mut t1 = (max[axis] - ray_origin[axis]) * inv_dir;
                
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                
                t_min = t_min.max(t0);
                t_max = t_max.min(t1);
                
                if t_min > t_max || t_max < 0.0 {
                    return false;
                }
            }
        }
        
        true
    }

    /// Traverse the k-d tree to find triangle candidates for ray intersection
    pub fn traverse<F>(&self, ray_origin: &Point, ray_direction: &Vec3, mut callback: F)
    where
        F: FnMut(&[usize]),
    {
        if let Some(ref root) = self.root {
            self.traverse_recursive(root, ray_origin, ray_direction, &mut callback);
        }
    }

    /// Recursive traversal of the k-d tree
    fn traverse_recursive<F>(
        &self,
        node: &KdNode,
        ray_origin: &Point,
        ray_direction: &Vec3,
        callback: &mut F,
    ) where
        F: FnMut(&[usize]),
    {
        match node {
            KdNode::Leaf { triangles, bounds } => {
                // Check if ray intersects this leaf's bounds
                if Self::ray_intersects_bounds(ray_origin, ray_direction, bounds) {
                    callback(triangles);
                }
            }
            KdNode::Internal { axis, split_pos, left, right, bounds } => {
                // Check if ray intersects this node's bounds
                if !Self::ray_intersects_bounds(ray_origin, ray_direction, bounds) {
                    return;
                }

                let origin_pos = ray_origin[*axis];
                let dir = ray_direction[*axis];

                // If ray is parallel to the splitting plane, only traverse the side it's on
                if dir.abs() < 1e-9 {
                    if origin_pos <= *split_pos {
                        self.traverse_recursive(left.as_ref(), ray_origin, ray_direction, callback);
                    } else {
                        self.traverse_recursive(right.as_ref(), ray_origin, ray_direction, callback);
                    }
                    return;
                }

                // Calculate where ray intersects the splitting plane
                let t_split = (*split_pos - origin_pos) / dir;

                // Determine which child to traverse first based on ray direction
                let (first, second) = if origin_pos <= *split_pos {
                    (left.as_ref(), right.as_ref())
                } else {
                    (right.as_ref(), left.as_ref())
                };

                // Always traverse the first child (the one containing the ray origin)
                self.traverse_recursive(first, ray_origin, ray_direction, callback);

                // Only traverse the second child if the ray crosses the splitting plane
                // at a positive t value (i.e., it actually reaches the other side)
                if t_split >= 0.0 {
                    self.traverse_recursive(second, ray_origin, ray_direction, callback);
                }
            }
        }
    }
}

/// Immutable mesh object containing triangles
#[derive(Debug, Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub bounds_min: Point,
    pub bounds_max: Point,
    pub kdtree: KdTree,
}

impl Mesh {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self {
            triangles: Vec::new(),
            bounds_min: Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            bounds_max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
            kdtree: KdTree::new(&[], 16, 10), // Empty k-d tree
        }
    }

    /// Load mesh from STL file (auto-detects binary vs ASCII)
    pub fn from_stl_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(&path)?;
        
        // Try to determine if this is ASCII or binary STL
        let mut header = [0u8; 80];
        file.read_exact(&mut header)?;
        
        let header_str = String::from_utf8_lossy(&header);
        if header_str.trim_start().starts_with("solid") {
            // Might be ASCII, but we need to check if it's actually ASCII throughout
            file.seek(SeekFrom::Start(0))?;
            if Self::is_ascii_stl(&mut file)? {
                file.seek(SeekFrom::Start(0))?;
                return Self::load_ascii_stl(file);
            }
        }
        
        // Binary STL
        file.seek(SeekFrom::Start(0))?;
        Self::load_binary_stl(file)
    }

    /// Load mesh from STL byte buffer (auto-detects binary vs ASCII)
    pub fn from_stl_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() < 80 {
            return Err("STL data too short".into());
        }

        let header_str = String::from_utf8_lossy(&bytes[0..80]);
        if header_str.trim_start().starts_with("solid") && Self::is_ascii_stl_bytes(bytes)? {
            Self::load_ascii_stl_bytes(bytes)
        } else {
            Self::load_binary_stl_bytes(bytes)
        }
    }

    /// Check if STL file is ASCII format by looking for ASCII markers
    fn is_ascii_stl(file: &mut File) -> Result<bool, Box<dyn std::error::Error>> {
        let reader = BufReader::new(file);
        let mut line_count = 0;
        
        for line in reader.lines() {
            let line = line?;
            line_count += 1;
            
            if line_count > 10 {
                break;
            }
            
            let trimmed = line.trim();
            if trimmed.starts_with("facet normal") || trimmed == "outer loop" || trimmed == "endloop" {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Check if STL bytes represent ASCII format
    fn is_ascii_stl_bytes(bytes: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        let content = String::from_utf8_lossy(bytes);
        let lines: Vec<&str> = content.lines().take(10).collect();
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("facet normal") || trimmed == "outer loop" || trimmed == "endloop" {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Load ASCII STL format
    fn load_ascii_stl(mut file: File) -> Result<Self, Box<dyn std::error::Error>> {
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Self::load_ascii_stl_bytes(content.as_bytes())
    }

    /// Load ASCII STL from bytes
    fn load_ascii_stl_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let content = String::from_utf8_lossy(bytes);
        let lines: Vec<&str> = content.lines().collect();
        
        let mut mesh = Mesh::new();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            if line.starts_with("facet normal") {
                // Parse normal vector
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 5 {
                    return Err("Invalid facet normal format".into());
                }
                
                let nx: f64 = parts[2].parse()?;
                let ny: f64 = parts[3].parse()?; 
                let nz: f64 = parts[4].parse()?;
                let normal = Vec3::new(nx, ny, nz);
                
                i += 1; // Skip "outer loop"
                if i >= lines.len() || lines[i].trim() != "outer loop" {
                    return Err("Expected 'outer loop' after facet normal".into());
                }
                
                // Parse three vertices
                let mut vertices = [Point::origin(); 3];
                for j in 0..3 {
                    i += 1;
                    if i >= lines.len() {
                        return Err("Unexpected end of file while reading vertex".into());
                    }
                    
                    let vertex_line = lines[i].trim();
                    if !vertex_line.starts_with("vertex") {
                        return Err("Expected vertex line".into());
                    }
                    
                    let parts: Vec<&str> = vertex_line.split_whitespace().collect();
                    if parts.len() != 4 {
                        return Err("Invalid vertex format".into());
                    }
                    
                    let x: f64 = parts[1].parse()?;
                    let y: f64 = parts[2].parse()?;
                    let z: f64 = parts[3].parse()?;
                    vertices[j] = Point::new(x, y, z);
                }
                
                i += 1; // Skip "endloop"
                if i >= lines.len() || lines[i].trim() != "endloop" {
                    return Err("Expected 'endloop'".into());
                }
                
                i += 1; // Skip "endfacet"  
                if i >= lines.len() || lines[i].trim() != "endfacet" {
                    return Err("Expected 'endfacet'".into());
                }
                
                mesh.add_triangle(Triangle { vertices, normal });
            }
            
            i += 1;
        }
        
        mesh.compute_bounds();
        mesh.build_kdtree();
        Ok(mesh)
    }

    /// Load binary STL format
    fn load_binary_stl(mut file: File) -> Result<Self, Box<dyn std::error::Error>> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Self::load_binary_stl_bytes(&bytes)
    }

    /// Load binary STL from bytes
    fn load_binary_stl_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() < 84 {
            return Err("Binary STL too short".into());
        }

        // Skip 80-byte header, read triangle count
        let triangle_count = u32::from_le_bytes([
            bytes[80], bytes[81], bytes[82], bytes[83]
        ]) as usize;

        let expected_size = 84 + triangle_count * 50;
        if bytes.len() < expected_size {
            return Err(format!("Binary STL size mismatch: expected {}, got {}", expected_size, bytes.len()).into());
        }

        let mut mesh = Mesh::new();
        let mut offset = 84;

        for _ in 0..triangle_count {
            if offset + 50 > bytes.len() {
                return Err("Unexpected end of binary STL data".into());
            }

            // Read normal (3 * f32)
            let nx = f32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]) as f64;
            let ny = f32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]) as f64;
            let nz = f32::from_le_bytes([bytes[offset+8], bytes[offset+9], bytes[offset+10], bytes[offset+11]]) as f64;
            let normal = Vec3::new(nx, ny, nz);
            offset += 12;

            // Read three vertices (3 * 3 * f32)
            let mut vertices = [Point::origin(); 3];
            for i in 0..3 {
                let x = f32::from_le_bytes([bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]]) as f64;
                let y = f32::from_le_bytes([bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]]) as f64;
                let z = f32::from_le_bytes([bytes[offset+8], bytes[offset+9], bytes[offset+10], bytes[offset+11]]) as f64;
                vertices[i] = Point::new(x, y, z);
                offset += 12;
            }

            // Skip 2-byte attribute
            offset += 2;

            mesh.add_triangle(Triangle { vertices, normal });
        }

        mesh.compute_bounds();
        mesh.build_kdtree();
        Ok(mesh)
    }

    /// Add a triangle to the mesh
    fn add_triangle(&mut self, triangle: Triangle) {
        // Update bounding box
        for vertex in &triangle.vertices {
            self.bounds_min.coords = self.bounds_min.coords.inf(&vertex.coords);
            self.bounds_max.coords = self.bounds_max.coords.sup(&vertex.coords);
        }
        
        self.triangles.push(triangle);
    }

    /// Compute bounding box for the mesh
    fn compute_bounds(&mut self) {
        if self.triangles.is_empty() {
            self.bounds_min = Point::origin();
            self.bounds_max = Point::origin();
            return;
        }

        self.bounds_min = Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        self.bounds_max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for triangle in &self.triangles {
            for vertex in &triangle.vertices {
                self.bounds_min.coords = self.bounds_min.coords.inf(&vertex.coords);
                self.bounds_max.coords = self.bounds_max.coords.sup(&vertex.coords);
            }
        }
    }

    /// Build k-d tree for accelerating ray intersections
    fn build_kdtree(&mut self) {
        // Use reasonable defaults: max depth 16, max 10 triangles per leaf
        self.kdtree = KdTree::new(&self.triangles, 16, 10);
    }

    /// Get the number of triangles in the mesh
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Get mesh bounding box
    pub fn bounds(&self) -> (Point, Point) {
        (self.bounds_min, self.bounds_max)
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_mesh() {
        let mesh = Mesh::new();
        assert_eq!(mesh.triangle_count(), 0);
    }

    #[test]
    fn test_mesh_bounds() {
        let mut mesh = Mesh::new();
        let triangle = Triangle {
            vertices: [
                Point::new(-1.0, -1.0, -1.0),
                Point::new(1.0, -1.0, -1.0),
                Point::new(0.0, 1.0, -1.0),
            ],
            normal: Vec3::new(0.0, 0.0, 1.0),
        };
        
        mesh.add_triangle(triangle);
        mesh.compute_bounds();
        
        let (min, max) = mesh.bounds();
        assert_eq!(min, Point::new(-1.0, -1.0, -1.0));
        assert_eq!(max, Point::new(1.0, 1.0, -1.0));
    }

    #[test]
    fn test_ascii_detection() {
        let ascii_content = b"solid test
facet normal 0 0 1
  outer loop
    vertex -1 -1 0
    vertex 1 -1 0
    vertex 0 1 0
  endloop
endfacet
endsolid test";
        
        assert!(Mesh::is_ascii_stl_bytes(ascii_content).unwrap());
    }

    #[test]
    fn test_ascii_stl_parsing() {
        let ascii_content = b"solid test
facet normal 0 0 1
  outer loop
    vertex -1 -1 0
    vertex 1 -1 0
    vertex 0 1 0
  endloop
endfacet
facet normal 0 0 -1
  outer loop
    vertex 0 1 0
    vertex 1 -1 0
    vertex -1 -1 0
  endloop
endfacet
endsolid test";
        
        let mesh = Mesh::from_stl_bytes(ascii_content).unwrap();
        assert_eq!(mesh.triangle_count(), 2);
        
        // Check first triangle
        assert_eq!(mesh.triangles[0].vertices[0], Point::new(-1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[1], Point::new(1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[2], Point::new(0.0, 1.0, 0.0));
        assert_eq!(mesh.triangles[0].normal, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_binary_stl_parsing() {
        // Create a simple binary STL with one triangle
        let mut binary_data = vec![0u8; 80]; // header
        binary_data.extend_from_slice(&1u32.to_le_bytes()); // triangle count
        
        // Triangle data: normal + 3 vertices + attribute
        let normal = [0.0f32, 0.0f32, 1.0f32];
        let vertex1 = [-1.0f32, -1.0f32, 0.0f32];
        let vertex2 = [1.0f32, -1.0f32, 0.0f32];
        let vertex3 = [0.0f32, 1.0f32, 0.0f32];
        let attribute = 0u16;
        
        // Add normal
        for &f in &normal {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        // Add vertices
        for &f in &vertex1 {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        for &f in &vertex2 {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        for &f in &vertex3 {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        // Add attribute
        binary_data.extend_from_slice(&attribute.to_le_bytes());
        
        let mesh = Mesh::from_stl_bytes(&binary_data).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        
        // Check triangle data
        assert_eq!(mesh.triangles[0].vertices[0], Point::new(-1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[1], Point::new(1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[2], Point::new(0.0, 1.0, 0.0));
        assert_eq!(mesh.triangles[0].normal, Vec3::new(0.0, 0.0, 1.0));
    }
}