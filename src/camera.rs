use nalgebra::Unit;
use crate::scene::{Vec3, Point, Camera as CameraConfig};
use crate::ray::Ray;

/// Orthographic camera implementation
#[derive(Debug)]
pub struct Camera {
    pub origin: Point,
    pub horizontal: Vec3,
    pub vertical: Vec3,
    pub lower_left_corner: Point,
}

impl Camera {
    /// Create a new orthographic camera from configuration
    pub fn from_config(config: &CameraConfig, aspect_ratio: f64) -> Result<Self, String> {
        if config.kind != "ortho" {
            return Err(format!("Unsupported camera type: {}", config.kind));
        }
        
        let origin = Point::new(config.position[0], config.position[1], config.position[2]);
        let target = Point::new(config.target[0], config.target[1], config.target[2]);
        let up = Vec3::new(config.up[0], config.up[1], config.up[2]);
        
        // Calculate camera coordinate system
        let w = Unit::new_normalize(origin - target); // Points away from target
        let u = Unit::new_normalize(up.cross(&w));     // Right vector
        let v = w.cross(&u);                           // Up vector
        
        // Calculate viewport dimensions
        let viewport_height = config.height;
        let viewport_width = config.width.max(viewport_height * aspect_ratio);
        
        // Calculate the horizontal and vertical vectors for the viewport
        let horizontal = viewport_width * u.as_ref();
        let vertical = viewport_height * v;
        
        // Calculate the lower left corner of the viewport
        let lower_left_corner = origin - horizontal/2.0 - vertical/2.0;
        
        Ok(Self {
            origin,
            horizontal,
            vertical,
            lower_left_corner,
        })
    }
    
    /// Generate a ray for the given screen coordinates (u, v are in [0, 1])
    pub fn get_ray(&self, u: f64, v: f64) -> Ray {
        let direction = (self.lower_left_corner + u * self.horizontal + v * self.vertical) - self.origin;
        Ray::new(self.origin, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Camera as CameraConfig;
    
    #[test]
    fn test_camera_creation() {
        let config = CameraConfig::default();
        let camera = Camera::from_config(&config, 16.0/9.0).unwrap();
        
        // Test that we can generate rays
        let ray = camera.get_ray(0.5, 0.5);
        assert_eq!(ray.origin, Point::new(0.0, 0.0, 5.0));
    }
    
    #[test]
    fn test_unsupported_camera_type() {
        let mut config = CameraConfig::default();
        config.kind = "perspective".to_string();
        
        let result = Camera::from_config(&config, 1.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported camera type"));
    }
}