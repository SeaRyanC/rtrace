use rtrace::{
    mesh::Mesh,
    ray::{Ray, MeshObject, Intersectable},
    scene::{Point, Vec3, Color},
};

fn intersect_triangle_moller_trumbore(ray: &Ray, triangle: &rtrace::mesh::Triangle) -> Option<(f64, Vec3, (f64, f64))> {
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

    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = s.cross(&edge1);
    let v = f * ray.direction.dot(&q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(&q);

    if t > 0.001 {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Debugging a single problematic ray");
    
    // Load plus.stl mesh
    let mesh = Mesh::from_stl_file("examples/plus.stl")?;
    println!("Loaded plus.stl with {} triangles", mesh.triangle_count());
    
    // Create two mesh objects: one with k-d tree, one without
    let mesh_kdtree = MeshObject::new(mesh.clone(), Color::new(1.0, 1.0, 1.0), 0);
    let mesh_brute_force = MeshObject::new_brute_force(mesh.clone(), Color::new(1.0, 1.0, 1.0), 0);
    
    // Use one of the failing rays
    let ray = Ray::new(
        Point::new(-152.7, 0.0, -7.5),
        Vec3::new(1.0, 0.0, 0.0)
    );
    
    println!("Testing ray: origin=({:.3}, {:.3}, {:.3}), dir=({:.3}, {:.3}, {:.3})", 
        ray.origin.x, ray.origin.y, ray.origin.z,
        ray.direction.x, ray.direction.y, ray.direction.z);
    
    // Test intersection with both methods
    let hit_kdtree = mesh_kdtree.hit(&ray, 0.001, f64::INFINITY);
    let hit_brute_force = mesh_brute_force.hit(&ray, 0.001, f64::INFINITY);
    
    println!("\nResults:");
    match hit_kdtree.as_ref() {
        Some(h) => println!("  K-d tree: HIT at t={:.6}, point=({:.3}, {:.3}, {:.3})", 
            h.t, h.point.x, h.point.y, h.point.z),
        None => println!("  K-d tree: MISS"),
    }
    
    match hit_brute_force.as_ref() {
        Some(h) => println!("  Brute force: HIT at t={:.6}, point=({:.3}, {:.3}, {:.3})", 
            h.t, h.point.x, h.point.y, h.point.z),
        None => println!("  Brute force: MISS"),
    }

    // Let's find which triangle the brute force algorithm intersects
    if let Some(hit) = hit_brute_force.as_ref() {
        println!("\nFinding which triangle was hit...");
        for (tri_idx, triangle) in mesh.triangles.iter().enumerate() {
            // Use the same intersection method as MeshObject
            if let Some((t, _, _)) = intersect_triangle_moller_trumbore(&ray, triangle) {
                if (t - hit.t).abs() < 1e-6 {
                    println!("Hit triangle {}: vertices = [{:.2}, {:.2}, {:.2}], [{:.2}, {:.2}, {:.2}], [{:.2}, {:.2}, {:.2}]",
                        tri_idx,
                        triangle.vertices[0].x, triangle.vertices[0].y, triangle.vertices[0].z,
                        triangle.vertices[1].x, triangle.vertices[1].y, triangle.vertices[1].z,
                        triangle.vertices[2].x, triangle.vertices[2].y, triangle.vertices[2].z);
                    let (tri_min, tri_max) = triangle.bounds();
                    println!("Triangle bounds: ({:.2}, {:.2}, {:.2}) to ({:.2}, {:.2}, {:.2})",
                        tri_min.x, tri_min.y, tri_min.z, tri_max.x, tri_max.y, tri_max.z);
                    break;
                }
            }
        }
    }
    
    // Let's manually check what triangles the k-d tree visits
    println!("\nTriangles visited by k-d tree (brief debug):");
    let mut triangle_count = 0;
    mesh.kdtree.traverse_debug(&ray.origin, ray.direction.as_ref(), |triangle_indices| {
        triangle_count += triangle_indices.len();
        println!("  Leaf with {} triangles: {:?}", triangle_indices.len(), &triangle_indices[..triangle_indices.len().min(10)]);
    });
    println!("Total triangles visited by k-d tree: {}", triangle_count);
    
    // Let's check if the ray intersects the overall mesh bounds
    let (bounds_min, bounds_max) = mesh.bounds();
    println!("\nMesh bounds: min=({:.3}, {:.3}, {:.3}), max=({:.3}, {:.3}, {:.3})", 
        bounds_min.x, bounds_min.y, bounds_min.z, 
        bounds_max.x, bounds_max.y, bounds_max.z);
    
    // Manual ray-box intersection test
    let ray_intersects_overall = ray_box_intersection(&ray, &bounds_min, &bounds_max);
    match ray_intersects_overall {
        Some((t_near, t_far)) => {
            println!("Ray intersects overall bounds: YES");
            println!("  Intersection range: t_near={:.6}, t_far={:.6}", t_near, t_far);
            let near_point = ray.at(t_near);
            let far_point = ray.at(t_far);
            println!("  Near point: ({:.3}, {:.3}, {:.3})", near_point.x, near_point.y, near_point.z);
            println!("  Far point: ({:.3}, {:.3}, {:.3})", far_point.x, far_point.y, far_point.z);
        },
        None => {
            println!("Ray intersects overall bounds: NO");
        }
    }
    
    Ok(())
}

fn ray_box_intersection(ray: &Ray, min: &Point, max: &Point) -> Option<(f64, f64)> {
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;
    
    for axis in 0..3 {
        if ray.direction[axis].abs() < 1e-9 {
            // Ray is parallel to the slab
            if ray.origin[axis] < min[axis] || ray.origin[axis] > max[axis] {
                return None;
            }
        } else {
            let inv_dir = 1.0 / ray.direction[axis];
            let mut t0 = (min[axis] - ray.origin[axis]) * inv_dir;
            let mut t1 = (max[axis] - ray.origin[axis]) * inv_dir;
            
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
            
            if t_min > t_max {
                return None;
            }
        }
    }
    
    if t_max < 0.0 {
        None
    } else {
        Some((t_min.max(0.0), t_max))
    }
}