# Task Runner Guide

This project uses the [hereby](https://github.com/jakebailey/hereby) NPM package as a task runner to organize and manage development tasks.

## Usage

### List All Available Tasks
```bash
npm run tasks
# or
npx hereby -T
```

### Run a Task
```bash
npx hereby <task-name>
# or  
npm run task <task-name>
```

## Available Task Categories

### Build Tasks
- `build` - Build Node.js bindings (default build)
- `build:node` - Build Node.js bindings specifically
- `build:rust` - Build Rust core library (debug mode)
- `build:rust:release` - Build Rust core library (release mode)  
- `build:cli` - Build CLI tools
- `build:all` - Build all components including CLI
- `dev` - Development build (debug mode for Rust, release for Node.js)

### Test Tasks
- `test` - Run all standard tests (Rust unit tests + Node.js binding tests)
- `test:rust` - Run Rust unit tests only
- `test:node` - Run Node.js binding tests only  
- `test:kdtree` - Run KD-tree vs brute force consistency tests
- `test:all` - Run all tests including KD-tree consistency tests
- `test:bounds` - Run plus model bounds testing

### Example Tasks
- `example` - Run basic Node.js bindings example
- `example:radial` - Run radial spheres example
- `example:multithreaded` - Run multithreaded demo
- `example:analyze` - Run plus model analysis
- `example:all` - Run all example scripts

### Rendering Tasks
- `render:simple` - Render simple sphere example
- `render:radial` - Render radial spheres example  
- `render:plus` - Render plus perspective example
- `render:espresso` - Render espresso tray example
- `render:all` - Render all example images
- `render:hires` - Render high-resolution images
- `render:debug` - Render debug images

### Debug and Development Tasks
- `debug:kdtree` - Run KD-tree debugging tool
- `lint` - Run Rust linting (clippy)
- `format` - Format Rust code
- `format:check` - Check Rust code formatting

### Clean Tasks
- `clean` - Clean all build artifacts
- `clean:rendered` - Clean rendered image files

### Workflow Tasks
- `ci` - CI pipeline: format check, lint, build all, and test all
- `precommit` - Pre-commit checks: format, lint, and test
- `default` - Default task: build and test

## Examples

```bash
# Build everything for development
npx hereby dev

# Run all tests including consistency checks
npx hereby test:all

# Run the KD-tree vs brute force consistency test
npx hereby test:kdtree

# Render all example images
npx hereby render:all

# Format code and run linting
npx hereby precommit

# Clean and rebuild everything
npx hereby clean
npx hereby build:all

# Run CI pipeline
npx hereby ci
```

## Existing NPM Scripts

The existing NPM scripts are preserved for backward compatibility:

```bash
npm run build      # Same as: npx hereby build
npm run test       # Existing npm test behavior
npm run example    # Existing npm example behavior  
npm run tasks      # Lists all hereby tasks
```

## Dependencies Between Tasks

Tasks automatically handle dependencies. For example:
- `test:node` depends on `build:node`
- `render:*` tasks depend on `build:cli` 
- `example:*` tasks depend on `build:node`

This means you can run `npx hereby test:node` and it will automatically build the Node.js bindings first if needed.