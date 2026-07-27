/// Benchmark tool: renders the Espresso Tray scene (35k+ triangle mesh)
/// multiple times and reports average render time.
use rtrace::{Renderer, Scene};
use std::time::Instant;

fn make_scene() -> Scene {
    let json = format!(
        r#"{{"camera":{{"kind":"perspective","position":[200,-200,150],"target":[0,0,40],"up":[0,0,1],"fov":45,"width":10,"height":7.5}},"objects":[{{"kind":"mesh","filename":"examples/Espresso Tray.stl","material":{{"color":"{color_grey}","ambient":0.1,"diffuse":0.8,"specular":0.5,"shininess":32}}}}],"lights":[{{"position":[200,-100,300],"color":"{color_white}","intensity":1.0}}],"scene_settings":{{"ambient_illumination":{{"color":"{color_white}","intensity":0.2}},"background_color":"{color_bg}"}}}}"#,
        color_grey = "#AAAAAA",
        color_white = "#FFFFFF",
        color_bg = "#112233"
    );

    Scene::from_json_str(&json).expect("Failed to parse scene")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runs: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let width = 400u32;
    let height = 300u32;

    println!("=== rtrace render benchmark ===");
    println!("Resolution: {}x{}", width, height);
    println!("Runs: {}", runs);
    println!();

    let scene = make_scene();

    let renderer = Renderer::new(width, height);

    let mut times_ms = Vec::new();

    for i in 0..runs {
        let start = Instant::now();
        let img = renderer.render(&scene)?;
        let elapsed_ms = start.elapsed().as_millis();
        times_ms.push(elapsed_ms);

        // Save first render for pixel-correctness verification
        if i == 0 {
            img.save("target/bench_output.png")?;
        }

        println!("Run {}: {}ms", i + 1, elapsed_ms);
    }

    let avg_ms: u128 = times_ms.iter().sum::<u128>() / times_ms.len() as u128;
    let min_ms = times_ms.iter().min().unwrap();
    let max_ms = times_ms.iter().max().unwrap();

    println!();
    println!("--- Results ---");
    println!("Min:  {}ms", min_ms);
    println!("Max:  {}ms", max_ms);
    println!("Avg:  {}ms", avg_ms);

    Ok(())
}
