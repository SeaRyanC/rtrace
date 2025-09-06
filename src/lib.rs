pub mod auto_camera;
pub mod camera;
pub mod lighting;
pub mod mesh;
pub mod ray;
pub mod renderer;
/// Ray tracing library for rtrace
///
/// This library provides a complete ray tracer with support for:
/// - Orthographic and perspective camera projection
/// - Basic geometric primitives (sphere, plane, cube)
/// - Phong lighting model with ambient lighting
/// - Atmospheric fog
/// - Texture support (grid patterns)
/// - JSON scene description format
/// - Auto camera bounds functionality
pub mod scene;

pub use auto_camera::{AutoCamera, AutoCameraResult};
pub use mesh::{Mesh, Triangle};
pub use renderer::Renderer;
pub use scene::{
    AmbientIllumination, Camera, Fog, Light, Material, Object, Scene, SceneSettings, Texture,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(hello_world(), "hello world");
    }
}
