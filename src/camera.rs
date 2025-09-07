use crate::ray::Ray;
use crate::scene::{Camera as CameraConfig, Point, Vec3};
use nalgebra::Unit;

/// Camera implementation supporting both orthographic and perspective projection
#[derive(Debug)]
pub struct Camera {
    pub origin: Point,
    pub horizontal: Vec3,
    pub vertical: Vec3,
    pub lower_left_corner: Point,
    pub view_direction: Unit<Vec3>,
    pub is_perspective: bool,
    pub focal_length: f64,
    // Grid background fields for orthographic cameras
    pub grid_pitch: Option<f64>,
    pub grid_color: Option<crate::scene::Color>,
    pub grid_thickness: Option<f64>,
}

impl Camera {
    /// Create a new camera from configuration (supports both ortho and perspective)
    pub fn from_config(config: &CameraConfig, aspect_ratio: f64) -> Result<Self, String> {
        let origin = Point::new(config.position[0], config.position[1], config.position[2]);
        let target = Point::new(config.target[0], config.target[1], config.target[2]);
        let up = Vec3::new(config.up[0], config.up[1], config.up[2]);

        // Calculate camera coordinate system
        let w = Unit::new_normalize(origin - target); // Points away from target
        let u = Unit::new_normalize(up.cross(&w)); // Right vector
        let v = w.cross(&u); // Up vector
        let view_direction = Unit::new_normalize(-*w.as_ref());

        // Parse grid color if provided
        let grid_color = if let Some(color_str) = &config.grid_color {
            Some(crate::scene::hex_to_color(color_str)?)
        } else {
            None
        };

        match config.kind.as_str() {
            "ortho" => {
                Self::create_orthographic(origin, u, v, w, view_direction, config, aspect_ratio, grid_color)
            }
            "perspective" => {
                Self::create_perspective(origin, u, v, w, view_direction, config, aspect_ratio, grid_color)
            }
            _ => Err(format!("Unsupported camera type: {}", config.kind)),
        }
    }

    /// Create orthographic camera
    fn create_orthographic(
        origin: Point,
        u: Unit<Vec3>,
        v: Vec3,
        _w: Unit<Vec3>,
        view_direction: Unit<Vec3>,
        config: &CameraConfig,
        aspect_ratio: f64,
        grid_color: Option<crate::scene::Color>,
    ) -> Result<Self, String> {
        // Calculate viewport dimensions
        let viewport_height = config.height;
        let viewport_width = config.width.max(viewport_height * aspect_ratio);

        // Calculate the horizontal and vertical vectors for the viewport
        let horizontal = viewport_width * u.as_ref();
        let vertical = viewport_height * v;

        // Calculate the lower left corner of the viewport
        let lower_left_corner = origin - horizontal / 2.0 - vertical / 2.0;

        Ok(Self {
            origin,
            horizontal,
            vertical,
            lower_left_corner,
            view_direction,
            is_perspective: false,
            focal_length: 0.0, // Not used for orthographic
            grid_pitch: config.grid_pitch,
            grid_color,
            grid_thickness: config.grid_thickness,
        })
    }

    /// Create perspective camera
    fn create_perspective(
        origin: Point,
        u: Unit<Vec3>,
        v: Vec3,
        _w: Unit<Vec3>,
        view_direction: Unit<Vec3>,
        config: &CameraConfig,
        aspect_ratio: f64,
        grid_color: Option<crate::scene::Color>,
    ) -> Result<Self, String> {
        // Get field of view, default to 45 degrees if not specified
        let fov = config.fov.unwrap_or(45.0);
        if fov <= 0.0 || fov >= 180.0 {
            return Err("Field of view must be between 0 and 180 degrees".to_string());
        }

        // Set focal length (distance to viewport plane)
        let focal_length = 1.0;

        // Calculate viewport dimensions based on FOV
        let theta = fov.to_radians();
        let half_height = (theta / 2.0).tan();
        let half_width = aspect_ratio * half_height;

        // Scale the viewport by focal length
        let viewport_height = 2.0 * half_height * focal_length;
        let viewport_width = 2.0 * half_width * focal_length;

        // Calculate the horizontal and vertical vectors for the viewport
        let horizontal = viewport_width * u.as_ref();
        let vertical = viewport_height * v;

        // Calculate the lower left corner of the viewport
        // For perspective, this is offset by the focal length from the camera
        let viewport_center = origin + focal_length * view_direction.as_ref();
        let lower_left_corner = viewport_center - horizontal / 2.0 - vertical / 2.0;

        Ok(Self {
            origin,
            horizontal,
            vertical,
            lower_left_corner,
            view_direction,
            is_perspective: true,
            focal_length,
            grid_pitch: config.grid_pitch,
            grid_color,
            grid_thickness: config.grid_thickness,
        })
    }

    /// Generate a ray for the given screen coordinates (u, v are in [0, 1])
    pub fn get_ray(&self, u: f64, v: f64) -> Ray {
        if self.is_perspective {
            // For perspective projection, rays diverge from the camera origin
            let viewport_point = self.lower_left_corner + u * self.horizontal + v * self.vertical;
            let ray_direction = Unit::new_normalize(viewport_point - self.origin);
            Ray::new(self.origin, *ray_direction.as_ref())
        } else {
            // For orthographic projection, all rays are parallel to the view direction
            // The ray origin should be on the viewport plane, not at the camera position
            let viewport_point = self.lower_left_corner + u * self.horizontal + v * self.vertical;
            Ray::new(viewport_point, *self.view_direction.as_ref())
        }
    }

    /// Check if an orthographic camera ray intersects with grid lines
    /// Returns the grid color if the ray hits a grid line, None otherwise
    pub fn get_grid_color(&self, ray: &Ray) -> Option<crate::scene::Color> {
        // Only orthographic cameras support grid backgrounds
        if self.is_perspective {
            return None;
        }

        // Check if grid is configured
        let (grid_pitch, grid_color, grid_thickness) = match (
            self.grid_pitch,
            &self.grid_color,
            self.grid_thickness,
        ) {
            (Some(pitch), Some(color), Some(thickness)) if pitch > 0.0 && thickness > 0.0 => {
                (pitch, color, thickness)
            }
            _ => return None,
        };

        let half_thickness = grid_thickness / 2.0;

        // For orthographic rays, we need to find intersections with the origin planes
        // and check if we're close to grid lines

        // Check intersection with XY plane (z = 0)
        if ray.direction.z.abs() > 1e-10 {
            let t = -ray.origin.z / ray.direction.z;
            if t > 0.0 {
                let intersection_point = ray.origin + t * ray.direction.as_ref();
                let x = intersection_point.x;
                let y = intersection_point.y;

                // Check if we're on a grid line
                let x_mod = (x / grid_pitch).fract().abs();
                let y_mod = (y / grid_pitch).fract().abs();

                let x_grid_dist = (x_mod * grid_pitch).min((1.0 - x_mod) * grid_pitch);
                let y_grid_dist = (y_mod * grid_pitch).min((1.0 - y_mod) * grid_pitch);

                if x_grid_dist <= half_thickness || y_grid_dist <= half_thickness {
                    return Some(*grid_color);
                }
            }
        }

        // Check intersection with XZ plane (y = 0)
        if ray.direction.y.abs() > 1e-10 {
            let t = -ray.origin.y / ray.direction.y;
            if t > 0.0 {
                let intersection_point = ray.origin + t * ray.direction.as_ref();
                let x = intersection_point.x;
                let z = intersection_point.z;

                let x_mod = (x / grid_pitch).fract().abs();
                let z_mod = (z / grid_pitch).fract().abs();

                let x_grid_dist = (x_mod * grid_pitch).min((1.0 - x_mod) * grid_pitch);
                let z_grid_dist = (z_mod * grid_pitch).min((1.0 - z_mod) * grid_pitch);

                if x_grid_dist <= half_thickness || z_grid_dist <= half_thickness {
                    return Some(*grid_color);
                }
            }
        }

        // Check intersection with YZ plane (x = 0)
        if ray.direction.x.abs() > 1e-10 {
            let t = -ray.origin.x / ray.direction.x;
            if t > 0.0 {
                let intersection_point = ray.origin + t * ray.direction.as_ref();
                let y = intersection_point.y;
                let z = intersection_point.z;

                let y_mod = (y / grid_pitch).fract().abs();
                let z_mod = (z / grid_pitch).fract().abs();

                let y_grid_dist = (y_mod * grid_pitch).min((1.0 - y_mod) * grid_pitch);
                let z_grid_dist = (z_mod * grid_pitch).min((1.0 - z_mod) * grid_pitch);

                if y_grid_dist <= half_thickness || z_grid_dist <= half_thickness {
                    return Some(*grid_color);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Camera as CameraConfig;

    #[test]
    fn test_orthographic_camera_creation() {
        let config = CameraConfig::default();
        let camera = Camera::from_config(&config, 16.0 / 9.0).unwrap();

        // Test that we can generate rays
        let ray = camera.get_ray(0.5, 0.5);
        assert_eq!(ray.origin, Point::new(0.0, 0.0, 5.0));
        assert!(!camera.is_perspective);
    }

    #[test]
    fn test_perspective_camera_creation() {
        let mut config = CameraConfig::default();
        config.kind = "perspective".to_string();
        config.fov = Some(45.0);

        let camera = Camera::from_config(&config, 1.0).unwrap();
        assert!(camera.is_perspective);
        assert_eq!(camera.focal_length, 1.0);

        // Test that we can generate rays
        let ray = camera.get_ray(0.5, 0.5);
        // For perspective camera, ray should originate from camera origin
        assert_eq!(ray.origin, Point::new(0.0, 0.0, 5.0));
        // Ray direction should be towards the center of the viewport (roughly -z direction)
        let expected_direction = Vec3::new(0.0, 0.0, -1.0);
        assert!((ray.direction.as_ref() - expected_direction).magnitude() < 1e-10);
    }

    #[test]
    fn test_perspective_camera_default_fov() {
        let mut config = CameraConfig::default();
        config.kind = "perspective".to_string();
        // Don't specify fov, should default to 45 degrees

        let camera = Camera::from_config(&config, 1.0).unwrap();
        assert!(camera.is_perspective);
    }

    #[test]
    fn test_perspective_camera_invalid_fov() {
        let mut config = CameraConfig::default();
        config.kind = "perspective".to_string();
        config.fov = Some(0.0); // Invalid FOV

        let result = Camera::from_config(&config, 1.0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Field of view must be between"));

        config.fov = Some(180.0); // Also invalid
        let result = Camera::from_config(&config, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_camera_type() {
        let mut config = CameraConfig::default();
        config.kind = "fisheye".to_string();

        let result = Camera::from_config(&config, 1.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported camera type"));
    }

    #[test]
    fn test_perspective_ray_divergence() {
        let mut config = CameraConfig::default();
        config.kind = "perspective".to_string();
        config.fov = Some(90.0); // Wide angle for clear divergence

        let camera = Camera::from_config(&config, 1.0).unwrap();

        // Test rays from different screen positions
        let ray_center = camera.get_ray(0.5, 0.5);
        let ray_left = camera.get_ray(0.0, 0.5);
        let ray_right = camera.get_ray(1.0, 0.5);

        // All rays should originate from camera origin
        assert_eq!(ray_center.origin, camera.origin);
        assert_eq!(ray_left.origin, camera.origin);
        assert_eq!(ray_right.origin, camera.origin);

        // Ray directions should be different (diverging)
        assert!((ray_center.direction.as_ref() - ray_left.direction.as_ref()).magnitude() > 1e-6);
        assert!((ray_center.direction.as_ref() - ray_right.direction.as_ref()).magnitude() > 1e-6);
    }

    #[test]
    fn test_orthographic_grid_background() {
        let mut config = CameraConfig::default();
        config.grid_pitch = Some(1.0);
        config.grid_color = Some("#FF0000".to_string()); // Red grid
        config.grid_thickness = Some(0.1);
        
        let camera = Camera::from_config(&config, 1.0).unwrap();
        
        // Test that grid fields are properly set
        assert_eq!(camera.grid_pitch, Some(1.0));
        assert!(camera.grid_color.is_some());
        assert_eq!(camera.grid_thickness, Some(0.1));
        
        // Create a ray that should hit the grid (looking towards a grid line on XY plane)
        // Grid lines are at integer coordinates, so (0.0, 0.3) should hit the x=0 line
        let ray = crate::ray::Ray::new(
            crate::scene::Point::new(0.0, 0.3, 5.0), // Start above the XY plane, on x=0 line
            crate::scene::Vec3::new(0.0, 0.0, -1.0), // Look down towards XY plane
        );
        
        // Check if the ray hits the grid
        let grid_color = camera.get_grid_color(&ray);
        assert!(grid_color.is_some(), "Ray should hit grid line at x=0");
        
        // Create a ray that should also hit the grid (y=1 line)
        let ray_y_grid = crate::ray::Ray::new(
            crate::scene::Point::new(0.3, 1.0, 5.0), // Start above the XY plane, on y=1 line
            crate::scene::Vec3::new(0.0, 0.0, -1.0), 
        );
        
        let grid_color_y = camera.get_grid_color(&ray_y_grid);
        assert!(grid_color_y.is_some(), "Ray should hit grid line at y=1");
        
        // Create a ray that should miss the grid (between lines)
        let ray_miss = crate::ray::Ray::new(
            crate::scene::Point::new(0.3, 0.3, 5.0), // Position away from grid lines
            crate::scene::Vec3::new(0.0, 0.0, -1.0), 
        );
        
        let grid_color_miss = camera.get_grid_color(&ray_miss);
        assert!(grid_color_miss.is_none(), "Ray should miss grid lines");
    }

    #[test]
    fn test_perspective_camera_no_grid() {
        let mut config = CameraConfig::default();
        config.kind = "perspective".to_string();
        config.fov = Some(45.0);
        config.grid_pitch = Some(1.0); // Grid settings should be ignored for perspective
        config.grid_color = Some("#FF0000".to_string());
        config.grid_thickness = Some(0.1);
        
        let camera = Camera::from_config(&config, 1.0).unwrap();
        
        // Create any ray
        let ray = crate::ray::Ray::new(
            crate::scene::Point::new(0.0, 0.0, 5.0),
            crate::scene::Vec3::new(0.0, 0.0, -1.0),
        );
        
        // Grid should not work for perspective cameras
        let grid_color = camera.get_grid_color(&ray);
        assert!(grid_color.is_none(), "Perspective cameras should not support grid backgrounds");
    }
}
