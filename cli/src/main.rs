use clap::Parser;
use rtrace::{AntiAliasingMode, Rasterizer, Renderer, Scene};
use std::path::Path;
use std::process::Command;

/// Ray tracer CLI - renders 3D scenes from JSON descriptions
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input JSON scene file
    #[arg(short, long)]
    input: String,

    /// Output file (PNG for single image, WebM for movie)
    #[arg(short, long)]
    output: String,

    /// Image diagonal size in pixels (aspect ratio computed from camera settings)
    #[arg(short, long, default_value_t = 1000)]
    size: u32,

    /// Maximum ray bounces for reflections
    #[arg(long, default_value_t = 10)]
    max_depth: i32,

    /// Number of samples per pixel
    #[arg(long)]
    samples: Option<u32>,

    /// Anti-aliasing mode: quincunx, stochastic, dynamic, or none
    #[arg(long, default_value = "none")]
    anti_aliasing: String,

    /// Minimum samples per pixel for dynamic mode (default: 4)
    #[arg(long)]
    min_samples: Option<u32>,

    /// Maximum samples per pixel for dynamic mode (default: 256)
    #[arg(long)]
    max_samples: Option<u32>,

    /// Target standard-error tolerance for dynamic mode (default: 0.005)
    #[arg(long)]
    tolerance: Option<f64>,

    /// Use rasterization instead of raytracing for fast preview
    #[arg(long)]
    rasterize: bool,

    /// Generate a movie by rotating the scene 360 degrees about the Z axis.
    /// Uses rasterization and outputs a .webm file.
    #[arg(long)]
    movie: bool,
}

/// Number of frames in the movie (one per degree of rotation)
const MOVIE_FRAMES: u32 = 360;

/// Progress update interval (show progress every 10%)
const PROGRESS_INTERVAL: u32 = MOVIE_FRAMES / 10;

/// Render a 360-degree rotation movie of the scene
fn render_movie(scene: &Scene, output_path: &str, width: u32, height: u32) {
    println!("Generating 360-degree rotation movie using rasterization...");

    // Create temporary directory for frames
    let temp_dir = std::env::temp_dir().join("rtrace_movie_frames");
    if temp_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            eprintln!("Warning: Could not clean up temp directory: {}", e);
        }
    }
    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        eprintln!("Error creating temp directory: {}", e);
        std::process::exit(1);
    }

    let rasterizer = Rasterizer::new(width, height);

    // Render 360 frames (one per degree)
    for angle in 0..MOVIE_FRAMES {
        let mut rotated_scene = scene.clone();
        rotated_scene.rotate_objects_z(angle as f64);

        let frame_path = temp_dir.join(format!("frame_{:04}.png", angle));
        let frame_path_str = frame_path
            .to_str()
            .expect("Temporary path should be valid UTF-8");
        if let Err(e) = rasterizer.render_to_file(&rotated_scene, frame_path_str) {
            eprintln!("Error rendering frame {}: {}", angle, e);
            std::process::exit(1);
        }

        // Progress indicator (every 10%)
        if angle % PROGRESS_INTERVAL == 0 {
            println!(
                "Rendered frame {}/{} ({:.0}%)",
                angle,
                MOVIE_FRAMES,
                (angle as f64 / MOVIE_FRAMES as f64) * 100.0
            );
        }
    }
    println!("Rendered frame {0}/{0} (100%)", MOVIE_FRAMES);

    // Use ffmpeg to encode frames into WebM
    println!("Encoding frames to WebM...");

    let frame_pattern = temp_dir.join("frame_%04d.png");
    let frame_pattern_str = frame_pattern
        .to_str()
        .expect("Temporary path should be valid UTF-8");
    let status = Command::new("ffmpeg")
        .args([
            "-y", // Overwrite output file if it exists
            "-framerate",
            "30",
            "-i",
            frame_pattern_str,
            "-c:v",
            "libvpx-vp9",
            "-b:v",
            "2M",
            "-pix_fmt",
            "yuva420p",
            output_path,
        ])
        .status();

    match status {
        Ok(exit_status) if exit_status.success() => {
            println!("Successfully created movie: {}", output_path);
        }
        Ok(exit_status) => {
            eprintln!("ffmpeg exited with status: {}", exit_status);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error running ffmpeg: {}", e);
            eprintln!("Make sure ffmpeg is installed and available in PATH");
            std::process::exit(1);
        }
    }

    // Clean up temp directory
    if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
        eprintln!("Warning: Could not clean up temp directory: {}", e);
    }
}

fn main() {
    let args = Args::parse();

    // Validate input file exists
    if !Path::new(&args.input).exists() {
        eprintln!("Error: Input file '{}' does not exist", args.input);
        std::process::exit(1);
    }

    // Load scene from JSON
    let scene = match Scene::from_json_file(&args.input) {
        Ok(scene) => scene,
        Err(e) => {
            eprintln!("Error loading scene from '{}': {}", args.input, e);
            std::process::exit(1);
        }
    };

    println!(
        "Loaded scene with {} objects and {} lights",
        scene.objects.len(),
        scene.lights.len()
    );

    // Compute pixel dimensions from diagonal size and camera aspect ratio
    let camera_aspect_ratio = scene.camera.width / scene.camera.height;
    let diagonal = args.size as f64;

    // Using diagonal D and aspect ratio R = W/H:
    // H = D / sqrt(R² + 1)
    // W = R * H
    let height_f64 = diagonal / (camera_aspect_ratio * camera_aspect_ratio + 1.0).sqrt();
    let width_f64 = camera_aspect_ratio * height_f64;

    let width = width_f64.round() as u32;
    let height = height_f64.round() as u32;

    println!(
        "Using camera aspect ratio {:.3} to compute {}×{} pixels from diagonal {}",
        camera_aspect_ratio, width, height, args.size
    );

    // Handle movie mode
    if args.movie {
        render_movie(&scene, &args.output, width, height);
        return;
    }

    // Use rasterization if requested
    if args.rasterize {
        println!("Using rasterization mode for fast preview...");

        let rasterizer = Rasterizer::new(width, height);

        if let Err(e) = rasterizer.render_to_file(&scene, &args.output) {
            eprintln!("Error rasterizing image: {}", e);
            std::process::exit(1);
        }

        println!("Successfully rasterized to '{}'", args.output);
        return;
    }

    // Parse anti-aliasing mode
    let anti_aliasing_mode = match args.anti_aliasing.as_str() {
        "quincunx" => AntiAliasingMode::Quincunx,
        "stochastic" => AntiAliasingMode::Stochastic,
        "none" => AntiAliasingMode::None,
        "dynamic" => {
            let min_samples = args.min_samples.unwrap_or(4);
            let max_samples = args.max_samples.unwrap_or(256);
            let tolerance = args.tolerance.unwrap_or(0.005);
            AntiAliasingMode::Dynamic { min_samples, max_samples, tolerance }
        }
        _ => {
            eprintln!("Error: Invalid anti-aliasing mode '{}'. Valid options are: quincunx, stochastic, dynamic, none", args.anti_aliasing);
            std::process::exit(1);
        }
    };

    // Determine sample count based on mode and user input
    let samples = args.samples.unwrap_or(1); // Default to 1 sample for all modes

    // Validate samples parameter
    if samples == 0 {
        eprintln!("Error: Samples must be greater than 0");
        std::process::exit(1);
    }

    // Create renderer
    let mut renderer = Renderer::new(width, height);
    renderer.max_depth = args.max_depth;
    renderer.samples = samples;
    renderer.seed = Some(0); // Always use deterministic seed 0

    // Check if outline detection is enabled and handle anti-aliasing compatibility
    match scene.get_outline_config() {
        Ok(Some(_outline_config)) => {
            println!("Outline detection enabled from scene configuration");

            // Check if current anti-aliasing mode is compatible with outline detection
            if anti_aliasing_mode == AntiAliasingMode::Quincunx {
                println!("Warning: Quincunx anti-aliasing is not compatible with outline detection. Switching to none mode.");
                renderer.anti_aliasing_mode = AntiAliasingMode::None;
            } else {
                renderer.anti_aliasing_mode = anti_aliasing_mode;
            }
        }
        Ok(None) => {
            // No outline detection configured - use original anti-aliasing mode
            renderer.anti_aliasing_mode = anti_aliasing_mode;
        }
        Err(e) => {
            eprintln!("Error: Invalid outline color in scene: {}", e);
            std::process::exit(1);
        }
    }

    let final_anti_aliasing_name = match &renderer.anti_aliasing_mode {
        AntiAliasingMode::Quincunx => "quincunx".to_string(),
        AntiAliasingMode::Stochastic => "stochastic".to_string(),
        AntiAliasingMode::None => "none".to_string(),
        AntiAliasingMode::Dynamic { min_samples, max_samples, tolerance } => {
            format!("dynamic (min={}, max={}, tol={:.4})", min_samples, max_samples, tolerance)
        }
    };

    let samples_desc = match &renderer.anti_aliasing_mode {
        AntiAliasingMode::Dynamic { min_samples, max_samples, .. } => {
            format!("{}-{} adaptive", min_samples, max_samples)
        }
        _ => format!("{}", samples),
    };

    println!(
        "Rendering {}×{} image (diagonal {}) with {} anti-aliasing ({} samples)...",
        width, height, args.size, final_anti_aliasing_name, samples_desc
    );

    // Render and save
    if let Err(e) = renderer.render_to_file(&scene, &args.output) {
        eprintln!("Error rendering image: {}", e);
        std::process::exit(1);
    }

    println!("Successfully rendered to '{}'", args.output);
}
