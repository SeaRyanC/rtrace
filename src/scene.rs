use nalgebra::{Point3, Vector3};
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
    },
    #[serde(rename = "plane")]
    Plane {
        point: [f64; 3],
        normal: [f64; 3],
        material: Material,
    },
    #[serde(rename = "cube")]
    Cube {
        center: [f64; 3],
        size: [f64; 3], // width, height, depth
        material: Material,
    },
    #[serde(rename = "mesh")]
    Mesh {
        filename: String, // path to STL file
        material: Material,
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
                Object::Sphere { center, radius, .. } => {
                    let r = Vec3::new(*radius, *radius, *radius);
                    let center_point = Point::new(center[0], center[1], center[2]);
                    Some((center_point - r, center_point + r))
                }
                Object::Cube { center, size, .. } => {
                    let center_point = Point::new(center[0], center[1], center[2]);
                    let half_size = Vec3::new(size[0], size[1], size[2]) / 2.0;
                    Some((center_point - half_size, center_point + half_size))
                }
                Object::Mesh { mesh_data, .. } => {
                    mesh_data.as_ref().map(|mesh| mesh.bounds())
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
