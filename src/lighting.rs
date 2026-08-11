use crate::ray::{apply_perlin_surface_effects, HitRecord, PrimitiveKind, Ray, World};
use crate::scene::{
    hex_to_color, Color, Fog, Light, Material, Point, SurfacePerlinNoise, Texture, Vec3,
};
use nalgebra::Unit;
use rand::{Rng, SeedableRng};

// ---------------------------------------------------------------------------
// Precomputed scene data – built once per render, used in the hot path.
// All String fields from the JSON scene are resolved to Color (f64 triplets)
// so the per-pixel path never allocates or parses hex strings.
// ---------------------------------------------------------------------------

/// Texture variant with all colors already parsed.
pub enum PreparedTexture {
    Grid {
        line_color: Color,
        line_width: f64,
        cell_size: f64,
    },
    Checkerboard {
        material_b: Box<PreparedMaterial>,
    },
}

/// Material with all colors pre-parsed — no heap allocation in the hot path.
pub struct PreparedMaterial {
    pub color: Color,
    pub ambient: f64,
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
    pub reflectivity: Option<f64>,
    pub texture: Option<PreparedTexture>,
    pub planar_perlin: Option<SurfacePerlinNoise>,
}

impl Default for PreparedMaterial {
    fn default() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 1.0),
            ambient: 0.1,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
            reflectivity: None,
            texture: None,
            planar_perlin: None,
        }
    }
}

impl PreparedMaterial {
    pub fn from_material(m: &Material) -> Self {
        Self {
            color: hex_to_color(&m.color).unwrap_or(Color::new(1.0, 1.0, 1.0)),
            ambient: m.ambient,
            diffuse: m.diffuse,
            specular: m.specular,
            shininess: m.shininess,
            reflectivity: m.reflectivity,
            texture: m.texture.as_ref().map(PreparedTexture::from_texture),
            planar_perlin: m.planar_perlin.clone(),
        }
    }
}

impl PreparedTexture {
    fn from_texture(t: &Texture) -> Self {
        match t {
            Texture::Grid {
                line_color,
                line_width,
                cell_size,
            } => Self::Grid {
                line_color: hex_to_color(line_color).unwrap_or(Color::new(0.0, 0.0, 0.0)),
                line_width: *line_width,
                cell_size: *cell_size,
            },
            Texture::Checkerboard { material_b } => Self::Checkerboard {
                material_b: Box::new(PreparedMaterial::from_material(material_b)),
            },
        }
    }
}

/// Light with position and color already converted — no hex parsing in the hot path.
pub struct PreparedLight {
    pub position: Point,
    pub color: Color,
    pub intensity: f64,
    pub diameter: Option<f64>,
}

impl PreparedLight {
    pub fn from_light(l: &Light) -> Self {
        Self {
            position: Point::new(l.position[0], l.position[1], l.position[2]),
            color: hex_to_color(&l.color).unwrap_or(Color::new(1.0, 1.0, 1.0)),
            intensity: l.intensity,
            diameter: l.diameter,
        }
    }
}

/// Fog with color already parsed.
pub struct PreparedFog {
    pub color: Color,
    pub start: f64,
    pub end: f64,
    pub density: f64,
}

impl PreparedFog {
    pub fn from_fog(f: &Fog) -> Self {
        Self {
            color: hex_to_color(&f.color).unwrap_or(Color::new(0.5, 0.5, 0.5)),
            start: f.start,
            end: f.end,
            density: f.density,
        }
    }
}

/// Resolve the effective material and surface color, applying any texture.
/// Returns `(&PreparedMaterial, Color)` — borrows, zero allocation.
/// - Grid: base material properties, color overridden by grid line color when on a line.
/// - Checkerboard: alternates between base material and material_b (full material switch).
#[inline]
fn effective_material_and_color(
    material: &PreparedMaterial,
    texture_coords: Option<(f64, f64)>,
) -> (&PreparedMaterial, Color) {
    if let Some(texture) = &material.texture {
        if let Some((u, v)) = texture_coords {
            return apply_prepared_texture(texture, u, v, material);
        }
    }
    (material, material.color)
}

/// Returns `(&PreparedMaterial, effective_color)` for the given texture + UV.
#[inline]
fn apply_prepared_texture<'a>(
    texture: &'a PreparedTexture,
    u: f64,
    v: f64,
    base: &'a PreparedMaterial,
) -> (&'a PreparedMaterial, Color) {
    match texture {
        PreparedTexture::Grid {
            line_color,
            line_width,
            cell_size,
        } => {
            let half_width = line_width / 2.0;
            let u_mod = (u / cell_size).fract().abs();
            let v_mod = (v / cell_size).fract().abs();
            let on_u_line = u_mod <= half_width || u_mod >= (1.0 - half_width);
            let on_v_line = v_mod <= half_width || v_mod >= (1.0 - half_width);
            // Grid only changes surface color; material shading properties stay the same
            let color = if on_u_line || on_v_line {
                *line_color
            } else {
                base.color
            };
            (base, color)
        }
        PreparedTexture::Checkerboard { material_b } => {
            let checker_u = u.floor() as i32;
            let checker_v = v.floor() as i32;
            if (checker_u + checker_v) % 2 == 0 {
                (base, base.color)
            } else {
                (material_b, material_b.color)
            }
        }
    }
}

/// Sample a random point on a disk of given radius, centered at origin in local coordinates
fn sample_disk_point<R: Rng>(rng: &mut R, radius: f64) -> (f64, f64) {
    // Use rejection sampling to get uniform distribution on disk
    loop {
        let x = rng.gen_range(-radius..radius);
        let y = rng.gen_range(-radius..radius);
        if x * x + y * y <= radius * radius {
            return (x, y);
        }
    }
}

/// Generate a random point on a disk perpendicular to the light direction
fn sample_disk_light_point<R: Rng>(
    rng: &mut R,
    light_center: &Point,
    hit_point: &Point,
    diameter: f64,
) -> Point {
    let radius = diameter / 2.0;

    // Direction from hit point to light center
    let light_dir = Unit::new_normalize(*light_center - *hit_point);

    // Create an orthogonal basis for the disk
    // Find a vector not parallel to light_dir
    let up = if light_dir.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };

    // Create orthogonal vectors for the disk plane
    let u = Unit::new_normalize(up.cross(light_dir.as_ref()));
    let v = Unit::new_normalize(light_dir.cross(u.as_ref()));

    // Sample random point on disk
    let (disk_u, disk_v) = sample_disk_point(rng, radius);

    // Convert to world coordinates
    light_center + disk_u * u.as_ref() + disk_v * v.as_ref()
}

/// Calculate light contribution from a point light source
#[allow(clippy::too_many_arguments)]
fn calculate_point_light_contribution(
    hit_record: &HitRecord,
    material: &PreparedMaterial,
    light_pos: &Point,
    light_color: &Color,
    light_intensity: f64,
    camera_pos: &Point,
    world: &World,
    material_color: &Color,
) -> Color {
    let light_dir = Unit::new_normalize(*light_pos - hit_record.point);

    // Check for shadows - cast ray from hit point to light
    let shadow_ray = Ray::new(
        hit_record.point + 0.001 * hit_record.normal.as_ref(),
        *light_dir.as_ref(),
    );
    let light_distance = (*light_pos - hit_record.point).magnitude();

    // If there's an object between the hit point and the light, we're in shadow
    // Use any_hit for early termination - we don't need the closest hit
    if world.any_hit(&shadow_ray, 0.001, light_distance) {
        return Color::new(0.0, 0.0, 0.0);
    }

    calculate_diffuse_and_specular(
        hit_record,
        material,
        &light_dir,
        light_color,
        light_intensity,
        camera_pos,
        material_color,
    )
}

/// Calculate light contribution from a diffuse (area) light source
#[allow(clippy::too_many_arguments)]
fn calculate_diffuse_light_contribution(
    hit_record: &HitRecord,
    material: &PreparedMaterial,
    light_center: &Point,
    light_color: &Color,
    light_intensity: f64,
    diameter: f64,
    camera_pos: &Point,
    world: &World,
    material_color: &Color,
    seed: u64,
) -> Color {
    // Number of samples to take on the light disk
    const SAMPLES: u32 = 16;

    // Create deterministic RNG seeded by hit point coordinates and global seed
    let light_seed = seed
        .wrapping_mul(0x9E3779B97F4A7C15_u64)
        .wrapping_add(((hit_record.point.x * 1000.0) as u64).wrapping_mul(0x85EBCA6B))
        .wrapping_add(((hit_record.point.y * 1000.0) as u64).wrapping_mul(0xC2B2AE35))
        .wrapping_add(((hit_record.point.z * 1000.0) as u64).wrapping_mul(0x6C8E9CF5));
    let mut rng = rand::rngs::StdRng::seed_from_u64(light_seed);
    let mut total_contribution = Color::new(0.0, 0.0, 0.0);
    let mut visible_samples = 0;

    for _ in 0..SAMPLES {
        // Sample a random point on the light disk
        let sample_point =
            sample_disk_light_point(&mut rng, light_center, &hit_record.point, diameter);

        let light_dir = Unit::new_normalize(sample_point - hit_record.point);
        let light_distance = (sample_point - hit_record.point).magnitude();

        // Check for shadows - cast ray from hit point to sampled light point
        let shadow_ray = Ray::new(
            hit_record.point + 0.001 * hit_record.normal.as_ref(),
            *light_dir.as_ref(),
        );

        // If there's an object between the hit point and the light sample, skip this sample
        // Use any_hit for early termination - we don't need the closest hit
        if world.any_hit(&shadow_ray, 0.001, light_distance) {
            continue;
        }

        visible_samples += 1;

        total_contribution += calculate_diffuse_and_specular(
            hit_record,
            material,
            &light_dir,
            light_color,
            light_intensity,
            camera_pos,
            material_color,
        );
    }

    // Scale the contributions based on visibility - more visible samples means more light received
    if SAMPLES > 0 {
        (total_contribution / SAMPLES as f64) * (visible_samples as f64 / SAMPLES as f64)
    } else {
        Color::new(0.0, 0.0, 0.0)
    }
}

/// Phong lighting calculation using precomputed scene data (no heap allocation).
#[allow(clippy::too_many_arguments)]
pub fn phong_lighting(
    hit_record: &HitRecord,
    material: &PreparedMaterial,
    lights: &[PreparedLight],
    ambient_color: Color,
    ambient_intensity: f64,
    camera_pos: &Point,
    world: &World,
    seed: u64,
) -> Color {
    let mut shaded_hit = hit_record.clone();

    if shaded_hit.primitive_kind == PrimitiveKind::Plane {
        if let Some(perlin) = &material.planar_perlin {
            let (u_axis, v_axis) = tangent_basis_from_normal(&shaded_hit.normal);
            let u = shaded_hit.point.coords.dot(u_axis.as_ref());
            let v = shaded_hit.point.coords.dot(v_axis.as_ref());
            apply_perlin_surface_effects(
                &mut shaded_hit.normal,
                &mut shaded_hit.color_modulation,
                perlin,
                u,
                v,
                u_axis.as_ref(),
                v_axis.as_ref(),
                1.0,
            );
        }
    }

    // Resolve texture — returns borrowed material + surface color; zero allocation.
    let (effective_mat, material_color) =
        effective_material_and_color(material, shaded_hit.texture_coords);
    let material_color = material_color.component_mul(&shaded_hit.color_modulation);

    // Start with ambient lighting
    let mut color =
        effective_mat.ambient * ambient_intensity * ambient_color.component_mul(&material_color);

    // Add contribution from each light source
    for light in lights {
        let light_contribution = if let Some(diameter) = light.diameter {
            calculate_diffuse_light_contribution(
                &shaded_hit,
                effective_mat,
                &light.position,
                &light.color,
                light.intensity,
                diameter,
                camera_pos,
                world,
                &material_color,
                seed,
            )
        } else {
            calculate_point_light_contribution(
                &shaded_hit,
                effective_mat,
                &light.position,
                &light.color,
                light.intensity,
                camera_pos,
                world,
                &material_color,
            )
        };

        color += light_contribution;
    }

    fn tangent_basis_from_normal(normal: &Unit<Vec3>) -> (Unit<Vec3>, Unit<Vec3>) {
        let helper = if normal.z.abs() < 0.9 {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        let u = Unit::new_normalize(normal.cross(&helper));
        let v = Unit::new_normalize(normal.cross(u.as_ref()));
        (u, v)
    }

    color
}

/// Calculate diffuse and specular components for a single light direction
fn calculate_diffuse_and_specular(
    hit_record: &HitRecord,
    material: &PreparedMaterial,
    light_dir: &Unit<Vec3>,
    light_color: &Color,
    light_intensity: f64,
    camera_pos: &Point,
    material_color: &Color,
) -> Color {
    // Diffuse component
    let diffuse_strength = hit_record.normal.dot(light_dir).max(0.0);
    let diffuse = material.diffuse
        * diffuse_strength
        * light_intensity
        * light_color.component_mul(material_color);

    // Specular component (Phong model)
    let specular = if diffuse_strength > 0.0 {
        let view_dir = Unit::new_normalize(*camera_pos - hit_record.point);
        let reflect_dir = reflect(&(-light_dir.as_ref()), &hit_record.normal);
        let spec_strength = view_dir.dot(&reflect_dir).max(0.0).powf(material.shininess);
        material.specular * spec_strength * light_intensity * light_color
    } else {
        Color::new(0.0, 0.0, 0.0)
    };

    diffuse + specular
}

/// Reflect a vector around a normal
fn reflect(incident: &Vec3, normal: &Unit<Vec3>) -> Unit<Vec3> {
    let reflected = incident - 2.0 * incident.dot(normal) * normal.as_ref();
    Unit::new_normalize(reflected)
}

/// Apply atmospheric fog to a color based on distance
pub fn apply_fog(color: Color, fog: Option<&PreparedFog>, distance: f64) -> Color {
    if let Some(fog_settings) = fog {
        // Linear fog falloff
        let fog_factor = if distance <= fog_settings.start {
            0.0
        } else if distance >= fog_settings.end {
            1.0
        } else {
            (distance - fog_settings.start) / (fog_settings.end - fog_settings.start)
        };

        // Apply fog density
        let fog_factor = 1.0 - (-fog_settings.density * fog_factor).exp();
        let fog_factor = fog_factor.clamp(0.0, 1.0);

        // Blend original color with fog color
        color * (1.0 - fog_factor) + fog_settings.color * fog_factor
    } else {
        color
    }
}

/// Internal ray tracing parameters to reduce function argument count
struct RayTraceParams<'a> {
    world: &'a World,
    lights: &'a [PreparedLight],
    ambient_color: Color,
    ambient_intensity: f64,
    fog: Option<&'a PreparedFog>,
    camera_pos: &'a Point,
    background_color: Color,
    materials: &'a [PreparedMaterial],
    camera: Option<&'a crate::camera::Camera>,
    seed: u64,
}

/// Internal function that handles the core ray tracing logic
/// Returns (color, Option<depth>, Option<normal>)
fn trace_ray_internal(
    ray: &Ray,
    params: &RayTraceParams,
    max_depth: i32,
) -> (Color, Option<f64>, Option<Vec3>) {
    if max_depth <= 0 {
        return (Color::new(0.0, 0.0, 0.0), None, None);
    }

    if let Some(hit) = params.world.hit(ray, 0.001, f64::INFINITY) {
        // Calculate camera-space depth and normal for outline detection
        let camera_space_depth = (hit.point - *params.camera_pos).magnitude();
        let world_normal = *hit.normal.as_ref();

        // Get material for this object — indexed lookup, no clone, no allocation.
        let default_mat = PreparedMaterial::default();
        let material = params
            .materials
            .get(hit.material_index)
            .unwrap_or(&default_mat);

        // Calculate lighting
        let mut color = phong_lighting(
            &hit,
            material,
            params.lights,
            params.ambient_color,
            params.ambient_intensity,
            params.camera_pos,
            params.world,
            params.seed,
        );

        // Apply fog based on distance from camera
        color = apply_fog(color, params.fog, camera_space_depth);

        // Handle reflections if material has reflectivity
        if let Some(reflectivity) = material.reflectivity {
            if reflectivity > 0.0 && max_depth > 1 {
                let view_dir = Unit::new_normalize(*params.camera_pos - hit.point);
                let reflect_dir = reflect(&(-view_dir.as_ref()), &hit.normal);
                let reflect_ray = Ray::new(
                    hit.point + 0.001 * hit.normal.as_ref(),
                    *reflect_dir.as_ref(),
                );

                // For reflected rays, we only care about color, not depth/normal data
                let (reflected_color, _, _) =
                    trace_ray_internal(&reflect_ray, params, max_depth - 1);

                color = color * (1.0 - reflectivity) + reflected_color * reflectivity;
            }
        }

        (color, Some(camera_space_depth), Some(world_normal))
    } else {
        // Background pixel - check for grid background
        let background = if let Some(camera) = params.camera {
            camera
                .get_grid_color(ray)
                .unwrap_or(params.background_color)
        } else {
            params.background_color
        };

        (background, None, None)
    }
}

/// Ray color calculation that also captures depth and normal data for outline detection.
/// Accepts precomputed scene data to avoid per-pixel allocations.
#[allow(clippy::too_many_arguments)]
pub fn ray_color_with_data(
    ray: &Ray,
    world: &World,
    lights: &[PreparedLight],
    ambient_color: Color,
    ambient_intensity: f64,
    fog: Option<&PreparedFog>,
    camera_pos: &Point,
    background_color: Color,
    materials: &[PreparedMaterial],
    max_depth: i32,
    camera: Option<&crate::camera::Camera>,
    seed: u64,
) -> (Color, Option<f64>, Option<Vec3>) {
    let params = RayTraceParams {
        world,
        lights,
        ambient_color,
        ambient_intensity,
        fog,
        camera_pos,
        background_color,
        materials,
        camera,
        seed,
    };
    trace_ray_internal(ray, &params, max_depth)
}

/// Main ray color calculation (convenience wrapper).
#[allow(clippy::too_many_arguments)]
pub fn ray_color(
    ray: &Ray,
    world: &World,
    lights: &[PreparedLight],
    ambient_color: Color,
    ambient_intensity: f64,
    fog: Option<&PreparedFog>,
    camera_pos: &Point,
    background_color: Color,
    materials: &[PreparedMaterial],
    max_depth: i32,
    seed: u64,
) -> Color {
    ray_color_with_data(
        ray,
        world,
        lights,
        ambient_color,
        ambient_intensity,
        fog,
        camera_pos,
        background_color,
        materials,
        max_depth,
        None,
        seed,
    )
    .0
}

/// Main ray color calculation with optional camera for grid background.
#[allow(clippy::too_many_arguments)]
pub fn ray_color_with_camera(
    ray: &Ray,
    world: &World,
    lights: &[PreparedLight],
    ambient_color: Color,
    ambient_intensity: f64,
    fog: Option<&PreparedFog>,
    camera_pos: &Point,
    background_color: Color,
    materials: &[PreparedMaterial],
    max_depth: i32,
    camera: Option<&crate::camera::Camera>,
    seed: u64,
) -> Color {
    ray_color_with_data(
        ray,
        world,
        lights,
        ambient_color,
        ambient_intensity,
        fog,
        camera_pos,
        background_color,
        materials,
        max_depth,
        camera,
        seed,
    )
    .0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_hex_to_color() {
        let color = hex_to_color("#FF0000").unwrap();
        assert!((color.x - 1.0).abs() < 1e-6);
        assert!((color.y - 0.0).abs() < 1e-6);
        assert!((color.z - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_reflect() {
        let incident = Vec3::new(1.0, -1.0, 0.0);
        let normal = Unit::new_normalize(Vec3::new(0.0, 1.0, 0.0));
        let reflected = reflect(&incident, &normal);

        // For incident (1, -1, 0) reflecting off normal (0, 1, 0)
        // The reflection should be normalized, so we need to check direction
        // Expected reflection direction should be roughly (0.707, 0.707, 0)
        assert!((reflected.x - reflected.y).abs() < 1e-6); // x and y should be equal
        assert!(reflected.y > 0.0); // y should be positive (reflected upward)
        assert!((reflected.z - 0.0).abs() < 1e-6); // z should remain 0

        // The reflected vector should be normalized
        assert!((reflected.magnitude() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_sample_disk_point() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let radius = 2.0;

        // Sample multiple points and verify they're within the disk
        for _ in 0..100 {
            let (x, y) = sample_disk_point(&mut rng, radius);
            let distance_from_center = (x * x + y * y).sqrt();
            assert!(
                distance_from_center <= radius,
                "Point ({}, {}) is outside disk of radius {}",
                x,
                y,
                radius
            );
        }
    }

    #[test]
    fn test_sample_disk_light_point() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let light_center = Point::new(0.0, 5.0, 0.0);
        let hit_point = Point::new(0.0, 0.0, 0.0);
        let diameter = 2.0;

        // Sample multiple points on the light disk
        for _ in 0..100 {
            let sample_point =
                sample_disk_light_point(&mut rng, &light_center, &hit_point, diameter);

            // The sampled point should be roughly the same distance from hit point as the light center
            let center_distance = (light_center - hit_point).magnitude();
            let sample_distance = (sample_point - hit_point).magnitude();

            // Allow for some variation due to the disk sampling, but it shouldn't be too far off
            let distance_diff = (sample_distance - center_distance).abs();
            assert!(
                distance_diff <= diameter / 2.0,
                "Sample point distance {} varies too much from center distance {}",
                sample_distance,
                center_distance
            );
        }
    }

    #[test]
    fn test_checkerboard_texture() {
        use crate::scene::Material;
        // Build prepared materials for checkerboard
        let mat_a = Material {
            color: "#FF0000".to_string(),
            ambient: 0.1,
            diffuse: 0.8,
            specular: 0.2,
            shininess: 32.0,
            reflectivity: None,
            planar_perlin: None,
            texture: Some(Texture::Checkerboard {
                material_b: Box::new(Material {
                    color: "#0000FF".to_string(),
                    ambient: 0.2,
                    diffuse: 0.6,
                    specular: 0.4,
                    shininess: 16.0,
                    reflectivity: None,
                    planar_perlin: None,
                    texture: None,
                }),
            }),
        };
        let prepared = PreparedMaterial::from_material(&mat_a);

        // At (0.0, 0.0): floor(0) + floor(0) = 0, 0 % 2 = 0 -> base_material (red)
        let (mat, color) = effective_material_and_color(&prepared, Some((0.0, 0.0)));
        assert!((color.x - 1.0).abs() < 1e-3); // red
        assert!(color.y.abs() < 1e-3);
        assert!((mat.shininess - 32.0).abs() < 1e-6);
        assert!((mat.ambient - 0.1).abs() < 1e-6);
        assert!((mat.diffuse - 0.8).abs() < 1e-6);

        // At (1.0, 0.0): floor(1) + floor(0) = 1, 1 % 2 = 1 -> material_b (blue)
        let (mat, color) = effective_material_and_color(&prepared, Some((1.0, 0.0)));
        assert!(color.x.abs() < 1e-3); // blue
        assert!((color.z - 1.0).abs() < 1e-3);
        assert!((mat.shininess - 16.0).abs() < 1e-6);
        assert!((mat.ambient - 0.2).abs() < 1e-6);
        assert!((mat.diffuse - 0.6).abs() < 1e-6);

        // At (0.0, 1.0): floor(0) + floor(1) = 1 -> material_b (blue)
        let (mat, color) = effective_material_and_color(&prepared, Some((0.0, 1.0)));
        assert!(color.x.abs() < 1e-3);
        assert!((mat.shininess - 16.0).abs() < 1e-6);

        // At (1.0, 1.0): floor(1) + floor(1) = 2, 2 % 2 = 0 -> base_material (red)
        let (mat, color) = effective_material_and_color(&prepared, Some((1.0, 1.0)));
        assert!((color.x - 1.0).abs() < 1e-3);
        assert!((mat.shininess - 32.0).abs() < 1e-6);

        // At (0.7, 0.3): floor(0.7) + floor(0.3) = 0 -> base_material
        let (_mat, color) = effective_material_and_color(&prepared, Some((0.7, 0.3)));
        assert!((color.x - 1.0).abs() < 1e-3);

        // At (1.2, 0.8): floor(1.2) + floor(0.8) = 1 -> material_b
        let (_mat, color) = effective_material_and_color(&prepared, Some((1.2, 0.8)));
        assert!(color.x.abs() < 1e-3);
    }

    #[test]
    fn test_grid_texture_backwards_compatibility() {
        use crate::scene::Material;
        let mat = Material {
            color: "#FFFFFF".to_string(),
            ambient: 0.2,
            diffuse: 0.8,
            specular: 0.1,
            shininess: 10.0,
            reflectivity: None,
            planar_perlin: None,
            texture: Some(Texture::Grid {
                line_color: "#FF0000".to_string(),
                line_width: 0.1,
                cell_size: 1.0,
            }),
        };
        let prepared = PreparedMaterial::from_material(&mat);

        // At (0.0, 0.0) we should be on a grid line -> red
        let (_m, color) = effective_material_and_color(&prepared, Some((0.0, 0.0)));
        assert!((color.x - 1.0).abs() < 1e-3); // red
        assert!(color.y.abs() < 1e-3);

        // At (0.5, 0.5) we should NOT be on a grid line -> white
        let (_m, color) = effective_material_and_color(&prepared, Some((0.5, 0.5)));
        assert!((color.x - 1.0).abs() < 1e-3); // white
        assert!((color.y - 1.0).abs() < 1e-3);
        assert!((color.z - 1.0).abs() < 1e-3);
    }
}
