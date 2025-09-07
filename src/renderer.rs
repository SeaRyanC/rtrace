use image::{ImageBuffer, Rgb, RgbImage};
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::camera::Camera;
use crate::lighting::ray_color;
use crate::ray::{Cube, MeshObject, Plane, Sphere, World};
use crate::scene::{hex_to_color, Color, Object, Point, Scene, Vec3};

/// Anti-aliasing sampling modes
#[derive(Debug, Clone, PartialEq)]
pub enum AntiAliasingMode {
    /// No jittering - deterministic center-pixel sampling
    NoJitter,
    /// Quincunx pattern - 5 samples (center + 4 corners) per pixel
    Quincunx,
    /// Stochastic sampling - random jittered sampling
    Stochastic,
}

pub struct Renderer {
    pub width: u32,
    pub height: u32,
    pub max_depth: i32,
    pub use_kdtree: bool, // New field to control k-d tree usage for meshes
    pub thread_count: Option<usize>, // Number of threads to use (None = use all available cores)
    pub samples: u32,     // Number of samples per pixel for stochastic subsampling
    pub anti_aliasing_mode: AntiAliasingMode, // Anti-aliasing sampling mode
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            max_depth: 10,
            use_kdtree: true,   // Default to using k-d tree
            thread_count: None, // Use all available cores by default
            samples: 1,         // Default to 1 sample (quincunx adds shared corner samples)
            anti_aliasing_mode: AntiAliasingMode::Quincunx, // Default to quincunx anti-aliasing
        }
    }

    /// Create a renderer with k-d tree disabled (brute force mesh intersection)
    pub fn new_brute_force(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            max_depth: 10,
            use_kdtree: false,                              // Disable k-d tree
            thread_count: None,                             // Use all available cores by default
            samples: 1,                                     // Default to 1 sample (quincunx adds shared corner samples)
            anti_aliasing_mode: AntiAliasingMode::Quincunx, // Default to quincunx anti-aliasing
        }
    }

    /// Create a renderer with a specific thread count
    pub fn new_with_threads(width: u32, height: u32, thread_count: usize) -> Self {
        Self {
            width,
            height,
            max_depth: 10,
            use_kdtree: true,
            thread_count: Some(thread_count),
            samples: 1, // Default to 1 sample (quincunx adds shared corner samples)
            anti_aliasing_mode: AntiAliasingMode::Quincunx, // Default to quincunx anti-aliasing
        }
    }

    /// Create a renderer with specific thread count and k-d tree settings
    pub fn new_with_options(
        width: u32,
        height: u32,
        use_kdtree: bool,
        thread_count: Option<usize>,
    ) -> Self {
        Self {
            width,
            height,
            max_depth: 10,
            use_kdtree,
            thread_count,
            samples: 1, // Default to 1 sample (quincunx adds shared corner samples)
            anti_aliasing_mode: AntiAliasingMode::Quincunx, // Default to quincunx anti-aliasing
        }
    }

    pub fn render(&self, scene: &Scene) -> Result<RgbImage, Box<dyn std::error::Error>> {
        // Validate samples parameter
        if self.samples == 0 {
            return Err("Samples must be greater than 0".into());
        }

        let render_start_time = Instant::now();

        // Create camera
        let aspect_ratio = self.width as f64 / self.height as f64;
        let camera = Camera::from_config(&scene.camera, aspect_ratio)?;
        let camera_pos = Point::new(
            scene.camera.position[0],
            scene.camera.position[1],
            scene.camera.position[2],
        );

        // Build world with objects
        let mut world = World::new();
        let mut materials = HashMap::new();

        for (index, object) in scene.objects.iter().enumerate() {
            match object {
                Object::Sphere {
                    center,
                    radius,
                    material,
                } => {
                    let center = Point::new(center[0], center[1], center[2]);
                    let color = hex_to_color(&material.color)?;
                    let sphere = Box::new(Sphere {
                        center,
                        radius: *radius,
                        material_color: color,
                        material_index: index,
                    });
                    world.add(sphere);
                    materials.insert(index, material.clone());
                }
                Object::Plane {
                    point,
                    normal,
                    material,
                } => {
                    let point = Point::new(point[0], point[1], point[2]);
                    let normal =
                        nalgebra::Unit::new_normalize(Vec3::new(normal[0], normal[1], normal[2]));
                    let color = hex_to_color(&material.color)?;
                    let plane = Box::new(Plane {
                        point,
                        normal,
                        material_color: color,
                        material_index: index,
                    });
                    world.add(plane);
                    materials.insert(index, material.clone());
                }
                Object::Cube {
                    center,
                    size,
                    material,
                } => {
                    let center = Point::new(center[0], center[1], center[2]);
                    let size = Vec3::new(size[0], size[1], size[2]);
                    let color = hex_to_color(&material.color)?;
                    let cube = Box::new(Cube::new(center, size, color, index));
                    world.add(cube);
                    materials.insert(index, material.clone());
                }
                Object::Mesh {
                    mesh_data,
                    material,
                    ..
                } => {
                    if let Some(mesh) = mesh_data {
                        let color = hex_to_color(&material.color)?;
                        let mesh_object = if self.use_kdtree {
                            Box::new(MeshObject::new(mesh.clone(), color, index))
                        } else {
                            Box::new(MeshObject::new_brute_force(mesh.clone(), color, index))
                        };
                        world.add(mesh_object);
                        materials.insert(index, material.clone());
                    }
                }
            }
        }

        // Get background color
        let background_color = if let Some(bg) = &scene.scene_settings.background_color {
            hex_to_color(bg)?
        } else {
            Color::new(0.0, 0.0, 0.0)
        };

        // Set up thread pool if specific thread count is requested
        if let Some(thread_count) = self.thread_count {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build()
                .map_err(|e| format!("Failed to create thread pool: {}", e))?;

            // Use the thread pool for rendering
            let image_data = pool.install(|| {
                self.render_parallel(
                    &world,
                    &camera,
                    &scene.lights,
                    &scene.scene_settings.ambient_illumination,
                    &scene.scene_settings.fog,
                    &camera_pos,
                    background_color,
                    &materials,
                )
            });

            let total_time = render_start_time.elapsed();
            let image = self.create_image_from_data(image_data);
            println!("Total rendering time: {}", format_duration(total_time.as_secs_f64()));
            Ok(image)
        } else {
            // Use default parallel rendering with all available cores
            let image_data = self.render_parallel(
                &world,
                &camera,
                &scene.lights,
                &scene.scene_settings.ambient_illumination,
                &scene.scene_settings.fog,
                &camera_pos,
                background_color,
                &materials,
            );

            let total_time = render_start_time.elapsed();
            let image = self.create_image_from_data(image_data);
            println!("Total rendering time: {}", format_duration(total_time.as_secs_f64()));
            Ok(image)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_parallel(
        &self,
        world: &World,
        camera: &Camera,
        lights: &[crate::scene::Light],
        ambient: &crate::scene::AmbientIllumination,
        fog: &Option<crate::scene::Fog>,
        camera_pos: &Point,
        background_color: Color,
        materials: &HashMap<usize, crate::scene::Material>,
    ) -> Vec<(u32, u32, Color)> {
        match self.anti_aliasing_mode {
            AntiAliasingMode::Quincunx => {
                self.render_quincunx(world, camera, lights, ambient, fog, camera_pos, background_color, materials)
            }
            _ => {
                self.render_standard(world, camera, lights, ambient, fog, camera_pos, background_color, materials)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_standard(
        &self,
        world: &World,
        camera: &Camera,
        lights: &[crate::scene::Light],
        ambient: &crate::scene::AmbientIllumination,
        fog: &Option<crate::scene::Fog>,
        camera_pos: &Point,
        background_color: Color,
        materials: &HashMap<usize, crate::scene::Material>,
    ) -> Vec<(u32, u32, Color)> {
        // Create a vector of all pixel coordinates
        let pixels: Vec<(u32, u32)> = (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .collect();

        // Progress tracking setup
        let total_pixels = self.width * self.height;
        let progress_step = (total_pixels / 10).max(1);
        let completed_pixels = AtomicUsize::new(0);
        let progress_mutex = Mutex::new(());
        let start_time = Instant::now();

        // Render pixels in parallel
        let results: Vec<(u32, u32, Color)> = pixels
            .par_iter()
            .map(|&(x, y)| {
                // Calculate base pixel coordinates
                let pixel_u = x as f64 / (self.width - 1) as f64;
                let pixel_v = (self.height - 1 - y) as f64 / (self.height - 1) as f64; // Flip Y coordinate

                // Calculate pixel size in UV coordinates
                let pixel_width = 1.0 / (self.width - 1) as f64;
                let pixel_height = 1.0 / (self.height - 1) as f64;

                // Collect samples for this pixel
                let mut total_color = Color::new(0.0, 0.0, 0.0);
                let mut rng = rand::thread_rng();

                for sample in 0..self.samples {
                    let (sample_u, sample_v) = match self.anti_aliasing_mode {
                        AntiAliasingMode::NoJitter => {
                            // No jittering: sample at exact pixel center
                            (pixel_u, pixel_v)
                        }
                        AntiAliasingMode::Stochastic => {
                            if self.samples == 1 {
                                // Single sample with random jitter within pixel bounds
                                let jitter_u = rng.gen::<f64>() - 0.5; // [-0.5, 0.5]
                                let jitter_v = rng.gen::<f64>() - 0.5; // [-0.5, 0.5]
                                (
                                    pixel_u + jitter_u * pixel_width,
                                    pixel_v + jitter_v * pixel_height,
                                )
                            } else {
                                // Multiple samples: radially symmetric pattern with random phase
                                let angle = 2.0 * std::f64::consts::PI * sample as f64
                                    / self.samples as f64;
                                let random_phase = rng.gen::<f64>() * 2.0 * std::f64::consts::PI;
                                let rotated_angle = angle + random_phase;

                                // Use a smaller radius to keep samples within pixel bounds
                                let radius = 0.5 * rng.gen::<f64>(); // Random radius [0, 0.5]
                                let jitter_u = radius * rotated_angle.cos();
                                let jitter_v = radius * rotated_angle.sin();

                                (
                                    pixel_u + jitter_u * pixel_width,
                                    pixel_v + jitter_v * pixel_height,
                                )
                            }
                        }
                        AntiAliasingMode::Quincunx => unreachable!(), // Handled separately
                    };

                    let ray = camera.get_ray(sample_u, sample_v);
                    let sample_color = ray_color(
                        &ray,
                        world,
                        lights,
                        ambient,
                        fog,
                        camera_pos,
                        background_color,
                        materials,
                        self.max_depth,
                    );

                    total_color += sample_color;
                }

                // Average the samples
                let color = total_color / self.samples as f64;

                // Update progress tracking
                let current_completed = completed_pixels.fetch_add(1, Ordering::Relaxed) + 1;
                
                // Print progress periodically with thread-safe output
                if current_completed % progress_step as usize == 0 || current_completed == total_pixels as usize {
                    if let Ok(_guard) = progress_mutex.lock() {
                        let progress = (current_completed as f64 / total_pixels as f64) * 100.0;
                        let elapsed = start_time.elapsed();
                        
                        if current_completed == total_pixels as usize {
                            // Final progress update
                            println!("Rendering: 100.0%");
                        } else if progress > 0.0 {
                            // Calculate estimated time remaining
                            let estimated_total_time = elapsed.as_secs_f64() / (current_completed as f64 / total_pixels as f64);
                            let estimated_remaining = estimated_total_time - elapsed.as_secs_f64();
                            let eta_formatted = format_duration(estimated_remaining);
                            println!("Rendering: {:.1}% (ETA: {})", progress, eta_formatted);
                        }
                    }
                }

                (x, y, color)
            })
            .collect();

        results
    }

    #[allow(clippy::too_many_arguments)]
    fn render_quincunx(
        &self,
        world: &World,
        camera: &Camera,
        lights: &[crate::scene::Light],
        ambient: &crate::scene::AmbientIllumination,
        fog: &Option<crate::scene::Fog>,
        camera_pos: &Point,
        background_color: Color,
        materials: &HashMap<usize, crate::scene::Material>,
    ) -> Vec<(u32, u32, Color)> {
        use std::sync::{Arc, Mutex};
        use std::collections::HashMap as StdHashMap;

        // Pre-compute corner samples that will be shared between pixels
        // Each corner is identified by its grid position
        let corner_cache: Arc<Mutex<StdHashMap<(u32, u32), Color>>> = Arc::new(Mutex::new(StdHashMap::new()));

        // Calculate pixel size in UV coordinates
        let pixel_width = 1.0 / self.width as f64;
        let pixel_height = 1.0 / self.height as f64;

        // Helper function to get corner sample color (with caching)
        let get_corner_sample = |corner_x: u32, corner_y: u32, 
                               corner_cache: Arc<Mutex<StdHashMap<(u32, u32), Color>>>,
                               world: &World, camera: &Camera| -> Color {
            let key = (corner_x, corner_y);
            
            // Check cache first
            {
                let cache = corner_cache.lock().unwrap();
                if let Some(&color) = cache.get(&key) {
                    return color;
                }
            }
            
            // Calculate corner UV coordinates (corners are at pixel boundaries)
            let corner_u = (corner_x as f64 * pixel_width).clamp(0.0, 1.0);
            let corner_v = (1.0 - corner_y as f64 * pixel_height).clamp(0.0, 1.0); // Flip Y coordinate
            
            let ray = camera.get_ray(corner_u, corner_v);
            let color = ray_color(
                &ray,
                world,
                lights,
                ambient,
                fog,
                camera_pos,
                background_color,
                materials,
                self.max_depth,
            );
            
            // Cache the result
            {
                let mut cache = corner_cache.lock().unwrap();
                cache.insert(key, color);
            }
            
            color
        };

        // Create a vector of all pixel coordinates
        let pixels: Vec<(u32, u32)> = (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .collect();

        // Progress tracking setup
        let total_pixels = self.width * self.height;
        let progress_step = (total_pixels / 10).max(1);

        // Render pixels in parallel
        pixels
            .par_iter()
            .enumerate()
            .map(|(pixel_index, &(x, y))| {
                // Calculate center sample coordinates
                let pixel_center_u = (x as f64 + 0.5) * pixel_width;
                let pixel_center_v = 1.0 - (y as f64 + 0.5) * pixel_height; // Flip Y coordinate

                // Center sample
                let center_ray = camera.get_ray(pixel_center_u, pixel_center_v);
                let center_color = ray_color(
                    &center_ray,
                    world,
                    lights,
                    ambient,
                    fog,
                    camera_pos,
                    background_color,
                    materials,
                    self.max_depth,
                );

                // Get corner samples (these are shared between neighboring pixels)
                // Corner positions are at pixel grid intersections
                let corner_colors = [
                    get_corner_sample(x, y, corner_cache.clone(), world, camera),               // Top-left corner
                    get_corner_sample(x + 1, y, corner_cache.clone(), world, camera),         // Top-right corner
                    get_corner_sample(x, y + 1, corner_cache.clone(), world, camera),         // Bottom-left corner
                    get_corner_sample(x + 1, y + 1, corner_cache.clone(), world, camera),     // Bottom-right corner
                ];

                // Average center + 4 corner samples (true quincunx pattern)
                let total_color = center_color + corner_colors[0] + corner_colors[1] + corner_colors[2] + corner_colors[3];
                let color = total_color / 5.0;

                // Print progress periodically (note: this might be out of order due to parallelism)
                if pixel_index % progress_step as usize == 0 {
                    let progress = (pixel_index as f64 / total_pixels as f64) * 100.0;
                    println!("Rendering: {:.1}%", progress);
                }

                (x, y, color)
            })
            .collect()
    }

    fn create_image_from_data(&self, image_data: Vec<(u32, u32, Color)>) -> RgbImage {
        let mut image = ImageBuffer::new(self.width, self.height);

        for (x, y, color) in image_data {
            // Convert to RGB values (0-255)
            let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
            let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
            let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;

            image.put_pixel(x, y, Rgb([r, g, b]));
        }

        image
    }

    pub fn render_to_file(
        &self,
        scene: &Scene,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let image = self.render(scene)?;
        image.save(output_path)?;
        println!("Image saved to: {}", output_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Light, Material, Object, Scene};

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new(800, 600);
        assert_eq!(renderer.width, 800);
        assert_eq!(renderer.height, 600);
        assert_eq!(renderer.thread_count, None);
        assert_eq!(renderer.anti_aliasing_mode, AntiAliasingMode::Quincunx);
        assert_eq!(renderer.samples, 1); // Default for quincunx with shared samples

        // Test with specific thread count
        let renderer_threaded = Renderer::new_with_threads(800, 600, 4);
        assert_eq!(renderer_threaded.thread_count, Some(4));
        assert_eq!(
            renderer_threaded.anti_aliasing_mode,
            AntiAliasingMode::Quincunx
        );
    }

    #[test]
    fn test_simple_render() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
        });

        // Add a light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: None,
        });

        let renderer = Renderer::new(100, 100);
        let result = renderer.render(&scene);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stochastic_sampling() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
        });

        // Add a light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: None,
        });

        // Test with multiple samples
        let mut renderer = Renderer::new(50, 50);
        renderer.anti_aliasing_mode = AntiAliasingMode::Stochastic;
        renderer.samples = 4;
        let result = renderer.render(&scene);
        assert!(result.is_ok());

        // Test with single sample
        renderer.samples = 1;
        let result = renderer.render(&scene);
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_jitter_sampling() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
        });

        // Add a light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: None,
        });

        // Test no-jitter mode with single sample
        let mut renderer = Renderer::new(50, 50);
        renderer.anti_aliasing_mode = AntiAliasingMode::NoJitter;
        renderer.samples = 1;
        let result = renderer.render(&scene);
        assert!(result.is_ok());

        // Test no-jitter mode with multiple samples (should still work)
        renderer.samples = 4;
        let result = renderer.render(&scene);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quincunx_sampling() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
        });

        // Add a light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
        });

        // Test quincunx mode with default samples
        let renderer = Renderer::new(50, 50);
        assert_eq!(renderer.anti_aliasing_mode, AntiAliasingMode::Quincunx);
        assert_eq!(renderer.samples, 1);
        let result = renderer.render(&scene);
        assert!(result.is_ok());

        // Test quincunx mode with custom samples  
        let mut renderer2 = Renderer::new(50, 50);
        renderer2.samples = 4;
        let result = renderer2.render(&scene);
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_samples_error() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
        });

        let mut renderer = Renderer::new(10, 10);
        renderer.samples = 0;
        let result = renderer.render(&scene);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Samples must be greater than 0"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "0s");
        assert_eq!(format_duration(-1.0), "0s");
        assert_eq!(format_duration(5.0), "5s");
        assert_eq!(format_duration(59.0), "59s");
        assert_eq!(format_duration(60.0), "1m");
        assert_eq!(format_duration(65.0), "1m5s");
        assert_eq!(format_duration(125.0), "2m5s");
        assert_eq!(format_duration(3600.0), "1h");
        assert_eq!(format_duration(3665.0), "1h1m");
        assert_eq!(format_duration(7200.0), "2h");
        assert_eq!(format_duration(7325.0), "2h2m");
    }
}

/// Format duration in seconds to a human-readable string (e.g., "3m45s", "1h23m", "45s")
fn format_duration(seconds: f64) -> String {
    if seconds < 0.0 {
        return "0s".to_string();
    }
    
    let total_seconds = seconds.round() as u64;
    
    if total_seconds == 0 {
        return "0s".to_string();
    }
    
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    
    if hours > 0 {
        if minutes > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else if minutes > 0 {
        if secs > 0 {
            format!("{}m{}s", minutes, secs)
        } else {
            format!("{}m", minutes)
        }
    } else {
        format!("{}s", secs)
    }
}
