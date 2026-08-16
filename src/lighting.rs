use crate::noise;
use crate::ray::{apply_perlin_surface_effects, HitRecord, PrimitiveKind, Ray, World};
use crate::scene::{
    hex_to_color, Color, Fog, Light, Material, Point, SurfacePerlinNoise, Texture,
    TextureTransform, Vec3,
};
use nalgebra::{Matrix3, Rotation3, Unit};
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
    Marble {
        colors: Vec<Color>,
        direction: Vec3,
        bands_per_unit: f64,
        noise_scale: f64,
        warp_strength: f64,
        vein_sharpness: f64,
        branch_strength: f64,
        octaves: u32,
        persistence: f64,
        lacunarity: f64,
        seed: u64,
        translate: Vec3,
        rotation: Matrix3<f64>,
        scale: Vec3,
    },
    Wood {
        colors: Vec<Color>,
        origin: Vec3,
        axis: Vec3,
        rings_per_unit: f64,
        ring_width: f64,
        noise_scale: f64,
        ring_warp: f64,
        grain_scale: f64,
        grain_strength: f64,
        axis_u: Vec3,
        axis_v: Vec3,
        octaves: u32,
        persistence: f64,
        lacunarity: f64,
        seed: u64,
        translate: Vec3,
        rotation: Matrix3<f64>,
        scale: Vec3,
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
            Texture::Marble {
                colors,
                direction,
                bands_per_unit,
                noise_scale,
                warp_strength,
                vein_sharpness,
                branch_strength,
                octaves,
                persistence,
                lacunarity,
                seed,
                transform,
            } => {
                let (translate, rotation, scale) = prepare_texture_transform(transform);
                Self::Marble {
                    colors: prepare_palette(colors, Color::new(1.0, 1.0, 1.0)),
                    direction: normalized_vector(*direction),
                    bands_per_unit: *bands_per_unit,
                    noise_scale: noise_scale.abs().max(1e-6),
                    warp_strength: warp_strength.max(0.0),
                    vein_sharpness: vein_sharpness.max(0.1),
                    branch_strength: branch_strength.max(0.0),
                    octaves: *octaves,
                    persistence: *persistence,
                    lacunarity: *lacunarity,
                    seed: *seed,
                    translate,
                    rotation,
                    scale,
                }
            }
            Texture::Wood {
                colors,
                origin,
                axis,
                rings_per_unit,
                ring_width,
                noise_scale,
                ring_warp,
                grain_scale,
                grain_strength,
                octaves,
                persistence,
                lacunarity,
                seed,
                transform,
            } => {
                let (translate, rotation, scale) = prepare_texture_transform(transform);
                let axis = normalized_vector(*axis);
                let (axis_u, axis_v) = texture_basis(axis);
                Self::Wood {
                    colors: prepare_palette(colors, Color::new(0.55, 0.3, 0.12)),
                    origin: nalgebra::Vector3::new(origin[0], origin[1], origin[2]),
                    axis,
                    rings_per_unit: *rings_per_unit,
                    ring_width: ring_width.clamp(0.0, 1.0),
                    noise_scale: noise_scale.abs().max(1e-6),
                    ring_warp: ring_warp.max(0.0),
                    grain_scale: grain_scale.abs().max(1e-6),
                    grain_strength: grain_strength.max(0.0),
                    axis_u,
                    axis_v,
                    octaves: *octaves,
                    persistence: *persistence,
                    lacunarity: *lacunarity,
                    seed: *seed,
                    translate,
                    rotation,
                    scale,
                }
            }
        }
    }
}

fn prepare_palette(colors: &[String], fallback: Color) -> Vec<Color> {
    if colors.is_empty() {
        return vec![fallback];
    }

    colors
        .iter()
        .map(|color| hex_to_color(color).unwrap_or(fallback))
        .collect()
}

fn normalized_vector(values: [f64; 3]) -> Vec3 {
    let vector = Vec3::new(values[0], values[1], values[2]);
    if vector.magnitude_squared() < 1e-12 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        vector.normalize()
    }
}

fn texture_basis(axis: Vec3) -> (Vec3, Vec3) {
    let reference = if axis.x.abs() < 0.7 {
        Vec3::x()
    } else {
        Vec3::y()
    };
    let axis_u = axis.cross(&reference).normalize();
    let axis_v = axis.cross(&axis_u).normalize();
    (axis_u, axis_v)
}

fn prepare_texture_transform(transform: &TextureTransform) -> (Vec3, Matrix3<f64>, Vec3) {
    let rotation = Rotation3::from_euler_angles(
        transform.rotate_degrees[0].to_radians(),
        transform.rotate_degrees[1].to_radians(),
        transform.rotate_degrees[2].to_radians(),
    )
    .into_inner();

    (
        Vec3::new(
            transform.translate[0],
            transform.translate[1],
            transform.translate[2],
        ),
        rotation,
        Vec3::new(transform.scale[0], transform.scale[1], transform.scale[2]),
    )
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
fn effective_material_and_color<'a>(
    material: &'a PreparedMaterial,
    texture_coords: Option<(f64, f64)>,
    point: &Point,
) -> (&'a PreparedMaterial, Color) {
    if let Some(texture) = &material.texture {
        return apply_prepared_texture(texture, texture_coords, point, material);
    }
    (material, material.color)
}

/// Returns `(&PreparedMaterial, effective_color)` for the given texture.
#[inline]
fn apply_prepared_texture<'a>(
    texture: &'a PreparedTexture,
    texture_coords: Option<(f64, f64)>,
    point: &Point,
    base: &'a PreparedMaterial,
) -> (&'a PreparedMaterial, Color) {
    match texture {
        PreparedTexture::Grid {
            line_color,
            line_width,
            cell_size,
        } => {
            let Some((u, v)) = texture_coords else {
                return (base, base.color);
            };
            let cell_size = cell_size.abs().max(1e-6);
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
            let Some((u, v)) = texture_coords else {
                return (base, base.color);
            };
            let checker_u = u.floor() as i32;
            let checker_v = v.floor() as i32;
            if (checker_u + checker_v) % 2 == 0 {
                (base, base.color)
            } else {
                (material_b, material_b.color)
            }
        }
        PreparedTexture::Marble {
            colors,
            direction,
            bands_per_unit,
            noise_scale,
            warp_strength,
            vein_sharpness,
            branch_strength,
            octaves,
            persistence,
            lacunarity,
            seed,
            translate,
            rotation,
            scale,
        } => {
            let point = map_texture_point(point, translate, rotation, scale);
            let warp_frequency = *noise_scale * 0.42;
            let warp = Vec3::new(
                noise::fbm3(
                    point.x * warp_frequency + 17.13,
                    point.y * warp_frequency - 4.71,
                    point.z * warp_frequency + 9.27,
                    (*seed).wrapping_add(0xA24BAED4963EE407),
                    *octaves,
                    *persistence,
                    *lacunarity,
                ),
                noise::fbm3(
                    point.x * warp_frequency - 11.41,
                    point.y * warp_frequency + 23.07,
                    point.z * warp_frequency + 2.83,
                    (*seed).wrapping_add(0x9FB21C651E98DF25),
                    *octaves,
                    *persistence,
                    *lacunarity,
                ),
                noise::fbm3(
                    point.x * warp_frequency + 5.39,
                    point.y * warp_frequency + 14.63,
                    point.z * warp_frequency - 19.17,
                    (*seed).wrapping_add(0xC13FA9A902A6328F),
                    *octaves,
                    *persistence,
                    *lacunarity,
                ),
            );
            let warped_point = point + warp * (*warp_strength * 0.28);
            let variation_strength = if *warp_strength <= 0.0 {
                0.0
            } else {
                *branch_strength
            };
            let secondary_frequency = *noise_scale * 1.05;
            let secondary_warp = Vec3::new(
                noise::fbm3(
                    warped_point.x * secondary_frequency + 31.17,
                    warped_point.y * secondary_frequency - 8.43,
                    warped_point.z * secondary_frequency + 4.29,
                    (*seed).wrapping_add(0x51ED270B8A3D4F19),
                    *octaves,
                    *persistence,
                    *lacunarity,
                ),
                noise::fbm3(
                    warped_point.x * secondary_frequency - 6.71,
                    warped_point.y * secondary_frequency + 18.39,
                    warped_point.z * secondary_frequency + 12.53,
                    (*seed).wrapping_add(0x7C3A2D1F5B9E6A43),
                    *octaves,
                    *persistence,
                    *lacunarity,
                ),
                noise::fbm3(
                    warped_point.x * secondary_frequency + 14.07,
                    warped_point.y * secondary_frequency + 2.61,
                    warped_point.z * secondary_frequency - 21.33,
                    (*seed).wrapping_add(0xE4D9B7C35A1F806D),
                    *octaves,
                    *persistence,
                    *lacunarity,
                ),
            );
            let flow_point = warped_point + secondary_warp * (variation_strength * 0.16);
            let axial = flow_point.dot(direction);
            let network_point = flow_point - direction * (axial * 0.28);
            let network_frequency = *noise_scale * (0.35 + 0.6 * bands_per_unit.abs());
            let structure = noise::fbm3(
                flow_point.x * *noise_scale * 0.52,
                flow_point.y * *noise_scale * 0.52,
                flow_point.z * *noise_scale * 0.52,
                (*seed).wrapping_add(0x2F6E2B1D4C8A9537),
                *octaves,
                *persistence,
                *lacunarity,
            );
            let local_bands =
                *bands_per_unit * (1.0 + variation_strength * 0.24 * structure).max(0.2);
            let detail = noise::fbm3(
                flow_point.x * *noise_scale * 1.7,
                flow_point.y * *noise_scale * 1.7,
                flow_point.z * *noise_scale * 1.7,
                (*seed).wrapping_add(0xD1B54A32D192ED03),
                *octaves,
                *persistence,
                *lacunarity,
            );
            let phase = std::f64::consts::TAU * local_bands * flow_point.dot(direction)
                + detail * *warp_strength * 1.4;
            let wave = 0.5 + 0.5 * phase.sin();
            let density = 0.5
                + 0.5
                    * noise::fbm3(
                        flow_point.x * *noise_scale * 0.36,
                        flow_point.y * *noise_scale * 0.36,
                        flow_point.z * *noise_scale * 0.36,
                        (*seed).wrapping_add(0x94D049BB133111EB),
                        *octaves,
                        *persistence,
                        *lacunarity,
                    );
            let primary_factor = if variation_strength <= 0.0 {
                1.0
            } else {
                0.25 + 1.05 * density
            };
            let ridged_field = 1.0
                - noise::fbm3(
                    network_point.x * network_frequency,
                    network_point.y * network_frequency,
                    network_point.z * network_frequency,
                    (*seed).wrapping_add(0xB5C0FBCFEC4D3B2F),
                    *octaves,
                    *persistence,
                    *lacunarity,
                )
                .abs();
            let branch_mask = smoothstep(0.86, 0.98, ridged_field);
            let branch_noise = 0.5
                + 0.5
                    * noise::fbm3(
                        network_point.x * network_frequency * 1.9,
                        network_point.y * network_frequency * 1.9,
                        network_point.z * network_frequency * 1.9,
                        (*seed).wrapping_add(0x8D58AC26AFE12E47),
                        *octaves,
                        *persistence,
                        *lacunarity,
                    );
            let branch_gate = smoothstep(0.58, 0.82, branch_noise);
            let fine_ridged_field = 1.0
                - noise::fbm3(
                    network_point.x * network_frequency * 2.6 + 12.41,
                    network_point.y * network_frequency * 2.6 - 5.73,
                    network_point.z * network_frequency * 2.6 + 18.29,
                    (*seed).wrapping_add(0x6A09E667F3BCC909),
                    *octaves,
                    *persistence,
                    *lacunarity,
                )
                .abs();
            let macro_vein = smoothstep(0.88, 0.985, ridged_field);
            let fine_vein = smoothstep(0.92, 0.995, fine_ridged_field);
            let directional_vein = wave.powf(*vein_sharpness);
            let network_vein =
                (macro_vein * (0.72 + 0.28 * branch_gate) + fine_vein * 0.18).clamp(0.0, 1.0);
            let primary_vein = if variation_strength <= 0.0 {
                directional_vein
            } else {
                (network_vein * 0.95 + directional_vein * 0.04) * primary_factor
            };
            let secondary_vein =
                branch_mask * (0.06 + 0.24 * branch_gate) * variation_strength * 0.45;
            let vein = (primary_vein + secondary_vein).clamp(0.0, 1.0);
            (base, palette_color(colors, vein))
        }
        PreparedTexture::Wood {
            colors,
            origin,
            axis,
            rings_per_unit,
            ring_width,
            noise_scale,
            ring_warp,
            grain_scale,
            grain_strength,
            axis_u,
            axis_v,
            octaves,
            persistence,
            lacunarity,
            seed,
            translate,
            rotation,
            scale,
        } => {
            let point = map_texture_point(point, translate, rotation, scale);
            let offset = point - origin;
            let axial = offset.dot(axis);
            let radial = offset - axial * axis;
            let radial_u = radial.dot(axis_u);
            let radial_v = radial.dot(axis_v);
            let ring_frequency = *noise_scale * 0.42;
            let ring_warp_u = noise::fbm3(
                point.x * ring_frequency + 8.17,
                point.y * ring_frequency - 3.41,
                point.z * ring_frequency + 12.73,
                (*seed).wrapping_add(0xE35A7BD93E1B4C29),
                *octaves,
                *persistence,
                *lacunarity,
            );
            let ring_warp_v = noise::fbm3(
                point.x * ring_frequency - 14.27,
                point.y * ring_frequency + 5.19,
                point.z * ring_frequency + 1.63,
                (*seed).wrapping_add(0xB7E151628AED2A6B),
                *octaves,
                *persistence,
                *lacunarity,
            );
            let ring_displacement = *ring_warp * 0.35 / rings_per_unit.max(0.1);
            let warped_u = radial_u + ring_displacement * ring_warp_u;
            let warped_v = radial_v + ring_displacement * ring_warp_v;
            let radius = warped_u.hypot(warped_v);
            let ring_phase_noise = noise::fbm3(
                point.x * *noise_scale,
                point.y * *noise_scale,
                point.z * *noise_scale,
                (*seed).wrapping_add(0x6A09E667F3BCC909),
                *octaves,
                *persistence,
                *lacunarity,
            );
            let phase = std::f64::consts::TAU * *rings_per_unit * radius
                + *ring_warp * ring_phase_noise * 1.8;
            let ring = 0.5 + 0.5 * phase.sin();
            let latewood = if *ring_width <= 0.0 {
                if ring >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            } else {
                smoothstep(1.0 - *ring_width, 1.0, ring)
            };
            let grain_noise = noise::fbm3(
                radial_u * *grain_scale,
                radial_v * *grain_scale,
                axial * *grain_scale * 0.16,
                (*seed).wrapping_add(0x3C6EF372FE94F82B),
                *octaves,
                *persistence,
                *lacunarity,
            );
            let fine_grain = noise::fbm3(
                radial_u * *grain_scale * 2.7,
                radial_v * *grain_scale * 2.7,
                axial * *grain_scale * 0.42,
                (*seed).wrapping_add(0xBB67AE8584CAA73B),
                *octaves,
                *persistence,
                *lacunarity,
            );
            let grain = 0.5 + 0.5 * grain_noise;
            let natural_value = latewood * (0.72 + 0.28 * grain)
                + (1.0 - latewood) * (0.12 + 0.26 * grain)
                + *grain_strength * 0.08 * fine_grain;
            let value = latewood + *grain_strength * (natural_value - latewood);
            (base, palette_color(colors, value))
        }
    }
}

#[inline]
fn map_texture_point(
    point: &Point,
    translate: &Vec3,
    rotation: &Matrix3<f64>,
    scale: &Vec3,
) -> Vec3 {
    let rotated = rotation * (point.coords - translate);
    rotated.component_mul(scale)
}

#[inline]
fn smoothstep(edge0: f64, edge1: f64, value: f64) -> f64 {
    if (edge1 - edge0).abs() < 1e-12 {
        return if value >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn palette_color(colors: &[Color], value: f64) -> Color {
    match colors {
        [] => Color::new(1.0, 1.0, 1.0),
        [color] => *color,
        _ => {
            let position = value.clamp(0.0, 1.0) * (colors.len() - 1) as f64;
            let index = position.floor() as usize;
            let next_index = (index + 1).min(colors.len() - 1);
            let fraction = position - index as f64;
            colors[index] + (colors[next_index] - colors[index]) * fraction
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
        effective_material_and_color(material, shaded_hit.texture_coords, &shaded_hit.point);
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
        let (mat, color) =
            effective_material_and_color(&prepared, Some((0.0, 0.0)), &Point::origin());
        assert!((color.x - 1.0).abs() < 1e-3); // red
        assert!(color.y.abs() < 1e-3);
        assert!((mat.shininess - 32.0).abs() < 1e-6);
        assert!((mat.ambient - 0.1).abs() < 1e-6);
        assert!((mat.diffuse - 0.8).abs() < 1e-6);

        // At (1.0, 0.0): floor(1) + floor(0) = 1, 1 % 2 = 1 -> material_b (blue)
        let (mat, color) =
            effective_material_and_color(&prepared, Some((1.0, 0.0)), &Point::origin());
        assert!(color.x.abs() < 1e-3); // blue
        assert!((color.z - 1.0).abs() < 1e-3);
        assert!((mat.shininess - 16.0).abs() < 1e-6);
        assert!((mat.ambient - 0.2).abs() < 1e-6);
        assert!((mat.diffuse - 0.6).abs() < 1e-6);

        // At (0.0, 1.0): floor(0) + floor(1) = 1 -> material_b (blue)
        let (mat, color) =
            effective_material_and_color(&prepared, Some((0.0, 1.0)), &Point::origin());
        assert!(color.x.abs() < 1e-3);
        assert!((mat.shininess - 16.0).abs() < 1e-6);

        // At (1.0, 1.0): floor(1) + floor(1) = 2, 2 % 2 = 0 -> base_material (red)
        let (mat, color) =
            effective_material_and_color(&prepared, Some((1.0, 1.0)), &Point::origin());
        assert!((color.x - 1.0).abs() < 1e-3);
        assert!((mat.shininess - 32.0).abs() < 1e-6);

        // At (0.7, 0.3): floor(0.7) + floor(0.3) = 0 -> base_material
        let (_mat, color) =
            effective_material_and_color(&prepared, Some((0.7, 0.3)), &Point::origin());
        assert!((color.x - 1.0).abs() < 1e-3);

        // At (1.2, 0.8): floor(1.2) + floor(0.8) = 1 -> material_b
        let (_mat, color) =
            effective_material_and_color(&prepared, Some((1.2, 0.8)), &Point::origin());
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
        let (_m, color) =
            effective_material_and_color(&prepared, Some((0.0, 0.0)), &Point::origin());
        assert!((color.x - 1.0).abs() < 1e-3); // red
        assert!(color.y.abs() < 1e-3);

        // At (0.5, 0.5) we should NOT be on a grid line -> white
        let (_m, color) =
            effective_material_and_color(&prepared, Some((0.5, 0.5)), &Point::origin());
        assert!((color.x - 1.0).abs() < 1e-3); // white
        assert!((color.y - 1.0).abs() < 1e-3);
        assert!((color.z - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_marble_texture_without_warp_has_analytic_samples() {
        let material = Material {
            texture: Some(Texture::Marble {
                colors: vec!["#000000".to_string(), "#FFFFFF".to_string()],
                direction: [1.0, 0.0, 0.0],
                bands_per_unit: 0.5,
                noise_scale: 1.0,
                warp_strength: 0.0,
                vein_sharpness: 1.0,
                branch_strength: 0.0,
                octaves: 1,
                persistence: 0.5,
                lacunarity: 2.0,
                seed: 0,
                transform: TextureTransform::default(),
            }),
            ..Material::default()
        };
        let prepared = PreparedMaterial::from_material(&material);

        let sample = |x| effective_material_and_color(&prepared, None, &Point::new(x, 0.0, 0.0)).1;

        assert!(sample(-0.5).magnitude() < 1e-12);
        assert!((sample(0.0).x - 0.5).abs() < 1e-12);
        assert!((sample(0.5).x - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_marble_domain_warp_changes_transverse_profile() {
        let material = Material {
            texture: Some(Texture::Marble {
                colors: vec!["#000000".to_string(), "#FFFFFF".to_string()],
                direction: [1.0, 0.0, 0.0],
                bands_per_unit: 0.5,
                noise_scale: 0.8,
                warp_strength: 3.0,
                vein_sharpness: 2.4,
                branch_strength: 0.8,
                octaves: 4,
                persistence: 0.5,
                lacunarity: 2.0,
                seed: 19,
                transform: TextureTransform::default(),
            }),
            ..Material::default()
        };
        let prepared = PreparedMaterial::from_material(&material);
        let sample = |y| {
            effective_material_and_color(&prepared, None, &Point::new(0.25, y, 0.0))
                .1
                .x
        };
        let center = sample(-2.0);
        assert!([-1.5, -1.0, 0.0, 1.0, 1.5, 2.0]
            .into_iter()
            .any(|y| (sample(y) - center).abs() > 1e-4));
    }

    #[test]
    fn test_wood_texture_uses_cylindrical_rings() {
        let material = Material {
            texture: Some(Texture::Wood {
                colors: vec!["#000000".to_string(), "#FFFFFF".to_string()],
                origin: [0.0, 0.0, 0.0],
                axis: [0.0, 1.0, 0.0],
                rings_per_unit: 0.5,
                ring_width: 0.25,
                noise_scale: 1.0,
                ring_warp: 0.0,
                grain_scale: 1.0,
                grain_strength: 0.0,
                octaves: 1,
                persistence: 0.5,
                lacunarity: 2.0,
                seed: 0,
                transform: TextureTransform::default(),
            }),
            ..Material::default()
        };
        let prepared = PreparedMaterial::from_material(&material);

        let sample = |point| effective_material_and_color(&prepared, None, &point).1.x;

        assert!(sample(Point::new(0.0, 0.0, 0.0)).abs() < 1e-12);
        assert!((sample(Point::new(0.5, 0.0, 0.0)) - 1.0).abs() < 1e-12);
        assert!(sample(Point::new(1.0, 0.0, 0.0)).abs() < 1e-12);
        assert!(
            (sample(Point::new(0.5, 4.0, 0.0)) - sample(Point::new(0.5, -3.0, 0.0))).abs() < 1e-12
        );
    }

    #[test]
    fn test_wood_texture_adds_long_grain_variation() {
        let material = Material {
            texture: Some(Texture::Wood {
                colors: vec!["#000000".to_string(), "#FFFFFF".to_string()],
                origin: [0.0, 0.0, 0.0],
                axis: [1.0, 0.0, 0.0],
                rings_per_unit: 1.5,
                ring_width: 0.2,
                noise_scale: 1.0,
                ring_warp: 0.5,
                grain_scale: 2.0,
                grain_strength: 1.0,
                octaves: 4,
                persistence: 0.5,
                lacunarity: 2.0,
                seed: 23,
                transform: TextureTransform::default(),
            }),
            ..Material::default()
        };
        let prepared = PreparedMaterial::from_material(&material);
        let sample = |x| {
            effective_material_and_color(&prepared, None, &Point::new(x, 0.0, 0.35))
                .1
                .x
        };
        let center = sample(-2.0);
        assert!([-1.0, 0.0, 1.0, 2.0]
            .into_iter()
            .any(|x| (sample(x) - center).abs() > 1e-4));
    }
}
