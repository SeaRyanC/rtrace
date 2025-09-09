# rtrace

A high-performance ray tracer library written in Rust with Node.js bindings.

## Features

- **Ray Tracer**: Complete ray tracing engine with modern lighting models
  - Orthographic and perspective camera projections
  - Geometric primitives (sphere, plane, cube, STL mesh)
  - **Object transforms** (rotate, translate, scale) for flexible positioning
  - Phong lighting model with ambient, diffuse, and specular components
  - Point and area light sources with soft shadows
  - Anti-aliasing with multiple sampling modes (quincunx, stochastic, no-jitter)
  - Atmospheric fog with distance-based linear-to-exponential density calculation
  - Surface reflections
  - Grid texture patterns for planes
  - **Deterministic rendering** for reproducible results
- **CLI Tool**: Command-line ray tracer for rendering scenes from JSON
- **Node.js Bindings**: Native Node.js modules using napi-rs
- **JSON Scene Format**: Flexible scene description with JSON schema validation
- **PNG Output**: High-quality image generation

## Project Structure

```
rtrace/
├── src/
│   ├── lib.rs               # Core library with ray tracing modules
│   ├── scene.rs             # Scene definition and JSON schema types
│   ├── ray.rs               # Ray-object intersection math
│   ├── camera.rs            # Camera projection (orthographic)
│   ├── lighting.rs          # Phong lighting and fog effects
│   └── renderer.rs          # Main rendering engine
├── cli/                     # CLI binary crate
│   └── src/main.rs          # Command-line ray tracer
├── bindings/
│   └── node/                # Node.js bindings
│       └── src/lib.rs
├── examples/                # Example scene files and outputs
│   ├── *.json               # Scene definition files
│   ├── *.png                # Rendered example images
│   └── README.md            # Example documentation
├── schema.json              # JSON schema for scene files
├── Cargo.toml               # Workspace configuration
├── package.json             # Node.js package configuration
└── README.md
```

## Installation & Usage

### Ray Tracer CLI

```bash
# Build the ray tracer CLI
cargo build --release -p rtrace-cli

# Render a scene
./target/release/rtrace --input examples/simple_sphere.json --output my_render.png --width 800 --height 600

# View CLI help
./target/release/rtrace --help
```

**CLI Options:**
- `-i, --input <FILE>`: Input JSON scene file (required)
- `-o, --output <FILE>`: Output PNG image file (required)  
- `-w, --width <WIDTH>`: Image width in pixels (default: 800)
- `-H, --height <HEIGHT>`: Image height in pixels (default: 600)
- `--max-depth <DEPTH>`: Maximum ray bounces for reflections (default: 10)
- `--samples <SAMPLES>`: Number of samples per pixel for anti-aliasing
- `--anti-aliasing <MODE>`: Anti-aliasing mode - `quincunx` (default), `stochastic`, or `no-jitter`

**Deterministic Rendering:**

The ray tracer ensures reproducible results by using deterministic randomness for all stochastic operations:
- Same input scene = identical output image (byte-for-byte)
- Consistent results across different hardware and thread counts
- Works across different thread counts and hardware

```bash
# Renders are always deterministic and reproducible
./target/release/rtrace --input scene.json --output render1.png
./target/release/rtrace --input scene.json --output render2.png
# render1.png and render2.png are identical
```

### Auto Camera Bounds CLI

Generate optimal camera views for any scene automatically:

```bash
# Build the auto camera CLI
cargo build --release -p rtrace-cli

# Generate 4 camera views for a scene
./target/release/rtrace-auto-camera --input examples/plus_perspective.json --output cameras.json

# View auto camera help  
./target/release/rtrace-auto-camera --help
```

The auto camera tool generates 4 optimized camera configurations:

1. **Left View**: Orthographic camera viewing from positive Y direction
2. **Front View**: Orthographic camera viewing from positive X direction  
3. **Top View**: Orthographic camera viewing from positive Z direction
4. **Perspective View**: 50° FOV perspective camera at 35° down angle from positive X/Y/Z octant

All cameras automatically:
- Target the scene center
- Frame the entire scene with 15% aesthetic margin
- Exclude infinite objects (planes) from bounds calculation
- Follow JSON schema for seamless integration

**Auto Camera CLI Options:**
- `-i, --input <FILE>`: Input JSON scene file (required)
- `-o, --output <FILE>`: Output JSON file with camera configurations (required)

### Scene Format

Create JSON files following the schema in `schema.json`. Example:

```json
{
  "camera": {
    "kind": "ortho",
    "position": [0, 0, 10],
    "target": [0, 0, 0],
    "up": [0, 1, 0],
    "width": 6,
    "height": 6
  },
  "objects": [
    {
      "kind": "sphere",
      "center": [0, 0, 0],
      "radius": 1.5,
      "material": {
        "color": "#FF4444",
        "ambient": 0.1,
        "diffuse": 0.8,
        "specular": 0.4,
        "shininess": 32
      },
      "transform": [
        "rotate(0, 0, 45)",
        "translate(2, 0, 0)",
        "scale(1.5, 1.5, 1.5)"
      ]
    }
  ],
  "lights": [
    {
      "position": [3, 3, 5],
      "color": "#FFFFFF",
      "intensity": 1.0
    }
  ],
  "scene_settings": {
    "ambient_illumination": {
      "color": "#FFFFFF",
      "intensity": 0.1
    },
    "background_color": "#001122"
  }
}
```

**Object Transforms:**

All objects support optional transforms for positioning and scaling:

- **`"rotate(x, y, z)"`** - Rotate around X, Y, Z axes (degrees)  
- **`"translate(x, y, z)"`** - Move along X, Y, Z axes (world units)
- **`"scale(x, y, z)"`** - Scale along X, Y, Z axes (multipliers)

Transforms are applied in the order listed, allowing complex positioning:

```json
"transform": [
  "scale(2, 2, 2)",      // Double the size first
  "rotate(0, 45, 0)",    // Then rotate 45° around Y-axis
  "translate(10, 0, 0)"  // Finally move to position
]
```

### Core Library

```rust
use rtrace::{Scene, Renderer, AutoCamera};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load scene from JSON
    let scene = Scene::from_json_file("scene.json")?;
    
    // Create renderer
    let renderer = Renderer::new(800, 600);
    
    // Render to file
    renderer.render_to_file(&scene, "output.png")?;
    
    Ok(())
}
```

**Auto Camera Bounds API:**

```rust  
use rtrace::{Scene, AutoCamera};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load scene (camera settings will be ignored)
    let scene = Scene::from_json_file("input_scene.json")?;
    
    // Generate optimized camera views
    let cameras = AutoCamera::generate_cameras(&scene)?;
    
    // Access individual cameras
    println!("Left camera: {:?}", cameras.left);
    println!("Front camera: {:?}", cameras.front); 
    println!("Top camera: {:?}", cameras.top);
    println!("Perspective camera: {:?}", cameras.perspective);
    
    // Convert to JSON
    let cameras_json = cameras.to_cameras_json();
    
    Ok(())
}
```
```

### Node.js Bindings

Prerequisites:
```bash
# Install Node.js dependencies
npm install
```

Build and use:
```bash
# Build Node.js bindings
npm run build

# Test the bindings
npm test

# Run example
npm run example
```

**JavaScript Usage:**
```javascript
const { helloWorld, greetWithName, renderScene } = require('./rtrace.node');

// Basic functions
console.log(helloWorld()); // "hello world"
console.log(greetWithName("Alice")); // "hello world, Alice"

// Ray tracer API - render scenes programmatically
const scene = {
    camera: {
        kind: "ortho",
        position: [0, 0, 5],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 6,
        height: 6
    },
    objects: [
        {
            kind: "sphere",
            center: [0, 0, 0],
            radius: 1.0,
            material: {
                color: "#FF4444",
                ambient: 0.1,
                diffuse: 0.8,
                specular: 0.4,
                shininess: 32
            }
        }
    ],
    lights: [
        {
            position: [2, 2, 5],
            color: "#FFFFFF",
            intensity: 1.0
        }
    ],
    scene_settings: {
        ambient_illumination: {
            color: "#FFFFFF",
            intensity: 0.1
        },
        background_color: "#001122"
    }
};

// Render to PNG file
const result = renderScene(JSON.stringify(scene), 'output.png', 800, 600);
console.log(result); // "Successfully rendered 800x600 image to 'output.png'"
```

**TypeScript Support:**
TypeScript definitions are automatically generated:
```typescript
import { helloWorld, greetWithName, renderScene } from './rtrace.node';

const message: string = helloWorld();
const greeting: string = greetWithName("Bob");

// Render a scene programmatically
const scene = { /* scene object */ };
const result: string = renderScene(JSON.stringify(scene), 'output.png', 800, 600);
```

## Examples

The `examples/` directory contains several demonstration scenes:

1. **Simple Sphere**: Basic sphere with Phong lighting
2. **Multiple Objects**: Sphere, cube, and textured plane with multiple lights
3. **Fog Scene**: Atmospheric fog effects with reflective surfaces
4. **Transform Demo**: Object transforms (rotate, translate, scale) demonstration

Each example includes both the JSON scene file and rendered PNG output at 800x600 resolution.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v16+) - only for Node.js bindings

### Building

```bash
# Build all components
cargo build --workspace

# Build specific components
cargo build -p rtrace           # Core library
cargo build -p rtrace-cli       # CLI tool
cargo build -p rtrace-node      # Node.js bindings
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Test specific component
cargo test -p rtrace
```

### Linting

```bash
# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --workspace -- -D warnings
```

## Package Distribution

### Rust Crates

```bash
# Publish core library
cargo publish -p rtrace

# Publish CLI
cargo publish -p rtrace-cli
```

### npm Package

```bash
# Build all bindings
npm run build

# Publish to npm
npm publish
```

## Technical Details

### Dependencies

**Core Library:**
- `serde` - JSON serialization/deserialization
- `nalgebra` - Linear algebra and 3D math
- `image` - PNG image generation

**CLI:**
- `clap` - Modern command-line argument parsing

**Node.js Bindings:**
- `napi` - Safe Node.js API bindings
- `napi-derive` - Procedural macros for napi

### Architecture

The project uses a Cargo workspace to organize multiple related crates:

1. **Root crate** (`rtrace`): Ray tracing engine with scene loading and rendering
2. **CLI crate** (`rtrace-cli`): Command-line interface for rendering scenes
3. **Node.js crate** (`rtrace-node`): Native Node.js bindings

The ray tracer supports:
- Orthographic camera projection (perspective planned for future)
- Three primitive types: sphere, plane, and axis-aligned cube
- Phong lighting model with ambient, diffuse, and specular components
- Multiple point light sources with individual colors and intensities
- Atmospheric fog with linear falloff
- Surface reflections for mirror-like materials
- Grid texture patterns for planes
- Future-proofed design for triangle mesh support

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `cargo test --workspace`
6. Format your code: `cargo fmt`
7. Run clippy: `cargo clippy --workspace -- -D warnings`
8. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Troubleshooting

### Common Issues

**Build Errors:**
- Ensure you have the latest stable Rust: `rustup update stable`
- Clear build cache: `cargo clean`

**Node.js Binding Issues:**
- Make sure you have the correct Node.js version (v16+)
- Rebuild bindings: `napi build --platform --release`

### Getting Help

- Open an issue on [GitHub](https://github.com/SeaRyanC/rtrace/issues)
- Check existing issues for similar problems
- Provide detailed error messages and system information