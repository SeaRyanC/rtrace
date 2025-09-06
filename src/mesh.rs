use nalgebra::{Vector3, Point3};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use serde::{Deserialize, Serialize};

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

/// Immutable mesh object containing triangles
#[derive(Debug, Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub bounds_min: Point,
    pub bounds_max: Point,
}

impl Mesh {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self {
            triangles: Vec::new(),
            bounds_min: Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            bounds_max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
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
}