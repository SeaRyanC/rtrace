use serde::{Deserialize, Serialize};
use nalgebra::{Vector3, Point3};

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
}

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
        let scene: Scene = serde_json::from_str(&content)?;
        Ok(scene)
    }
    
    /// Save scene to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}