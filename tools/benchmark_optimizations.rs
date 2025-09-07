use std::time::Instant;
use rtrace::{Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("STL Rendering Performance Benchmark");
    println!("===================================");
    
    let test_cases = vec![
        ("examples/plus_perspective.json", "Plus STL (77KB)", 400, 300),
        ("examples/espresso_tray_perspective.json", "Espresso Tray STL (1.7MB)", 400, 300),
    ];

    for (scene_file, description, width, height) in test_cases {
        println!("\nTesting: {}", description);
        println!("Scene: {}", scene_file);
        println!("Resolution: {}x{}", width, height);
        
        // Load scene
        let scene = Scene::from_json_file(scene_file)?;
        
        // Test with K-d tree (optimized)
        let renderer_kdtree = Renderer::new(width, height);
        let start = Instant::now();
        let _image = renderer_kdtree.render(&scene)?;
        let kdtree_time = start.elapsed();
        
        // Test without K-d tree (brute force)
        let renderer_brute = Renderer::new_brute_force(width, height);
        let start = Instant::now();
        let _image = renderer_brute.render(&scene)?;
        let brute_time = start.elapsed();
        
        // Calculate speedup
        let speedup = brute_time.as_secs_f64() / kdtree_time.as_secs_f64();
        
        println!("K-d Tree:    {:.2} seconds", kdtree_time.as_secs_f64());
        println!("Brute Force: {:.2} seconds", brute_time.as_secs_f64());
        println!("Speedup:     {:.2}x", speedup);
        println!("{}", "-".repeat(50));
    }
    
    Ok(())
}