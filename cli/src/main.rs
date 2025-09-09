use clap::Parser;
use rtrace::{AntiAliasingMode, Renderer, Scene};
use std::path::Path;

/// Ray tracer CLI - renders 3D scenes from JSON descriptions
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input JSON scene file
    #[arg(short, long)]
    input: String,

    /// Output PNG image file
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

    /// Anti-aliasing mode: quincunx (default), stochastic, or no-jitter
    #[arg(long, default_value = "quincunx")]
    anti_aliasing: String,
}

fn main() {
    let args = Args::parse();

    // Validate input file exists
    if !Path::new(&args.input).exists() {
        eprintln!("Error: Input file '{}' does not exist", args.input);
        std::process::exit(1);
    }

    // Parse anti-aliasing mode
    let anti_aliasing_mode = match args.anti_aliasing.as_str() {
        "quincunx" => AntiAliasingMode::Quincunx,
        "stochastic" => AntiAliasingMode::Stochastic,
        "no-jitter" => AntiAliasingMode::NoJitter,
        _ => {
            eprintln!("Error: Invalid anti-aliasing mode '{}'. Valid options are: quincunx, stochastic, no-jitter", args.anti_aliasing);
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

    // Create renderer
    let mut renderer = Renderer::new(width, height);
    renderer.max_depth = args.max_depth;
    renderer.samples = samples;
    renderer.anti_aliasing_mode = anti_aliasing_mode;
    renderer.seed = Some(0); // Always use deterministic seed 0

    println!(
        "Rendering {}×{} image (diagonal {}) with {} anti-aliasing ({} samples)...",
        width, height, args.size, args.anti_aliasing, samples
    );

    // Render and save
    if let Err(e) = renderer.render_to_file(&scene, &args.output) {
        eprintln!("Error rendering image: {}", e);
        std::process::exit(1);
    }

    println!("Successfully rendered to '{}'", args.output);
}
