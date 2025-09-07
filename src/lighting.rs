use crate::ray::{HitRecord, Ray, World};
use crate::scene::{
    hex_to_color, AmbientIllumination, Color, Fog, Light, Material, Point, Texture, Vec3,
};
use nalgebra::Unit;

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

/// Phong lighting calculation
pub fn phong_lighting(
    hit_record: &HitRecord,
    material: &Material,
    lights: &[Light],
    ambient: &AmbientIllumination,
    camera_pos: &Point,
    world: &World,
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

        let light_dir = Unit::new_normalize(light_pos - hit_record.point);

        // Check for shadows - cast ray from hit point to light
        let shadow_ray = Ray::new(
            hit_record.point + 0.001 * hit_record.normal.as_ref(),
            *light_dir.as_ref(),
        );
        let light_distance = (light_pos - hit_record.point).magnitude();

        // If there's an object between the hit point and the light, we're in shadow
        if world.hit(&shadow_ray, 0.001, light_distance).is_some() {
            continue;
        }

        // Diffuse component
        let diffuse_strength = hit_record.normal.dot(&light_dir).max(0.0);
        let diffuse = material.diffuse
            * diffuse_strength
            * light.intensity
            * light_color.component_mul(&material_color);
        color += diffuse;

        // Specular component (Phong model)
        if diffuse_strength > 0.0 {
            let view_dir = Unit::new_normalize(*camera_pos - hit_record.point);
            let reflect_dir = reflect(&(-light_dir.as_ref()), &hit_record.normal);
            let spec_strength = view_dir.dot(&reflect_dir).max(0.0).powf(material.shininess);
            let specular = material.specular * spec_strength * light.intensity * light_color;
            color += specular;
        }
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
        let mut color = phong_lighting(&hit, &material, lights, ambient, camera_pos, world);

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

                let reflected_color = ray_color(
                    &reflect_ray,
                    world,
                    lights,
                    ambient,
                    fog,
                    camera_pos,
                    background_color,
                    materials,
                    max_depth - 1,
                );

                color = color * (1.0 - reflectivity) + reflected_color * reflectivity;
            }
        }

        color
    } else {
        background_color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
