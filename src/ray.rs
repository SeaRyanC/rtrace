use crate::mesh::Mesh;
use crate::noise;
use crate::scene::{Color, MeshTopBottomPerlin, Point, SurfacePerlinNoise, Vec3};
use nalgebra::Unit;

/// A ray in 3D space
#[derive(Debug, Clone)]
pub struct Ray {
    pub origin: Point,
    pub direction: Unit<Vec3>,
    /// Pre-computed inverse direction for fast AABB intersection
    pub inv_direction: Vec3,
}

impl Ray {
    pub fn new(origin: Point, direction: Vec3) -> Self {
        let direction = Unit::new_normalize(direction);
        // Pre-compute inverse direction for fast AABB intersection tests.
        // Division by zero produces +/- infinity, which is handled correctly
        // by ray_intersects_bounds_fast() via is_infinite() checks.
        let inv_direction = Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);
        Self {
            origin,
            direction,
            inv_direction,
        }
    }

    /// Get a point along the ray at parameter t
    pub fn at(&self, t: f64) -> Point {
        self.origin + t * self.direction.as_ref()
    }
}

/// Result of a ray-object intersection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Sphere,
    Plane,
    Cube,
    Mesh,
}

/// Result of a ray-object intersection
#[derive(Debug, Clone)]
pub struct HitRecord {
    pub point: Point,
    pub normal: Unit<Vec3>,
    pub t: f64,
    pub front_face: bool,
    pub material_color: Color,
    pub material_index: usize,
    pub texture_coords: Option<(f64, f64)>, // u, v coordinates for texturing
    pub color_modulation: Color,
    pub primitive_kind: PrimitiveKind,
}

impl HitRecord {
    pub fn new(
        point: Point,
        outward_normal: Vec3,
        t: f64,
        ray: &Ray,
        material_color: Color,
        material_index: usize,
    ) -> Self {
        let front_face = ray.direction.dot(&outward_normal) < 0.0;
        let normal = if front_face {
            Unit::new_normalize(outward_normal)
        } else {
            Unit::new_normalize(-outward_normal)
        };

        Self {
            point,
            normal,
            t,
            front_face,
            material_color,
            material_index,
            texture_coords: None,
            color_modulation: Color::new(1.0, 1.0, 1.0),
            primitive_kind: PrimitiveKind::Sphere,
        }
    }
}

/// Trait for objects that can be intersected by rays
pub trait Intersectable {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>;
    fn material_index(&self) -> usize;
    /// Check if the ray hits anything in the range (for shadow rays).
    /// Default implementation uses hit(), but can be overridden for early termination.
    fn any_hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        self.hit(ray, t_min, t_max).is_some()
    }
}

/// Sphere primitive
pub struct Sphere {
    pub center: Point,
    pub radius: f64,
    pub material_color: Color,
    pub material_index: usize,
}

impl Sphere {
    /// Get the bounding box of the sphere
    pub fn bounds(&self) -> (Point, Point) {
        let r = Vec3::new(self.radius, self.radius, self.radius);
        (self.center - r, self.center + r)
    }
}

impl Intersectable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let oc = ray.origin - self.center;
        let a = ray.direction.dot(&ray.direction);
        let half_b = oc.dot(&ray.direction);
        let c = oc.dot(&oc) - self.radius * self.radius;

        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrtd = discriminant.sqrt();
        let mut root = (-half_b - sqrtd) / a;
        if root < t_min || t_max < root {
            root = (-half_b + sqrtd) / a;
            if root < t_min || t_max < root {
                return None;
            }
        }

        let point = ray.at(root);
        let outward_normal = (point - self.center) / self.radius;

        Some(HitRecord::new(
            point,
            outward_normal,
            root,
            ray,
            self.material_color,
            self.material_index,
        ))
    }

    fn material_index(&self) -> usize {
        self.material_index
    }
}

/// Plane primitive
pub struct Plane {
    pub point: Point,
    pub normal: Unit<Vec3>,
    pub material_color: Color,
    pub material_index: usize,
}

impl Intersectable for Plane {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let denom = self.normal.dot(&ray.direction);

        // Ray is parallel to plane
        if denom.abs() < 1e-8 {
            return None;
        }

        let t = (self.point - ray.origin).dot(&self.normal) / denom;

        if t < t_min || t > t_max {
            return None;
        }

        let point = ray.at(t);
        let mut hit_record = HitRecord::new(
            point,
            *self.normal.as_ref(),
            t,
            ray,
            self.material_color,
            self.material_index,
        );
        hit_record.primitive_kind = PrimitiveKind::Plane;

        // Calculate texture coordinates for the plane (simple projection)
        let u_axis = if self.normal.x.abs() > 0.9 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        let u_axis = Unit::new_normalize(u_axis.cross(&self.normal));
        let v_axis = Unit::new_normalize(self.normal.cross(&u_axis));

        let relative_pos = point - self.point;
        let u = relative_pos.dot(&u_axis);
        let v = relative_pos.dot(&v_axis);

        hit_record.texture_coords = Some((u, v));

        Some(hit_record)
    }

    fn material_index(&self) -> usize {
        self.material_index
    }
}

/// Oriented box (cube) primitive - supports rotation
pub struct Cube {
    pub center: Point,
    pub half_size: Vec3,
    pub transform: nalgebra::Matrix4<f64>, // World to local transform
    pub inverse_transform: nalgebra::Matrix4<f64>, // Local to world transform
    pub material_color: Color,
    pub material_index: usize,
}

impl Cube {
    pub fn new(center: Point, size: Vec3, material_color: Color, material_index: usize) -> Self {
        let half_size = size / 2.0;
        let transform = nalgebra::Matrix4::identity();
        Self {
            center,
            half_size,
            transform,
            inverse_transform: transform,
            material_color,
            material_index,
        }
    }

    /// Create a new oriented cube with a transform matrix
    pub fn new_with_transform(
        center: Point,
        size: Vec3,
        transform_matrix: nalgebra::Matrix4<f64>,
        material_color: Color,
        material_index: usize,
    ) -> Self {
        let half_size = size / 2.0;
        let inverse = transform_matrix
            .try_inverse()
            .unwrap_or_else(nalgebra::Matrix4::identity);
        Self {
            center,
            half_size,
            transform: inverse,                  // Store world-to-local transform
            inverse_transform: transform_matrix, // Store local-to-world transform
            material_color,
            material_index,
        }
    }

    /// Get the axis-aligned bounding box of the oriented cube in world space
    pub fn bounds(&self) -> (Point, Point) {
        // If no rotation, use simple AABB
        if self.transform == nalgebra::Matrix4::identity() {
            return (self.center - self.half_size, self.center + self.half_size);
        }

        // For oriented cubes, we need to transform all 8 corners and find the AABB
        let corners = [
            Point::new(-self.half_size.x, -self.half_size.y, -self.half_size.z),
            Point::new(-self.half_size.x, -self.half_size.y, self.half_size.z),
            Point::new(-self.half_size.x, self.half_size.y, -self.half_size.z),
            Point::new(-self.half_size.x, self.half_size.y, self.half_size.z),
            Point::new(self.half_size.x, -self.half_size.y, -self.half_size.z),
            Point::new(self.half_size.x, -self.half_size.y, self.half_size.z),
            Point::new(self.half_size.x, self.half_size.y, -self.half_size.z),
            Point::new(self.half_size.x, self.half_size.y, self.half_size.z),
        ];

        // Transform corners to world space and find AABB
        let mut min = Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for corner in &corners {
            // Transform corner to world space: center + rotation * corner
            let world_corner =
                self.center + (self.inverse_transform * corner.to_homogeneous()).xyz();
            min.x = min.x.min(world_corner.x);
            min.y = min.y.min(world_corner.y);
            min.z = min.z.min(world_corner.z);
            max.x = max.x.max(world_corner.x);
            max.y = max.y.max(world_corner.y);
            max.z = max.z.max(world_corner.z);
        }

        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Color, Point, Vec3};

    #[test]
    fn test_sphere_bounds() {
        let sphere = Sphere {
            center: Point::new(1.0, 2.0, 3.0),
            radius: 1.5,
            material_color: Color::new(1.0, 0.0, 0.0),
            material_index: 0,
        };

        let (min, max) = sphere.bounds();
        assert_eq!(min, Point::new(-0.5, 0.5, 1.5));
        assert_eq!(max, Point::new(2.5, 3.5, 4.5));
    }

    #[test]
    fn test_cube_bounds() {
        let cube = Cube::new(
            Point::new(1.0, 2.0, 3.0),
            Vec3::new(2.0, 4.0, 6.0),
            Color::new(0.0, 1.0, 0.0),
            0,
        );

        let (min, max) = cube.bounds();
        assert_eq!(min, Point::new(0.0, 0.0, 0.0));
        assert_eq!(max, Point::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn test_cube_rotation_z() {
        use nalgebra::Matrix4;

        // Create a 45-degree rotation around Z-axis
        let rotation_matrix = Matrix4::from_euler_angles(0.0, 0.0, 45.0_f64.to_radians());

        let cube = Cube::new_with_transform(
            Point::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
            rotation_matrix,
            Color::new(1.0, 0.0, 0.0),
            0,
        );

        // Test ray intersection from above should still work
        let ray = Ray::new(Point::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = cube.hit(&ray, 0.001, 1000.0);

        assert!(hit.is_some(), "Ray should intersect rotated cube");

        let hit_record = hit.unwrap();
        assert!(
            (hit_record.point.z - 1.0).abs() < 1e-10,
            "Hit should be at z=1 (top face)"
        );
        assert!(hit_record.point.x.abs() < 1e-10, "Hit x should be near 0");
        assert!(hit_record.point.y.abs() < 1e-10, "Hit y should be near 0");
    }

    #[test]
    fn test_cube_rotation_bounds() {
        use nalgebra::Matrix4;

        // Test that rotating a cube around Z-axis expands its bounding box correctly
        let rotation_matrix = Matrix4::from_euler_angles(0.0, 0.0, 45.0_f64.to_radians());

        let cube = Cube::new_with_transform(
            Point::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0), // 2x2x2 cube
            rotation_matrix,
            Color::new(1.0, 0.0, 0.0),
            0,
        );

        let (min, max) = cube.bounds();

        // When a 2x2 square is rotated 45 degrees, its diagonal becomes the new width/height
        // Diagonal = sqrt(2^2 + 2^2) = sqrt(8) = 2*sqrt(2) ≈ 2.828
        let expected_half_diagonal = 2.0_f64.sqrt();

        assert!(
            (min.x - (-expected_half_diagonal)).abs() < 1e-10,
            "Min X should be expanded"
        );
        assert!(
            (max.x - expected_half_diagonal).abs() < 1e-10,
            "Max X should be expanded"
        );
        assert!(
            (min.y - (-expected_half_diagonal)).abs() < 1e-10,
            "Min Y should be expanded"
        );
        assert!(
            (max.y - expected_half_diagonal).abs() < 1e-10,
            "Max Y should be expanded"
        );

        // Z bounds should remain unchanged
        assert!((min.z - (-1.0)).abs() < 1e-10, "Min Z should be -1");
        assert!((max.z - 1.0).abs() < 1e-10, "Max Z should be 1");
    }

    #[test]
    fn test_cube_no_transform_identity() {
        // Test that cubes without transforms behave identically to before
        let cube = Cube::new(
            Point::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
            Color::new(1.0, 0.0, 0.0),
            0,
        );

        // Test ray intersection
        let ray = Ray::new(Point::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = cube.hit(&ray, 0.001, 1000.0);

        assert!(hit.is_some(), "Ray should intersect unrotated cube");

        let hit_record = hit.unwrap();
        assert!(
            (hit_record.point.z - 1.0).abs() < 1e-10,
            "Hit should be at z=1"
        );

        // Test bounds
        let (min, max) = cube.bounds();
        assert_eq!(min, Point::new(-1.0, -1.0, -1.0));
        assert_eq!(max, Point::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_cube_positioning() {
        // Test that cubes are positioned correctly at non-origin locations
        let cube = Cube::new(
            Point::new(5.0, 3.0, 2.0), // Cube center at (5, 3, 2)
            Vec3::new(2.0, 2.0, 2.0),  // 2x2x2 size
            Color::new(1.0, 0.0, 0.0),
            0,
        );

        // Test ray intersection from above the cube
        let ray = Ray::new(Point::new(5.0, 3.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = cube.hit(&ray, 0.001, 1000.0);

        assert!(
            hit.is_some(),
            "Ray should intersect cube at correct position"
        );

        let hit_record = hit.unwrap();
        // Should hit the top face at z = center.z + half_size.z = 2 + 1 = 3
        assert!(
            (hit_record.point.z - 3.0).abs() < 1e-10,
            "Hit should be at z=3 (top face of cube at center z=2)"
        );
        assert!(
            (hit_record.point.x - 5.0).abs() < 1e-10,
            "Hit x should be at cube center x=5"
        );
        assert!(
            (hit_record.point.y - 3.0).abs() < 1e-10,
            "Hit y should be at cube center y=3"
        );

        // Test bounds - should be centered around (5, 3, 2)
        let (min, max) = cube.bounds();
        assert_eq!(min, Point::new(4.0, 2.0, 1.0)); // center - half_size
        assert_eq!(max, Point::new(6.0, 4.0, 3.0)); // center + half_size
    }

    #[test]
    fn test_layer_random_is_deterministic() {
        let a = layer_random_signed(12, 0xAA55_1020);
        let b = layer_random_signed(12, 0xAA55_1020);
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn test_adjacent_layers_are_decorrelated() {
        let a = layer_random_signed(100, 0x13C7_4D89);
        let b = layer_random_signed(101, 0x13C7_4D89);
        assert!((a - b).abs() > 1e-6);
    }
}

impl Intersectable for Cube {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        // Transform ray to cube's local coordinate space
        let local_origin =
            Point::from((self.transform * (ray.origin - self.center).to_homogeneous()).xyz());
        let local_direction = (self.transform * ray.direction.to_homogeneous()).xyz();

        // Handle degenerate direction (shouldn't happen with normalized rays, but be safe)
        if local_direction.magnitude() < 1e-8 {
            return None;
        }

        let local_ray = Ray::new(local_origin, local_direction);

        // Perform intersection against axis-aligned box in local space
        let mut t_min_hit = t_min;
        let mut t_max_hit = t_max;
        let mut normal = Vec3::new(0.0, 0.0, 0.0);
        let mut _hit_front = true;

        // Check intersection with each pair of parallel planes (in local space)
        for axis in 0..3 {
            let inv_dir = 1.0 / local_ray.direction[axis];
            let mut t0 = (-self.half_size[axis] - local_ray.origin[axis]) * inv_dir;
            let mut t1 = (self.half_size[axis] - local_ray.origin[axis]) * inv_dir;

            let mut axis_normal = Vec3::new(0.0, 0.0, 0.0);
            axis_normal[axis] = if inv_dir < 0.0 { 1.0 } else { -1.0 };

            if inv_dir < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
                axis_normal[axis] = -axis_normal[axis];
            }

            if t0 > t_min_hit {
                t_min_hit = t0;
                normal = axis_normal;
                _hit_front = true;
            }

            if t1 < t_max_hit {
                t_max_hit = t1;
            }

            if t_min_hit > t_max_hit {
                return None;
            }
        }

        let t = if t_min_hit > t_min {
            t_min_hit
        } else {
            t_max_hit
        };
        if t < t_min || t > t_max {
            return None;
        }

        // Calculate hit point in local space
        let local_hit_point = local_ray.at(t);

        // Transform hit point back to world space
        let world_hit_point =
            self.center + (self.inverse_transform * local_hit_point.to_homogeneous()).xyz();

        // Transform normal back to world space (use inverse transpose for normals)
        let world_normal = if self.transform == nalgebra::Matrix4::identity() {
            normal
        } else {
            // For normals, we need the inverse transpose of the rotation part
            let rotation_part = self.transform.fixed_view::<3, 3>(0, 0);
            let normal_transform = rotation_part
                .try_inverse()
                .unwrap_or_else(nalgebra::Matrix3::identity)
                .transpose();
            normal_transform * normal
        };

        let mut hit_record = HitRecord::new(
            world_hit_point,
            world_normal,
            t,
            ray,
            self.material_color,
            self.material_index,
        );
        hit_record.primitive_kind = PrimitiveKind::Cube;
        Some(hit_record)
    }

    fn material_index(&self) -> usize {
        self.material_index
    }
}

/// Triangle mesh primitive
pub struct MeshObject {
    pub mesh: Mesh,
    pub material_color: Color,
    pub material_index: usize,
    /// When false, uses brute-force intersection (useful for validation).
    pub use_bvh: bool,
    pub print_effects: MeshPrintEffects,
}

#[derive(Clone)]
pub struct MeshPrintEffects {
    pub print_direction: Unit<Vec3>,
    pub layer_line_thickness: f64,
    pub layer_jitter: f64,
    pub top_bottom_perlin: Option<MeshTopBottomPerlin>,
    pub print_u_axis: Unit<Vec3>,
    pub print_v_axis: Unit<Vec3>,
    pub min_projection: f64,
    pub max_projection: f64,
}

impl MeshObject {
    pub fn new(
        mesh: Mesh,
        material_color: Color,
        material_index: usize,
        print_direction: [f64; 3],
        layer_line_thickness: f64,
        layer_jitter: f64,
        top_bottom_perlin: Option<MeshTopBottomPerlin>,
    ) -> Self {
        let print_effects = MeshPrintEffects::from_mesh(
            &mesh,
            print_direction,
            layer_line_thickness,
            layer_jitter,
            top_bottom_perlin,
        );
        Self {
            mesh,
            material_color,
            material_index,
            use_bvh: true,
            print_effects,
        }
    }

    /// Create a MeshObject that always uses brute-force intersection (for testing).
    pub fn new_brute_force(
        mesh: Mesh,
        material_color: Color,
        material_index: usize,
        print_direction: [f64; 3],
        layer_line_thickness: f64,
        layer_jitter: f64,
        top_bottom_perlin: Option<MeshTopBottomPerlin>,
    ) -> Self {
        let print_effects = MeshPrintEffects::from_mesh(
            &mesh,
            print_direction,
            layer_line_thickness,
            layer_jitter,
            top_bottom_perlin,
        );
        Self {
            mesh,
            material_color,
            material_index,
            use_bvh: false,
            print_effects,
        }
    }

    fn apply_mesh_surface_effects(&self, hit: &mut HitRecord) {
        hit.primitive_kind = PrimitiveKind::Mesh;
        let effects = &self.print_effects;
        let axis_coord = hit.point.coords.dot(effects.print_direction.as_ref());
        let u = hit.point.coords.dot(effects.print_u_axis.as_ref());
        let v = hit.point.coords.dot(effects.print_v_axis.as_ref());

        let mut normal = hit.normal;
        apply_layer_line_deflection(
            &mut normal,
            hit.normal,
            &effects.print_direction,
            effects.layer_line_thickness,
            effects.layer_jitter,
            axis_coord,
            u,
            v,
        );

        if let Some(top_bottom) = &effects.top_bottom_perlin {
            let distance_to_bottom = axis_coord - effects.min_projection;
            let distance_to_top = effects.max_projection - axis_coord;
            let distance = distance_to_bottom.min(distance_to_top);
            let depth = top_bottom.depth.max(1e-6);
            if distance <= depth {
                let blend = (1.0 - (distance / depth)).clamp(0.0, 1.0);
                apply_perlin_surface_effects(
                    &mut normal,
                    &mut hit.color_modulation,
                    &top_bottom.perlin,
                    u,
                    v,
                    effects.print_u_axis.as_ref(),
                    effects.print_v_axis.as_ref(),
                    blend,
                );
            }
        }

        hit.normal = normal;
    }

    /// Build a HitRecord from a BVH hit result.
    #[inline]
    fn make_hit_record(
        &self,
        ray: &Ray,
        t: f64,
        normal_arr: [f64; 3],
        u: f64,
        v: f64,
    ) -> HitRecord {
        let point = ray.at(t);
        let normal = Vec3::new(normal_arr[0], normal_arr[1], normal_arr[2]);
        let mut hit_record = HitRecord::new(
            point,
            normal,
            t,
            ray,
            self.material_color,
            self.material_index,
        );
        hit_record.texture_coords = Some((u, v));
        self.apply_mesh_surface_effects(&mut hit_record);
        hit_record
    }
}

impl MeshPrintEffects {
    fn from_mesh(
        mesh: &Mesh,
        print_direction: [f64; 3],
        layer_line_thickness: f64,
        layer_jitter: f64,
        top_bottom_perlin: Option<MeshTopBottomPerlin>,
    ) -> Self {
        let mut dir = Vec3::new(print_direction[0], print_direction[1], print_direction[2]);
        if dir.magnitude_squared() < 1e-12 {
            dir = Vec3::new(0.0, 0.0, 1.0);
        }
        let print_direction = Unit::new_normalize(dir);
        let (print_u_axis, print_v_axis) = tangent_basis_from_direction(&print_direction);

        let mut min_projection = f64::INFINITY;
        let mut max_projection = f64::NEG_INFINITY;

        for tri in &mesh.triangles {
            for v in &tri.vertices {
                let projection = v.coords.dot(print_direction.as_ref());
                min_projection = min_projection.min(projection);
                max_projection = max_projection.max(projection);
            }
        }

        if !min_projection.is_finite() || !max_projection.is_finite() {
            min_projection = 0.0;
            max_projection = 0.0;
        }

        Self {
            print_direction,
            layer_line_thickness: layer_line_thickness.max(1e-4),
            layer_jitter: layer_jitter.max(0.0),
            top_bottom_perlin,
            print_u_axis,
            print_v_axis,
            min_projection,
            max_projection,
        }
    }
}

fn tangent_basis_from_direction(direction: &Unit<Vec3>) -> (Unit<Vec3>, Unit<Vec3>) {
    let helper = if direction.z.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let u = Unit::new_normalize(direction.cross(&helper));
    let v = Unit::new_normalize(direction.cross(u.as_ref()));
    (u, v)
}

#[inline]
fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

#[inline]
fn layer_random_signed(layer_index: i64, salt: u64) -> f64 {
    let bits = mix64((layer_index as u64) ^ salt);
    let unit = bits as f64 / u64::MAX as f64; // [0, 1]
    unit * 2.0 - 1.0 // [-1, 1]
}

fn apply_layer_line_deflection(
    shading_normal: &mut Unit<Vec3>,
    geometric_normal: Unit<Vec3>,
    print_direction: &Unit<Vec3>,
    layer_line_thickness: f64,
    layer_jitter: f64,
    axis_coord: f64,
    u: f64,
    v: f64,
) {
    if layer_jitter <= 0.0 {
        return;
    }

    let layer_coord = axis_coord / layer_line_thickness;
    let layer_index = layer_coord.floor() as i64;
    let layer_pos = layer_coord.rem_euclid(1.0);

    // Per-layer deterministic jitter: each layer gets an independent random offset.
    let ridge_offset = layer_random_signed(layer_index, 0xA4C9_11D2_7F31_DA4B) * 0.22;
    let shifted = (layer_pos + ridge_offset).rem_euclid(1.0);
    let dist_to_boundary = shifted.min(1.0 - shifted);
    let ridge_strength = (1.0 - (dist_to_boundary * 2.0)).clamp(0.0, 1.0);

    let mut tangent = print_direction.cross(geometric_normal.as_ref());
    if tangent.magnitude_squared() < 1e-12 {
        let (u_axis, _) = tangent_basis_from_direction(print_direction);
        tangent = *u_axis.as_ref();
    } else {
        tangent = tangent.normalize();
    }

    let layer_bias = layer_random_signed(layer_index, 0x8B7D_4F1A_61E2_4C93);
    let layer_seed = mix64((layer_index as u64) ^ 0xC2B2_AE35_87F4_A9D1);
    let micro_noise = noise::fbm2(
        u * 2.5 + layer_bias * 1.3,
        v * 2.5 - layer_bias * 0.9,
        layer_seed,
        2,
        0.5,
        2.0,
    ) * 0.2;
    let layer_noise = (layer_bias * 0.85 + micro_noise * 0.15).clamp(-1.0, 1.0);

    let deflection = layer_jitter * ridge_strength * layer_noise * 0.35;
    let candidate = Unit::new_normalize(*geometric_normal.as_ref() + tangent * deflection);
    *shading_normal = if candidate.dot(geometric_normal.as_ref()) >= 0.0 {
        candidate
    } else {
        Unit::new_normalize(-candidate.as_ref())
    };
}

pub fn apply_perlin_surface_effects(
    shading_normal: &mut Unit<Vec3>,
    color_modulation: &mut Color,
    perlin: &SurfacePerlinNoise,
    u: f64,
    v: f64,
    tangent_u: &Vec3,
    tangent_v: &Vec3,
    blend: f64,
) {
    let freq = perlin.frequency.max(1e-6);
    let octaves = perlin.octaves.max(1);
    let persistence = perlin.persistence;
    let lacunarity = perlin.lacunarity;
    let base_noise = noise::fbm2(
        u * freq,
        v * freq,
        perlin.seed,
        octaves,
        persistence,
        lacunarity,
    );

    if perlin.color_strength != 0.0 {
        let tint = (1.0 + base_noise * perlin.color_strength * blend).max(0.0);
        *color_modulation = color_modulation.component_mul(&Color::new(tint, tint, tint));
    }

    if perlin.bump_strength != 0.0 {
        let eps = 0.01;
        let du = (noise::fbm2(
            (u + eps) * freq,
            v * freq,
            perlin.seed,
            octaves,
            persistence,
            lacunarity,
        ) - base_noise)
            / eps;
        let dv = (noise::fbm2(
            u * freq,
            (v + eps) * freq,
            perlin.seed,
            octaves,
            persistence,
            lacunarity,
        ) - base_noise)
            / eps;

        let bump = (*tangent_u * du + *tangent_v * dv) * (perlin.bump_strength * blend * 0.2);
        let candidate = Unit::new_normalize(*shading_normal.as_ref() + bump);
        if candidate.dot(shading_normal.as_ref()) >= 0.0 {
            *shading_normal = candidate;
        }
    }
}

impl Intersectable for MeshObject {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let origin = [ray.origin.x, ray.origin.y, ray.origin.z];
        let dir = [ray.direction.x, ray.direction.y, ray.direction.z];
        let inv = [
            ray.inv_direction.x,
            ray.inv_direction.y,
            ray.inv_direction.z,
        ];

        if self.use_bvh {
            self.mesh
                .bvh
                .hit_closest(&origin, &dir, &inv, t_min, t_max)
                .map(|(t, _tri_idx, normal, u, v)| self.make_hit_record(ray, t, normal, u, v))
        } else {
            // Brute-force path (for validation only): test all triangles.
            let precomputed = &self.mesh.bvh.precomputed;
            let mut best: Option<(f64, [f64; 3], f64, f64)> = None;
            let mut best_t = t_max;

            for tri in precomputed {
                if let Some((t, normal, u, v)) =
                    crate::mesh::mt_intersect_pub(tri, &origin, &dir, t_min, best_t)
                {
                    best_t = t;
                    best = Some((t, normal, u, v));
                }
            }

            best.map(|(t, normal, u, v)| self.make_hit_record(ray, t, normal, u, v))
        }
    }

    fn material_index(&self) -> usize {
        self.material_index
    }

    fn any_hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        let origin = [ray.origin.x, ray.origin.y, ray.origin.z];
        let dir = [ray.direction.x, ray.direction.y, ray.direction.z];
        let inv = [
            ray.inv_direction.x,
            ray.inv_direction.y,
            ray.inv_direction.z,
        ];

        if self.use_bvh {
            self.mesh.bvh.hit_any(&origin, &dir, &inv, t_min, t_max)
        } else {
            // Brute-force with early termination.
            for tri in &self.mesh.bvh.precomputed {
                if crate::mesh::mt_intersect_any_pub(tri, &origin, &dir, t_min, t_max) {
                    return true;
                }
            }
            false
        }
    }
}

/// Collection of intersectable objects
#[derive(Default)]
pub struct World {
    pub objects: Vec<Box<dyn Intersectable + Send + Sync>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn add(&mut self, object: Box<dyn Intersectable + Send + Sync>) {
        self.objects.push(object);
    }

    pub fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let mut closest_hit = None;
        let mut closest_so_far = t_max;

        for object in &self.objects {
            if let Some(hit) = object.hit(ray, t_min, closest_so_far) {
                closest_so_far = hit.t;
                closest_hit = Some(hit);
            }
        }

        closest_hit
    }

    /// Check if the ray hits any object in the world (for shadow rays).
    /// Returns early on first hit without finding the closest intersection.
    pub fn any_hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        for object in &self.objects {
            if object.any_hit(ray, t_min, t_max) {
                return true;
            }
        }
        false
    }
}
