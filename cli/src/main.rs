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

    /// Image width in pixels
    #[arg(short, long, default_value_t = 800)]
    width: u32,

    /// Image height in pixels  
    #[arg(short = 'H', long, default_value_t = 600)]
    height: u32,

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

    // Create renderer
    let mut renderer = Renderer::new(args.width, args.height);
    renderer.max_depth = args.max_depth;
    renderer.samples = samples;
    renderer.anti_aliasing_mode = anti_aliasing_mode;

    println!(
        "Rendering {}x{} image with {} anti-aliasing ({} samples)...",
        args.width, args.height, args.anti_aliasing, samples
    );

    // Render and save
    if let Err(e) = renderer.render_to_file(&scene, &args.output) {
        eprintln!("Error rendering image: {}", e);
        std::process::exit(1);
    }

    println!("Successfully rendered to '{}'", args.output);
}
