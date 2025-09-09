use crate::mesh::{Mesh, Triangle};
use crate::scene::{Color, Point, Vec3};
use nalgebra::Unit;

/// A ray in 3D space
#[derive(Debug, Clone)]
pub struct Ray {
    pub origin: Point,
    pub direction: Unit<Vec3>,
}

impl Ray {
    pub fn new(origin: Point, direction: Vec3) -> Self {
        Self {
            origin,
            direction: Unit::new_normalize(direction),
        }
    }

    /// Get a point along the ray at parameter t
    pub fn at(&self, t: f64) -> Point {
        self.origin + t * self.direction.as_ref()
    }
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
        }
    }
}

/// Trait for objects that can be intersected by rays
pub trait Intersectable {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>;
    fn material_index(&self) -> usize;
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
        let inverse = transform_matrix.try_inverse().unwrap_or_else(nalgebra::Matrix4::identity);
        Self {
            center,
            half_size,
            transform: inverse, // Store world-to-local transform
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
            Point::new(-self.half_size.x, -self.half_size.y,  self.half_size.z),
            Point::new(-self.half_size.x,  self.half_size.y, -self.half_size.z),
            Point::new(-self.half_size.x,  self.half_size.y,  self.half_size.z),
            Point::new( self.half_size.x, -self.half_size.y, -self.half_size.z),
            Point::new( self.half_size.x, -self.half_size.y,  self.half_size.z),
            Point::new( self.half_size.x,  self.half_size.y, -self.half_size.z),
            Point::new( self.half_size.x,  self.half_size.y,  self.half_size.z),
        ];

        // Transform corners to world space and find AABB
        let mut min = Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for corner in &corners {
            // Transform corner to world space: center + rotation * corner
            let world_corner = self.center + (self.inverse_transform * corner.to_homogeneous()).xyz();
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
        assert!((hit_record.point.z - 1.0).abs() < 1e-10, "Hit should be at z=1 (top face)");
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
        
        assert!((min.x - (-expected_half_diagonal)).abs() < 1e-10, "Min X should be expanded");
        assert!((max.x - expected_half_diagonal).abs() < 1e-10, "Max X should be expanded");
        assert!((min.y - (-expected_half_diagonal)).abs() < 1e-10, "Min Y should be expanded");
        assert!((max.y - expected_half_diagonal).abs() < 1e-10, "Max Y should be expanded");
        
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
        assert!((hit_record.point.z - 1.0).abs() < 1e-10, "Hit should be at z=1");
        
        // Test bounds
        let (min, max) = cube.bounds();
        assert_eq!(min, Point::new(-1.0, -1.0, -1.0));
        assert_eq!(max, Point::new(1.0, 1.0, 1.0));
    }
}

impl Intersectable for Cube {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        // Transform ray to cube's local coordinate space
        let local_origin = self.center + (self.transform * (ray.origin - self.center).to_homogeneous()).xyz();
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
        let world_hit_point = self.center + (self.inverse_transform * local_hit_point.to_homogeneous()).xyz();
        
        // Transform normal back to world space (use inverse transpose for normals)
        let world_normal = if self.transform == nalgebra::Matrix4::identity() {
            normal
        } else {
            // For normals, we need the inverse transpose of the rotation part
            let rotation_part = self.transform.fixed_view::<3, 3>(0, 0);
            let normal_transform = rotation_part.try_inverse().unwrap_or_else(nalgebra::Matrix3::identity).transpose();
            normal_transform * normal
        };
        
        Some(HitRecord::new(
            world_hit_point,
            world_normal,
            t,
            ray,
            self.material_color,
            self.material_index,
        ))
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
    pub use_kdtree: bool, // New field to control k-d tree usage
}

impl MeshObject {
    pub fn new(mesh: Mesh, material_color: Color, material_index: usize) -> Self {
        Self {
            mesh,
            material_color,
            material_index,
            use_kdtree: true, // Default to using k-d tree
        }
    }

    /// Create a new MeshObject with k-d tree disabled (brute force intersection)
    pub fn new_brute_force(mesh: Mesh, material_color: Color, material_index: usize) -> Self {
        Self {
            mesh,
            material_color,
            material_index,
            use_kdtree: false, // Disable k-d tree
        }
    }

    /// Ray-triangle intersection using Möller-Trumbore algorithm
    fn intersect_triangle(
        &self,
        ray: &Ray,
        triangle: &Triangle,
        t_min: f64,
        t_max: f64,
    ) -> Option<(f64, Vec3, (f64, f64))> {
        let edge1 = triangle.vertices[1] - triangle.vertices[0];
        let edge2 = triangle.vertices[2] - triangle.vertices[0];
        let h = ray.direction.cross(&edge2);
        let a = edge1.dot(&h);

        if a > -1e-8 && a < 1e-8 {
            return None; // Ray is parallel to triangle
        }

        let f = 1.0 / a;
        let s = ray.origin - triangle.vertices[0];
        let u = f * s.dot(&h);

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(&edge1);
        let v = f * ray.direction.dot(&q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(&q);

        if t > t_min && t < t_max {
            // Compute normal from vertex geometry, considering vertex winding order
            let mut normal = edge1.cross(&edge2);

            // Ensure normal is not zero (degenerate triangle)
            if normal.magnitude() < 1e-8 {
                return None;
            }

            // The sign of 'a' tells us about vertex winding:
            // - If a > 0: vertices are counter-clockwise, normal points toward ray
            // - If a < 0: vertices are clockwise, normal points away from ray
            // We want the normal to point toward the "outside" of the mesh
            if a < 0.0 {
                normal = -normal;
            }

            normal = normal.normalize();

            Some((t, normal, (u, v)))
        } else {
            None
        }
    }

    /// Fast bounding box intersection test
    fn intersect_bounds(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        let (bounds_min, bounds_max) = self.mesh.bounds();

        let mut t_min_bound = t_min;
        let mut t_max_bound = t_max;

        for axis in 0..3 {
            let inv_dir = 1.0 / ray.direction[axis];
            let mut t0 = (bounds_min[axis] - ray.origin[axis]) * inv_dir;
            let mut t1 = (bounds_max[axis] - ray.origin[axis]) * inv_dir;

            if inv_dir < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }

            t_min_bound = t_min_bound.max(t0);
            t_max_bound = t_max_bound.min(t1);

            if t_min_bound > t_max_bound {
                return false;
            }
        }

        true
    }
}

impl Intersectable for MeshObject {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        // Simple bounding box check first
        if !self.intersect_bounds(ray, t_min, t_max) {
            return None;
        }

        let mut closest_hit = None;
        let mut closest_t = t_max;

        if self.use_kdtree {
            // Use k-d tree to find triangle candidates
            self.mesh
                .kdtree
                .traverse(&ray.origin, ray.direction.as_ref(), |triangle_indices| {
                    for &triangle_idx in triangle_indices {
                        let triangle = &self.mesh.triangles[triangle_idx];
                        if let Some((t, normal, (u, v))) =
                            self.intersect_triangle(ray, triangle, t_min, closest_t)
                        {
                            if t < closest_t {
                                closest_t = t;
                                let point = ray.at(t);
                                let mut hit_record = HitRecord::new(
                                    point,
                                    normal,
                                    t,
                                    ray,
                                    self.material_color,
                                    self.material_index,
                                );
                                hit_record.texture_coords = Some((u, v));
                                closest_hit = Some(hit_record);
                            }
                        }
                    }
                });
        } else {
            // Brute force: test all triangles
            for triangle in self.mesh.triangles.iter() {
                if let Some((t, normal, (u, v))) =
                    self.intersect_triangle(ray, triangle, t_min, closest_t)
                {
                    if t < closest_t {
                        closest_t = t;
                        let point = ray.at(t);
                        let mut hit_record = HitRecord::new(
                            point,
                            normal,
                            t,
                            ray,
                            self.material_color,
                            self.material_index,
                        );
                        hit_record.texture_coords = Some((u, v));
                        closest_hit = Some(hit_record);
                    }
                }
            }
        }

        closest_hit
    }

    fn material_index(&self) -> usize {
        self.material_index
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
}
