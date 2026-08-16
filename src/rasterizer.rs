//! Rasterizer module for fast preview rendering
//!
//! This module provides a basic triangle painting algorithm as an alternative to raytracing.
//! It's designed for fast preview rendering where speed is more important than visual quality.
//!
//! ## Features:
//! - Triangle rasterization with depth buffer and backface culling
//! - Automatic tessellation of geometric primitives (spheres, cubes, planes)
//! - Phong lighting model matching the raytracer
//! - Support for mesh objects and all transform operations
//!
//! ## Limitations:
//! - No shadows or reflections (preview mode only)
//! - No anti-aliasing
//! - No texture support

use image::{ImageBuffer, Rgb, RgbImage};
use nalgebra::{Matrix4, Point3, Unit, Vector3};

use crate::camera::Camera;
use crate::mesh::Mesh;
use crate::scene::{hex_to_color, Color, Material, Object, Point, Scene, Vec3};

// Tessellation parameters
const SPHERE_LAT_SEGMENTS: usize = 20;
const SPHERE_LON_SEGMENTS: usize = 20;
const PLANE_GRID_SIZE: usize = 10;

/// Rasterizer for fast preview rendering using triangle painting
pub struct Rasterizer {
    pub width: u32,
    pub height: u32,
}

/// Tessellated triangle with world coordinates and material
#[derive(Debug, Clone)]
struct WorldTriangle<'a> {
    pub vertices: [Point; 3],
    pub normal: Vec3,
    pub color: Color,
    pub material: &'a Material,
}

impl Rasterizer {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Rasterize a scene to an image
    pub fn render(&self, scene: &Scene) -> Result<RgbImage, Box<dyn std::error::Error>> {
        println!("Rasterizing {}×{} image...", self.width, self.height);

        // Get background color
        let background_color = if let Some(bg) = &scene.scene_settings.background_color {
            hex_to_color(bg)?
        } else {
            Color::new(0.0, 0.0, 0.0)
        };

        // Create camera with aspect ratio
        let aspect_ratio = self.width as f64 / self.height as f64;
        let camera = Camera::from_config(&scene.camera, aspect_ratio)?;
        let camera_pos = Point::new(
            scene.camera.position[0],
            scene.camera.position[1],
            scene.camera.position[2],
        );

        // Collect all triangles from the scene
        let mut triangles = Vec::new();

        for object in &scene.objects {
            match object {
                Object::Sphere {
                    center,
                    radius,
                    material,
                    transform,
                } => {
                    let color = hex_to_color(&material.color)?;
                    let sphere_tris = tessellate_sphere(
                        Point::new(center[0], center[1], center[2]),
                        *radius,
                        color,
                        material,
                    );
                    apply_transforms_and_collect(&mut triangles, sphere_tris, transform);
                }
                Object::Cube {
                    center,
                    size,
                    material,
                    transform,
                } => {
                    let color = hex_to_color(&material.color)?;
                    let cube_tris = tessellate_cube(
                        Point::new(center[0], center[1], center[2]),
                        Vec3::new(size[0], size[1], size[2]),
                        color,
                        material,
                    );
                    apply_transforms_and_collect(&mut triangles, cube_tris, transform);
                }
                Object::Plane {
                    point,
                    normal,
                    material,
                    transform,
                } => {
                    let color = hex_to_color(&material.color)?;
                    let plane_tris = tessellate_plane(
                        Point::new(point[0], point[1], point[2]),
                        Vec3::new(normal[0], normal[1], normal[2]),
                        color,
                        material,
                    );
                    apply_transforms_and_collect(&mut triangles, plane_tris, transform);
                }
                Object::Mesh {
                    mesh_data,
                    material,
                    transform,
                    ..
                } => {
                    if let Some(mesh) = mesh_data {
                        let color = hex_to_color(&material.color)?;
                        append_mesh_world_triangles(
                            &mut triangles,
                            mesh,
                            color,
                            material,
                            transform,
                        );
                    }
                }
            }
        }

        println!("Tessellated scene into {} triangles", triangles.len());

        // Rasterize triangles
        let image =
            self.rasterize_triangles(&triangles, &camera, &camera_pos, scene, background_color);

        Ok(image)
    }

    /// Rasterize a list of triangles
    fn rasterize_triangles(
        &self,
        triangles: &[WorldTriangle<'_>],
        camera: &Camera,
        camera_pos: &Point,
        scene: &Scene,
        background_color: Color,
    ) -> RgbImage {
        // Create depth buffer and color buffer
        let mut depth_buffer = vec![f64::INFINITY; (self.width * self.height) as usize];
        let mut color_buffer = vec![background_color; (self.width * self.height) as usize];
        // Store world position and normal for each pixel for lighting calculations
        let mut position_buffer = vec![None; (self.width * self.height) as usize];
        let mut normal_buffer = vec![None; (self.width * self.height) as usize];
        let mut material_buffer = vec![None; (self.width * self.height) as usize];

        // Project and rasterize each triangle
        let is_perspective = camera.is_perspective;

        for tri in triangles {
            self.rasterize_triangle(
                tri,
                camera,
                &mut depth_buffer,
                &mut position_buffer,
                &mut normal_buffer,
                &mut material_buffer,
                camera_pos,
                is_perspective,
            );
        }

        // Apply lighting to each pixel
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                if let (Some(pos), Some(normal), Some((color, material))) = (
                    position_buffer[idx],
                    normal_buffer[idx],
                    &material_buffer[idx],
                ) {
                    // Calculate Phong lighting
                    let lit_color =
                        self.calculate_lighting(&pos, &normal, color, material, camera_pos, scene);
                    color_buffer[idx] = lit_color;
                }
            }
        }

        // Convert to image
        let mut image = ImageBuffer::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let color = color_buffer[idx];
                let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
                let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
                let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;
                image.put_pixel(x, y, Rgb([r, g, b]));
            }
        }

        image
    }

    /// Calculate Phong lighting for a point
    fn calculate_lighting(
        &self,
        point: &Point,
        normal: &Vec3,
        material_color: &Color,
        material: &Material,
        camera_pos: &Point,
        scene: &Scene,
    ) -> Color {
        // Ambient component
        let ambient_color = hex_to_color(&scene.scene_settings.ambient_illumination.color)
            .unwrap_or(Color::new(1.0, 1.0, 1.0));
        let ambient = material.ambient
            * scene.scene_settings.ambient_illumination.intensity
            * ambient_color.component_mul(material_color);

        let mut total_light = ambient;

        // Process each light
        for light in &scene.lights {
            let light_pos = Point::new(light.position[0], light.position[1], light.position[2]);
            let light_color = hex_to_color(&light.color).unwrap_or(Color::new(1.0, 1.0, 1.0));

            let light_dir = Unit::new_normalize(light_pos - point);

            // Diffuse component
            let diffuse_strength = normal.dot(&light_dir).max(0.0);
            let diffuse = material.diffuse
                * diffuse_strength
                * light.intensity
                * light_color.component_mul(material_color);

            // Specular component (Phong model)
            let specular = if diffuse_strength > 0.0 {
                let view_dir = Unit::new_normalize(*camera_pos - point);
                let reflect_dir = reflect(&(-light_dir.as_ref()), normal);
                let spec_strength = view_dir.dot(&reflect_dir).max(0.0).powf(material.shininess);
                material.specular * spec_strength * light.intensity * light_color
            } else {
                Color::new(0.0, 0.0, 0.0)
            };

            total_light += diffuse + specular;
        }

        total_light
    }

    /// Rasterize a single triangle using basic scanline algorithm
    #[allow(clippy::too_many_arguments)]
    fn rasterize_triangle<'a>(
        &self,
        tri: &WorldTriangle<'a>,
        camera: &Camera,
        depth_buffer: &mut [f64],
        position_buffer: &mut [Option<Point>],
        normal_buffer: &mut [Option<Vec3>],
        material_buffer: &mut [Option<(Color, &'a Material)>],
        camera_pos: &Point,
        is_perspective: bool,
    ) {
        // Backface culling for perspective cameras
        if is_perspective {
            // For perspective: check if triangle faces the camera in world space
            let tri_center =
                (tri.vertices[0].coords + tri.vertices[1].coords + tri.vertices[2].coords) / 3.0;
            let to_camera = camera_pos - Point::from(tri_center);
            let dot = tri.normal.dot(&to_camera);

            if dot <= 0.0 {
                return; // Triangle faces away from camera
            }
        }

        // Project vertices to screen space
        let mut screen_verts = [Point3::new(0.0, 0.0, 0.0); 3];
        let mut depths = [0.0; 3];

        for i in 0..3 {
            let (screen_pos, depth) = camera.project_point(&tri.vertices[i]);

            // Convert from [0,1] UV space to pixel coordinates
            let px = screen_pos.x * (self.width - 1) as f64;
            let py = (1.0 - screen_pos.y) * (self.height - 1) as f64; // Flip Y

            screen_verts[i] = Point3::new(px, py, depth);
            depths[i] = depth;
        }

        // Screen-space backface culling (for orthographic) or degenerate triangle check
        let v0 = screen_verts[0];
        let v1 = screen_verts[1];
        let v2 = screen_verts[2];

        let cross = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);

        // Skip degenerate triangles (projected to a line or point)
        if cross.abs() < 0.001 {
            return;
        }

        // For perspective cameras, do world-space backface culling (already done above)
        // For orthographic cameras, skip screen-space backface culling as it's unreliable
        // The depth buffer will handle which faces are visible

        // Compute bounding box
        let min_x = v0.x.min(v1.x).min(v2.x).floor().max(0.0) as u32;
        let max_x = v0.x.max(v1.x).max(v2.x).ceil().min(self.width as f64 - 1.0) as u32;
        let min_y = v0.y.min(v1.y).min(v2.y).floor().max(0.0) as u32;
        let max_y =
            v0.y.max(v1.y)
                .max(v2.y)
                .ceil()
                .min(self.height as f64 - 1.0) as u32;

        // Rasterize pixels within bounding box
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let p = Point3::new(px as f64 + 0.5, py as f64 + 0.5, 0.0);

                // Compute barycentric coordinates
                if let Some((w0, w1, w2)) = barycentric(p, v0, v1, v2) {
                    if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                        // Interpolate depth
                        let depth = w0 * depths[0] + w1 * depths[1] + w2 * depths[2];

                        let idx = (py * self.width + px) as usize;

                        // Depth test
                        if depth < depth_buffer[idx] {
                            depth_buffer[idx] = depth;

                            // Interpolate world position
                            let world_pos = tri.vertices[0].coords * w0
                                + tri.vertices[1].coords * w1
                                + tri.vertices[2].coords * w2;
                            position_buffer[idx] = Some(Point::from(world_pos));
                            normal_buffer[idx] = Some(tri.normal);
                            material_buffer[idx] = Some((tri.color, tri.material));
                        }
                    }
                }
            }
        }
    }

    pub fn render_to_file(
        &self,
        scene: &Scene,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let image = self.render(scene)?;
        image.save(output_path)?;
        println!("Rasterized image saved to: {}", output_path);
        Ok(())
    }
}

/// Reflect a vector around a normal
fn reflect(incident: &Vec3, normal: &Vec3) -> Unit<Vec3> {
    let normal_unit = Unit::new_normalize(*normal);
    Unit::new_normalize(incident - 2.0 * incident.dot(&normal_unit) * normal_unit.as_ref())
}

/// Compute barycentric coordinates of point p with respect to triangle (a, b, c)
fn barycentric(
    p: Point3<f64>,
    a: Point3<f64>,
    b: Point3<f64>,
    c: Point3<f64>,
) -> Option<(f64, f64, f64)> {
    let v0 = Vector3::new(b.x - a.x, b.y - a.y, 0.0);
    let v1 = Vector3::new(c.x - a.x, c.y - a.y, 0.0);
    let v2 = Vector3::new(p.x - a.x, p.y - a.y, 0.0);

    let dot00 = v0.dot(&v0);
    let dot01 = v0.dot(&v1);
    let dot02 = v0.dot(&v2);
    let dot11 = v1.dot(&v1);
    let dot12 = v1.dot(&v2);

    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < 1e-10 {
        return None;
    }

    let inv_denom = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    let w = 1.0 - u - v;

    Some((w, u, v))
}

/// Apply transform matrix to a list of triangles
fn apply_transform_to_triangles(triangles: &mut [WorldTriangle<'_>], transform: &Matrix4<f64>) {
    let inverse_transpose = transform.try_inverse().map(|inverse| inverse.transpose());

    for tri in triangles {
        for vertex in &mut tri.vertices {
            let transformed = transform * vertex.to_homogeneous();
            *vertex = Point::new(transformed.x, transformed.y, transformed.z);
        }

        // Transform normal (use inverse transpose for normals)
        if let Some(inverse_transpose) = &inverse_transpose {
            let normal_homogeneous = inverse_transpose * tri.normal.to_homogeneous();
            tri.normal = Vec3::new(
                normal_homogeneous.x,
                normal_homogeneous.y,
                normal_homogeneous.z,
            )
            .normalize();
        }
    }
}

/// Apply optional transforms to triangles and extend the collection
fn apply_transforms_and_collect<'a>(
    triangles: &mut Vec<WorldTriangle<'a>>,
    mut new_triangles: Vec<WorldTriangle<'a>>,
    transform: &Option<Vec<String>>,
) {
    if let Some(transform_strings) = transform {
        if let Ok(transform_matrix) = crate::scene::parse_transforms(transform_strings) {
            apply_transform_to_triangles(&mut new_triangles, &transform_matrix);
        }
    }
    triangles.extend(new_triangles);
}

/// Append mesh triangles directly to avoid a second full-size allocation for large meshes.
fn append_mesh_world_triangles<'a>(
    triangles: &mut Vec<WorldTriangle<'a>>,
    mesh: &Mesh,
    color: Color,
    material: &'a Material,
    transform: &Option<Vec<String>>,
) {
    let start = triangles.len();
    triangles.reserve(mesh.triangles.len());
    triangles.extend(mesh.triangles.iter().map(|tri| WorldTriangle {
        vertices: tri.vertices,
        normal: tri.normal,
        color,
        material,
    }));

    if let Some(transform_strings) = transform {
        if let Ok(transform_matrix) = crate::scene::parse_transforms(transform_strings) {
            apply_transform_to_triangles(&mut triangles[start..], &transform_matrix);
        }
    }
}

/// Tessellate a sphere into triangles
fn tessellate_sphere(
    center: Point,
    radius: f64,
    color: Color,
    material: &Material,
) -> Vec<WorldTriangle<'_>> {
    let mut triangles = Vec::new();

    // Use UV sphere tessellation with reasonable detail
    let lat_segments = SPHERE_LAT_SEGMENTS;
    let lon_segments = SPHERE_LON_SEGMENTS;

    for lat in 0..lat_segments {
        let theta0 = std::f64::consts::PI * (lat as f64) / (lat_segments as f64);
        let theta1 = std::f64::consts::PI * ((lat + 1) as f64) / (lat_segments as f64);

        for lon in 0..lon_segments {
            let phi0 = 2.0 * std::f64::consts::PI * (lon as f64) / (lon_segments as f64);
            let phi1 = 2.0 * std::f64::consts::PI * ((lon + 1) as f64) / (lon_segments as f64);

            // Create two triangles for each quad
            let v00 = sphere_point(center, radius, theta0, phi0);
            let v01 = sphere_point(center, radius, theta0, phi1);
            let v10 = sphere_point(center, radius, theta1, phi0);
            let v11 = sphere_point(center, radius, theta1, phi1);

            // First triangle
            if lat > 0 {
                let normal = ((v00 - center) + (v01 - center) + (v10 - center)).normalize();
                triangles.push(WorldTriangle {
                    vertices: [v00, v01, v10],
                    normal,
                    color,
                    material,
                });
            }

            // Second triangle
            if lat < lat_segments - 1 {
                let normal = ((v01 - center) + (v11 - center) + (v10 - center)).normalize();
                triangles.push(WorldTriangle {
                    vertices: [v01, v11, v10],
                    normal,
                    color,
                    material,
                });
            }
        }
    }

    triangles
}

/// Get a point on a sphere surface
fn sphere_point(center: Point, radius: f64, theta: f64, phi: f64) -> Point {
    let x = radius * theta.sin() * phi.cos();
    let y = radius * theta.sin() * phi.sin();
    let z = radius * theta.cos();
    Point::new(center.x + x, center.y + y, center.z + z)
}

/// Tessellate a cube into triangles
fn tessellate_cube(
    center: Point,
    size: Vec3,
    color: Color,
    material: &Material,
) -> Vec<WorldTriangle<'_>> {
    let mut triangles = Vec::new();

    let half_size = size / 2.0;

    // Define 8 corners of the cube
    let corners = [
        Point::new(
            center.x - half_size.x,
            center.y - half_size.y,
            center.z - half_size.z,
        ),
        Point::new(
            center.x + half_size.x,
            center.y - half_size.y,
            center.z - half_size.z,
        ),
        Point::new(
            center.x + half_size.x,
            center.y + half_size.y,
            center.z - half_size.z,
        ),
        Point::new(
            center.x - half_size.x,
            center.y + half_size.y,
            center.z - half_size.z,
        ),
        Point::new(
            center.x - half_size.x,
            center.y - half_size.y,
            center.z + half_size.z,
        ),
        Point::new(
            center.x + half_size.x,
            center.y - half_size.y,
            center.z + half_size.z,
        ),
        Point::new(
            center.x + half_size.x,
            center.y + half_size.y,
            center.z + half_size.z,
        ),
        Point::new(
            center.x - half_size.x,
            center.y + half_size.y,
            center.z + half_size.z,
        ),
    ];

    // Define faces (two triangles per face)
    let faces = [
        // Front face (z+)
        ([4, 5, 6], Vec3::new(0.0, 0.0, 1.0)),
        ([4, 6, 7], Vec3::new(0.0, 0.0, 1.0)),
        // Back face (z-)
        ([0, 2, 1], Vec3::new(0.0, 0.0, -1.0)),
        ([0, 3, 2], Vec3::new(0.0, 0.0, -1.0)),
        // Right face (x+)
        ([1, 2, 6], Vec3::new(1.0, 0.0, 0.0)),
        ([1, 6, 5], Vec3::new(1.0, 0.0, 0.0)),
        // Left face (x-)
        ([0, 4, 7], Vec3::new(-1.0, 0.0, 0.0)),
        ([0, 7, 3], Vec3::new(-1.0, 0.0, 0.0)),
        // Top face (y+)
        ([3, 7, 6], Vec3::new(0.0, 1.0, 0.0)),
        ([3, 6, 2], Vec3::new(0.0, 1.0, 0.0)),
        // Bottom face (y-)
        ([0, 1, 5], Vec3::new(0.0, -1.0, 0.0)),
        ([0, 5, 4], Vec3::new(0.0, -1.0, 0.0)),
    ];

    for (indices, normal) in &faces {
        triangles.push(WorldTriangle {
            vertices: [
                corners[indices[0]],
                corners[indices[1]],
                corners[indices[2]],
            ],
            normal: *normal,
            color,
            material,
        });
    }

    triangles
}

/// Tessellate a plane into triangles (limited to 1000x1000 as per spec)
fn tessellate_plane(
    point: Point,
    normal: Vec3,
    color: Color,
    material: &Material,
) -> Vec<WorldTriangle<'_>> {
    let mut triangles = Vec::new();

    // Limit plane to 1000x1000 as specified
    let size = 1000.0;

    // Find two perpendicular vectors to the normal
    let normal_unit = normal.normalize();
    let up = if normal_unit.z.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };

    let right = normal_unit.cross(&up).normalize();
    let forward = right.cross(&normal_unit).normalize();

    // Create a grid of triangles on the plane
    let grid_size = PLANE_GRID_SIZE;
    let cell_size = size / grid_size as f64;

    for i in 0..grid_size {
        for j in 0..grid_size {
            let u0 = (i as f64 - grid_size as f64 / 2.0) * cell_size;
            let u1 = ((i + 1) as f64 - grid_size as f64 / 2.0) * cell_size;
            let v0 = (j as f64 - grid_size as f64 / 2.0) * cell_size;
            let v1 = ((j + 1) as f64 - grid_size as f64 / 2.0) * cell_size;

            let p00 = point + right * u0 + forward * v0;
            let p10 = point + right * u1 + forward * v0;
            let p01 = point + right * u0 + forward * v1;
            let p11 = point + right * u1 + forward * v1;

            // Two triangles per quad
            triangles.push(WorldTriangle {
                vertices: [p00, p10, p01],
                normal: normal_unit,
                color,
                material,
            });

            triangles.push(WorldTriangle {
                vertices: [p10, p11, p01],
                normal: normal_unit,
                color,
                material,
            });
        }
    }

    triangles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rasterizer_creation() {
        let rasterizer = Rasterizer::new(800, 600);
        assert_eq!(rasterizer.width, 800);
        assert_eq!(rasterizer.height, 600);
    }

    #[test]
    fn test_sphere_tessellation() {
        let center = Point::new(0.0, 0.0, 0.0);
        let radius = 1.0;
        let color = Color::new(1.0, 0.0, 0.0);
        let material = Material::default();
        let triangles = tessellate_sphere(center, radius, color, &material);

        // Should generate some triangles
        assert!(!triangles.is_empty());

        // All vertices should be approximately at radius distance from center
        for tri in &triangles {
            for vertex in &tri.vertices {
                let dist = (vertex - center).magnitude();
                assert!(
                    (dist - radius).abs() < 0.1,
                    "Vertex distance from center: {}",
                    dist
                );
            }
        }
    }

    #[test]
    fn test_cube_tessellation() {
        let center = Point::new(0.0, 0.0, 0.0);
        let size = Vec3::new(2.0, 2.0, 2.0);
        let color = Color::new(0.0, 1.0, 0.0);
        let material = Material::default();
        let triangles = tessellate_cube(center, size, color, &material);

        // Cube should have 12 triangles (2 per face, 6 faces)
        assert_eq!(triangles.len(), 12);
    }

    #[test]
    fn test_plane_tessellation() {
        let point = Point::new(0.0, 0.0, 0.0);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let color = Color::new(0.0, 0.0, 1.0);
        let material = Material::default();
        let triangles = tessellate_plane(point, normal, color, &material);

        // Should generate grid of triangles
        assert!(!triangles.is_empty());
    }

    #[test]
    fn test_barycentric() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);

        // Point at center of triangle
        let p = Point3::new(0.33, 0.33, 0.0);
        let coords = barycentric(p, a, b, c);
        assert!(coords.is_some());

        let (w0, w1, w2) = coords.unwrap();
        assert!(w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0);
        assert!((w0 + w1 + w2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn mesh_triangles_borrow_the_object_material() {
        let mesh = Mesh::from_stl_bytes(
            br"solid triangle
facet normal 0 0 1
outer loop
vertex 0 0 0
vertex 1 0 0
vertex 0 1 0
endloop
endfacet
endsolid triangle",
        )
        .unwrap();
        let material = Material::default();
        let mut triangles = Vec::new();

        append_mesh_world_triangles(
            &mut triangles,
            &mesh,
            Color::new(1.0, 1.0, 1.0),
            &material,
            &None,
        );

        assert_eq!(triangles.len(), 1);
        assert!(std::ptr::eq(triangles[0].material, &material));
    }
}
