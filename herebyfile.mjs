import { task } from "hereby";
import { spawn } from "child_process";
import { promisify } from "util";

// Helper function to execute shell commands
function exec(command, options = {}) {
    return () => new Promise((resolve, reject) => {
        const [cmd, ...args] = command.split(' ');
        const child = spawn(cmd, args, {
            stdio: 'inherit',
            shell: true,
            ...options
        });

        child.on('close', (code) => {
            if (code === 0) {
                resolve();
            } else {
                reject(new Error(`Command failed with exit code ${code}: ${command}`));
            }
        });

        child.on('error', (error) => {
            reject(error);
        });
    });
}

// Helper function to run tasks in series
function series(...tasks) {
    return tasks.reduce((prev, curr) => {
        if (!prev) return curr;
        if (!curr) return prev;
        
        return task({
            name: `series-${Math.random().toString(36).substr(2, 9)}`,
            dependencies: [prev, curr],
            hiddenFromTaskList: true
        });
    });
}

// Helper function to run tasks in parallel  
function parallel(...tasks) {
    return task({
        name: `parallel-${Math.random().toString(36).substr(2, 9)}`, 
        dependencies: tasks.filter(Boolean),
        hiddenFromTaskList: true
    });
}

// Build tasks
export const buildRust = task({
    name: "build:rust",
    description: "Build the Rust core library",
    run: exec("cargo build --workspace")
});

export const buildRustRelease = task({
    name: "build:rust:release", 
    description: "Build the Rust core library in release mode",
    run: exec("cargo build --workspace --release")
});

export const buildCli = task({
    name: "build:cli",
    description: "Build the CLI tools",
    run: exec("cargo build --release -p rtrace-cli")
});

export const buildNode = task({
    name: "build:node",
    description: "Build Node.js bindings",
    run: exec("napi build --release --cargo-cwd bindings/node")
});

export const build = task({
    name: "build",
    description: "Build all components",
    dependencies: [buildNode]
});

export const buildAll = task({
    name: "build:all",
    description: "Build all components including CLI",
    dependencies: [buildRustRelease, buildCli, buildNode]
});

// Test tasks
export const testRust = task({
    name: "test:rust",
    description: "Run Rust unit tests",
    run: exec("cargo test --workspace")
});

export const testNode = task({
    name: "test:node", 
    description: "Run Node.js binding tests",
    dependencies: [buildNode],
    run: exec("node scripts/test.js")
});

export const testKdtree = task({
    name: "test:kdtree",
    description: "Run KD-tree vs brute force consistency tests",
    run: exec("cargo run --bin test_kdtree_consistency")
});

export const test = task({
    name: "test",
    description: "Run all tests",
    dependencies: [testRust, testNode]
});

export const testAll = task({
    name: "test:all", 
    description: "Run all tests including KD-tree consistency tests",
    dependencies: [testRust, testNode, testKdtree]
});

// Example and demo tasks
export const example = task({
    name: "example",
    description: "Run basic Node.js bindings example",
    dependencies: [buildNode],
    run: exec("node scripts/example.js")
});

export const exampleRadial = task({
    name: "example:radial",
    description: "Run radial spheres example",
    dependencies: [buildNode],
    run: exec("node scripts/radial_spheres_example.js")
});

export const exampleMultithreaded = task({
    name: "example:multithreaded",
    description: "Run multithreaded demo",
    dependencies: [buildNode],
    run: exec("node scripts/multithreaded_demo.js")
});

export const exampleAnalyze = task({
    name: "example:analyze",
    description: "Run plus model analysis",
    dependencies: [buildNode],
    run: exec("node scripts/analyze_plus.js")
});

export const exampleAll = task({
    name: "example:all",
    description: "Run all example scripts",
    dependencies: [example, exampleRadial, exampleMultithreaded, exampleAnalyze]
});

// Rendering tasks
export const renderExampleSimple = task({
    name: "render:simple",
    description: "Render simple sphere example",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i examples/simple_sphere.json -o simple_sphere_rendered.png -w 800 -H 600")
});

export const renderExampleRadial = task({
    name: "render:radial", 
    description: "Render radial spheres example",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i examples/radial_spheres.json -o radial_spheres_rendered.png -w 800 -H 600")
});

export const renderExamplePlus = task({
    name: "render:plus",
    description: "Render plus perspective example", 
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i examples/plus_perspective.json -o plus_perspective_rendered.png -w 800 -H 600")
});

export const renderExampleEspresso = task({
    name: "render:espresso",
    description: "Render espresso tray example",
    dependencies: [buildCli], 
    run: exec("./target/release/rtrace -i examples/espresso_tray_top.json -o espresso_tray_rendered.png -w 800 -H 600")
});

export const renderAll = task({
    name: "render:all",
    description: "Render all example images",
    dependencies: [renderExampleSimple, renderExampleRadial, renderExamplePlus, renderExampleEspresso]
});

export const renderHires = task({
    name: "render:hires",
    description: "Render high-resolution images",
    dependencies: [buildNode],
    run: exec("node scripts/render_plus_hires.js")
});

export const renderDebug = task({
    name: "render:debug",
    description: "Render debug images",
    dependencies: [buildNode],
    run: exec("node scripts/render_plus_debug.js")
});

// Documentation rendering tasks
export const renderDocCameraBasic = task({
    name: "render:doc:camera-basic",
    description: "Render camera basic example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/camera-basic.json -o doc/images/camera-basic.png -w 800 -H 600")
});

export const renderDocObjectSphere = task({
    name: "render:doc:object-sphere",
    description: "Render object sphere example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/object-sphere.json -o doc/images/object-sphere.png -w 800 -H 600")
});

export const renderDocObjectPlaneGrid = task({
    name: "render:doc:object-plane-grid",
    description: "Render object plane grid example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/object-plane-grid.json -o doc/images/object-plane-grid.png -w 800 -H 600")
});

export const renderDocObjectCube = task({
    name: "render:doc:object-cube",
    description: "Render object cube example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/object-cube.json -o doc/images/object-cube.png -w 800 -H 600")
});

export const renderDocObjectMesh = task({
    name: "render:doc:object-mesh",
    description: "Render object mesh example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/object-mesh.json -o doc/images/object-mesh.png -w 800 -H 600")
});

export const renderDocMaterialProperties = task({
    name: "render:doc:material-properties",
    description: "Render material properties example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/material-properties.json -o doc/images/material-properties.png -w 800 -H 600")
});

export const renderDocMaterialReflectivity = task({
    name: "render:doc:material-reflectivity",
    description: "Render material reflectivity example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/material-reflectivity.json -o doc/images/material-reflectivity.png -w 800 -H 600")
});

export const renderDocTextureGridVariations = task({
    name: "render:doc:texture-grid-variations",
    description: "Render texture grid variations example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/texture-grid-variations.json -o doc/images/texture-grid-variations.png -w 800 -H 600")
});

export const renderDocLightingMultiple = task({
    name: "render:doc:lighting-multiple",
    description: "Render lighting multiple example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/lighting-multiple.json -o doc/images/lighting-multiple.png -w 800 -H 600")
});

export const renderDocSceneBackgrounds = task({
    name: "render:doc:scene-backgrounds",
    description: "Render scene backgrounds example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/scene-backgrounds-1.json -o doc/images/scene-backgrounds-1.png -w 400 -H 300 && ./target/release/rtrace -i doc/scenes/scene-backgrounds-2.json -o doc/images/scene-backgrounds-2.png -w 400 -H 300")
});

export const renderDocSceneFog = task({
    name: "render:doc:scene-fog",
    description: "Render scene fog comparison example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/scene-fog-light.json -o doc/images/scene-fog-light.png -w 400 -H 300 && ./target/release/rtrace -i doc/scenes/scene-fog-heavy.json -o doc/images/scene-fog-heavy.png -w 400 -H 300")
});

export const renderDocExampleComplete = task({
    name: "render:doc:example-complete",
    description: "Render complete example for documentation",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace -i doc/scenes/example-complete.json -o doc/images/example-complete.png -w 800 -H 600")
});

export const renderDocAll = task({
    name: "render:doc:all",
    description: "Render all documentation images",
    dependencies: [
        renderDocCameraBasic,
        renderDocObjectSphere,
        renderDocObjectPlaneGrid,
        renderDocObjectCube,
        renderDocObjectMesh,
        renderDocMaterialProperties,
        renderDocMaterialReflectivity,
        renderDocTextureGridVariations,
        renderDocLightingMultiple,
        renderDocSceneBackgrounds,
        renderDocSceneFog,
        renderDocExampleComplete
    ]
});

export const docRender = task({
    name: "doc:render",
    description: "Generate all documentation images",
    dependencies: [renderDocAll]
});

// Debug and development tasks
export const debugKdtree = task({
    name: "debug:kdtree",
    description: "Run KD-tree debugging tool",
    run: exec("cargo run --bin debug_kdtree")
});

export const testBounds = task({
    name: "test:bounds",
    description: "Run plus model bounds testing",
    dependencies: [buildNode],
    run: exec("node scripts/test_plus_bounds.js")
});

// Lint and format tasks
export const lint = task({
    name: "lint",
    description: "Run Rust linting (clippy)",
    run: exec("cargo clippy --workspace -- -D warnings")
});

export const format = task({
    name: "format",
    description: "Format Rust code",
    run: exec("cargo fmt")
});

export const formatCheck = task({
    name: "format:check",
    description: "Check Rust code formatting",
    run: exec("cargo fmt --check")
});

// Clean tasks
export const clean = task({
    name: "clean",
    description: "Clean all build artifacts",
    run: exec("cargo clean && rm -rf target/ rtrace.node *.png node_modules/.cache/")
});

export const cleanRendered = task({
    name: "clean:rendered",
    description: "Clean rendered image files",
    run: exec("rm -f *_rendered.png *.png")
});

// Development workflow tasks
export const dev = task({
    name: "dev",
    description: "Development build (debug mode)",
    dependencies: [buildRust, buildNode]
});

export const ci = task({
    name: "ci", 
    description: "CI pipeline: format check, lint, build all, and test all",
    dependencies: [formatCheck, lint, buildAll, testAll]
});

export const precommit = task({
    name: "precommit",
    description: "Pre-commit checks: format, lint, and test",
    dependencies: [format, lint, test]
});

// Default task
export const defaultTask = task({
    name: "default",
    description: "Default task: build and test",
    dependencies: [build, test]
});

// Make default task available as the default export
export default defaultTask;