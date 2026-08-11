use rtrace::mesh::Mesh;
fn main() {
    match Mesh::from_stl_file("doc/scenes/benchy.stl") {
        Ok(mesh) => println!("Loaded {} triangles", mesh.triangles().len()),
        Err(e) => println!("Error: {}", e),
    }
}
