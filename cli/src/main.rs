use clap::{Parser, Subcommand};
use rtrace::{hello_world, parse_stl_file};
use std::path::PathBuf;

/// A CLI demonstrating the rtrace library features
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Display hello world message
    Hello {
        /// Name to greet
        #[arg(short, long, default_value = "world")]
        name: String,

        /// Number of times to repeat
        #[arg(short, long, default_value_t = 1)]
        count: u8,

        /// Whether to use uppercase
        #[arg(short, long)]
        uppercase: bool,
    },
    /// Parse and display information about an STL file
    Stl {
        /// Path to the STL file
        path: PathBuf,

        /// Show detailed triangle information
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Hello { name, count, uppercase } => {
            let mut message = hello_world();

            if uppercase {
                message = message.to_uppercase();
            }

            for i in 0..count {
                if count > 1 {
                    println!("{}: {}", i + 1, message);
                } else {
                    println!("{}", message);
                }
            }

            if name != "world" {
                println!("(Note: Hello to {} as well!)", name);
            }
        }
        Commands::Stl { path, verbose } => {
            match parse_stl_file(&path) {
                Ok(mesh) => {
                    println!("Successfully parsed STL file: {:?}", path);
                    println!("Triangle count: {}", mesh.triangle_count());
                    
                    if mesh.is_empty() {
                        println!("The mesh is empty.");
                    } else {
                        println!("The mesh contains {} triangles.", mesh.triangle_count());
                        
                        if verbose && mesh.triangle_count() <= 10 {
                            println!("\nTriangle details:");
                            for (i, triangle) in mesh.triangles().iter().enumerate() {
                                println!("  Triangle {}:", i + 1);
                                println!("    Normal: ({:.3}, {:.3}, {:.3})", 
                                    triangle.normal.x, triangle.normal.y, triangle.normal.z);
                                for (j, vertex) in triangle.vertices.iter().enumerate() {
                                    println!("    Vertex {}: ({:.3}, {:.3}, {:.3})", 
                                        j + 1, vertex.x, vertex.y, vertex.z);
                                }
                            }
                        } else if verbose {
                            println!("Too many triangles to display details. Use a smaller file for verbose output.");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing STL file: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
