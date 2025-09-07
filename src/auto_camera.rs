use crate::scene::{Camera, Point, Scene, Vec3};

/// Auto camera bounds functionality
/// Generates 4 camera views for a given scene: left, front, top, and perspective
pub struct AutoCamera;

impl AutoCamera {
    /// Generate all four camera configurations for a scene
    pub fn generate_cameras(scene: &Scene) -> Result<AutoCameraResult, String> {
        let bounds = scene
            .compute_finite_bounds()
            .ok_or("Scene has no finite objects to compute bounds")?;

        let (min, max) = bounds;
        let center = Point::new(
            (min.x + max.x) / 2.0,
            (min.y + max.y) / 2.0,
            (min.z + max.z) / 2.0,
        );

        // Calculate scene dimensions with 15% margin
        let size = max - min;
        let margin_factor = 1.15;
        let viewport_width = size.x.max(size.y).max(size.z) * margin_factor;
        let viewport_height = viewport_width; // Square viewports for consistency

        Ok(AutoCameraResult {
            left: Self::generate_left_camera(center, viewport_width, viewport_height),
            front: Self::generate_front_camera(center, viewport_width, viewport_height),
            top: Self::generate_top_camera(center, viewport_width, viewport_height),
            perspective: Self::generate_perspective_camera(center, &size, margin_factor)?,
        })
    }

    /// Generate left camera (viewing from positive Y direction onto origin)
    fn generate_left_camera(center: Point, viewport_width: f64, viewport_height: f64) -> Camera {
        // Maximum extent in Y direction to position camera far enough
        let camera_distance = viewport_width * 2.0; // Far enough to avoid clipping

        Camera {
            kind: "ortho".to_string(),
            position: [center.x, center.y + camera_distance, center.z],
            target: [center.x, center.y, center.z],
            up: [0.0, 0.0, 1.0],
            width: viewport_width,
            height: viewport_height,
            fov: None,
            grid_pitch: None,
            grid_color: None,
            grid_thickness: None,
        }
    }

    /// Generate front camera (viewing from positive X direction onto origin)
    fn generate_front_camera(center: Point, viewport_width: f64, viewport_height: f64) -> Camera {
        // Maximum extent in X direction to position camera far enough
        let camera_distance = viewport_width * 2.0; // Far enough to avoid clipping

        Camera {
            kind: "ortho".to_string(),
            position: [center.x + camera_distance, center.y, center.z],
            target: [center.x, center.y, center.z],
            up: [0.0, 0.0, 1.0],
            width: viewport_width,
            height: viewport_height,
            fov: None,
            grid_pitch: None,
            grid_color: None,
            grid_thickness: None,
        }
    }

    /// Generate top camera (viewing from positive Z direction onto origin)
    fn generate_top_camera(center: Point, viewport_width: f64, viewport_height: f64) -> Camera {
        // Maximum extent in Z direction to position camera far enough
        let camera_distance = viewport_width * 2.0; // Far enough to avoid clipping

        Camera {
            kind: "ortho".to_string(),
            position: [center.x, center.y, center.z + camera_distance],
            target: [center.x, center.y, center.z],
            up: [0.0, 1.0, 0.0],
            width: viewport_width,
            height: viewport_height,
            fov: None,
            grid_pitch: None,
            grid_color: None,
            grid_thickness: None,
        }
    }

    /// Generate perspective camera (located in positive X/Y/Z octant, looking down at 35° angle, 50° FOV)
    fn generate_perspective_camera(
        center: Point,
        size: &Vec3,
        margin_factor: f64,
    ) -> Result<Camera, String> {
        let fov: f64 = 50.0; // Field of view in degrees
        let down_angle: f64 = 35.0; // Angle looking down from horizontal

        // Calculate maximum scene dimension for camera distance calculation
        let max_dimension = size.x.max(size.y).max(size.z) * margin_factor;

        // Calculate camera distance based on FOV to ensure entire scene is visible
        let fov_rad = fov.to_radians();
        let distance_for_fov = max_dimension / (fov_rad / 2.0).tan();

        // Position camera along 45-degree X-Y axis line in positive octant
        let xy_angle = 45.0_f64.to_radians(); // 45 degrees in X-Y plane
        let down_angle_rad = down_angle.to_radians();

        // Calculate position components
        let horizontal_distance = distance_for_fov * down_angle_rad.cos();
        let x = center.x + horizontal_distance * xy_angle.cos();
        let y = center.y + horizontal_distance * xy_angle.sin();
        let z = center.z + distance_for_fov * down_angle_rad.sin();

        Ok(Camera {
            kind: "perspective".to_string(),
            position: [x, y, z],
            target: [center.x, center.y, center.z],
            up: [0.0, 0.0, 1.0],
            width: 1.0,  // Not used for perspective cameras
            height: 1.0, // Not used for perspective cameras
            fov: Some(fov),
            grid_pitch: None,
            grid_color: None,
            grid_thickness: None,
        })
    }
}

/// Result containing all four auto-generated cameras
#[derive(Debug)]
pub struct AutoCameraResult {
    pub left: Camera,
    pub front: Camera,
    pub top: Camera,
    pub perspective: Camera,
}

impl AutoCameraResult {
    /// Convert to a complete scene with cameras array
    pub fn to_cameras_json(&self) -> serde_json::Value {
        serde_json::json!({
            "left": self.left,
            "front": self.front,
            "top": self.top,
            "perspective": self.perspective
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Light, Material, Object, SceneSettings};

    #[test]
    fn test_auto_camera_with_sphere() {
        let sphere = Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
        };

        let scene = Scene {
            camera: Camera::default(), // Will be ignored
            objects: vec![sphere],
            lights: vec![Light {
                position: [2.0, 2.0, 2.0],
                color: "#FFFFFF".to_string(),
                intensity: 1.0,
                diameter: None,
            }],
            scene_settings: SceneSettings::default(),
        };

        let result = AutoCamera::generate_cameras(&scene).unwrap();

        // Test that all cameras point to origin (sphere center)
        assert_eq!(result.left.target, [0.0, 0.0, 0.0]);
        assert_eq!(result.front.target, [0.0, 0.0, 0.0]);
        assert_eq!(result.top.target, [0.0, 0.0, 0.0]);
        assert_eq!(result.perspective.target, [0.0, 0.0, 0.0]);

        // Test that orthographic cameras have proper orientations
        assert!(result.left.position[1] > 0.0); // Positive Y
        assert!(result.front.position[0] > 0.0); // Positive X
        assert!(result.top.position[2] > 0.0); // Positive Z

        // Test that perspective camera is in positive octant
        assert!(result.perspective.position[0] > 0.0);
        assert!(result.perspective.position[1] > 0.0);
        assert!(result.perspective.position[2] > 0.0);

        // Test FOV
        assert_eq!(result.perspective.fov, Some(50.0));
    }

    #[test]
    fn test_auto_camera_with_cube() {
        let cube = Object::Cube {
            center: [1.0, 1.0, 1.0],
            size: [2.0, 2.0, 2.0],
            material: Material::default(),
            transform: None,
        };

        let scene = Scene {
            camera: Camera::default(),
            objects: vec![cube],
            lights: vec![],
            scene_settings: SceneSettings::default(),
        };

        let result = AutoCamera::generate_cameras(&scene).unwrap();

        // Test that all cameras point to cube center
        assert_eq!(result.left.target, [1.0, 1.0, 1.0]);
        assert_eq!(result.front.target, [1.0, 1.0, 1.0]);
        assert_eq!(result.top.target, [1.0, 1.0, 1.0]);
        assert_eq!(result.perspective.target, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_auto_camera_empty_scene() {
        let scene = Scene {
            camera: Camera::default(),
            objects: vec![], // Empty
            lights: vec![],
            scene_settings: SceneSettings::default(),
        };

        let result = AutoCamera::generate_cameras(&scene);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no finite objects"));
    }

    #[test]
    fn test_auto_camera_only_planes() {
        let plane = Object::Plane {
            point: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            material: Material::default(),
            transform: None,
        };

        let scene = Scene {
            camera: Camera::default(),
            objects: vec![plane], // Only planes (infinite bounds)
            lights: vec![],
            scene_settings: SceneSettings::default(),
        };

        let result = AutoCamera::generate_cameras(&scene);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no finite objects"));
    }
}
