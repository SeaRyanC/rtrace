use serde::{Deserialize, Serialize};
use nalgebra::{Vector3, Point3};
use std::path::{Path, PathBuf};

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
    
    Ok(Color::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0))
}

/// Camera configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Camera {
    pub kind: String, // "ortho" for now
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub width: f64,
    pub height: f64,
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
        path: String, // path to STL file, relative to JSON file
        center: Option<[f64; 3]>, // optional translation
        scale: Option<[f64; 3]>,  // optional scaling
        material: Material,
    },
}

/// Light source
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Light {
    pub position: [f64; 3],
    pub color: String, // hex color
    pub intensity: f64,
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
    #[serde(skip)]
    pub base_path: Option<PathBuf>, // Directory of the JSON file for resolving relative paths
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            objects: Vec::new(),
            lights: Vec::new(),
            scene_settings: SceneSettings::default(),
            base_path: None,
        }
    }
}

impl Scene {
    /// Load scene from JSON file
    pub fn from_json_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let mut scene: Scene = serde_json::from_str(&content)?;
        
        // Extract the directory path from the JSON file path for resolving relative paths
        let json_path = Path::new(path);
        scene.base_path = json_path.parent().map(|p| p.to_path_buf());
        
        Ok(scene)
    }
    
    /// Load scene from JSON string
    pub fn from_json_str(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let scene: Scene = serde_json::from_str(json)?;
        Ok(scene)
    }
    
    /// Save scene to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// Resolve a relative path against the scene's base path
    pub fn resolve_path(&self, relative_path: &str) -> PathBuf {
        if let Some(base_path) = &self.base_path {
            base_path.join(relative_path)
        } else {
            // Fallback to relative to current directory if no base path
            PathBuf::from(relative_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_resolution() {
        // Test loading a scene file with mesh that has relative path
        let scene = Scene::from_json_file("examples/mesh_example.json").expect("Failed to load scene");
        
        // Verify that the base path is extracted correctly
        assert!(scene.base_path.is_some());
        let base_path = scene.base_path.as_ref().unwrap();
        assert!(base_path.ends_with("examples"));
        
        // Test path resolution
        let resolved_path = scene.resolve_path("Espresso Tray.stl");
        assert!(resolved_path.ends_with("examples/Espresso Tray.stl"));
        
        // Test that the mesh object was parsed correctly
        assert_eq!(scene.objects.len(), 1);
        if let Object::Mesh { path, center, scale, material: _ } = &scene.objects[0] {
            assert_eq!(path, "Espresso Tray.stl");
            assert_eq!(center, &Some([0.0, 0.0, 0.0]));
            assert_eq!(scale, &Some([0.1, 0.1, 0.1]));
        } else {
            panic!("Expected mesh object");
        }
    }
    
    #[test]
    fn test_resolve_path_without_base() {
        let mut scene = Scene::default();
        scene.base_path = None;
        
        let resolved = scene.resolve_path("test.stl");
        assert_eq!(resolved, PathBuf::from("test.stl"));
    }
    
    #[test]
    fn test_resolve_path_with_base() {
        let mut scene = Scene::default();
        scene.base_path = Some(PathBuf::from("/path/to/scenes"));
        
        let resolved = scene.resolve_path("models/test.stl");
        assert_eq!(resolved, PathBuf::from("/path/to/scenes/models/test.stl"));
    }
}