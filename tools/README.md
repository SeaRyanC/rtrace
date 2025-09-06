# Tools Directory

This directory contains Rust debugging and testing tools for the rtrace project.

## Files

- `debug_kdtree.rs` - Debugging tool for k-d tree implementation
- `test_kdtree_consistency.rs` - Tool for testing k-d tree vs brute force consistency

## Usage

These tools can be built and run using Cargo:

```bash
# Build a specific tool
cargo build --bin test_kdtree_consistency
cargo build --bin debug_kdtree

# Run a tool
cargo run --bin test_kdtree_consistency
cargo run --bin debug_kdtree
```

These are development and debugging utilities used during development to verify the correctness of the ray tracing algorithms.