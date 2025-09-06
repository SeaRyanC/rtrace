use rtrace::{
    mesh::Mesh,
    ray::{Ray, MeshObject, Intersectable},
    scene::{Point, Vec3, Color},
};
use nalgebra::Vector3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing k-d tree vs brute force consistency for plus.stl");
    
    // Load plus.stl mesh
    let mesh = Mesh::from_stl_file("examples/plus.stl")?;
    println!("Loaded plus.stl with {} triangles", mesh.triangle_count());
    
    // Create two mesh objects: one with k-d tree, one without
    let mesh_kdtree = MeshObject::new(mesh.clone(), Color::new(1.0, 1.0, 1.0), 0);
    let mesh_brute_force = MeshObject::new_brute_force(mesh.clone(), Color::new(1.0, 1.0, 1.0), 0);
    
    // Generate test rays, focusing on axis-aligned rays that are problematic
    let test_rays = generate_test_rays();
    println!("Generated {} test rays", test_rays.len());
    
    let mut mismatches = 0;
    let mut total_tests = 0;
    
    for (i, ray) in test_rays.iter().enumerate() {
        total_tests += 1;
        
        // Test intersection with both methods
        let hit_kdtree = mesh_kdtree.hit(&ray, 0.001, f64::INFINITY);
        let hit_brute_force = mesh_brute_force.hit(&ray, 0.001, f64::INFINITY);
        
        // Compare results
        let mismatch = match (hit_kdtree.as_ref(), hit_brute_force.as_ref()) {
            (None, None) => false, // Both miss - consistent
            (Some(h1), Some(h2)) => {
                // Both hit - check if they're consistent
                let t_diff = (h1.t - h2.t).abs();
                let point_diff = (h1.point - h2.point).magnitude();
                let normal_diff = (h1.normal.as_ref() - h2.normal.as_ref()).magnitude();
                
                // Allow small numerical differences
                t_diff > 1e-6 || point_diff > 1e-6 || normal_diff > 1e-6
            },
            _ => true, // One hits, one misses - inconsistent
        };
        
        if mismatch {
            mismatches += 1;
            println!("\nMISMATCH #{} on ray {}", mismatches, i);
            println!("  Ray: origin=({:.3}, {:.3}, {:.3}), dir=({:.3}, {:.3}, {:.3})", 
                ray.origin.x, ray.origin.y, ray.origin.z,
                ray.direction.x, ray.direction.y, ray.direction.z);
            
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
            
            if mismatches >= 10 {
                println!("\nStopping after 10 mismatches to avoid spam.");
                break;
            }
        }
    }
    
    println!("\nSummary:");
    println!("  Total rays tested: {}", total_tests);
    println!("  Mismatches found: {}", mismatches);
    println!("  Success rate: {:.2}%", 100.0 * (total_tests - mismatches) as f64 / total_tests as f64);
    
    if mismatches == 0 {
        println!("✅ K-d tree is consistent with brute force!");
    } else {
        println!("❌ K-d tree has inconsistencies with brute force.");
    }
    
    Ok(())
}

fn generate_test_rays() -> Vec<Ray> {
    let mut rays = Vec::new();
    
    // Get mesh bounds to generate meaningful rays
    let mesh = Mesh::from_stl_file("examples/plus.stl").unwrap();
    let (bounds_min, bounds_max) = mesh.bounds();
    let center = (bounds_min + bounds_max.coords) / 2.0;
    let size = bounds_max - bounds_min;
    let max_extent = size.x.max(size.y).max(size.z);
    
    println!("Mesh bounds: min=({:.3}, {:.3}, {:.3}), max=({:.3}, {:.3}, {:.3})", 
        bounds_min.x, bounds_min.y, bounds_min.z, 
        bounds_max.x, bounds_max.y, bounds_max.z);
    
    // 1. Axis-aligned rays (these are the most problematic according to the user)
    let positions = vec![
        // Rays from outside the mesh pointing inward along each axis
        Point::new(bounds_min.x - max_extent, center.y, center.z),
        Point::new(bounds_max.x + max_extent, center.y, center.z),
        Point::new(center.x, bounds_min.y - max_extent, center.z),
        Point::new(center.x, bounds_max.y + max_extent, center.z),
        Point::new(center.x, center.y, bounds_min.z - max_extent),
        Point::new(center.x, center.y, bounds_max.z + max_extent),
        
        // Rays from slightly off-center positions
        Point::new(bounds_min.x - max_extent, center.y + 0.1, center.z + 0.1),
        Point::new(bounds_max.x + max_extent, center.y - 0.1, center.z - 0.1),
        Point::new(center.x + 0.1, bounds_min.y - max_extent, center.z + 0.1),
        Point::new(center.x - 0.1, bounds_max.y + max_extent, center.z - 0.1),
        Point::new(center.x + 0.1, center.y + 0.1, bounds_min.z - max_extent),
        Point::new(center.x - 0.1, center.y - 0.1, bounds_max.z + max_extent),
    ];
    
    let directions = vec![
        // Perfect axis-aligned directions
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
        
        // Nearly axis-aligned directions (small epsilon to test edge cases)
        Vec3::new(1.0, 1e-9, 1e-9),
        Vec3::new(-1.0, 1e-9, -1e-9),
        Vec3::new(1e-9, 1.0, 1e-9),
        Vec3::new(-1e-9, -1.0, 1e-9),
        Vec3::new(1e-9, 1e-9, 1.0),
        Vec3::new(-1e-9, -1e-9, -1.0),
    ];
    
    // Generate all combinations of positions and directions
    for pos in &positions {
        for dir in &directions {
            rays.push(Ray::new(*pos, *dir));
        }
    }
    
    // 2. Rays that pass through mesh center from various angles
    let num_angle_rays = 20;
    for i in 0..num_angle_rays {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / num_angle_rays as f64;
        let phi = std::f64::consts::PI * 0.5; // Horizontal
        
        let dir = Vec3::new(
            theta.sin() * phi.cos(),
            theta.cos() * phi.cos(),
            phi.sin(),
        );
        
        let origin = center - dir * max_extent * 2.0;
        rays.push(Ray::new(origin, dir));
    }
    
    // 3. Random rays for comprehensive testing
    for i in 0..50 {
        let angle1 = 2.0 * std::f64::consts::PI * (i as f64 / 50.0);
        let angle2 = std::f64::consts::PI * ((i * 3) % 50) as f64 / 50.0;
        
        let origin = Point::new(
            center.x + (angle1.sin() * max_extent * 1.5),
            center.y + (angle2.cos() * max_extent * 1.5),
            center.z + ((angle1 + angle2).sin() * max_extent * 1.5),
        );
        
        let dir = (center - origin).normalize();
        rays.push(Ray::new(origin, dir));
    }
    
    rays
}