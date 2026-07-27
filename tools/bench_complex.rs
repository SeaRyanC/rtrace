use rtrace::{Renderer, Scene};
use std::time::Instant;

fn make_scene() -> Scene {
    let json = format!(
        r#"{{"camera":{{"kind":"perspective","position":[200,-200,150],"target":[0,0,40],"up":[0,0,1],"fov":45,"width":10,"height":7.5}},"objects":[{{"kind":"mesh","filename":"examples/Espresso Tray.stl","material":{{"color":"{cg}","ambient":0.1,"diffuse":0.8,"specular":0.5,"shininess":32}}}}],"lights":[{{"position":[200,-100,300],"color":"{cw}","intensity":1.0}},{{"position":[-100,200,200],"color":"{cl}","intensity":0.5}}],"scene_settings":{{"ambient_illumination":{{"color":"{cw}","intensity":0.2}},"background_color":"{cb}"}}}}"#,
        cg = "#AAAAAA", cw = "#FFFFFF", cl = "#AAFFAA", cb = "#112233"
    );
    Scene::from_json_str(&json).expect("parse failed")
}

fn bench(label: &str, width: u32, height: u32, samples: u32) {
    let scene = make_scene();
    let mut renderer = Renderer::new(width, height);
    renderer.samples = samples;
    let start = Instant::now();
    renderer.render(&scene).unwrap();
    let ms = start.elapsed().as_millis();
    println!("{label}: {width}x{height} s={samples} → {ms}ms");
}

fn main() {
    bench("400x300 1s", 400, 300, 1);
    bench("800x600 1s", 800, 600, 1);
    bench("800x600 4s", 800, 600, 4);
    bench("400x300 4s stochastic", 400, 300, 4);
}
