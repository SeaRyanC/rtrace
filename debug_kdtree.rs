use rtrace::{
    mesh::Mesh,
    ray::{Ray, MeshObject, Intersectable},
    scene::{Point, Vec3, Color},
};

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
    
    // Let's manually check what triangles the k-d tree visits
    println!("\nTriangles visited by k-d tree:");
    let mut triangle_count = 0;
    mesh.kdtree.traverse(&ray.origin, ray.direction.as_ref(), |triangle_indices| {
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