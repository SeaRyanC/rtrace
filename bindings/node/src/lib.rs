use napi_derive::napi;
use napi::{Result, Error, Status};

/// Returns a hello world message (Node.js binding)
#[napi]
pub fn hello_world() -> String {
    rtrace::hello_world()
}

/// Advanced function that takes parameters (demonstration)
#[napi]
pub fn greet_with_name(name: String) -> String {
    format!("{}, {}", rtrace::hello_world(), name)
}

/// Render a scene from JSON string directly
#[napi]
pub fn render_scene(scene_json: String, output_path: String, width: Option<u32>, height: Option<u32>) -> Result<String> {
    let width = width.unwrap_or(800);
    let height = height.unwrap_or(600);
    
    // Parse the JSON scene
    let scene = rtrace::Scene::from_json_str(&scene_json)
        .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to parse scene JSON: {}", e)))?;
    
    // Create renderer
    let renderer = rtrace::Renderer::new(width, height);
    
    // Render and save
    renderer.render_to_file(&scene, &output_path)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to render scene: {}", e)))?;
    
    Ok(format!("Successfully rendered {}x{} image to '{}'", width, height, output_path))
}
