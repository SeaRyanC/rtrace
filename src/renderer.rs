use std::collections::HashMap;
use image::{ImageBuffer, Rgb, RgbImage};

use crate::scene::{Scene, Object, hex_to_color, Color, Point, Vec3};
use crate::camera::Camera;
use crate::ray::{World, Sphere, Plane, Cube, MeshObject};
use crate::lighting::ray_color;

pub struct Renderer {
    pub width: u32,
    pub height: u32,
    pub max_depth: i32,
    pub use_kdtree: bool, // New field to control k-d tree usage for meshes
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            max_depth: 10,
            use_kdtree: true, // Default to using k-d tree
        }
    }

    /// Create a renderer with k-d tree disabled (brute force mesh intersection)
    pub fn new_brute_force(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            max_depth: 10,
            use_kdtree: false, // Disable k-d tree
        }
    }
    
    pub fn render(&self, scene: &Scene) -> Result<RgbImage, Box<dyn std::error::Error>> {
        // Create camera
        let aspect_ratio = self.width as f64 / self.height as f64;
        let camera = Camera::from_config(&scene.camera, aspect_ratio)?;
        let camera_pos = Point::new(
            scene.camera.position[0],
            scene.camera.position[1], 
            scene.camera.position[2]
        );
        
        // Build world with objects
        let mut world = World::new();
        let mut materials = HashMap::new();
        
        for (index, object) in scene.objects.iter().enumerate() {
            match object {
                Object::Sphere { center, radius, material } => {
                    let center = Point::new(center[0], center[1], center[2]);
                    let color = hex_to_color(&material.color)?;
                    let sphere = Box::new(Sphere {
                        center,
                        radius: *radius,
                        material_color: color,
                        material_index: index,
                    });
                    world.add(sphere);
                    materials.insert(index, material.clone());
                }
                Object::Plane { point, normal, material } => {
                    let point = Point::new(point[0], point[1], point[2]);
                    let normal = nalgebra::Unit::new_normalize(Vec3::new(normal[0], normal[1], normal[2]));
                    let color = hex_to_color(&material.color)?;
                    let plane = Box::new(Plane {
                        point,
                        normal,
                        material_color: color,
                        material_index: index,
                    });
                    world.add(plane);
                    materials.insert(index, material.clone());
                }
                Object::Cube { center, size, material } => {
                    let center = Point::new(center[0], center[1], center[2]);
                    let size = Vec3::new(size[0], size[1], size[2]);
                    let color = hex_to_color(&material.color)?;
                    let cube = Box::new(Cube::new(center, size, color, index));
                    world.add(cube);
                    materials.insert(index, material.clone());
                }
                Object::Mesh { mesh_data, material, .. } => {
                    if let Some(mesh) = mesh_data {
                        let color = hex_to_color(&material.color)?;
                        let mesh_object = if self.use_kdtree {
                            Box::new(MeshObject::new(mesh.clone(), color, index))
                        } else {
                            Box::new(MeshObject::new_brute_force(mesh.clone(), color, index))
                        };
                        world.add(mesh_object);
                        materials.insert(index, material.clone());
                    }
                }
            }
        }
        
        // Get background color
        let background_color = if let Some(bg) = &scene.scene_settings.background_color {
            hex_to_color(bg)?
        } else {
            Color::new(0.0, 0.0, 0.0)
        };
        
        // Create image buffer
        let mut image = ImageBuffer::new(self.width, self.height);
        
        // Render each pixel
        for y in 0..self.height {
            for x in 0..self.width {
                let u = x as f64 / (self.width - 1) as f64;
                let v = (self.height - 1 - y) as f64 / (self.height - 1) as f64; // Flip Y coordinate
                
                let ray = camera.get_ray(u, v);
                let color = ray_color(
                    &ray,
                    &world,
                    &scene.lights,
                    &scene.scene_settings.ambient_illumination,
                    &scene.scene_settings.fog,
                    &camera_pos,
                    background_color,
                    &materials,
                    self.max_depth,
                );
                
                // Convert to RGB values (0-255)
                let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
                let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
                let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;
                
                image.put_pixel(x, y, Rgb([r, g, b]));
            }
            
            // Print progress
            if y % (self.height / 10).max(1) == 0 {
                println!("Rendering: {:.1}%", (y as f64 / self.height as f64) * 100.0);
            }
        }
        
        println!("Rendering: 100.0%");
        Ok(image)
    }
    
    pub fn render_to_file(&self, scene: &Scene, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let image = self.render(scene)?;
        image.save(output_path)?;
        println!("Image saved to: {}", output_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Object, Material, Light, Scene};
    
    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new(800, 600);
        assert_eq!(renderer.width, 800);
        assert_eq!(renderer.height, 600);
    }
    
    #[test]
    fn test_simple_render() {
        let mut scene = Scene::default();
        
        // Add a simple sphere
        scene.objects.push(Object::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
            material: Material::default(),
        });
        
        // Add a light
        scene.lights.push(Light {
            position: [2.0, 2.0, 2.0],
            color: "#FFFFFF".to_string(),
            intensity: 1.0,
        });
        
        let renderer = Renderer::new(100, 100);
        let result = renderer.render(&scene);
        assert!(result.is_ok());
    }
}