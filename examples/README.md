# Ray Tracer Examples

This directory contains example scene files demonstrating the ray tracer capabilities:

## Example Scenes

### Individual Primitive Examples

#### 1. Simple Sphere (`simple_sphere.json`)
- Single red sphere with Phong lighting
- Demonstrates basic sphere rendering and lighting
- **Output**: `simple_sphere_800x600.png`

#### 2. Simple Plane (`simple_plane.json`)
- Single green plane with Phong lighting
- Demonstrates plane rendering and basic lighting
- **Output**: `simple_plane_800x600.png`

#### 3. Simple Cube (`simple_cube.json`)
- Single blue cube with Phong lighting
- Demonstrates cube rendering and lighting
- **Output**: `simple_cube_800x600.png`

### Complex Scene Examples

#### 4. Multiple Objects (`multiple_objects.json`)
- Sphere, cube, and plane with checkered texture
- Multiple light sources with different colors
- Demonstrates:
  - All three primitive types combined
  - Grid texture on the plane
  - Multiple light sources
  - Different material properties
- **Output**: `multiple_objects_800x600.png`

#### 5. Fog Scene (`fog_scene.json`)
- Atmospheric fog demonstration
- Reflective sphere
- Demonstrates:
  - Atmospheric fog with linear falloff
  - Surface reflections
  - Depth-based lighting effects
- **Output**: `fog_scene_800x600.png`

## Running the Examples

To render any of these scenes:

```bash
# From the root directory
./target/release/rtrace --input examples/SCENE_FILE.json --output OUTPUT.png --width 800 --height 600
```

For example:
```bash
./target/release/rtrace --input examples/simple_sphere.json --output my_render.png --width 800 --height 600
```

## Scene Format

All scenes follow the JSON schema defined in `../schema.json`. Key features:

- **Camera**: Orthographic projection only (for now)
- **Objects**: Sphere, plane, cube with materials
- **Materials**: Phong lighting model with ambient, diffuse, specular components
- **Textures**: Grid pattern support for planes
- **Lighting**: Point light sources with color and intensity
- **Effects**: Ambient lighting and atmospheric fog

## Material Properties

- `ambient`: How much ambient light the surface reflects (0.0-1.0)
- `diffuse`: How much diffuse light the surface reflects (0.0-1.0)  
- `specular`: How much specular light the surface reflects (0.0-1.0)
- `shininess`: Phong exponent for specular highlights (higher = shinier)
- `reflectivity`: Optional mirror-like reflection coefficient (0.0-1.0)

## Textures

Currently supported textures:
- **Grid**: Creates a grid pattern with specified line color, width, and cell size