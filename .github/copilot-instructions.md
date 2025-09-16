# Copilot Instructions for rtrace

This document provides development guidance for the rtrace project, helping developers (and AI assistants) understand the project structure, conventions, and best practices.

## Project Overview

## Coordinate System Convention

rtrace uses a **Z-up coordinate system** optimized for 3D printing and CAD workflows:
- **+X axis**: Points right (positive X direction)
- **+Y axis**: Points forward/away from viewer (positive Y direction)
- **+Z axis**: Points up (positive Z direction is "up")

This Z-up convention is standard in 3D printing, CAD software, and many engineering applications. When creating cameras, examples, or documentation:
- Use `up: [0, 0, 1]` for typical camera orientations
- Position cameras looking toward the origin from negative Y for front views
- Position lights and objects with Z coordinates representing height/elevation

## Architecture Principles

### 1. Cargo Workspace Structure
- **Root crate** (`rtrace`): Core library with pure Rust implementation
- **Binding crates**: Thin wrappers around core library
- **CLI crate**: Independent binary using the core library
- Each crate has a focused responsibility and minimal dependencies

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

## Adding New Features

### Core Library Changes

1. Add functionality
2. Write unit tests
3. Document with examples
4. Update bindings if needed
5. **Update existing documentation in `doc/README.md` and add example scenes in `doc/` folder**
6. Ensure zod schema is up to date

## Publishing Checklist
1. Run full test suite: `cargo test --workspace`
2. Check formatting: `cargo fmt --check`
3. Run clippy: `cargo clippy --workspace -- -D warnings`
4. Build all targets: `cargo build --workspace --release`
5. Update CHANGELOG.md with new features
6. Regenerate documentation images: `npx hereby doc:render` (validates rendering consistency)
7. Create git tag for release
8. Publish crates to crates.io
9. Publish npm package if bindings changed

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

## Contributing Guidelines

When contributing to rtrace:

1. **Follow the architecture**: Keep core library pure, bindings thin
2. **Test thoroughly**: Add tests for new functionality
3. **Document changes**: Update README and inline docs
4. **Check all targets**: Ensure changes work across all bindings
5. **Performance aware**: Consider impact on bundle size and speed
6. **DO NOT CARE ABOUT BACKWARD COMPATABILITY**: This is a v.0 project and this is NOT A FACTOR in how to code something. Do the RIGHT THING even if it means some clients will need to be updated
7. **Regenerate image samples**: As the final step in any PR, regenerate documentation images in `/doc/images/` using `npx hereby doc:render`. These images serve as an ad hoc test suite and should NOT change unless rendering code has actually been modified.
