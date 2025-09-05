# rtrace

A minimal Rust library with Node.js and WebAssembly bindings, demonstrating modern Rust development practices.

## Features

- **Core Library**: Simple Rust library with a hello world function
- **CLI Tool**: Command-line interface with argument parsing
- **Node.js Bindings**: Native Node.js modules using napi-rs
- **WebAssembly**: WASM bindings for web applications
- **Modern Tooling**: Cargo workspace, proper dependency management, and comprehensive documentation

## Project Structure

```
rtrace/
├── src/lib.rs                 # Core library
├── cli/                       # CLI binary crate
│   └── src/main.rs
├── bindings/
│   ├── node/                  # Node.js bindings
│   │   └── src/lib.rs
│   └── wasm/                  # WebAssembly bindings
│       └── src/lib.rs
├── Cargo.toml                 # Workspace configuration
├── package.json               # Node.js package configuration
└── README.md
```

## Installation & Usage

### Core Library

```rust
use rtrace::hello_world;

fn main() {
    let message = hello_world();
    println!("{}", message); // prints "hello world"
}
```

### CLI Tool

```bash
# Build the CLI
cargo build --release -p rtrace-cli

# Basic usage
./target/release/rtrace

# With options
./target/release/rtrace --name Alice --count 3 --uppercase
```

**CLI Options:**
- `-n, --name <NAME>`: Name to greet (default: "world")  
- `-c, --count <COUNT>`: Number of times to repeat (default: 1)
- `-u, --uppercase`: Convert output to uppercase
- `-h, --help`: Show help information

### Node.js Bindings

Prerequisites:
```bash
# Install Node.js dependencies
npm install

# Install napi-rs CLI
npm install -g @napi-rs/cli
```

Build and use:
```bash
# Build Node.js bindings
napi build --platform --release

# Or use npm script
npm run build:node
```

```javascript
const { helloWorld, greetWithName } = require('./bindings/node');

console.log(helloWorld()); // "hello world"
console.log(greetWithName("Alice")); // "hello world, Alice"
```

### WebAssembly

Prerequisites:
```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Build and use:
```bash
# Build WASM package
wasm-pack build bindings/wasm --target web --out-dir pkg

# Or use npm script
npm run build:wasm
```

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>rtrace WASM Demo</title>
</head>
<body>
    <script type="module">
        import init, { hello_world, greet_with_name_and_log } from './bindings/wasm/pkg/rtrace_wasm.js';
        
        async function run() {
            await init();
            
            console.log(hello_world()); // "hello world"
            console.log(greet_with_name_and_log("WebAssembly")); // "hello world, WebAssembly"
        }
        
        run();
    </script>
</body>
</html>
```

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v16+)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (for WASM builds)

### Building

```bash
# Build all components
cargo build --workspace

# Build specific components
cargo build -p rtrace           # Core library
cargo build -p rtrace-cli       # CLI tool
cargo build -p rtrace-node      # Node.js bindings
cargo build -p rtrace-wasm      # WASM bindings
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
- Pure Rust, no external dependencies

**CLI:**
- `clap` - Modern command-line argument parsing

**Node.js Bindings:**
- `napi` - Safe Node.js API bindings
- `napi-derive` - Procedural macros for napi

**WebAssembly:**
- `wasm-bindgen` - High-level WASM bindings
- `web-sys` - Web API bindings

### Architecture

The project uses a Cargo workspace to organize multiple related crates:

1. **Root crate** (`rtrace`): Core library functionality
2. **CLI crate** (`rtrace-cli`): Command-line interface
3. **Node.js crate** (`rtrace-node`): Native Node.js bindings
4. **WASM crate** (`rtrace-wasm`): WebAssembly bindings

Each binding crate is thin wrapper around the core library, ensuring consistency across all interfaces.

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

**WASM Build Issues:**
- Install/update wasm-pack: `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`
- Check browser console for detailed error messages

### Getting Help

- Open an issue on [GitHub](https://github.com/SeaRyanC/rtrace/issues)
- Check existing issues for similar problems
- Provide detailed error messages and system information