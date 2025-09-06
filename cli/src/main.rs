use clap::Parser;
use rtrace::{Renderer, Scene};
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

    // Create renderer
    let mut renderer = Renderer::new(args.width, args.height);
    renderer.max_depth = args.max_depth;

    println!("Rendering {}x{} image...", args.width, args.height);

    // Render and save
    if let Err(e) = renderer.render_to_file(&scene, &args.output) {
        eprintln!("Error rendering image: {}", e);
        std::process::exit(1);
    }

    println!("Successfully rendered to '{}'", args.output);
}
