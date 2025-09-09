use napi::{Error, Result, Status};
use napi_derive::napi;

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
pub fn render_scene(
    scene_json: String,
    output_path: String,
    size: Option<u32>,
) -> Result<String> {
    let diagonal_size = size.unwrap_or(1000);

    // Parse the JSON scene
    let scene = rtrace::Scene::from_json_str(&scene_json).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to parse scene JSON: {}", e),
        )
    })?;

    // Compute pixel dimensions from diagonal size and camera aspect ratio
    let camera_aspect_ratio = scene.camera.width / scene.camera.height;
    let diagonal = diagonal_size as f64;
    
    // Using diagonal D and aspect ratio R = W/H:
    // H = D / sqrt(R² + 1)  
    // W = R * H
    let height_f64 = diagonal / (camera_aspect_ratio * camera_aspect_ratio + 1.0).sqrt();
    let width_f64 = camera_aspect_ratio * height_f64;
    
    let width = width_f64.round() as u32;
    let height = height_f64.round() as u32;

    // Create renderer with k-d tree enabled and multi-threading
    let renderer = rtrace::Renderer::new(width, height);

    // Render and save
    renderer.render_to_file(&scene, &output_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to render scene: {}", e),
        )
    })?;

    Ok(format!(
        "Successfully rendered {}×{} image (diagonal {}) to '{}' (multi-threaded)",
        width, height, diagonal_size, output_path
    ))
}

/// Render a scene from JSON string with specific thread count
#[napi]
pub fn render_scene_threaded(
    scene_json: String,
    output_path: String,
    size: Option<u32>,
    thread_count: Option<u32>,
) -> Result<String> {
    let diagonal_size = size.unwrap_or(1000);

    // Parse the JSON scene
    let scene = rtrace::Scene::from_json_str(&scene_json).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to parse scene JSON: {}", e),
        )
    })?;

    // Compute pixel dimensions from diagonal size and camera aspect ratio
    let camera_aspect_ratio = scene.camera.width / scene.camera.height;
    let diagonal = diagonal_size as f64;
    
    // Using diagonal D and aspect ratio R = W/H:
    // H = D / sqrt(R² + 1)  
    // W = R * H
    let height_f64 = diagonal / (camera_aspect_ratio * camera_aspect_ratio + 1.0).sqrt();
    let width_f64 = camera_aspect_ratio * height_f64;
    
    let width = width_f64.round() as u32;
    let height = height_f64.round() as u32;

    // Create renderer with specific thread count
    let renderer = if let Some(threads) = thread_count {
        rtrace::Renderer::new_with_threads(width, height, threads as usize)
    } else {
        rtrace::Renderer::new(width, height)
    };

    // Render and save
    renderer.render_to_file(&scene, &output_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to render scene: {}", e),
        )
    })?;

    let thread_info = if let Some(threads) = thread_count {
        format!(" with {} threads", threads)
    } else {
        " with all available threads".to_string()
    };

    Ok(format!(
        "Successfully rendered {}×{} image (diagonal {}) to '{}'{}",
        width, height, diagonal_size, output_path, thread_info
    ))
}

/// Render a scene from JSON string with brute force (no k-d tree)
#[napi]
pub fn render_scene_brute_force(
    scene_json: String,
    output_path: String,
    size: Option<u32>,
) -> Result<String> {
    let diagonal_size = size.unwrap_or(1000);

    // Parse the JSON scene
    let scene = rtrace::Scene::from_json_str(&scene_json).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to parse scene JSON: {}", e),
        )
    })?;

    // Compute pixel dimensions from diagonal size and camera aspect ratio
    let camera_aspect_ratio = scene.camera.width / scene.camera.height;
    let diagonal = diagonal_size as f64;
    
    // Using diagonal D and aspect ratio R = W/H:
    // H = D / sqrt(R² + 1)  
    // W = R * H
    let height_f64 = diagonal / (camera_aspect_ratio * camera_aspect_ratio + 1.0).sqrt();
    let width_f64 = camera_aspect_ratio * height_f64;
    
    let width = width_f64.round() as u32;
    let height = height_f64.round() as u32;

    // Create renderer with k-d tree disabled (brute force)
    let renderer = rtrace::Renderer::new_brute_force(width, height);

    // Render and save
    renderer.render_to_file(&scene, &output_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to render scene: {}", e),
        )
    })?;

    Ok(format!(
        "Successfully rendered {}×{} image (diagonal {}) to '{}' (brute force)",
        width, height, diagonal_size, output_path
    ))
}

/// Render a scene from JSON file directly (handles relative paths correctly)
#[napi]
pub fn render_scene_from_file(
    scene_file_path: String,
    output_path: String,
    size: Option<u32>,
) -> Result<String> {
    let diagonal_size = size.unwrap_or(1000);

    // Load scene from file (handles relative paths)
    let scene = rtrace::Scene::from_json_file(&scene_file_path).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to load scene file: {}", e),
        )
    })?;

    // Compute pixel dimensions from diagonal size and camera aspect ratio
    let camera_aspect_ratio = scene.camera.width / scene.camera.height;
    let diagonal = diagonal_size as f64;
    
    // Using diagonal D and aspect ratio R = W/H:
    // H = D / sqrt(R² + 1)  
    // W = R * H
    let height_f64 = diagonal / (camera_aspect_ratio * camera_aspect_ratio + 1.0).sqrt();
    let width_f64 = camera_aspect_ratio * height_f64;
    
    let width = width_f64.round() as u32;
    let height = height_f64.round() as u32;

    // Create renderer with k-d tree enabled and multi-threading
    let renderer = rtrace::Renderer::new(width, height);

    // Render and save
    renderer.render_to_file(&scene, &output_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to render scene: {}", e),
        )
    })?;

    Ok(format!(
        "Successfully rendered {}×{} image (diagonal {}) to '{}' (multi-threaded)",
        width, height, diagonal_size, output_path
    ))
}

/// Render a scene from JSON file with specific thread count
#[napi]
pub fn render_scene_from_file_threaded(
    scene_file_path: String,
    output_path: String,
    size: Option<u32>,
    thread_count: Option<u32>,
) -> Result<String> {
    let diagonal_size = size.unwrap_or(1000);

    // Load scene from file (handles relative paths)
    let scene = rtrace::Scene::from_json_file(&scene_file_path).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to load scene file: {}", e),
        )
    })?;

    // Compute pixel dimensions from diagonal size and camera aspect ratio
    let camera_aspect_ratio = scene.camera.width / scene.camera.height;
    let diagonal = diagonal_size as f64;
    
    // Using diagonal D and aspect ratio R = W/H:
    // H = D / sqrt(R² + 1)  
    // W = R * H
    let height_f64 = diagonal / (camera_aspect_ratio * camera_aspect_ratio + 1.0).sqrt();
    let width_f64 = camera_aspect_ratio * height_f64;
    
    let width = width_f64.round() as u32;
    let height = height_f64.round() as u32;

    // Create renderer with specific thread count
    let renderer = if let Some(threads) = thread_count {
        rtrace::Renderer::new_with_threads(width, height, threads as usize)
    } else {
        rtrace::Renderer::new(width, height)
    };

    // Render and save
    renderer.render_to_file(&scene, &output_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to render scene: {}", e),
        )
    })?;

    let thread_info = if let Some(threads) = thread_count {
        format!(" with {} threads", threads)
    } else {
        " with all available threads".to_string()
    };

    Ok(format!(
        "Successfully rendered {}×{} image (diagonal {}) to '{}'{}",
        width, height, diagonal_size, output_path, thread_info
    ))
}

/// Render a scene from JSON file with brute force (no k-d tree)
#[napi]
pub fn render_scene_from_file_brute_force(
    scene_file_path: String,
    output_path: String,
    size: Option<u32>,
) -> Result<String> {
    let diagonal_size = size.unwrap_or(1000);

    // Load scene from file (handles relative paths)
    let scene = rtrace::Scene::from_json_file(&scene_file_path).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to load scene file: {}", e),
        )
    })?;

    // Compute pixel dimensions from diagonal size and camera aspect ratio
    let camera_aspect_ratio = scene.camera.width / scene.camera.height;
    let diagonal = diagonal_size as f64;
    
    // Using diagonal D and aspect ratio R = W/H:
    // H = D / sqrt(R² + 1)  
    // W = R * H
    let height_f64 = diagonal / (camera_aspect_ratio * camera_aspect_ratio + 1.0).sqrt();
    let width_f64 = camera_aspect_ratio * height_f64;
    
    let width = width_f64.round() as u32;
    let height = height_f64.round() as u32;

    // Create renderer with k-d tree disabled (brute force)
    let renderer = rtrace::Renderer::new_brute_force(width, height);

    // Render and save
    renderer.render_to_file(&scene, &output_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to render scene: {}", e),
        )
    })?;

    Ok(format!(
        "Successfully rendered {}×{} image (diagonal {}) to '{}' (brute force)",
        width, height, diagonal_size, output_path
    ))
}
