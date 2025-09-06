import { task } from "hereby";
import { spawn } from "child_process";
import { promisify } from "util";
import { readdir } from "fs/promises";
import { extname, basename } from "path";

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

// Documentation rendering tasks - dynamically generated
const docSceneFiles = [
    "camera-basic.json",
    "camera-perspective.json", 
    "object-sphere.json",
    "object-plane-grid.json",
    "object-cube.json",
    "object-mesh.json",
    "material-properties.json",
    "material-reflectivity.json",
    "texture-grid-variations.json",
    "lighting-multiple.json",
    "sampling-antialiasing.json",
    "example-complete.json"
];

// Metadata for scenes that need special command line parameters
const docSceneMetadata = {
    "sampling-antialiasing.json": {
        "samples": 4,
        "description": "Demonstrates stochastic subsampling for anti-aliasing"
    }
};

// Special scenes that need multiple variants
const docSpecialScenes = [
    {
        name: "sampling-antialiasing-nosamples",
        scene: "sampling-antialiasing.json",
        params: "--samples 1 --no-jitter",
        description: "Demonstrates no sampling and no jitter (deterministic)"
    }
];

// Multi-file scenes that need special handling
const docMultiFileScenes = [
    {
        name: "scene-backgrounds",
        files: ["scene-backgrounds-1.json", "scene-backgrounds-2.json"]
    },
    {
        name: "scene-fog", 
        files: ["scene-fog-light.json", "scene-fog-heavy.json"]
    }
];

// Create tasks for single-file scenes
const docRenderTasks = {};
const docDependencies = [];

for (const file of docSceneFiles) {
    const baseName = basename(file, '.json');
    const taskName = `renderDoc${baseName.split('-').map(word => 
        word.charAt(0).toUpperCase() + word.slice(1)
    ).join('')}`;
    
    // Build command with base parameters
    let command = `./target/release/rtrace -i doc/scenes/${file} -o doc/images/${baseName}.png -w 400 -H 300`;
    
    // Add metadata-based parameters if available
    const metadata = docSceneMetadata[file];
    if (metadata) {
        if (metadata.samples) {
            command += ` --samples ${metadata.samples}`;
        }
    }
    
    docRenderTasks[taskName] = task({
        name: `render:doc:${baseName}`,
        description: `Render ${baseName} example for documentation`,
        dependencies: [buildCli],
        run: exec(command)
    });
    docDependencies.push(docRenderTasks[taskName]);
}

// Create tasks for special scene variants
for (const special of docSpecialScenes) {
    const taskName = `renderDoc${special.name.split('-').map(word => 
        word.charAt(0).toUpperCase() + word.slice(1)
    ).join('')}`;
    
    const command = `./target/release/rtrace -i doc/scenes/${special.scene} -o doc/images/${special.name}.png -w 400 -H 300 ${special.params}`;
    
    docRenderTasks[taskName] = task({
        name: `render:doc:${special.name}`,
        description: special.description,
        dependencies: [buildCli],
        run: exec(command)
    });
    docDependencies.push(docRenderTasks[taskName]);
}

// Create tasks for multi-file scenes
for (const scene of docMultiFileScenes) {
    const taskName = `renderDoc${scene.name.split('-').map(word => 
        word.charAt(0).toUpperCase() + word.slice(1)
    ).join('')}`;
    
    const commands = scene.files.map(file => {
        const outputName = basename(file, '.json');
        return `./target/release/rtrace -i doc/scenes/${file} -o doc/images/${outputName}.png -w 400 -H 300`;
    }).join(' && ');

    docRenderTasks[taskName] = task({
        name: `render:doc:${scene.name}`,
        description: `Render ${scene.name} examples for documentation`,
        dependencies: [buildCli],
        run: exec(commands)
    });
    docDependencies.push(docRenderTasks[taskName]);
}

// Export individual render tasks
export const renderDocCameraBasic = docRenderTasks.renderDocCameraBasic;
export const renderDocCameraPerspective = docRenderTasks.renderDocCameraPerspective;
export const renderDocObjectSphere = docRenderTasks.renderDocObjectSphere;
export const renderDocObjectPlaneGrid = docRenderTasks.renderDocObjectPlaneGrid;
export const renderDocObjectCube = docRenderTasks.renderDocObjectCube;
export const renderDocObjectMesh = docRenderTasks.renderDocObjectMesh;
export const renderDocMaterialProperties = docRenderTasks.renderDocMaterialProperties;
export const renderDocMaterialReflectivity = docRenderTasks.renderDocMaterialReflectivity;
export const renderDocTextureGridVariations = docRenderTasks.renderDocTextureGridVariations;
export const renderDocLightingMultiple = docRenderTasks.renderDocLightingMultiple;
export const renderDocSamplingAntialiasing = docRenderTasks.renderDocSamplingAntialiasing;
export const renderDocSamplingAntialiasingNosamples = docRenderTasks.renderDocSamplingAntialiasingNosamples;
export const renderDocExampleComplete = docRenderTasks.renderDocExampleComplete;
export const renderDocSceneBackgrounds = docRenderTasks.renderDocSceneBackgrounds;
export const renderDocSceneFog = docRenderTasks.renderDocSceneFog;

export const renderDocAll = task({
    name: "render:doc:all", 
    description: "Render all documentation images",
    dependencies: docDependencies
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