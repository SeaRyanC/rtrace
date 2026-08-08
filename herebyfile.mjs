import { task } from "hereby";
import { spawn } from "child_process";
import { promisify } from "util";
import { readdir, access } from "fs/promises";
import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, unlinkSync } from "fs";
import { extname, basename } from "path";
import { createRequire } from "module";

const require = createRequire(import.meta.url);

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

// Helper function to ensure directory exists
function ensureDir(dirPath) {
    if (!existsSync(dirPath)) {
        mkdirSync(dirPath, { recursive: true });
    }
}

// Create wrapper files for the build outputs (from scripts/create-wrappers.js)
function createWrappers() {
    return () => {
        console.log('Creating wrapper files...');
        
        // Ensure dist directory exists
        ensureDir('dist');
        ensureDir('dist/schema');

        // Select the native addon for the current runtime. The platform-qualified
        // names allow one package to contain artifacts for several platforms.
        const jsWrapper = `const { platform, arch } = process;
const libc = platform === 'linux' && process.report?.getReport().header.glibcVersionRuntime ? 'gnu' : 'musl';
const platformKey = {
  'darwin-arm64': 'darwin-arm64',
  'darwin-x64': 'darwin-x64',
  'linux-arm64': \`linux-arm64-\${libc}\`,
  'linux-x64': \`linux-x64-\${libc}\`,
  'win32-arm64': 'win32-arm64-msvc',
  'win32-x64': 'win32-x64-msvc',
}[\`\${platform}-\${arch}\`];

if (!platformKey) {
  throw new Error(\`Unsupported platform: \${platform}-\${arch}\`);
}

module.exports = require(\`./index.\${platformKey}.node\`);
`;
        writeFileSync('dist/index.js', jsWrapper);
        console.log('Created platform-aware dist/index.js wrapper');

        if (!readdirSync('dist').some(f => f.endsWith('.node'))) {
            throw new Error('No .node file found in dist/ directory');
        }

        // Create schema entry point
        const schemaEntryPoint = `module.exports = require('./schema');`;
        writeFileSync('dist/schema/index.js', schemaEntryPoint);
        console.log('Created dist/schema/index.js entry point');

        console.log('Wrapper files created successfully!');
    };
}

// Test NAPI bindings (from scripts/test.js)
function testNapi() {
    return async () => {
        console.log('Testing NAPI bindings with new API structure...');

        // Import the built bindings using require since it's CommonJS
        const { renderSceneToBuffer } = require('./dist/index.js');
        const { getMinimalTestSceneJson } = require('./scripts/test-scenes.js');

        // Test render scene to buffer function with a minimal scene
        const minimalScene = getMinimalTestSceneJson();

        const result = renderSceneToBuffer(minimalScene, 100);
        console.log('✓ renderSceneToBuffer() returned image with dimensions:', result.width + 'x' + result.height);

        if (!result.data || result.data.length === 0) {
            throw new Error('Expected image data, got empty buffer');
        }

        if (result.width <= 0 || result.height <= 0) {
            throw new Error('Expected positive dimensions, got: ' + result.width + 'x' + result.height);
        }

        console.log('🎉 All tests passed! NAPI bindings are working correctly with new API structure.');
    };
}

// Run example demo (from scripts/example.js)
function runExample() {
    return exec("node scripts/example.js");
}

// Analyze plus STL file (from scripts/analyze_plus.js)
function analyzePlus() {
    return exec("node scripts/analyze_plus.js");
}

// Run radial spheres example (from scripts/radial_spheres_example.js)
function runRadialSpheresExample() {
    return exec("node scripts/radial_spheres_example.js");
}

// Test plus bounds (from scripts/test_plus_bounds.js)
function testPlusBounds() {
    return async () => {
        console.log("Loading plus.stl scene...");
        
        const rtrace = await import('./dist/index.js');
        const sceneJson = readFileSync('examples/plus_front.json', 'utf8');
        
        // Render with k-d tree (default, optimized)
        console.log("\nRendering plus.stl with k-d tree acceleration...");
        const resultKdTree = rtrace.renderScene(sceneJson, 'examples/plus_kdtree_500.png', 500);
        console.log("K-d tree result:", resultKdTree);
        
        // Render with threading control
        console.log("\nRendering plus.stl with specific thread count (4 threads)...");
        const resultThreaded = rtrace.renderSceneThreaded(sceneJson, 'examples/plus_threaded_500.png', 500, 4);
        console.log("Threaded result:", resultThreaded);
        
        console.log("\n✅ Both renders completed successfully!");
        console.log("Compare these images:");
        console.log("- K-d tree:     examples/plus_kdtree_500.png");
        console.log("- 4 threads:    examples/plus_threaded_500.png");
        console.log("\nNote: renderSceneBruteForce was removed - all renders now use k-d tree optimization");
    };
}

// Render plus debug images (from scripts/render_plus_debug.js)
function renderPlusDebug() {
    return exec("node scripts/render_plus_debug.js");
}

// Render plus high resolution (from scripts/render_plus_hires.js)
function renderPlusHires() {
    return exec("node scripts/render_plus_hires.js");
}

// Run multithreaded demo (from scripts/multithreaded_demo.js)
function runMultithreadedDemo() {
    return exec("node scripts/multithreaded_demo.js");
}

// Schema tasks first
export const schemaCompile = task({
    name: "schema:compile",
    description: "Compile TypeScript schema files",
    run: exec("npx tsc")
});

// Build tasks (basic components)
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
    run: exec("npx napi build --platform --release --cargo-cwd bindings/node dist")
});

export const buildWrapper = task({
    name: "build:wrapper",
    description: "Create Node.js wrapper files",
    dependencies: [buildNode],
    run: createWrappers()
});

// Main build tasks (depend on components above)
export const build = task({
    name: "build",
    description: "Build all components",
    dependencies: [schemaCompile, buildNode, buildWrapper]
});

export const buildAll = task({
    name: "build:all",
    description: "Build all components including CLI",
    dependencies: [buildRustRelease, buildCli, schemaCompile, buildNode, buildWrapper]
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
    dependencies: [build],
    run: testNapi()
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
    dependencies: [build],
    run: runExample()
});

export const exampleRadial = task({
    name: "example:radial",
    description: "Run radial spheres example",
    dependencies: [build],
    run: runRadialSpheresExample()
});

export const exampleMultithreaded = task({
    name: "example:multithreaded",
    description: "Run multithreaded demo",
    dependencies: [build],
    run: runMultithreadedDemo()
});

export const exampleAnalyze = task({
    name: "example:analyze",
    description: "Run plus model analysis",
    dependencies: [build],
    run: analyzePlus()
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
    run: exec("./target/release/rtrace-cli -i examples/simple_sphere.json -o simple_sphere_rendered.png -s 1000")
});

export const renderExampleRadial = task({
    name: "render:radial", 
    description: "Render radial spheres example",
    dependencies: [buildCli],
    run: exec("./target/release/rtrace-cli -i examples/radial_spheres.json -o radial_spheres_rendered.png -s 1000")
});

export const renderExamplePlus = task({
    name: "render:plus",
    description: "Render plus perspective example", 
    dependencies: [buildCli],
    run: exec("./target/release/rtrace-cli -i examples/plus_perspective.json -o plus_perspective_rendered.png -s 1000")
});

export const renderExampleEspresso = task({
    name: "render:espresso",
    description: "Render espresso tray example",
    dependencies: [buildCli], 
    run: exec("./target/release/rtrace-cli -i examples/espresso_tray_top.json -o espresso_tray_rendered.png -s 1000")
});

export const renderAll = task({
    name: "render:all",
    description: "Render all example images",
    dependencies: [renderExampleSimple, renderExampleRadial, renderExamplePlus, renderExampleEspresso]
});

export const renderHires = task({
    name: "render:hires",
    description: "Render high-resolution images",
    dependencies: [build],
    run: renderPlusHires()
});

export const renderDebug = task({
    name: "render:debug",
    description: "Render debug images",
    dependencies: [build],
    run: renderPlusDebug()
});

// Documentation rendering tasks - dynamically generated
async function createDocRenderTasks() {
    // Discover all JSON scene files in doc/scenes/
    const docSceneFiles = (await readdir('doc/scenes'))
        .filter(file => extname(file) === '.json')
        .sort();

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
            params: "--anti-aliasing none",
            description: "Demonstrates no sampling and no jitter (deterministic)"
        }
    ];

    // Outline demo scenes that need special handling
    const outlineDemoScenes = [
        {
            name: "outline-demo-no-outline",
            scene: "outline_demo.json",
            params: "--anti-aliasing none",
            description: "Outline demo without outline detection",
            modifyScene: true, // Need to remove outline config
        },
        {
            name: "outline-demo-basic", 
            scene: "outline_demo.json",
            params: "--anti-aliasing none",
            description: "Outline demo with basic outline detection"
        },
        {
            name: "outline-demo-complex",
            scene: "doc/scenes/outline_complex.json", 
            params: "--anti-aliasing none",
            description: "Complex outline demo with advanced parameters"
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
        const taskName = `renderDoc${baseName.split(/[-_]/).map(word => 
            word.charAt(0).toUpperCase() + word.slice(1)
        ).join('')}`;
        
        // Build command with base parameters
        let command = `./target/release/rtrace-cli -i doc/scenes/${file} -o doc/images/${baseName}.png -s 500`;
        
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
        const taskName = `renderDoc${special.name.split(/[-_]/).map(word => 
            word.charAt(0).toUpperCase() + word.slice(1)
        ).join('')}`;
        
        const command = `./target/release/rtrace-cli -i doc/scenes/${special.scene} -o doc/images/${special.name}.png -s 500 ${special.params}`;
        
        docRenderTasks[taskName] = task({
            name: `render:doc:${special.name}`,
            description: special.description,
            dependencies: [buildCli],
            run: exec(command)
        });
        docDependencies.push(docRenderTasks[taskName]);
    }

    // Create tasks for outline demo scene variants
    for (const outlineDemo of outlineDemoScenes) {
        const taskName = `renderDoc${outlineDemo.name.split(/[-_]/).map(word => 
            word.charAt(0).toUpperCase() + word.slice(1)
        ).join('')}`;
        
        let command;
        if (outlineDemo.modifyScene) {
            // For the no-outline variant, we need to use a scene without outline config
            // We'll generate it on-the-fly by modifying the scene
            const sceneBasePath = outlineDemo.scene.includes('/') ? outlineDemo.scene : `examples/${outlineDemo.scene}`;
            command = `node -e "
                const fs = require('fs');
                const scene = JSON.parse(fs.readFileSync('${sceneBasePath}', 'utf8'));
                if (scene.scene_settings && scene.scene_settings.outline) {
                    delete scene.scene_settings.outline;
                }
                fs.writeFileSync('/tmp/${outlineDemo.name}.json', JSON.stringify(scene, null, 2));
            " && ./target/release/rtrace-cli -i /tmp/${outlineDemo.name}.json -o doc/images/${outlineDemo.name}.png -s 500 ${outlineDemo.params}`;
        } else {
            const sceneBasePath = outlineDemo.scene.includes('/') ? outlineDemo.scene : `examples/${outlineDemo.scene}`;
            command = `./target/release/rtrace-cli -i ${sceneBasePath} -o doc/images/${outlineDemo.name}.png -s 500 ${outlineDemo.params}`;
        }
        
        docRenderTasks[taskName] = task({
            name: `render:doc:${outlineDemo.name}`,
            description: outlineDemo.description,
            dependencies: [buildCli],
            run: exec(command)
        });
        docDependencies.push(docRenderTasks[taskName]);
    }

    // Create tasks for multi-file scenes
    for (const scene of docMultiFileScenes) {
        const taskName = `renderDoc${scene.name.split(/[-_]/).map(word => 
            word.charAt(0).toUpperCase() + word.slice(1)
        ).join('')}`;
        
        const commands = scene.files.map(file => {
            const outputName = basename(file, '.json');
            return `./target/release/rtrace-cli -i doc/scenes/${file} -o doc/images/${outputName}.png -s 500`;
        }).join(' && ');

        docRenderTasks[taskName] = task({
            name: `render:doc:${scene.name}`,
            description: `Render ${scene.name} examples for documentation`,
            dependencies: [buildCli],
            run: exec(commands)
        });
        docDependencies.push(docRenderTasks[taskName]);
    }

    return { docRenderTasks, docDependencies };
}

// Initialize doc render tasks
const { docRenderTasks, docDependencies } = await createDocRenderTasks();

// Export individual render tasks (dynamically generated)
export const renderDocCameraBasic = docRenderTasks.renderDocCameraBasic;
export const renderDocCameraPerspective = docRenderTasks.renderDocCameraPerspective;
export const renderDocObjectSphere = docRenderTasks.renderDocObjectSphere;
export const renderDocObjectPlaneGrid = docRenderTasks.renderDocObjectPlaneGrid;
export const renderDocObjectCube = docRenderTasks.renderDocObjectCube;
export const renderDocObjectMesh = docRenderTasks.renderDocObjectMesh;
export const renderDocMaterialProperties = docRenderTasks.renderDocMaterialProperties;
export const renderDocMaterialReflectivity = docRenderTasks.renderDocMaterialReflectivity;
export const renderDocTextureGridVariations = docRenderTasks.renderDocTextureGridVariations;
export const renderDocCheckerboardBasic = docRenderTasks.renderDocCheckerboardBasic;
export const renderDocCheckerboardAdvanced = docRenderTasks.renderDocCheckerboardAdvanced;
export const renderDocLightingMultiple = docRenderTasks.renderDocLightingMultiple;
export const renderDocSamplingAntialiasing = docRenderTasks.renderDocSamplingAntialiasing;
export const renderDocSamplingAntialiasingNosamples = docRenderTasks.renderDocSamplingAntialiasingNosamples;
export const renderDocExampleComplete = docRenderTasks.renderDocExampleComplete;
export const renderDocOutlineComplex = docRenderTasks.renderDocOutlineComplex;
export const renderDocSceneBackgrounds = docRenderTasks.renderDocSceneBackgrounds;
export const renderDocSceneFog = docRenderTasks.renderDocSceneFog;
export const renderDocOutlineDemoNoOutline = docRenderTasks.renderDocOutlineDemoNoOutline;
export const renderDocOutlineDemoBasic = docRenderTasks.renderDocOutlineDemoBasic;
export const renderDocOutlineDemoComplex = docRenderTasks.renderDocOutlineDemoComplex;

// New dynamically discovered tasks (safe access with fallback)
export const renderDocFineGridDemo = docRenderTasks.renderDocFineGridDemo;
export const renderDocFogDemonstration = docRenderTasks.renderDocFogDemonstration;
export const renderDocOrthoGridDemo = docRenderTasks.renderDocOrthoGridDemo;
export const renderDocQuincunxDemo = docRenderTasks.renderDocQuincunxDemo;
export const renderDocSamplingComparison = docRenderTasks.renderDocSamplingComparison;
export const renderDocSceneBackgrounds1 = docRenderTasks.renderDocSceneBackgrounds1;
export const renderDocSceneBackgrounds2 = docRenderTasks.renderDocSceneBackgrounds2;
export const renderDocSceneFogHeavy = docRenderTasks.renderDocSceneFogHeavy;
export const renderDocSceneFogLight = docRenderTasks.renderDocSceneFogLight;
export const renderDocSideViewGrid = docRenderTasks.renderDocSideViewGrid;

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
    dependencies: [build],
    run: testPlusBounds()
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

// Schema validation tasks
export const schemaGenerate = task({
    name: "schema:generate",
    description: "Generate JSON schema from Zod schema",
    dependencies: [schemaCompile],
    run: exec("node dist/schema/validate-schema.js --generate-schema")
});

export const schemaValidate = task({
    name: "schema:validate",
    description: "Validate all scene files using Zod schema",
    dependencies: [schemaCompile],
    run: exec("node dist/schema/validate-schema.js --validate")
});

export const schemaAll = task({
    name: "schema:all",
    description: "Compile schema, generate JSON schema, and validate all files",
    dependencies: [schemaGenerate, schemaValidate]
});

// Development workflow tasks
export const dev = task({
    name: "dev",
    description: "Development build (debug mode)",
    dependencies: [buildRust, buildNode]
});

export const ci = task({
    name: "ci", 
    description: "CI pipeline: format check, lint, build all, test all, and validate schema",
    dependencies: [formatCheck, lint, buildAll, testAll, schemaValidate]
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