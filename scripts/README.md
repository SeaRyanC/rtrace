# Scripts Directory

This directory contains JavaScript demo scripts, examples, and test utilities for the rtrace project.

## Files

### Test Scripts
- `test.js` - Main test script for NAPI bindings (run via `npm test`)
- `test_plus_bounds.js` - Test script for plus model bounds testing

### Demo/Example Scripts
- `example.js` - Basic usage example (run via `npm run example`)
- `analyze_plus.js` - Analysis script for plus.stl model
- `multithreaded_demo.js` - Multi-threading demonstration
- `radial_spheres_example.js` - Example creating radial array of spheres
- `render_plus_debug.js` - Debug rendering script for plus model
- `render_plus_hires.js` - High-resolution rendering script

## Usage

Most scripts should be run from the root directory of the project:

```bash
# Run main tests
npm test

# Run example
npm run example

# Run individual scripts
node scripts/analyze_plus.js
node scripts/multithreaded_demo.js
```

Note: Scripts reference `../examples/` for scene files and `../rtrace.node` for the native module.