/// Core library for rtrace
///
/// This library provides STL file parsing capabilities alongside basic functionality.
use std::fs;
use std::io::Cursor;
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

/// Returns a greeting message
///
/// # Examples
///
/// ```
/// use rtrace::hello_world;
///
/// let message = hello_world();
/// assert_eq!(message, "hello world");
/// ```
pub fn hello_world() -> String {
    "hello world".to_string()
}

/// A 3D vertex with x, y, z coordinates
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// A triangle with three vertices and a normal vector
#[derive(Debug, Clone, PartialEq)]
pub struct Triangle {
    pub normal: Vertex,
    pub vertices: [Vertex; 3],
}

impl Triangle {
    pub fn new(normal: Vertex, vertices: [Vertex; 3]) -> Self {
        Self { normal, vertices }
    }
}

/// An immutable mesh containing a collection of triangles
#[derive(Debug, Clone)]
pub struct Mesh {
    triangles: Vec<Triangle>,
}

impl Mesh {
    pub fn new(triangles: Vec<Triangle>) -> Self {
        Self { triangles }
    }

    pub fn triangles(&self) -> &[Triangle] {
        &self.triangles
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
}

/// Errors that can occur during STL parsing
#[derive(Error, Debug)]
pub enum StlError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid STL format: {0}")]
    InvalidFormat(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Binary STL has invalid triangle count: expected {expected}, found {found}")]
    InvalidTriangleCount { expected: u32, found: usize },
}

/// Parse STL file from filename
///
/// # Examples
///
/// ```no_run
/// use rtrace::parse_stl_file;
/// 
/// let mesh = parse_stl_file("model.stl").expect("Failed to parse STL");
/// println!("Loaded {} triangles", mesh.triangle_count());
/// ```
pub fn parse_stl_file<P: AsRef<Path>>(path: P) -> Result<Mesh, StlError> {
    let data = fs::read(path)?;
    parse_stl_bytes(&data)
}

/// Parse STL data from byte buffer
///
/// # Examples
///
/// ```
/// use rtrace::parse_stl_bytes;
/// 
/// let ascii_stl = b"solid test
/// facet normal 0.0 0.0 1.0
///   outer loop
///     vertex 0.0 0.0 0.0
///     vertex 1.0 0.0 0.0
///     vertex 0.5 1.0 0.0
///   endloop
/// endfacet
/// endsolid test";
/// 
/// let mesh = parse_stl_bytes(ascii_stl).expect("Failed to parse STL");
/// assert_eq!(mesh.triangle_count(), 1);
/// ```
pub fn parse_stl_bytes(data: &[u8]) -> Result<Mesh, StlError> {
    // Check if it's ASCII or binary STL
    if is_ascii_stl(data) {
        parse_ascii_stl(data)
    } else {
        parse_binary_stl(data)
    }
}

fn is_ascii_stl(data: &[u8]) -> bool {
    // ASCII STL files start with "solid" keyword
    // But we need to be careful as binary STL files can also have "solid" in the header
    if data.len() < 5 {
        return false;
    }
    
    if !data.starts_with(b"solid") {
        return false;
    }
    
    // For binary STL files, we can check if the file size matches the expected binary structure
    // Binary STL: 80 bytes header + 4 bytes triangle count + (50 bytes * triangle_count)
    if data.len() >= 84 {
        let triangle_count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
        let expected_size = 84 + (triangle_count as usize * 50);
        if data.len() == expected_size {
            // This matches binary STL format exactly - likely binary
            return false;
        }
    }
    
    // Check if it contains typical ASCII STL keywords and is valid UTF-8
    let sample_size = std::cmp::min(data.len(), 1000);
    let sample = &data[..sample_size];
    
    // Must be valid UTF-8 for ASCII STL
    if let Ok(s) = std::str::from_utf8(sample) {
        // Check for typical ASCII STL structure
        s.contains("facet") && (s.contains("vertex") || s.contains("endsolid"))
    } else {
        false
    }
}

fn parse_ascii_stl(data: &[u8]) -> Result<Mesh, StlError> {
    let content = std::str::from_utf8(data)
        .map_err(|_| StlError::InvalidFormat("Invalid UTF-8 in ASCII STL".to_string()))?;
    
    let mut triangles = Vec::new();
    let mut lines = content.lines();
    
    // Skip the first "solid" line
    if let Some(first_line) = lines.next() {
        if !first_line.trim().starts_with("solid") {
            return Err(StlError::InvalidFormat("ASCII STL must start with 'solid'".to_string()));
        }
    } else {
        return Err(StlError::InvalidFormat("Empty STL file".to_string()));
    }
    
    while let Some(line) = lines.next() {
        let line = line.trim();
        
        if line.starts_with("facet normal") {
            // Parse normal vector
            let normal_parts: Vec<&str> = line.split_whitespace().collect();
            if normal_parts.len() != 5 {
                return Err(StlError::Parse("Invalid facet normal line".to_string()));
            }
            
            let normal = Vertex::new(
                normal_parts[2].parse::<f32>()
                    .map_err(|_| StlError::Parse("Invalid normal x coordinate".to_string()))?,
                normal_parts[3].parse::<f32>()
                    .map_err(|_| StlError::Parse("Invalid normal y coordinate".to_string()))?,
                normal_parts[4].parse::<f32>()
                    .map_err(|_| StlError::Parse("Invalid normal z coordinate".to_string()))?,
            );
            
            // Skip "outer loop" line
            if let Some(loop_line) = lines.next() {
                if !loop_line.trim().starts_with("outer loop") {
                    return Err(StlError::Parse("Expected 'outer loop' after facet normal".to_string()));
                }
            } else {
                return Err(StlError::Parse("Unexpected end of file after facet normal".to_string()));
            }
            
            // Parse three vertices
            let mut vertices = [
                Vertex::new(0.0, 0.0, 0.0),
                Vertex::new(0.0, 0.0, 0.0),
                Vertex::new(0.0, 0.0, 0.0),
            ];
            
            for vertex in &mut vertices {
                if let Some(vertex_line) = lines.next() {
                    let vertex_line = vertex_line.trim();
                    if !vertex_line.starts_with("vertex") {
                        return Err(StlError::Parse(format!("Expected vertex line, got: {}", vertex_line)));
                    }
                    
                    let vertex_parts: Vec<&str> = vertex_line.split_whitespace().collect();
                    if vertex_parts.len() != 4 {
                        return Err(StlError::Parse("Invalid vertex line".to_string()));
                    }
                    
                    *vertex = Vertex::new(
                        vertex_parts[1].parse::<f32>()
                            .map_err(|_| StlError::Parse("Invalid vertex x coordinate".to_string()))?,
                        vertex_parts[2].parse::<f32>()
                            .map_err(|_| StlError::Parse("Invalid vertex y coordinate".to_string()))?,
                        vertex_parts[3].parse::<f32>()
                            .map_err(|_| StlError::Parse("Invalid vertex z coordinate".to_string()))?,
                    );
                } else {
                    return Err(StlError::Parse("Unexpected end of file while reading vertices".to_string()));
                }
            }
            
            // Skip "endloop" and "endfacet" lines
            if let Some(endloop_line) = lines.next() {
                if !endloop_line.trim().starts_with("endloop") {
                    return Err(StlError::Parse("Expected 'endloop' after vertices".to_string()));
                }
            } else {
                return Err(StlError::Parse("Unexpected end of file after vertices".to_string()));
            }
            
            if let Some(endfacet_line) = lines.next() {
                if !endfacet_line.trim().starts_with("endfacet") {
                    return Err(StlError::Parse("Expected 'endfacet' after endloop".to_string()));
                }
            } else {
                return Err(StlError::Parse("Unexpected end of file after endloop".to_string()));
            }
            
            triangles.push(Triangle::new(normal, vertices));
        } else if line.starts_with("endsolid") {
            break;
        }
        // Skip empty lines and other non-facet lines
    }
    
    Ok(Mesh::new(triangles))
}

fn parse_binary_stl(data: &[u8]) -> Result<Mesh, StlError> {
    if data.len() < 84 {
        return Err(StlError::InvalidFormat("Binary STL too short".to_string()));
    }
    
    let mut cursor = Cursor::new(data);
    
    // Skip 80-byte header
    cursor.set_position(80);
    
    // Read triangle count
    let triangle_count = cursor.read_u32::<LittleEndian>()?;
    
    let expected_size = 84 + (triangle_count as usize * 50);
    if data.len() < expected_size {
        return Err(StlError::InvalidFormat(
            format!("Binary STL file too short: expected {} bytes, got {}", expected_size, data.len())
        ));
    }
    
    let mut triangles = Vec::with_capacity(triangle_count as usize);
    
    for _ in 0..triangle_count {
        // Read normal vector (3 f32s)
        let normal = Vertex::new(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );
        
        // Read three vertices (9 f32s total)
        let vertices = [
            Vertex::new(
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ),
            Vertex::new(
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ),
            Vertex::new(
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ),
        ];
        
        // Skip 2-byte attribute
        cursor.read_u16::<LittleEndian>()?;
        
        triangles.push(Triangle::new(normal, vertices));
    }
    
    Ok(Mesh::new(triangles))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hello_world() {
        assert_eq!(hello_world(), "hello world");
    }

    #[test]
    fn test_vertex_creation() {
        let vertex = Vertex::new(1.0, 2.0, 3.0);
        assert_eq!(vertex.x, 1.0);
        assert_eq!(vertex.y, 2.0);
        assert_eq!(vertex.z, 3.0);
    }

    #[test]
    fn test_triangle_creation() {
        let normal = Vertex::new(0.0, 0.0, 1.0);
        let vertices = [
            Vertex::new(0.0, 0.0, 0.0),
            Vertex::new(1.0, 0.0, 0.0),
            Vertex::new(0.5, 1.0, 0.0),
        ];
        let triangle = Triangle::new(normal.clone(), vertices.clone());
        assert_eq!(triangle.normal, normal);
        assert_eq!(triangle.vertices, vertices);
    }

    #[test]
    fn test_mesh_creation() {
        let normal = Vertex::new(0.0, 0.0, 1.0);
        let vertices = [
            Vertex::new(0.0, 0.0, 0.0),
            Vertex::new(1.0, 0.0, 0.0),
            Vertex::new(0.5, 1.0, 0.0),
        ];
        let triangle = Triangle::new(normal, vertices);
        let mesh = Mesh::new(vec![triangle]);
        
        assert_eq!(mesh.triangle_count(), 1);
        assert!(!mesh.is_empty());
        assert_eq!(mesh.triangles().len(), 1);
    }

    #[test]
    fn test_empty_mesh() {
        let mesh = Mesh::new(vec![]);
        assert_eq!(mesh.triangle_count(), 0);
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_is_ascii_stl_detection() {
        let ascii_data = b"solid test\nfacet normal 0 0 1\nvertex 0 0 0\n";
        assert!(is_ascii_stl(ascii_data));
        
        let binary_data = vec![0u8; 100]; // Binary data starting with zeros
        assert!(!is_ascii_stl(&binary_data));
        
        let too_short = b"sol";
        assert!(!is_ascii_stl(too_short));
    }

    #[test]
    fn test_parse_ascii_stl_single_triangle() {
        let ascii_stl = b"solid test
facet normal 0.0 0.0 1.0
  outer loop
    vertex 0.0 0.0 0.0
    vertex 1.0 0.0 0.0
    vertex 0.5 1.0 0.0
  endloop
endfacet
endsolid test";

        let mesh = parse_stl_bytes(ascii_stl).expect("Failed to parse ASCII STL");
        assert_eq!(mesh.triangle_count(), 1);
        
        let triangle = &mesh.triangles()[0];
        assert_eq!(triangle.normal, Vertex::new(0.0, 0.0, 1.0));
        assert_eq!(triangle.vertices[0], Vertex::new(0.0, 0.0, 0.0));
        assert_eq!(triangle.vertices[1], Vertex::new(1.0, 0.0, 0.0));
        assert_eq!(triangle.vertices[2], Vertex::new(0.5, 1.0, 0.0));
    }

    #[test]
    fn test_parse_ascii_stl_multiple_triangles() {
        let ascii_stl = b"solid test
facet normal 0.0 0.0 1.0
  outer loop
    vertex 0.0 0.0 0.0
    vertex 1.0 0.0 0.0
    vertex 0.5 1.0 0.0
  endloop
endfacet
facet normal 1.0 0.0 0.0
  outer loop
    vertex 0.0 0.0 0.0
    vertex 0.0 1.0 0.0
    vertex 0.0 0.5 1.0
  endloop
endfacet
endsolid test";

        let mesh = parse_stl_bytes(ascii_stl).expect("Failed to parse ASCII STL");
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn test_parse_binary_stl() {
        let mut binary_stl = Vec::new();
        
        // 80-byte header (filled with zeros for simplicity)
        binary_stl.extend(vec![0u8; 80]);
        
        // Triangle count (1 triangle, little-endian)
        binary_stl.extend(&1u32.to_le_bytes());
        
        // Triangle data:
        // Normal vector (0.0, 0.0, 1.0)
        binary_stl.extend(&(0.0f32).to_le_bytes());
        binary_stl.extend(&(0.0f32).to_le_bytes());
        binary_stl.extend(&(1.0f32).to_le_bytes());
        
        // Vertex 1 (0.0, 0.0, 0.0)
        binary_stl.extend(&(0.0f32).to_le_bytes());
        binary_stl.extend(&(0.0f32).to_le_bytes());
        binary_stl.extend(&(0.0f32).to_le_bytes());
        
        // Vertex 2 (1.0, 0.0, 0.0)
        binary_stl.extend(&(1.0f32).to_le_bytes());
        binary_stl.extend(&(0.0f32).to_le_bytes());
        binary_stl.extend(&(0.0f32).to_le_bytes());
        
        // Vertex 3 (0.5, 1.0, 0.0)
        binary_stl.extend(&(0.5f32).to_le_bytes());
        binary_stl.extend(&(1.0f32).to_le_bytes());
        binary_stl.extend(&(0.0f32).to_le_bytes());
        
        // 2-byte attribute
        binary_stl.extend(&0u16.to_le_bytes());
        
        let mesh = parse_stl_bytes(&binary_stl).expect("Failed to parse binary STL");
        assert_eq!(mesh.triangle_count(), 1);
        
        let triangle = &mesh.triangles()[0];
        assert_eq!(triangle.normal, Vertex::new(0.0, 0.0, 1.0));
        assert_eq!(triangle.vertices[0], Vertex::new(0.0, 0.0, 0.0));
        assert_eq!(triangle.vertices[1], Vertex::new(1.0, 0.0, 0.0));
        assert_eq!(triangle.vertices[2], Vertex::new(0.5, 1.0, 0.0));
    }

    #[test]
    fn test_parse_binary_stl_too_short() {
        let short_data = vec![0u8; 50];
        let result = parse_stl_bytes(&short_data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StlError::InvalidFormat(_)));
    }

    #[test]
    fn test_parse_invalid_ascii_stl() {
        let invalid_ascii = b"invalid format";
        let result = parse_stl_bytes(invalid_ascii);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_test_stl_files() -> std::io::Result<()> {
        use std::fs;
        
        // Create temporary directory for test files
        let temp_dir = std::env::temp_dir().join("rtrace_stl_tests");
        fs::create_dir_all(&temp_dir)?;
        
        // Create ASCII STL test file
        let ascii_path = temp_dir.join("test_ascii.stl");
        let mut ascii_file = fs::File::create(&ascii_path)?;
        writeln!(ascii_file, "solid test")?;
        writeln!(ascii_file, "facet normal 0.0 0.0 1.0")?;
        writeln!(ascii_file, "  outer loop")?;
        writeln!(ascii_file, "    vertex 0.0 0.0 0.0")?;
        writeln!(ascii_file, "    vertex 1.0 0.0 0.0")?;
        writeln!(ascii_file, "    vertex 0.5 1.0 0.0")?;
        writeln!(ascii_file, "  endloop")?;
        writeln!(ascii_file, "endfacet")?;
        writeln!(ascii_file, "endsolid test")?;
        
        // Test parsing the ASCII file
        let mesh = parse_stl_file(&ascii_path).expect("Failed to parse ASCII STL file");
        assert_eq!(mesh.triangle_count(), 1);
        
        // Create binary STL test file
        let binary_path = temp_dir.join("test_binary.stl");
        let mut binary_file = fs::File::create(&binary_path)?;
        
        // Write binary STL data
        binary_file.write_all(&vec![0u8; 80])?; // Header
        binary_file.write_all(&1u32.to_le_bytes())?; // Triangle count
        
        // Triangle data
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Normal X
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Normal Y
        binary_file.write_all(&(1.0f32).to_le_bytes())?; // Normal Z
        
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Vertex 1 X
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Vertex 1 Y
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Vertex 1 Z
        
        binary_file.write_all(&(1.0f32).to_le_bytes())?; // Vertex 2 X
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Vertex 2 Y
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Vertex 2 Z
        
        binary_file.write_all(&(0.5f32).to_le_bytes())?; // Vertex 3 X
        binary_file.write_all(&(1.0f32).to_le_bytes())?; // Vertex 3 Y
        binary_file.write_all(&(0.0f32).to_le_bytes())?; // Vertex 3 Z
        
        binary_file.write_all(&0u16.to_le_bytes())?; // Attribute
        
        // Test parsing the binary file
        let mesh = parse_stl_file(&binary_path).expect("Failed to parse binary STL file");
        assert_eq!(mesh.triangle_count(), 1);
        
        // Clean up test files
        fs::remove_dir_all(&temp_dir)?;
        
        Ok(())
    }
}
