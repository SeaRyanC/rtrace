use crate::ray::{HitRecord, Ray, World};
use crate::scene::{
    hex_to_color, AmbientIllumination, Color, Fog, Light, Material, Point, Texture, Vec3,
};
use nalgebra::Unit;
use rand::{Rng, SeedableRng};

/// Calculate grid pattern for a texture at given texture coordinates
fn apply_grid_texture(texture: &Texture, u: f64, v: f64, base_color: Color) -> Color {
    match texture {
        Texture::Grid {
            line_color,
            line_width,
            cell_size,
        } => {
            let grid_color = hex_to_color(line_color).unwrap_or(Color::new(0.0, 0.0, 0.0));
            let half_width = line_width / 2.0;

            // Check if we're on a grid line
            let u_mod = (u / cell_size).fract().abs();
            let v_mod = (v / cell_size).fract().abs();

            let on_u_line = u_mod <= half_width || u_mod >= (1.0 - half_width);
            let on_v_line = v_mod <= half_width || v_mod >= (1.0 - half_width);

            if on_u_line || on_v_line {
                grid_color
            } else {
                base_color
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
fn calculate_point_light_contribution(
    hit_record: &HitRecord,
    material: &Material,
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
    if world.hit(&shadow_ray, 0.001, light_distance).is_some() {
        return Color::new(0.0, 0.0, 0.0);
    }

    // Diffuse component
    let diffuse_strength = hit_record.normal.dot(&light_dir).max(0.0);
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

/// Calculate light contribution from a diffuse (area) light source
fn calculate_diffuse_light_contribution(
    hit_record: &HitRecord,
    material: &Material,
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
        let sample_point = sample_disk_light_point(&mut rng, light_center, &hit_record.point, diameter);
        
        let light_dir = Unit::new_normalize(sample_point - hit_record.point);
        let light_distance = (sample_point - hit_record.point).magnitude();

        // Check for shadows - cast ray from hit point to sampled light point
        let shadow_ray = Ray::new(
            hit_record.point + 0.001 * hit_record.normal.as_ref(),
            *light_dir.as_ref(),
        );

        // If there's an object between the hit point and the light sample, skip this sample
        if world.hit(&shadow_ray, 0.001, light_distance).is_some() {
            continue;
        }

        visible_samples += 1;

        // Diffuse component
        let diffuse_strength = hit_record.normal.dot(&light_dir).max(0.0);
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

        total_contribution += diffuse + specular;
    }

    // Scale the contributions based on visibility - more visible samples means more light received
    if SAMPLES > 0 {
        (total_contribution / SAMPLES as f64) * (visible_samples as f64 / SAMPLES as f64)
    } else {
        Color::new(0.0, 0.0, 0.0)
    }
}

/// Phong lighting calculation
pub fn phong_lighting(
    hit_record: &HitRecord,
    material: &Material,
    lights: &[Light],
    ambient: &AmbientIllumination,
    camera_pos: &Point,
    world: &World,
    seed: u64,
) -> Color {
    // Get base material color
    let mut material_color = hex_to_color(&material.color).unwrap_or(Color::new(1.0, 1.0, 1.0));

    // Apply texture if present
    if let Some(texture) = &material.texture {
        if let Some((u, v)) = hit_record.texture_coords {
            material_color = apply_grid_texture(texture, u, v, material_color);
        }
    }

    // Start with ambient lighting
    let ambient_color = hex_to_color(&ambient.color).unwrap_or(Color::new(1.0, 1.0, 1.0));
    let mut color =
        material.ambient * ambient.intensity * ambient_color.component_mul(&material_color);

    // Add contribution from each light source
    for light in lights {
        let light_pos = Point::new(light.position[0], light.position[1], light.position[2]);
        let light_color = hex_to_color(&light.color).unwrap_or(Color::new(1.0, 1.0, 1.0));

        // Handle diffuse (area) lights vs point lights
        let light_contribution = if let Some(diameter) = light.diameter {
            // Diffuse light - sample multiple points on the disk
            calculate_diffuse_light_contribution(
                hit_record,
                material,
                &light_pos,
                &light_color,
                light.intensity,
                diameter,
                camera_pos,
                world,
                &material_color,
                seed,
            )
        } else {
            // Point light - use single shadow ray
            calculate_point_light_contribution(
                hit_record,
                material,
                &light_pos,
                &light_color,
                light.intensity,
                camera_pos,
                world,
                &material_color,
            )
        };
        
        color += light_contribution;
    }

    color
}

/// Reflect a vector around a normal
fn reflect(incident: &Vec3, normal: &Unit<Vec3>) -> Unit<Vec3> {
    let reflected = incident - 2.0 * incident.dot(normal) * normal.as_ref();
    Unit::new_normalize(reflected)
}

/// Apply atmospheric fog to a color based on distance
pub fn apply_fog(color: Color, fog: &Option<Fog>, distance: f64) -> Color {
    if let Some(fog_settings) = fog {
        let fog_color = hex_to_color(&fog_settings.color).unwrap_or(Color::new(0.5, 0.5, 0.5));

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
        color * (1.0 - fog_factor) + fog_color * fog_factor
    } else {
        color
    }
}

/// Main ray color calculation
#[allow(clippy::too_many_arguments)]
pub fn ray_color(
    ray: &Ray,
    world: &World,
    lights: &[Light],
    ambient: &AmbientIllumination,
    fog: &Option<Fog>,
    camera_pos: &Point,
    background_color: Color,
    materials: &std::collections::HashMap<usize, Material>,
    max_depth: i32,
    seed: u64,
) -> Color {
    ray_color_with_camera(
        ray,
        world,
        lights,
        ambient,
        fog,
        camera_pos,
        background_color,
        materials,
        max_depth,
        None,
        seed,
    )
}

/// Main ray color calculation with optional camera for grid background
#[allow(clippy::too_many_arguments)]
pub fn ray_color_with_camera(
    ray: &Ray,
    world: &World,
    lights: &[Light],
    ambient: &AmbientIllumination,
    fog: &Option<Fog>,
    camera_pos: &Point,
    background_color: Color,
    materials: &std::collections::HashMap<usize, Material>,
    max_depth: i32,
    camera: Option<&crate::camera::Camera>,
    seed: u64,
) -> Color {
    if max_depth <= 0 {
        return Color::new(0.0, 0.0, 0.0);
    }

    if let Some(hit) = world.hit(ray, 0.001, f64::INFINITY) {
        // Get material for this object using the material index from the hit record
        let material = materials
            .get(&hit.material_index)
            .cloned()
            .unwrap_or_else(Material::default);

        // Calculate lighting
        let mut color = phong_lighting(&hit, &material, lights, ambient, camera_pos, world, seed);

        // Apply fog based on distance from camera
        let distance = (hit.point - *camera_pos).magnitude();
        color = apply_fog(color, fog, distance);

        // Handle reflections if material has reflectivity
        if let Some(reflectivity) = material.reflectivity {
            if reflectivity > 0.0 && max_depth > 1 {
                let view_dir = Unit::new_normalize(*camera_pos - hit.point);
                let reflect_dir = reflect(&(-view_dir.as_ref()), &hit.normal);
                let reflect_ray = Ray::new(
                    hit.point + 0.001 * hit.normal.as_ref(),
                    *reflect_dir.as_ref(),
                );

                let reflected_color = ray_color_with_camera(
                    &reflect_ray,
                    world,
                    lights,
                    ambient,
                    fog,
                    camera_pos,
                    background_color,
                    materials,
                    max_depth - 1,
                    camera,
                    seed,
                );

                color = color * (1.0 - reflectivity) + reflected_color * reflectivity;
            }
        }

        color
    } else {
        // Ray missed all objects - check for grid background if camera is orthographic
        if let Some(camera) = camera {
            if let Some(grid_color) = camera.get_grid_color(ray) {
                return grid_color;
            }
        }
        background_color
    }
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
            assert!(distance_from_center <= radius, "Point ({}, {}) is outside disk of radius {}", x, y, radius);
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
            let sample_point = sample_disk_light_point(&mut rng, &light_center, &hit_point, diameter);
            
            // The sampled point should be roughly the same distance from hit point as the light center
            let center_distance = (light_center - hit_point).magnitude();
            let sample_distance = (sample_point - hit_point).magnitude();
            
            // Allow for some variation due to the disk sampling, but it shouldn't be too far off
            let distance_diff = (sample_distance - center_distance).abs();
            assert!(distance_diff <= diameter / 2.0, "Sample point distance {} varies too much from center distance {}", sample_distance, center_distance);
        }
    }

    #[test]
    fn test_diffuse_light_vs_point_light() {
        use crate::ray::World;
        use crate::scene::Material;
        use crate::ray::HitRecord;
        use nalgebra::Unit;

        // Create a simple test setup
        let hit_record = HitRecord {
            point: Point::new(0.0, 0.0, 0.0),
            normal: Unit::new_normalize(Vec3::new(0.0, 1.0, 0.0)),
            t: 1.0,
            front_face: true,
            material_color: Color::new(1.0, 0.0, 0.0),
            material_index: 0,
            texture_coords: None,
        };
        
        let material = Material::default();
        let light_center = Point::new(0.0, 5.0, 0.0);
        let light_color = Color::new(1.0, 1.0, 1.0);
        let light_intensity = 1.0;
        let camera_pos = Point::new(0.0, 0.0, 5.0);
        let world = World::new(); // Empty world - no shadows
        let material_color = Color::new(1.0, 0.0, 0.0);

        // Test point light contribution
        let point_contrib = calculate_point_light_contribution(
            &hit_record,
            &material,
            &light_center,
            &light_color,
            light_intensity,
            &camera_pos,
            &world,
            &material_color,
        );

        // Test diffuse light contribution (very small diameter should be similar to point light)
        let diffuse_contrib = calculate_diffuse_light_contribution(
            &hit_record,
            &material,
            &light_center,
            &light_color,
            light_intensity,
            0.01, // Very small diameter
            &camera_pos,
            &world,
            &material_color,
            42, // Fixed seed for test
        );

        // With no shadows and small diameter, diffuse light should be similar to point light
        // Allow some variation due to random sampling
        assert!((point_contrib.magnitude() - diffuse_contrib.magnitude()).abs() < 0.5,
                "Point light magnitude {} and small diffuse light magnitude {} should be similar",
                point_contrib.magnitude(), diffuse_contrib.magnitude());
    }
}
