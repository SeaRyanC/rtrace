use nalgebra::{Matrix4, Point3, Vector3};
use serde::{Deserialize, Serialize};

/// Color representation as RGB values (0.0-1.0)
pub type Color = Vector3<f64>;

/// 3D point
pub type Point = Point3<f64>;

/// 3D vector
pub type Vec3 = Vector3<f64>;

/// Convert hex color string to Color
pub fn hex_to_color(hex: &str) -> Result<Color, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err("Invalid hex color format".to_string());
    }

    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex color")?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex color")?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex color")?;

    Ok(Color::new(
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
    ))
}

/// Camera configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Camera {
    pub kind: String, // "ortho" or "perspective"
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub width: f64,
    pub height: f64,
    pub fov: Option<f64>, // field of view in degrees for perspective cameras
    // Grid background options for orthographic cameras
    pub grid_pitch: Option<f64>,     // Distance between grid lines
    pub grid_color: Option<String>,  // Hex color for grid lines
    pub grid_thickness: Option<f64>, // Thickness of grid lines
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            kind: "ortho".to_string(),
            position: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            width: 10.0,
            height: 10.0,
            fov: None,
            grid_pitch: None,
            grid_color: None,
            grid_thickness: None,
        }
    }
}

/// Material properties
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Material {
    pub color: String, // hex color
    pub ambient: f64,
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
    pub reflectivity: Option<f64>,
    pub texture: Option<Texture>,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            color: "#FFFFFF".to_string(),
            ambient: 0.1,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
            reflectivity: None,
            texture: None,
        }
    }
}

/// Texture configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum Texture {
    #[serde(rename = "grid")]
    Grid {
        line_color: String, // hex color
        line_width: f64,
        cell_size: f64,
    },
    #[serde(rename = "checkerboard")]
    Checkerboard {
        material_b: Box<Material>, // secondary material for alternate squares
    },
}

/// Transform operation
#[derive(Debug, Clone)]
pub enum Transform {
    Rotate(f64, f64, f64),    // rotation in degrees around x, y, z axes
    Translate(f64, f64, f64), // translation along x, y, z axes
    Scale(f64, f64, f64),     // scaling along x, y, z axes
}

impl Transform {
    /// Parse a transform string like "rotate(0, 0, 180)" or "translate(15, 0, 0)"
    pub fn from_str(s: &str) -> Result<Transform, String> {
        let s = s.trim();

        if let Some(params) = s.strip_prefix("rotate(") {
            let params = params
                .strip_suffix(")")
                .ok_or("Missing closing parenthesis in rotate transform")?;
            let values: Result<Vec<f64>, _> =
                params.split(',').map(|v| v.trim().parse::<f64>()).collect();
            match values
                .map_err(|e| format!("Invalid rotate parameters: {}", e))?
                .as_slice()
            {
                [x, y, z] => Ok(Transform::Rotate(*x, *y, *z)),
                _ => Err("Rotate transform requires exactly 3 parameters (x, y, z)".to_string()),
            }
        } else if let Some(params) = s.strip_prefix("translate(") {
            let params = params
                .strip_suffix(")")
                .ok_or("Missing closing parenthesis in translate transform")?;
            let values: Result<Vec<f64>, _> =
                params.split(',').map(|v| v.trim().parse::<f64>()).collect();
            match values
                .map_err(|e| format!("Invalid translate parameters: {}", e))?
                .as_slice()
            {
                [x, y, z] => Ok(Transform::Translate(*x, *y, *z)),
                _ => Err("Translate transform requires exactly 3 parameters (x, y, z)".to_string()),
            }
        } else if let Some(params) = s.strip_prefix("scale(") {
            let params = params
                .strip_suffix(")")
                .ok_or("Missing closing parenthesis in scale transform")?;
            let values: Result<Vec<f64>, _> =
                params.split(',').map(|v| v.trim().parse::<f64>()).collect();
            match values
                .map_err(|e| format!("Invalid scale parameters: {}", e))?
                .as_slice()
            {
                [x, y, z] => Ok(Transform::Scale(*x, *y, *z)),
                _ => Err("Scale transform requires exactly 3 parameters (x, y, z)".to_string()),
            }
        } else {
            Err(format!(
                "Unknown transform type. Expected rotate(), translate(), or scale(), got: {}",
                s
            ))
        }
    }

    /// Convert this transform to a 4x4 transformation matrix
    pub fn to_matrix(&self) -> Matrix4<f64> {
        match self {
            Transform::Rotate(x_deg, y_deg, z_deg) => {
                // Convert degrees to radians
                let x_rad = x_deg.to_radians();
                let y_rad = y_deg.to_radians();
                let z_rad = z_deg.to_radians();

                // Create rotation matrices for each axis
                let rx = Matrix4::from_euler_angles(x_rad, 0.0, 0.0);
                let ry = Matrix4::from_euler_angles(0.0, y_rad, 0.0);
                let rz = Matrix4::from_euler_angles(0.0, 0.0, z_rad);

                // Apply rotations in order: Z * Y * X (this is the common convention)
                rz * ry * rx
            }
            Transform::Translate(x, y, z) => Matrix4::new_translation(&Vector3::new(*x, *y, *z)),
            Transform::Scale(x, y, z) => Matrix4::new_nonuniform_scaling(&Vector3::new(*x, *y, *z)),
        }
    }
}

/// Parse a list of transform strings and return the combined transformation matrix
pub fn parse_transforms(transform_strings: &[String]) -> Result<Matrix4<f64>, String> {
    let mut combined_matrix = Matrix4::identity();

    // Apply transforms in the order they appear in the array
    // Each transform is multiplied from the left to maintain proper order
    for transform_str in transform_strings {
        let transform = Transform::from_str(transform_str)?;
        let matrix = transform.to_matrix();
        combined_matrix = matrix * combined_matrix;
    }

    Ok(combined_matrix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_parsing() {
        // Test rotate parsing
        let rotate = Transform::from_str("rotate(0, 0, 180)").unwrap();
        match rotate {
            Transform::Rotate(x, y, z) => {
                assert_eq!(x, 0.0);
                assert_eq!(y, 0.0);
                assert_eq!(z, 180.0);
            }
            _ => panic!("Expected Rotate transform"),
        }

        // Test translate parsing
        let translate = Transform::from_str("translate(15, 0, 0)").unwrap();
        match translate {
            Transform::Translate(x, y, z) => {
                assert_eq!(x, 15.0);
                assert_eq!(y, 0.0);
                assert_eq!(z, 0.0);
            }
            _ => panic!("Expected Translate transform"),
        }

        // Test scale parsing
        let scale = Transform::from_str("scale(8, 8, 8)").unwrap();
        match scale {
            Transform::Scale(x, y, z) => {
                assert_eq!(x, 8.0);
                assert_eq!(y, 8.0);
                assert_eq!(z, 8.0);
            }
            _ => panic!("Expected Scale transform"),
        }
    }

    #[test]
    fn test_transform_matrix() {
        // Test identity (no transforms)
        let transforms: Vec<String> = vec![];
        let matrix = parse_transforms(&transforms).unwrap();
        assert_eq!(matrix, Matrix4::identity());

        // Test translation
        let transforms = vec!["translate(1, 2, 3)".to_string()];
        let matrix = parse_transforms(&transforms).unwrap();
        let expected = Matrix4::new_translation(&Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(matrix, expected);
    }

    #[test]
    fn test_transform_error_handling() {
        // Test invalid format
        assert!(Transform::from_str("invalid(1, 2, 3)").is_err());

        // Test missing parenthesis
        assert!(Transform::from_str("rotate(1, 2, 3").is_err());

        // Test invalid parameters
        assert!(Transform::from_str("rotate(a, b, c)").is_err());

        // Test wrong parameter count
        assert!(Transform::from_str("rotate(1, 2)").is_err());
    }

    #[test]
    fn test_complete_transform_scenario() {
        // Test the exact scenario from the issue
        let transforms = vec![
            "rotate(0, 0, 180)".to_string(),
            "translate(15, 0, 0)".to_string(),
            "scale(8, 8, 8)".to_string(),
        ];

        let matrix = parse_transforms(&transforms).unwrap();
        let test_point = Point::new(1.0, 0.0, 0.0);
        let transformed = matrix * test_point.to_homogeneous();
        let result_point = Point::new(transformed.x, transformed.y, transformed.z);

        // The transforms are applied in order:
        // 1. rotate(0, 0, 180): (1, 0, 0) -> (-1, 0, 0)
        // 2. translate(15, 0, 0): (-1, 0, 0) -> (14, 0, 0)
        // 3. scale(8, 8, 8): (14, 0, 0) -> (112, 0, 0)

        assert!(
            (result_point.x - 112.0).abs() < 1e-10,
            "Expected x=112, got x={}",
            result_point.x
        );
        assert!(
            result_point.y.abs() < 1e-10,
            "Expected y=0, got y={}",
            result_point.y
        );
        assert!(
            result_point.z.abs() < 1e-10,
            "Expected z=0, got z={}",
            result_point.z
        );
    }
}

/// Object types in the scene
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "kind")]
pub enum Object {
    #[serde(rename = "sphere")]
    Sphere {
        center: [f64; 3],
        radius: f64,
        material: Material,
        transform: Option<Vec<String>>,
    },
    #[serde(rename = "plane")]
    Plane {
        point: [f64; 3],
        normal: [f64; 3],
        material: Material,
        transform: Option<Vec<String>>,
    },
    #[serde(rename = "cube")]
    Cube {
        center: [f64; 3],
        size: [f64; 3], // width, height, depth
        material: Material,
        transform: Option<Vec<String>>,
    },
    #[serde(rename = "mesh")]
    Mesh {
        filename: String, // path to STL file
        material: Material,
        transform: Option<Vec<String>>,
        #[serde(skip)]
        mesh_data: Option<crate::mesh::Mesh>, // loaded mesh data
    },
}

/// Light source
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Light {
    pub position: [f64; 3],
    pub color: String, // hex color
    pub intensity: f64,
    pub diameter: Option<f64>, // optional diameter for diffuse light sources
}

/// Ambient illumination settings
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AmbientIllumination {
    pub color: String, // hex color
    pub intensity: f64,
}

impl Default for AmbientIllumination {
    fn default() -> Self {
        Self {
            color: "#FFFFFF".to_string(),
            intensity: 0.1,
        }
    }
}

/// Fog settings
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Fog {
    pub color: String, // hex color
    pub density: f64,
    pub start: f64,
    pub end: f64,
}

/// Scene settings
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SceneSettings {
    pub ambient_illumination: AmbientIllumination,
    pub fog: Option<Fog>,
    pub background_color: Option<String>, // hex color
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            ambient_illumination: AmbientIllumination::default(),
            fog: None,
            background_color: Some("#000000".to_string()),
        }
    }
}

/// Complete scene definition
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<Object>,
    pub lights: Vec<Light>,
    pub scene_settings: SceneSettings,
}

#[allow(clippy::derivable_impls)]
impl Default for Scene {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            objects: Vec::new(),
            lights: Vec::new(),
            scene_settings: SceneSettings::default(),
        }
    }
}

impl Scene {
    /// Load scene from JSON file
    pub fn from_json_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let mut scene: Scene = serde_json::from_str(&content)?;

        // Load mesh data for any mesh objects
        scene.load_mesh_data(Some(path))?;

        Ok(scene)
    }

    /// Load scene from JSON string
    pub fn from_json_str(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut scene: Scene = serde_json::from_str(json)?;

        // Load mesh data for any mesh objects (relative to current directory)
        scene.load_mesh_data(None)?;

        Ok(scene)
    }

    /// Load mesh data for all mesh objects in the scene
    pub fn load_mesh_data(
        &mut self,
        scene_file_path: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let base_dir = scene_file_path
            .and_then(|p| std::path::Path::new(p).parent())
            .unwrap_or_else(|| std::path::Path::new("."));

        for object in &mut self.objects {
            if let Object::Mesh {
                filename,
                mesh_data,
                ..
            } = object
            {
                let mesh_path = base_dir.join(filename);
                let mesh = crate::mesh::Mesh::from_stl_file(&mesh_path)?;
                *mesh_data = Some(mesh);
            }
        }

        Ok(())
    }

    /// Save scene to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Compute the bounding box of all finite objects in the scene
    /// Only includes objects with finite bounds (spheres, cubes, meshes) - excludes planes
    pub fn compute_finite_bounds(&self) -> Option<(Point, Point)> {
        let mut min_bound: Option<Point> = None;
        let mut max_bound: Option<Point> = None;

        for object in &self.objects {
            let bounds = match object {
                Object::Sphere {
                    center,
                    radius,
                    transform,
                    ..
                } => {
                    let mut center_point = Point::new(center[0], center[1], center[2]);
                    let mut effective_radius = *radius;

                    // Apply transforms if present
                    if let Some(transform_strings) = transform {
                        if let Ok(transform_matrix) = parse_transforms(transform_strings) {
                            // Transform the center point
                            let center_homogeneous =
                                transform_matrix * center_point.to_homogeneous();
                            center_point = Point::new(
                                center_homogeneous.x,
                                center_homogeneous.y,
                                center_homogeneous.z,
                            );

                            // For radius, we need to consider scaling - use the maximum scale component
                            let scale_x = (transform_matrix.column(0).xyz().magnitude()) as f64;
                            let scale_y = (transform_matrix.column(1).xyz().magnitude()) as f64;
                            let scale_z = (transform_matrix.column(2).xyz().magnitude()) as f64;
                            let max_scale = scale_x.max(scale_y).max(scale_z);
                            effective_radius *= max_scale;
                        }
                    }

                    let r = Vec3::new(effective_radius, effective_radius, effective_radius);
                    Some((center_point - r, center_point + r))
                }
                Object::Cube {
                    center,
                    size,
                    transform,
                    ..
                } => {
                    let mut center_point = Point::new(center[0], center[1], center[2]);
                    let mut effective_size = Vec3::new(size[0], size[1], size[2]);

                    // Apply transforms if present
                    if let Some(transform_strings) = transform {
                        if let Ok(transform_matrix) = parse_transforms(transform_strings) {
                            // Transform the center point
                            let center_homogeneous =
                                transform_matrix * center_point.to_homogeneous();
                            center_point = Point::new(
                                center_homogeneous.x,
                                center_homogeneous.y,
                                center_homogeneous.z,
                            );

                            // For size, we need to consider scaling
                            let scale_x = (transform_matrix.column(0).xyz().magnitude()) as f64;
                            let scale_y = (transform_matrix.column(1).xyz().magnitude()) as f64;
                            let scale_z = (transform_matrix.column(2).xyz().magnitude()) as f64;
                            effective_size.x *= scale_x;
                            effective_size.y *= scale_y;
                            effective_size.z *= scale_z;
                        }
                    }

                    let half_size = effective_size / 2.0;
                    Some((center_point - half_size, center_point + half_size))
                }
                Object::Mesh {
                    mesh_data,
                    transform,
                    ..
                } => {
                    if let Some(mesh) = mesh_data {
                        if let Some(transform_strings) = transform {
                            if let Ok(transform_matrix) = parse_transforms(transform_strings) {
                                // For mesh, we need to transform all vertices to compute bounds
                                // This is a simplified approach - we transform the bounding box corners
                                let (original_min, original_max) = mesh.bounds();

                                // Get all 8 corners of the bounding box
                                let corners = [
                                    Point::new(original_min.x, original_min.y, original_min.z),
                                    Point::new(original_min.x, original_min.y, original_max.z),
                                    Point::new(original_min.x, original_max.y, original_min.z),
                                    Point::new(original_min.x, original_max.y, original_max.z),
                                    Point::new(original_max.x, original_min.y, original_min.z),
                                    Point::new(original_max.x, original_min.y, original_max.z),
                                    Point::new(original_max.x, original_max.y, original_min.z),
                                    Point::new(original_max.x, original_max.y, original_max.z),
                                ];

                                // Transform all corners
                                let transformed_corners: Vec<Point> = corners
                                    .iter()
                                    .map(|corner| {
                                        let transformed =
                                            transform_matrix * corner.to_homogeneous();
                                        Point::new(transformed.x, transformed.y, transformed.z)
                                    })
                                    .collect();

                                // Find the new min and max
                                let mut new_min = transformed_corners[0];
                                let mut new_max = transformed_corners[0];

                                for corner in &transformed_corners[1..] {
                                    new_min.x = new_min.x.min(corner.x);
                                    new_min.y = new_min.y.min(corner.y);
                                    new_min.z = new_min.z.min(corner.z);
                                    new_max.x = new_max.x.max(corner.x);
                                    new_max.y = new_max.y.max(corner.y);
                                    new_max.z = new_max.z.max(corner.z);
                                }

                                Some((new_min, new_max))
                            } else {
                                Some(mesh.bounds())
                            }
                        } else {
                            Some(mesh.bounds())
                        }
                    } else {
                        None
                    }
                }
                Object::Plane { .. } => {
                    // Planes have infinite bounds, so we exclude them
                    None
                }
            };

            if let Some((obj_min, obj_max)) = bounds {
                match (&min_bound, &max_bound) {
                    (None, None) => {
                        min_bound = Some(obj_min);
                        max_bound = Some(obj_max);
                    }
                    (Some(current_min), Some(current_max)) => {
                        min_bound = Some(Point::new(
                            current_min.x.min(obj_min.x),
                            current_min.y.min(obj_min.y),
                            current_min.z.min(obj_min.z),
                        ));
                        max_bound = Some(Point::new(
                            current_max.x.max(obj_max.x),
                            current_max.y.max(obj_max.y),
                            current_max.z.max(obj_max.z),
                        ));
                    }
                    _ => unreachable!(),
                }
            }
        }

        if let (Some(min), Some(max)) = (min_bound, max_bound) {
            Some((min, max))
        } else {
            None
        }
    }
}
