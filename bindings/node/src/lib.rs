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
    
    // Create renderer with k-d tree enabled and multi-threading
    let renderer = rtrace::Renderer::new(width, height);
    
    // Render and save
    renderer.render_to_file(&scene, &output_path)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to render scene: {}", e)))?;
    
    Ok(format!("Successfully rendered {}x{} image to '{}' (multi-threaded)", width, height, output_path))
}

/// Render a scene from JSON string with specific thread count
#[napi]
pub fn render_scene_threaded(scene_json: String, output_path: String, width: Option<u32>, height: Option<u32>, thread_count: Option<u32>) -> Result<String> {
    let width = width.unwrap_or(800);
    let height = height.unwrap_or(600);
    
    // Parse the JSON scene
    let scene = rtrace::Scene::from_json_str(&scene_json)
        .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to parse scene JSON: {}", e)))?;
    
    // Create renderer with specific thread count
    let renderer = if let Some(threads) = thread_count {
        rtrace::Renderer::new_with_threads(width, height, threads as usize)
    } else {
        rtrace::Renderer::new(width, height)
    };
    
    // Render and save
    renderer.render_to_file(&scene, &output_path)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to render scene: {}", e)))?;
    
    let thread_info = if let Some(threads) = thread_count {
        format!(" with {} threads", threads)
    } else {
        " with all available threads".to_string()
    };
    
    Ok(format!("Successfully rendered {}x{} image to '{}'{}", width, height, output_path, thread_info))
}

/// Render a scene from JSON string with brute force (no k-d tree)
#[napi]
pub fn render_scene_brute_force(scene_json: String, output_path: String, width: Option<u32>, height: Option<u32>) -> Result<String> {
    let width = width.unwrap_or(800);
    let height = height.unwrap_or(600);
    
    // Parse the JSON scene
    let scene = rtrace::Scene::from_json_str(&scene_json)
        .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to parse scene JSON: {}", e)))?;
    
    // Create renderer with k-d tree disabled (brute force)
    let renderer = rtrace::Renderer::new_brute_force(width, height);
    
    // Render and save
    renderer.render_to_file(&scene, &output_path)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to render scene: {}", e)))?;
    
    Ok(format!("Successfully rendered {}x{} image to '{}' (brute force)", width, height, output_path))
}

/// Render a scene from JSON file directly (handles relative paths correctly)
#[napi]
pub fn render_scene_from_file(scene_file_path: String, output_path: String, width: Option<u32>, height: Option<u32>) -> Result<String> {
    let width = width.unwrap_or(800);
    let height = height.unwrap_or(600);
    
    // Load scene from file (handles relative paths)
    let scene = rtrace::Scene::from_json_file(&scene_file_path)
        .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to load scene file: {}", e)))?;
    
    // Create renderer with k-d tree enabled and multi-threading
    let renderer = rtrace::Renderer::new(width, height);
    
    // Render and save
    renderer.render_to_file(&scene, &output_path)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to render scene: {}", e)))?;
    
    Ok(format!("Successfully rendered {}x{} image to '{}' (multi-threaded)", width, height, output_path))
}

/// Render a scene from JSON file with specific thread count
#[napi]
pub fn render_scene_from_file_threaded(scene_file_path: String, output_path: String, width: Option<u32>, height: Option<u32>, thread_count: Option<u32>) -> Result<String> {
    let width = width.unwrap_or(800);
    let height = height.unwrap_or(600);
    
    // Load scene from file (handles relative paths)
    let scene = rtrace::Scene::from_json_file(&scene_file_path)
        .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to load scene file: {}", e)))?;
    
    // Create renderer with specific thread count
    let renderer = if let Some(threads) = thread_count {
        rtrace::Renderer::new_with_threads(width, height, threads as usize)
    } else {
        rtrace::Renderer::new(width, height)
    };
    
    // Render and save
    renderer.render_to_file(&scene, &output_path)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to render scene: {}", e)))?;
    
    let thread_info = if let Some(threads) = thread_count {
        format!(" with {} threads", threads)
    } else {
        " with all available threads".to_string()
    };
    
    Ok(format!("Successfully rendered {}x{} image to '{}'{}", width, height, output_path, thread_info))
}

/// Render a scene from JSON file with brute force (no k-d tree)
#[napi]
pub fn render_scene_from_file_brute_force(scene_file_path: String, output_path: String, width: Option<u32>, height: Option<u32>) -> Result<String> {
    let width = width.unwrap_or(800);
    let height = height.unwrap_or(600);
    
    // Load scene from file (handles relative paths)
    let scene = rtrace::Scene::from_json_file(&scene_file_path)
        .map_err(|e| Error::new(Status::InvalidArg, format!("Failed to load scene file: {}", e)))?;
    
    // Create renderer with k-d tree disabled (brute force)
    let renderer = rtrace::Renderer::new_brute_force(width, height);
    
    // Render and save
    renderer.render_to_file(&scene, &output_path)
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to render scene: {}", e)))?;
    
    Ok(format!("Successfully rendered {}x{} image to '{}' (brute force)", width, height, output_path))
}
