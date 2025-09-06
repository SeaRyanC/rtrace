use clap::Parser;
use rtrace::{Scene, AutoCamera};
use std::path::Path;

/// Auto Camera Bounds CLI - generates 4 camera views for a scene
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input JSON scene file
    #[arg(short, long)]
    input: String,

    /// Output JSON file for camera configurations
    #[arg(short, long)]
    output: String,
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

    println!("Loaded scene with {} objects", scene.objects.len());

    // Generate auto cameras
    let cameras = match AutoCamera::generate_cameras(&scene) {
        Ok(cameras) => cameras,
        Err(e) => {
            eprintln!("Error generating auto cameras: {}", e);
            std::process::exit(1);
        }
    };

    // Convert to JSON
    let cameras_json = cameras.to_cameras_json();
    
    // Write to output file
    if let Err(e) = std::fs::write(&args.output, serde_json::to_string_pretty(&cameras_json).unwrap()) {
        eprintln!("Error writing output file: {}", e);
        std::process::exit(1);
    }

    println!("Successfully generated camera configurations to '{}'", args.output);
    
    // Print summary
    if let Some(bounds) = scene.compute_finite_bounds() {
        let (min, max) = bounds;
        let size = max - min;
        println!("Scene bounds: ({:.2}, {:.2}, {:.2}) to ({:.2}, {:.2}, {:.2})", 
                min.x, min.y, min.z, max.x, max.y, max.z);
        println!("Scene dimensions: {:.2} x {:.2} x {:.2}", size.x, size.y, size.z);
        
        let center = min + size / 2.0;
        println!("Scene center: ({:.2}, {:.2}, {:.2})", center.x, center.y, center.z);
    }
}