pub mod auto_camera;
pub mod camera;
pub mod lighting;
pub mod mesh;
pub mod outline;
pub mod rasterizer;
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
pub use outline::{OutlineBuffers, OutlineConfig};
pub use rasterizer::Rasterizer;
pub use renderer::{AntiAliasingMode, Renderer};
pub use scene::{
    AmbientIllumination, Camera, Fog, Light, Material, Object, Scene, SceneSettings, Texture,
};
