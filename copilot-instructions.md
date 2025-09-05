# Copilot Instructions for rtrace

This document provides development guidance for the rtrace project, helping developers (and AI assistants) understand the project structure, conventions, and best practices.

## Project Overview

rtrace is a minimal Rust library that demonstrates modern Rust development practices with multiple binding targets:
- Core Rust library
- Command-line interface
- Node.js native bindings (via napi-rs)  
- WebAssembly bindings (via wasm-bindgen)

## Architecture Principles

### 1. Cargo Workspace Structure
- **Root crate** (`rtrace`): Core library with pure Rust implementation
- **Binding crates**: Thin wrappers around core library
- **CLI crate**: Independent binary using the core library
- Each crate has a focused responsibility and minimal dependencies

### 2. Code Organization
```
rtrace/
├── src/lib.rs                 # Core library - keep minimal and pure
├── cli/src/main.rs            # CLI binary - uses clap for args
├── bindings/node/src/lib.rs   # Node.js - uses napi macros
├── bindings/wasm/src/lib.rs   # WASM - uses wasm-bindgen macros
```

### 3. Dependency Strategy
- **Core library**: Zero dependencies (for maximum compatibility)
- **Binding crates**: Only necessary binding dependencies
- **CLI**: Modern, well-maintained crates (clap for args)
- Always use latest stable versions when possible

## Development Guidelines

### Code Style
- Follow standard Rust formatting (`cargo fmt`)
- Use clippy lints (`cargo clippy --workspace -- -D warnings`)
- Document public APIs with examples
- Keep functions small and focused
- Use descriptive names for functions and variables

### Testing Strategy
- Unit tests for core library functions
- Integration tests for CLI behavior
- Binding tests to ensure consistency across targets
- Use `cargo test --workspace` to run all tests

### Error Handling
- Use `Result<T, E>` for fallible operations
- Provide meaningful error messages
- Consider error propagation across binding boundaries
- Use `?` operator for clean error chaining

### Documentation
- Document all public APIs with rustdoc comments
- Include usage examples in documentation
- Keep README.md updated with latest features
- Use inline comments sparingly, prefer self-documenting code

## Build System

### Local Development
```bash
# Full workspace build
cargo build --workspace

# Individual component builds
cargo build -p rtrace
cargo build -p rtrace-cli  
cargo build -p rtrace-node
cargo build -p rtrace-wasm

# Tests
cargo test --workspace

# Formatting and linting
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

### Node.js Bindings
- Use napi-rs for safe Node.js interop
- Export functions with `#[napi]` macro
- Consider async functions for I/O operations
- Test with various Node.js versions

### WASM Bindings  
- Use wasm-bindgen for web compatibility
- Export functions with `#[wasm_bindgen]`
- Consider bundle size impact
- Test in multiple browsers

## Adding New Features

### Core Library Changes
1. Add function to `src/lib.rs`
2. Write unit tests
3. Document with examples
4. Update bindings if needed

### New Binding Functions
1. Add to core library first
2. Create binding wrapper in appropriate crate
3. Test the binding works correctly
4. Update documentation

### CLI Features
1. Add new clap arguments/subcommands
2. Implement logic using core library
3. Add tests for new CLI behavior
4. Update help text and documentation

## Common Patterns

### Core Library Function Template
```rust
/// Brief description of what the function does
/// 
/// # Examples
/// 
/// ```
/// use rtrace::function_name;
/// 
/// let result = function_name("input");
/// assert_eq!(result, "expected output");
/// ```
pub fn function_name(input: &str) -> String {
    // Implementation here
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        assert_eq!(function_name("test"), "expected");
    }
}
```

### Node.js Binding Template
```rust
use napi_derive::napi;

/// Description for Node.js users
#[napi]
pub fn node_function(input: String) -> String {
    rtrace::core_function(&input)
}
```

### WASM Binding Template
```rust
use wasm_bindgen::prelude::*;

/// Description for web users
#[wasm_bindgen]
pub fn wasm_function(input: &str) -> String {
    rtrace::core_function(input)
}
```

## Release Process

### Version Management
- Use semantic versioning (MAJOR.MINOR.PATCH)
- Keep all crates in sync for major/minor releases
- Update version in all Cargo.toml files
- Update package.json version

### Publishing Checklist
1. Run full test suite: `cargo test --workspace`
2. Check formatting: `cargo fmt --check`
3. Run clippy: `cargo clippy --workspace -- -D warnings`
4. Build all targets: `cargo build --workspace --release`
5. Update CHANGELOG.md with new features
6. Create git tag for release
7. Publish crates to crates.io
8. Publish npm package if bindings changed

## Troubleshooting

### Build Issues
- Clean build cache: `cargo clean`
- Update Rust toolchain: `rustup update`
- Check dependency compatibility

### Binding Issues
- Verify binding dependencies are up to date
- Test with minimal examples
- Check platform-specific requirements

### Performance Considerations
- Profile with `cargo bench` for performance-critical code
- Consider memory allocation patterns
- Test WASM bundle size impact

## Contributing Guidelines

When contributing to rtrace:

1. **Follow the architecture**: Keep core library pure, bindings thin
2. **Test thoroughly**: Add tests for new functionality
3. **Document changes**: Update README and inline docs
4. **Check all targets**: Ensure changes work across all bindings
5. **Performance aware**: Consider impact on bundle size and speed
6. **Backward compatibility**: Avoid breaking changes when possible

## AI Assistant Guidelines

When helping with rtrace development:

1. **Prefer minimal changes**: Only modify what's necessary
2. **Maintain consistency**: Follow existing patterns and style
3. **Test changes**: Always verify builds and tests pass
4. **Consider all targets**: Changes may affect multiple bindings
5. **Update documentation**: Keep README and examples current
6. **Security conscious**: Avoid unsafe code unless necessary

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [napi-rs Documentation](https://napi.rs/)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [Clap Documentation](https://docs.rs/clap/)
- [Cargo Workspace Guide](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)