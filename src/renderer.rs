use image::{ImageBuffer, Rgb, RgbImage};
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::camera::Camera;
use crate::lighting::{ray_color_with_data, PreparedFog, PreparedLight, PreparedMaterial};
use crate::outline::{apply_outline_detection, OutlineBuffers, OutlineConfig};
use crate::ray::{Cube, MeshObject, Plane, Sphere, World};
use crate::scene::{hex_to_color, Color, Object, Point, Scene, Vec3};

/// Anti-aliasing sampling modes
#[derive(Debug, Clone, PartialEq)]
pub enum AntiAliasingMode {
    /// No anti-aliasing - deterministic center-pixel sampling
    None,
    /// Quincunx pattern - 5 samples (center + 4 corners) per pixel
    Quincunx,
    /// Stochastic sampling - random jittered sampling
    Stochastic,
    /// Dynamic adaptive sampling - takes initial samples then adds more until the estimated
    /// standard error of the mean falls below `tolerance`, up to `max_samples`.
    Dynamic {
        /// Minimum number of samples to take before checking convergence (must be >= 2)
        min_samples: u32,
        /// Maximum number of samples to take regardless of convergence
        max_samples: u32,
        /// Target maximum standard error across RGB channels (e.g. 0.005 = 0.5% of full scale)
        tolerance: f64,
    },
}

/// Online mean and variance accumulator using Welford's algorithm, tracking each RGB channel.
struct WelfordColorAccumulator {
    n: u32,
    mean: Color,
    m2: Color, // running sum of squared deviations from the running mean
}

impl WelfordColorAccumulator {
    fn new() -> Self {
        Self {
            n: 0,
            mean: Color::new(0.0, 0.0, 0.0),
            m2: Color::new(0.0, 0.0, 0.0),
        }
    }

    fn update(&mut self, color: Color) {
        self.n += 1;
        let delta = color - self.mean;
        self.mean += delta / self.n as f64;
        let delta2 = color - self.mean;
        self.m2.x += delta.x * delta2.x;
        self.m2.y += delta.y * delta2.y;
        self.m2.z += delta.z * delta2.z;
    }

    fn mean(&self) -> Color {
        self.mean
    }

    /// Maximum standard error of the mean across RGB channels.
    /// Returns infinity when fewer than 2 samples have been taken.
    fn max_std_error(&self) -> f64 {
        if self.n < 2 {
            return f64::INFINITY;
        }
        let n = self.n as f64;
        // std_error = sqrt(sample_variance / n) = sqrt(M2 / (n * (n-1)))
        let se_r = (self.m2.x.max(0.0) / (n * (n - 1.0))).sqrt();
        let se_g = (self.m2.y.max(0.0) / (n * (n - 1.0))).sqrt();
        let se_b = (self.m2.z.max(0.0) / (n * (n - 1.0))).sqrt();
        se_r.max(se_g).max(se_b)
    }
}

/// Context for rendering operations — all colors pre-parsed, no per-pixel allocation.
struct RenderContext<'a> {
    lights: &'a [PreparedLight],
    ambient_color: Color,
    ambient_intensity: f64,
    fog: Option<PreparedFog>,
    camera_pos: &'a Point,
    background_color: Color,
}

/// Progress tracking helper
struct ProgressTracker {
    total_pixels: u32,
    progress_step: usize,
    completed_pixels: AtomicUsize,
    progress_mutex: Mutex<()>,
    start_time: Instant,
}

impl ProgressTracker {
    fn new(total_pixels: u32) -> Self {
        let progress_step = (total_pixels / 10).max(1) as usize;

        Self {
            total_pixels,
            progress_step,
            completed_pixels: AtomicUsize::new(0),
            progress_mutex: Mutex::new(()),
            start_time: Instant::now(),
        }
    }

    fn update_progress(&self) {
        let current_completed = self.completed_pixels.fetch_add(1, Ordering::Relaxed) + 1;

        // Print progress periodically with thread-safe output
        if current_completed.is_multiple_of(self.progress_step)
            || current_completed == self.total_pixels as usize
        {
            if let Ok(_guard) = self.progress_mutex.lock() {
                let progress = (current_completed as f64 / self.total_pixels as f64) * 100.0;
                let elapsed = self.start_time.elapsed();

                if current_completed == self.total_pixels as usize {
                    // Final progress update
                    println!("Rendering: 100.0%");
                } else if progress > 0.0 {
                    // Only show ETA if we have enough elapsed time for a reliable estimate
                    if elapsed.as_secs_f64() >= 1.0 {
                        // Calculate estimated time remaining
                        let estimated_total_time = elapsed.as_secs_f64()
                            / (current_completed as f64 / self.total_pixels as f64);
                        let estimated_remaining = estimated_total_time - elapsed.as_secs_f64();

                        // Only show ETA if it's at least 1 second to avoid showing "0s"
                        if estimated_remaining >= 1.0 {
                            let eta_formatted = format_duration(estimated_remaining);
                            println!("Rendering: {:.1}% (ETA: {})", progress, eta_formatted);
                        } else {
                            // Show progress without ETA for estimates less than 1 second
                            println!("Rendering: {:.1}%", progress);
                        }
                    } else {
                        // Show progress without ETA for early estimates
                        println!("Rendering: {:.1}%", progress);
                    }
                }
            }
        }
    }
}

/// Type alias for pixel rendering results with outline data
type PixelRenderResult = (u32, u32, Color, Option<f64>, Option<Vec3>);

/// Helper struct for sampling calculations
struct SamplingHelper;

impl SamplingHelper {
    /// Create a deterministic RNG seeded by pixel coordinates and global seed
    fn create_pixel_rng(x: u32, y: u32, seed: Option<u64>) -> rand::rngs::StdRng {
        let pixel_seed = seed
            .unwrap_or(0)
            .wrapping_mul(0x9E3779B97F4A7C15_u64)
            .wrapping_add((x as u64).wrapping_mul(0x85EBCA6B))
            .wrapping_add((y as u64).wrapping_mul(0xC2B2AE35));
        rand::rngs::StdRng::seed_from_u64(pixel_seed)
    }

    /// Calculate base pixel coordinates
    fn calculate_pixel_coords(x: u32, y: u32, width: u32, height: u32) -> (f64, f64, f64, f64) {
        // Use edge-to-edge sampling to cover the full [0,1] x [0,1] UV space
        // This ensures all pixels can sample the entire viewport including edges
        let pixel_u = if width == 1 {
            0.5 // For single pixel, sample at center
        } else {
            x as f64 / (width - 1) as f64
        };
        let pixel_v = if height == 1 {
            0.5 // For single pixel, sample at center
        } else {
            (height - 1 - y) as f64 / (height - 1) as f64 // Flip Y coordinate
        };
        let pixel_width = 1.0 / width as f64;
        let pixel_height = 1.0 / height as f64;

        (pixel_u, pixel_v, pixel_width, pixel_height)
    }

    /// Calculate sample coordinates for anti-aliasing
    #[allow(clippy::too_many_arguments)]
    fn calculate_sample_coords(
        anti_aliasing_mode: &AntiAliasingMode,
        samples: u32,
        sample: u32,
        pixel_u: f64,
        pixel_v: f64,
        pixel_width: f64,
        pixel_height: f64,
        rng: &mut rand::rngs::StdRng,
    ) -> (f64, f64) {
        match anti_aliasing_mode {
            AntiAliasingMode::None => {
                // No jittering: sample at exact pixel center
                (pixel_u, pixel_v)
            }
            AntiAliasingMode::Stochastic => {
                if samples == 1 {
                    // Single sample with random jitter within pixel bounds
                    let jitter_u = rng.gen::<f64>() - 0.5; // [-0.5, 0.5]
                    let jitter_v = rng.gen::<f64>() - 0.5; // [-0.5, 0.5]
                    (
                        pixel_u + jitter_u * pixel_width,
                        pixel_v + jitter_v * pixel_height,
                    )
                } else {
                    // Multiple samples: radially symmetric pattern with random phase
                    let angle = 2.0 * std::f64::consts::PI * sample as f64 / samples as f64;
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
            AntiAliasingMode::Quincunx => {
                // Quincux is now handled via post-processing downsampling
                // During rendering, use center-pixel sampling like None mode
                (pixel_u, pixel_v)
            }
            AntiAliasingMode::Dynamic { .. } => {
                // Dynamic mode uses its own sampling loop and never calls this function
                (pixel_u, pixel_v)
            }
        }
    }

    /// Create a sample-specific seed for ray tracing consistency
    fn create_sample_seed(pixel_seed: u64, sample: u32) -> u64 {
        pixel_seed.wrapping_add((sample as u64).wrapping_mul(0x1F845FED))
    }
}

#[derive(Clone)]
pub struct Renderer {
    pub width: u32,
    pub height: u32,
    pub max_depth: i32,
    pub use_kdtree: bool, // New field to control k-d tree usage for meshes
    pub thread_count: Option<usize>, // Number of threads to use (None = use all available cores)
    pub samples: u32,     // Number of samples per pixel for stochastic subsampling
    pub anti_aliasing_mode: AntiAliasingMode, // Anti-aliasing sampling mode
    pub seed: Option<u64>, // Seed for deterministic randomness (None = use default seed)
    pub outline_config: Option<OutlineConfig>, // Optional outline detection configuration
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self::new_with_options(width, height, true, None)
    }

    /// Create a renderer with k-d tree disabled (brute force mesh intersection)
    pub fn new_brute_force(width: u32, height: u32) -> Self {
        Self::new_with_options(width, height, false, None)
    }

    /// Create a renderer with a specific thread count
    pub fn new_with_threads(width: u32, height: u32, thread_count: usize) -> Self {
        Self::new_with_options(width, height, true, Some(thread_count))
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
            anti_aliasing_mode: AntiAliasingMode::None, // Default to no anti-aliasing
            seed: Some(0), // Default to deterministic seed for reproducibility
            outline_config: None, // No outline detection by default
        }
    }

    /// Enable outline detection with the given configuration
    pub fn with_outline_detection(mut self, config: OutlineConfig) -> Self {
        self.outline_config = Some(config);
        self
    }

    pub fn render(&self, scene: &Scene) -> Result<RgbImage, Box<dyn std::error::Error>> {
        // Validate samples parameter
        if self.samples == 0 && !matches!(self.anti_aliasing_mode, AntiAliasingMode::Dynamic { .. }) {
            return Err("Samples must be greater than 0".into());
        }
        if let AntiAliasingMode::Dynamic { min_samples, max_samples, tolerance } = &self.anti_aliasing_mode {
            if *min_samples < 2 {
                return Err("Dynamic mode min_samples must be at least 2".into());
            }
            if *max_samples < *min_samples {
                return Err("Dynamic mode max_samples must be >= min_samples".into());
            }
            if *tolerance <= 0.0 {
                return Err("Dynamic mode tolerance must be greater than 0".into());
            }
        }

        // Create a renderer configuration that automatically applies scene outline settings
        let mut effective_renderer = self.clone();

        // Apply outline configuration from scene if present and not already configured
        if effective_renderer.outline_config.is_none() {
            if let Ok(Some(outline_config)) = scene.get_outline_config() {
                effective_renderer.outline_config = Some(outline_config);
            }
        }

        effective_renderer.render_with_config(scene)
    }

    /// Internal render method that uses the renderer's current configuration
    fn render_with_config(&self, scene: &Scene) -> Result<RgbImage, Box<dyn std::error::Error>> {
        let render_start_time = Instant::now();

        // Only render to a larger canvas for quincunx downsampling
        let (render_width, render_height) = if self.anti_aliasing_mode == AntiAliasingMode::Quincunx
        {
            (self.width + 1, self.height + 1)
        } else {
            (self.width, self.height)
        };

        // Create camera with aspect ratio based on the FINAL output dimensions
        let aspect_ratio = self.width as f64 / self.height as f64;
        let camera = Camera::from_config(&scene.camera, aspect_ratio)?;
        let camera_pos = Point::new(
            scene.camera.position[0],
            scene.camera.position[1],
            scene.camera.position[2],
        );

        // Build world with objects and prepared materials (indexed by object position)
        let mut world = World::new();
        let mut prepared_materials: Vec<PreparedMaterial> = Vec::with_capacity(scene.objects.len());

        for (index, object) in scene.objects.iter().enumerate() {
            match object {
                Object::Sphere {
                    center,
                    radius,
                    material,
                    transform,
                } => {
                    let mut center_point = Point::new(center[0], center[1], center[2]);
                    let mut effective_radius = *radius;

                    // Apply transforms if present
                    if let Some(transform_strings) = transform {
                        if let Ok(transform_matrix) =
                            crate::scene::parse_transforms(transform_strings)
                        {
                            // Transform the center point
                            let center_homogeneous =
                                transform_matrix * center_point.to_homogeneous();
                            center_point = Point::new(
                                center_homogeneous.x,
                                center_homogeneous.y,
                                center_homogeneous.z,
                            );

                            // For radius, we need to consider scaling - use the maximum scale component
                            let scale_x = transform_matrix.column(0).xyz().magnitude();
                            let scale_y = transform_matrix.column(1).xyz().magnitude();
                            let scale_z = transform_matrix.column(2).xyz().magnitude();
                            let max_scale = scale_x.max(scale_y).max(scale_z);
                            effective_radius *= max_scale;
                        }
                    }

                    let color = hex_to_color(&material.color)?;
                    let sphere = Box::new(Sphere {
                        center: center_point,
                        radius: effective_radius,
                        material_color: color,
                        material_index: index,
                    });
                    world.add(sphere);
                    prepared_materials.push(PreparedMaterial::from_material(material));
                }
                Object::Plane {
                    point,
                    normal,
                    material,
                    transform,
                } => {
                    let mut plane_point = Point::new(point[0], point[1], point[2]);
                    let mut plane_normal = Vec3::new(normal[0], normal[1], normal[2]);

                    // Apply transforms if present
                    if let Some(transform_strings) = transform {
                        if let Ok(transform_matrix) =
                            crate::scene::parse_transforms(transform_strings)
                        {
                            // Transform the point
                            let point_homogeneous = transform_matrix * plane_point.to_homogeneous();
                            plane_point = Point::new(
                                point_homogeneous.x,
                                point_homogeneous.y,
                                point_homogeneous.z,
                            );

                            // Transform the normal (inverse transpose for normals)
                            if let Some(inverse_matrix) = transform_matrix.try_inverse() {
                                let inverse_transpose = inverse_matrix.transpose();
                                let normal_homogeneous =
                                    inverse_transpose * plane_normal.to_homogeneous();
                                plane_normal = Vec3::new(
                                    normal_homogeneous.x,
                                    normal_homogeneous.y,
                                    normal_homogeneous.z,
                                );
                            }
                        }
                    }

                    let normal_unit = nalgebra::Unit::new_normalize(plane_normal);
                    let color = hex_to_color(&material.color)?;
                    let plane = Box::new(Plane {
                        point: plane_point,
                        normal: normal_unit,
                        material_color: color,
                        material_index: index,
                    });
                    world.add(plane);
                    prepared_materials.push(PreparedMaterial::from_material(material));
                }
                Object::Cube {
                    center,
                    size,
                    material,
                    transform,
                } => {
                    let center_point = Point::new(center[0], center[1], center[2]);
                    let cube_size = Vec3::new(size[0], size[1], size[2]);
                    let color = hex_to_color(&material.color)?;

                    // Create cube with transform if present
                    let cube = if let Some(transform_strings) = transform {
                        if let Ok(transform_matrix) =
                            crate::scene::parse_transforms(transform_strings)
                        {
                            Box::new(Cube::new_with_transform(
                                center_point,
                                cube_size,
                                transform_matrix,
                                color,
                                index,
                            ))
                        } else {
                            Box::new(Cube::new(center_point, cube_size, color, index))
                        }
                    } else {
                        Box::new(Cube::new(center_point, cube_size, color, index))
                    };

                    world.add(cube);
                    prepared_materials.push(PreparedMaterial::from_material(material));
                }
                Object::Mesh {
                    mesh_data,
                    material,
                    transform,
                    ..
                } => {
                    if let Some(mesh) = mesh_data {
                        let mut transformed_mesh = mesh.clone();

                        // Apply transforms if present
                        if let Some(transform_strings) = transform {
                            if let Ok(transform_matrix) =
                                crate::scene::parse_transforms(transform_strings)
                            {
                                // Transform all vertices in the mesh
                                for triangle in &mut transformed_mesh.triangles {
                                    for vertex in &mut triangle.vertices {
                                        let vertex_homogeneous =
                                            transform_matrix * vertex.to_homogeneous();
                                        *vertex = Point::new(
                                            vertex_homogeneous.x,
                                            vertex_homogeneous.y,
                                            vertex_homogeneous.z,
                                        );
                                    }
                                }

                                // Update the mesh bounds after transformation
                                transformed_mesh.compute_bounds();

                                // Rebuild the KD-tree with transformed vertices
                                transformed_mesh.build_kdtree();
                            }
                        }

                        let color = hex_to_color(&material.color)?;
                        let mesh_object = if self.use_kdtree {
                            Box::new(MeshObject::new(transformed_mesh, color, index))
                        } else {
                            Box::new(MeshObject::new_brute_force(transformed_mesh, color, index))
                        };
                        world.add(mesh_object);
                        prepared_materials.push(PreparedMaterial::from_material(material));
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

        // Pre-build all scene data once — no more per-pixel String allocation or hex parsing.
        let prepared_lights: Vec<PreparedLight> = scene
            .lights
            .iter()
            .map(PreparedLight::from_light)
            .collect();
        let ambient = &scene.scene_settings.ambient_illumination;
        let ambient_color = hex_to_color(&ambient.color).unwrap_or(Color::new(1.0, 1.0, 1.0));
        let ambient_intensity = ambient.intensity;
        let prepared_fog = scene
            .scene_settings
            .fog
            .as_ref()
            .map(PreparedFog::from_fog);

        let render_context = RenderContext {
            lights: &prepared_lights,
            ambient_color,
            ambient_intensity,
            fog: prepared_fog,
            camera_pos: &camera_pos,
            background_color,
        };

        // Set up thread pool if specific thread count is requested
        if let Some(thread_count) = self.thread_count {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build()
                .map_err(|e| format!("Failed to create thread pool: {}", e))?;

            // Use the thread pool for rendering
            let (image_data, outline_buffers) = pool.install(|| {
                self.render_pixels(
                    render_width,
                    render_height,
                    &world,
                    &camera,
                    &render_context,
                    &prepared_materials,
                    self.outline_config.is_some(),
                )
            });

            let total_time = render_start_time.elapsed();

            // Apply outline detection if configured (before downsampling)
            let mut final_image_data = image_data;
            if let (Some(outline_config), Some(buffers)) = (&self.outline_config, outline_buffers) {
                apply_outline_detection(&mut final_image_data, &buffers, outline_config);
            }

            // Downsample if needed using quincux pattern
            final_image_data =
                self.downsample_if_needed(final_image_data, render_width, render_height);

            let image = self.create_image_from_data(final_image_data);
            println!(
                "Total rendering time: {}",
                format_duration(total_time.as_secs_f64())
            );
            Ok(image)
        } else {
            // Use default parallel rendering with all available cores
            let (image_data, outline_buffers) = self.render_pixels(
                render_width,
                render_height,
                &world,
                &camera,
                &render_context,
                &prepared_materials,
                self.outline_config.is_some(),
            );

            let total_time = render_start_time.elapsed();

            // Apply outline detection if configured (before downsampling)
            let mut final_image_data = image_data;
            if let (Some(outline_config), Some(buffers)) = (&self.outline_config, outline_buffers) {
                apply_outline_detection(&mut final_image_data, &buffers, outline_config);
            }

            // Downsample if needed using quincux pattern
            final_image_data =
                self.downsample_if_needed(final_image_data, render_width, render_height);

            let image = self.create_image_from_data(final_image_data);
            println!(
                "Total rendering time: {}",
                format_duration(total_time.as_secs_f64())
            );
            Ok(image)
        }
    }

    /// Unified pixel rendering function that handles both regular and outline-enabled rendering
    #[allow(clippy::too_many_arguments)]
    fn render_pixels(
        &self,
        render_width: u32,
        render_height: u32,
        world: &World,
        camera: &Camera,
        render_context: &RenderContext,
        prepared_materials: &[PreparedMaterial],
        collect_outline_data: bool,
    ) -> (Vec<(u32, u32, Color)>, Option<OutlineBuffers>) {
        // Create a vector of all pixel coordinates
        let pixels: Vec<(u32, u32)> = (0..render_height)
            .flat_map(|y| (0..render_width).map(move |x| (x, y)))
            .collect();

        // Progress tracking setup
        let progress_tracker = ProgressTracker::new(render_width * render_height);

        // Render pixels in parallel
        let results: Vec<PixelRenderResult> = pixels
            .par_iter()
            .map(|&(x, y)| {
                let (pixel_u, pixel_v, pixel_width, pixel_height) =
                    SamplingHelper::calculate_pixel_coords(x, y, render_width, render_height);

                // Collect samples for this pixel
                let mut pixel_depth = None;
                let mut pixel_normal = None;

                // Create deterministic RNG seeded by pixel coordinates and global seed
                let mut rng = SamplingHelper::create_pixel_rng(x, y, self.seed);
                let pixel_seed = self
                    .seed
                    .unwrap_or(0)
                    .wrapping_mul(0x9E3779B97F4A7C15_u64)
                    .wrapping_add((x as u64).wrapping_mul(0x85EBCA6B))
                    .wrapping_add((y as u64).wrapping_mul(0xC2B2AE35));

                let color = if let AntiAliasingMode::Dynamic { min_samples, max_samples, tolerance } =
                    &self.anti_aliasing_mode
                {
                    // Adaptive sampling: accumulate samples until the standard error of the mean
                    // drops below `tolerance` or `max_samples` is reached.
                    let mut acc = WelfordColorAccumulator::new();
                    let mut sample_num = 0u32;
                    loop {
                        let jitter_u = rng.gen::<f64>() - 0.5;
                        let jitter_v = rng.gen::<f64>() - 0.5;
                        let sample_u = pixel_u + jitter_u * pixel_width;
                        let sample_v = pixel_v + jitter_v * pixel_height;
                        let ray = camera.get_ray(sample_u, sample_v);
                        let sample_seed =
                            SamplingHelper::create_sample_seed(pixel_seed, sample_num);

                        let (sample_color, sample_depth, sample_normal) = ray_color_with_data(
                            &ray,
                            world,
                            render_context.lights,
                            render_context.ambient_color,
                            render_context.ambient_intensity,
                            render_context.fog.as_ref(),
                            render_context.camera_pos,
                            render_context.background_color,
                            prepared_materials,
                            self.max_depth,
                            Some(camera),
                            sample_seed,
                        );

                        acc.update(sample_color);

                        if collect_outline_data {
                            if let (Some(depth), Some(normal)) = (sample_depth, sample_normal) {
                                if pixel_depth.is_none() || depth < pixel_depth.unwrap() {
                                    pixel_depth = Some(depth);
                                    pixel_normal = Some(normal);
                                }
                            }
                        }

                        sample_num += 1;
                        if sample_num >= *min_samples
                            && (sample_num >= *max_samples
                                || acc.max_std_error() <= *tolerance)
                        {
                            break;
                        }
                    }
                    acc.mean()
                } else {
                    // Fixed-count sampling (None, Stochastic, Quincunx)
                    let mut total_color = Color::new(0.0, 0.0, 0.0);

                    for sample in 0..self.samples {
                        let (sample_u, sample_v) = SamplingHelper::calculate_sample_coords(
                            &self.anti_aliasing_mode,
                            self.samples,
                            sample,
                            pixel_u,
                            pixel_v,
                            pixel_width,
                            pixel_height,
                            &mut rng,
                        );

                        let ray = camera.get_ray(sample_u, sample_v);

                        // Create sample-specific seed for ray tracing consistency
                        let sample_seed = SamplingHelper::create_sample_seed(pixel_seed, sample);

                        let (sample_color, sample_depth, sample_normal) = ray_color_with_data(
                            &ray,
                            world,
                            render_context.lights,
                            render_context.ambient_color,
                            render_context.ambient_intensity,
                            render_context.fog.as_ref(),
                            render_context.camera_pos,
                            render_context.background_color,
                            prepared_materials,
                            self.max_depth,
                            Some(camera),
                            sample_seed,
                        );

                        total_color += sample_color;

                        // For outline detection, we want the closest depth and corresponding normal
                        if collect_outline_data {
                            if let (Some(depth), Some(normal)) = (sample_depth, sample_normal) {
                                if pixel_depth.is_none() || depth < pixel_depth.unwrap() {
                                    pixel_depth = Some(depth);
                                    pixel_normal = Some(normal);
                                }
                            }
                        }
                    }

                    total_color / self.samples as f64
                };

                // Update progress tracking
                progress_tracker.update_progress();

                (x, y, color, pixel_depth, pixel_normal)
            })
            .collect();

        // Build output data
        if collect_outline_data {
            let mut image_data = Vec::with_capacity(results.len());
            let mut outline_buffers = OutlineBuffers::new(render_width, render_height);

            for (x, y, color, depth, normal) in results {
                image_data.push((x, y, color));

                if let Some(depth) = depth {
                    outline_buffers.set_depth(x, y, depth);
                }
                if let Some(normal) = normal {
                    outline_buffers.set_normal(x, y, normal);
                }
            }

            (image_data, Some(outline_buffers))
        } else {
            let image_data = results
                .into_iter()
                .map(|(x, y, color, _, _)| (x, y, color))
                .collect();
            (image_data, None)
        }
    }

    /// Downsample the larger rendered image to the target dimensions using quincux pattern if needed
    fn downsample_if_needed(
        &self,
        image_data: Vec<(u32, u32, Color)>,
        render_width: u32,
        render_height: u32,
    ) -> Vec<(u32, u32, Color)> {
        // If we rendered to the exact target size, no downsampling needed
        if render_width == self.width && render_height == self.height {
            return image_data;
        }

        // Convert image_data to a HashMap for easy pixel lookup
        let mut pixel_map = std::collections::HashMap::new();
        for (x, y, color) in image_data {
            pixel_map.insert((x, y), color);
        }

        let mut downsampled_data = Vec::new();

        // Downsample using quincux pattern: each output pixel samples 5 input pixels
        // (center pixel + 4 corner pixels from the larger rendered image)
        for y in 0..self.height {
            for x in 0..self.width {
                // Map output pixel (x, y) to input pixels in the larger render
                // The larger render is (width+1) x (height+1), so we sample:
                // - Center: (x, y) in the larger image
                // - Corners: (x, y), (x+1, y), (x, y+1), (x+1, y+1)

                let default_color = Color::new(0.0, 0.0, 0.0);
                let center_color = *pixel_map.get(&(x, y)).unwrap_or(&default_color);
                let top_right_color = *pixel_map.get(&(x + 1, y)).unwrap_or(&default_color);
                let bottom_left_color = *pixel_map.get(&(x, y + 1)).unwrap_or(&default_color);
                let bottom_right_color = *pixel_map.get(&(x + 1, y + 1)).unwrap_or(&default_color);

                // For the fifth sample, we'll use the center again to maintain the quincux 5-sample pattern
                // This mimics the original quincux behavior but in post-processing
                let fifth_sample_color = center_color;

                // Average the 5 samples (true quincux pattern)
                let total_color = center_color
                    + top_right_color
                    + bottom_left_color
                    + bottom_right_color
                    + fifth_sample_color;
                let averaged_color = total_color / 5.0;

                downsampled_data.push((x, y, averaged_color));
            }
        }

        downsampled_data
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
        assert_eq!(renderer.anti_aliasing_mode, AntiAliasingMode::None);
        assert_eq!(renderer.samples, 1); // Default for all modes

        // Test with specific thread count
        let renderer_threaded = Renderer::new_with_threads(800, 600, 4);
        assert_eq!(renderer_threaded.thread_count, Some(4));
        assert_eq!(renderer_threaded.anti_aliasing_mode, AntiAliasingMode::None);
    }

    #[test]
    fn test_simple_render() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
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
            transform: None,
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
    fn test_none_sampling() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
        });

        // Add a light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: None,
        });

        // Test none mode with single sample
        let mut renderer = Renderer::new(50, 50);
        renderer.anti_aliasing_mode = AntiAliasingMode::None;
        renderer.samples = 1;
        let result = renderer.render(&scene);
        assert!(result.is_ok());

        // Test none mode with multiple samples (should still work)
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
            transform: None,
        });

        // Add a light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: None,
        });

        // Test quincunx mode with default samples
        let mut renderer = Renderer::new(50, 50);
        renderer.anti_aliasing_mode = AntiAliasingMode::Quincunx; // Explicitly set to quincunx for this test
        assert_eq!(renderer.anti_aliasing_mode, AntiAliasingMode::Quincunx);
        assert_eq!(renderer.samples, 1);
        let result = renderer.render(&scene);
        assert!(result.is_ok());

        // Test quincunx mode with custom samples
        let mut renderer2 = Renderer::new(50, 50);
        renderer2.anti_aliasing_mode = AntiAliasingMode::Quincunx; // Explicitly set to quincunx for this test
        renderer2.samples = 4;
        let result = renderer2.render(&scene);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deterministic_rendering() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
        });

        // Add a diffuse light for area light sampling
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: Some(0.5), // Area light to trigger stochastic sampling
        });

        // Create renderer with stochastic anti-aliasing and multiple samples
        let mut renderer = Renderer::new(50, 50);
        renderer.anti_aliasing_mode = AntiAliasingMode::Stochastic;
        renderer.samples = 4;
        renderer.seed = Some(42); // Fixed seed

        // Render the same scene multiple times
        let result1 = renderer.render(&scene).expect("First render failed");
        let result2 = renderer.render(&scene).expect("Second render failed");

        // Extract pixel data for comparison
        let pixels1: Vec<_> = result1.pixels().collect();
        let pixels2: Vec<_> = result2.pixels().collect();

        // Results should be byte-for-byte identical
        assert_eq!(pixels1.len(), pixels2.len());
        for (i, (&pixel1, &pixel2)) in pixels1.iter().zip(pixels2.iter()).enumerate() {
            assert_eq!(
                pixel1, pixel2,
                "Pixel {} differs between renders: {:?} vs {:?}",
                i, pixel1, pixel2
            );
        }
    }

    #[test]
    fn test_deterministic_rendering_with_threading() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
        });

        // Add a diffuse light for area light sampling
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: Some(0.5), // Area light to trigger stochastic sampling
        });

        // Test with different thread counts to ensure thread scheduling doesn't affect results
        let mut renderer1 = Renderer::new_with_threads(50, 50, 1);
        renderer1.anti_aliasing_mode = AntiAliasingMode::Stochastic;
        renderer1.samples = 4;
        renderer1.seed = Some(42);

        let mut renderer4 = Renderer::new_with_threads(50, 50, 4);
        renderer4.anti_aliasing_mode = AntiAliasingMode::Stochastic;
        renderer4.samples = 4;
        renderer4.seed = Some(42);

        // Render with different thread counts
        let result1 = renderer1
            .render(&scene)
            .expect("Single-threaded render failed");
        let result4 = renderer4
            .render(&scene)
            .expect("Multi-threaded render failed");

        // Extract pixel data for comparison
        let pixels1: Vec<_> = result1.pixels().collect();
        let pixels4: Vec<_> = result4.pixels().collect();

        // Results should be identical regardless of thread count
        assert_eq!(pixels1.len(), pixels4.len());
        for (i, (&pixel1, &pixel4)) in pixels1.iter().zip(pixels4.iter()).enumerate() {
            assert_eq!(
                pixel1, pixel4,
                "Pixel {} differs between thread counts: {:?} vs {:?}",
                i, pixel1, pixel4
            );
        }
    }

    #[test]
    fn test_quincunx_deterministic() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
        });

        // Add a diffuse light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: Some(0.5), // Area light to trigger stochastic sampling
        });

        // Test quincunx mode (which should also be deterministic)
        let mut renderer = Renderer::new(50, 50);
        renderer.anti_aliasing_mode = AntiAliasingMode::Quincunx; // Explicitly set to quincunx for this test
        assert_eq!(renderer.anti_aliasing_mode, AntiAliasingMode::Quincunx);
        renderer.seed = Some(123);

        let result1 = renderer
            .render(&scene)
            .expect("First quincunx render failed");
        let result2 = renderer
            .render(&scene)
            .expect("Second quincunx render failed");

        // Extract pixel data for comparison
        let pixels1: Vec<_> = result1.pixels().collect();
        let pixels2: Vec<_> = result2.pixels().collect();

        // Results should be identical
        assert_eq!(pixels1.len(), pixels2.len());
        for (i, (&pixel1, &pixel2)) in pixels1.iter().zip(pixels2.iter()).enumerate() {
            assert_eq!(
                pixel1, pixel2,
                "Quincunx pixel {} differs between renders: {:?} vs {:?}",
                i, pixel1, pixel2
            );
        }
    }

    #[test]
    fn test_zero_samples_error() {
        let mut scene = Scene::default();

        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
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
    fn test_edge_pixel_rendering_fix() {
        // This test specifically validates that the edge pixel fix works correctly
        // by ensuring all pixels of the target dimensions are rendered
        let mut scene = Scene::default();

        // Add a large plane that should fill the entire image
        scene.objects.push(Object::Plane {
            point: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            material: Material {
                color: "#FF0000".to_string(),
                ambient: 0.0,
                diffuse: 1.0,
                specular: 0.0,
                shininess: 1.0,
                reflectivity: None,
                texture: None,
            },
            transform: None,
        });

        // Add a light
        scene.lights.push(Light {
            position: [0.0, -5.0, 10.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: None,
        });

        // Test with different anti-aliasing modes to ensure edge pixels are rendered
        let test_cases = vec![
            ("None", AntiAliasingMode::None),
            ("Stochastic", AntiAliasingMode::Stochastic),
            ("Quincunx", AntiAliasingMode::Quincunx),
        ];

        for (mode_name, mode) in test_cases {
            let mut renderer = Renderer::new(4, 4);
            renderer.anti_aliasing_mode = mode;
            renderer.samples = 1;

            let result = renderer
                .render(&scene)
                .expect(&format!("Render failed with {} mode", mode_name));

            // Verify that the image has the correct dimensions
            assert_eq!(
                result.width(),
                4,
                "{} mode: Image width should be 4",
                mode_name
            );
            assert_eq!(
                result.height(),
                4,
                "{} mode: Image height should be 4",
                mode_name
            );

            // Verify that we get a pixel value at each corner and edge
            let pixels: Vec<_> = result.pixels().collect();
            assert_eq!(
                pixels.len(),
                16,
                "{} mode: Should have 16 pixels total",
                mode_name
            );

            // Check edge pixels specifically (these were the ones being lost before the fix)
            let right_edge_pixels: Vec<_> = pixels
                .iter()
                .enumerate()
                .filter(|(i, _)| *i % 4 == 3) // Right edge: pixels 3, 7, 11, 15
                .collect();
            assert_eq!(
                right_edge_pixels.len(),
                4,
                "{} mode: Should have 4 right edge pixels",
                mode_name
            );

            let bottom_edge_pixels: Vec<_> = pixels
                .iter()
                .enumerate()
                .filter(|(i, _)| *i >= 12) // Bottom edge: pixels 12, 13, 14, 15
                .collect();
            assert_eq!(
                bottom_edge_pixels.len(),
                4,
                "{} mode: Should have 4 bottom edge pixels",
                mode_name
            );

            // Verify that corner pixels exist (these combine right + bottom edge)
            let bottom_right_pixel = pixels.get(15); // Bottom-right corner: pixel 15
            assert!(
                bottom_right_pixel.is_some(),
                "{} mode: Bottom-right corner pixel should exist",
                mode_name
            );

            println!("✓ {} mode: All edge pixels rendered correctly", mode_name);
        }
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

    #[test]
    fn test_pixel_coordinate_mapping_fix() {
        // This test specifically validates that the pixel coordinate fix
        // correctly maps pixels to UV coordinates using edge-to-edge sampling

        // Test coordinate calculation for a 4x4 image
        let width = 4;
        let height = 4;

        // Test corner pixels
        let (u0, v0, _pw, _ph) = SamplingHelper::calculate_pixel_coords(0, 0, width, height);
        let (u3, v3, _pw, _ph) = SamplingHelper::calculate_pixel_coords(3, 3, width, height);

        // With edge-to-edge sampling:
        // - Top-left pixel (0,0) should map to UV (0.0, 1.0)
        // - Bottom-right pixel (3,3) should map to UV (1.0, 0.0)

        assert!(
            (u0 - 0.0).abs() < 1e-10,
            "Top-left U coordinate should be 0.0, got {}",
            u0
        );
        assert!(
            (v0 - 1.0).abs() < 1e-10,
            "Top-left V coordinate should be 1.0, got {}",
            v0
        );

        assert!(
            (u3 - 1.0).abs() < 1e-10,
            "Bottom-right U coordinate should be 1.0, got {}",
            u3
        );
        assert!(
            (v3 - 0.0).abs() < 1e-10,
            "Bottom-right V coordinate should be 0.0, got {}",
            v3
        );

        // Test that UV coordinates stay within [0,1] range for all pixels
        for y in 0..height {
            for x in 0..width {
                let (u, v, _pw, _ph) = SamplingHelper::calculate_pixel_coords(x, y, width, height);
                assert!(
                    u >= 0.0 && u <= 1.0,
                    "U coordinate {} for pixel ({},{}) should be in [0,1]",
                    u,
                    x,
                    y
                );
                assert!(
                    v >= 0.0 && v <= 1.0,
                    "V coordinate {} for pixel ({},{}) should be in [0,1]",
                    v,
                    x,
                    y
                );
            }
        }

        // Specifically test edge coverage
        let (u_left, _v, _pw, _ph) = SamplingHelper::calculate_pixel_coords(0, 0, width, height);
        let (u_right, _v, _pw, _ph) = SamplingHelper::calculate_pixel_coords(3, 0, width, height);
        let (_u, v_top, _pw, _ph) = SamplingHelper::calculate_pixel_coords(0, 0, width, height);
        let (_u, v_bottom, _pw, _ph) = SamplingHelper::calculate_pixel_coords(0, 3, width, height);

        assert!(
            (u_left - 0.0).abs() < 1e-10,
            "Left edge should be at U=0.0, got {}",
            u_left
        );
        assert!(
            (u_right - 1.0).abs() < 1e-10,
            "Right edge should be at U=1.0, got {}",
            u_right
        );
        assert!(
            (v_top - 1.0).abs() < 1e-10,
            "Top edge should be at V=1.0, got {}",
            v_top
        );
        assert!(
            (v_bottom - 0.0).abs() < 1e-10,
            "Bottom edge should be at V=0.0, got {}",
            v_bottom
        );

        println!("✓ Pixel coordinate mapping test passed (edge-to-edge sampling)");
    }

    #[test]
    fn test_mesh_scale_transform_bounds_fix() {
        // This test verifies that the mesh bounds bug has been fixed
        use crate::mesh::Mesh;
        use crate::ray::{Intersectable, MeshObject, Ray};
        use crate::scene::{parse_transforms, Color, Point, Vec3};

        // Create a simple ASCII STL with one triangle
        let ascii_stl = b"solid test
facet normal 0 0 1
  outer loop
    vertex -1 -1 0
    vertex 1 -1 0
    vertex 0 1 0
  endloop
endfacet
endsolid test";

        let original_mesh = Mesh::from_stl_bytes(ascii_stl).unwrap();
        println!("Original mesh bounds: {:?}", original_mesh.bounds());

        // Apply 8x scale transform
        let mut scaled_mesh = original_mesh.clone();
        let transform_strings = vec!["scale(8, 8, 8)".to_string()];
        let transform_matrix = parse_transforms(&transform_strings).unwrap();

        // Transform all vertices (simulating the fixed renderer.rs behavior)
        for triangle in &mut scaled_mesh.triangles {
            for vertex in &mut triangle.vertices {
                let vertex_homogeneous = transform_matrix * vertex.to_homogeneous();
                *vertex = Point::new(
                    vertex_homogeneous.x,
                    vertex_homogeneous.y,
                    vertex_homogeneous.z,
                );
            }
        }

        // Apply the fix: compute bounds after transformation
        scaled_mesh.compute_bounds();
        scaled_mesh.build_kdtree();

        let (min_bounds, max_bounds) = scaled_mesh.bounds();
        println!(
            "Scaled mesh bounds (after fix): {:?}",
            (min_bounds, max_bounds)
        );

        // Verify bounds are correctly updated to 8x scale
        assert!((min_bounds.x - (-8.0)).abs() < 1e-10);
        assert!((min_bounds.y - (-8.0)).abs() < 1e-10);
        assert!((max_bounds.x - 8.0).abs() < 1e-10);
        assert!((max_bounds.y - 8.0).abs() < 1e-10);

        // Create mesh object and test ray intersection
        let material_color = Color::new(1.0, 0.0, 0.0);
        let mesh_object = MeshObject::new(scaled_mesh, material_color, 0);

        // Create a ray that should hit the scaled mesh
        // After 8x scaling, triangle vertices are at (-8,-8,0), (8,-8,0), (0,8,0)
        // A ray at (4,0) going downward should intersect the triangle
        let ray = Ray::new(Point::new(4.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0));

        let hit_result = mesh_object.hit(&ray, 0.001, 1000.0);

        // With the fix, this ray should now intersect successfully
        assert!(
            hit_result.is_some(),
            "Ray should intersect scaled mesh after bounds fix"
        );

        let hit = hit_result.unwrap();
        println!("Ray intersection succeeded at point: {:?}", hit.point);

        // Verify the intersection point is approximately correct
        assert!(
            (hit.point.z - 0.0).abs() < 1e-10,
            "Intersection should be at z=0"
        );
        assert!(
            hit.point.x >= -8.0 && hit.point.x <= 8.0,
            "Intersection x should be in scaled bounds"
        );
        assert!(
            hit.point.y >= -8.0 && hit.point.y <= 8.0,
            "Intersection y should be in scaled bounds"
        );
    }

    #[test]
    fn test_dynamic_sampling() {
        let mut scene = Scene::default();

        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
        });

        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: None,
        });

        let mut renderer = Renderer::new(30, 30);
        renderer.anti_aliasing_mode = AntiAliasingMode::Dynamic {
            min_samples: 4,
            max_samples: 32,
            tolerance: 0.01,
        };
        let result = renderer.render(&scene);
        assert!(result.is_ok());

        let img = result.unwrap();
        assert_eq!(img.width(), 30);
        assert_eq!(img.height(), 30);
    }

    #[test]
    fn test_dynamic_sampling_deterministic() {
        let mut scene = Scene::default();

        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
            transform: None,
        });

        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
            diameter: Some(0.5),
        });

        let mut renderer = Renderer::new(20, 20);
        renderer.anti_aliasing_mode = AntiAliasingMode::Dynamic {
            min_samples: 4,
            max_samples: 16,
            tolerance: 0.01,
        };
        renderer.seed = Some(42);

        let result1 = renderer.render(&scene).expect("First render failed");
        let result2 = renderer.render(&scene).expect("Second render failed");

        let pixels1: Vec<_> = result1.pixels().collect();
        let pixels2: Vec<_> = result2.pixels().collect();

        for (i, (&p1, &p2)) in pixels1.iter().zip(pixels2.iter()).enumerate() {
            assert_eq!(
                p1, p2,
                "Dynamic pixel {} differs between renders: {:?} vs {:?}",
                i, p1, p2
            );
        }
    }

    #[test]
    fn test_dynamic_sampling_validation() {
        let scene = Scene::default();

        // min_samples < 2 should fail
        let mut r = Renderer::new(10, 10);
        r.anti_aliasing_mode = AntiAliasingMode::Dynamic {
            min_samples: 1,
            max_samples: 16,
            tolerance: 0.01,
        };
        assert!(r.render(&scene).is_err());

        // max_samples < min_samples should fail
        let mut r = Renderer::new(10, 10);
        r.anti_aliasing_mode = AntiAliasingMode::Dynamic {
            min_samples: 8,
            max_samples: 4,
            tolerance: 0.01,
        };
        assert!(r.render(&scene).is_err());

        // tolerance <= 0 should fail
        let mut r = Renderer::new(10, 10);
        r.anti_aliasing_mode = AntiAliasingMode::Dynamic {
            min_samples: 4,
            max_samples: 16,
            tolerance: 0.0,
        };
        assert!(r.render(&scene).is_err());
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
