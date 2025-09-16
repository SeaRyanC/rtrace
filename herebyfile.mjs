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

        // Find the .node file in dist/ and create index.js wrapper
        const distFiles = readdirSync('dist');
        const nodeFile = distFiles.find(f => f.endsWith('.node'));

        if (nodeFile) {
            // Create a JS wrapper that exports the native module
            const jsWrapper = `module.exports = require('./${nodeFile}');`;
            writeFileSync('dist/index.js', jsWrapper);
            console.log('Created dist/index.js wrapper');
        } else {
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
    return () => {
        console.log('Testing NAPI bindings with new API structure...');

        // Import the built bindings using require since it's CommonJS
        const { renderSceneToBuffer } = require('./dist/index.js');

        // Test render scene to buffer function with a minimal scene
        const minimalScene = JSON.stringify({
            camera: {
                kind: "perspective",
                position: [0, -5, 0],
                target: [0, 0, 0],
                up: [0, 0, 1],
                fov: 45,
                width: 1.0,
                height: 1.0
            },
            scene_settings: {
                ambient_illumination: { 
                    color: "#202020",
                    intensity: 0.1 
                },
                background_color: "#001122"
            },
            objects: [
                {
                    kind: "sphere",
                    center: [0, 0, 0],
                    radius: 1,
                    material: { 
                        color: "#ff0000",
                        ambient: 0.1,
                        diffuse: 0.8,
                        specular: 0.4,
                        shininess: 32
                    }
                }
            ],
            lights: [
                {
                    position: [2, -3, 2],
                    color: "#FFFFFF",
                    intensity: 1.0
                }
            ]
        });

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
    return async () => {
        console.log('=== rtrace Node.js Bindings Demo ===\n');

        const rtrace = await import('./dist/index.js');

        // Example 1: Render scene to buffer and inspect metadata
        console.log('1. Render a simple scene to buffer:');
        const simpleScene = JSON.stringify({
            camera: {
                kind: "perspective",
                position: [0, -5, 0],
                target: [0, 0, 0],
                up: [0, 0, 1],
                fov: 45,
                width: 1.0,
                height: 1.0
            },
            scene_settings: {
                ambient_illumination: { 
                    color: "#202020",
                    intensity: 0.1 
                },
                background_color: "#001122"
            },
            objects: [
                {
                    kind: "sphere",
                    center: [0, 0, 0],
                    radius: 1,
                    material: { 
                        color: "#ff0000",
                        ambient: 0.1,
                        diffuse: 0.8,
                        specular: 0.4,
                        shininess: 32
                    }
                }
            ],
            lights: [
                {
                    position: [2, -3, 2],
                    color: "#FFFFFF",
                    intensity: 1.0
                }
            ]
        });

        const result = rtrace.renderSceneToBuffer(simpleScene, 50);
        console.log('   Image dimensions:', result.width + 'x' + result.height);
        console.log('   Stride (bytes per row):', result.stride);
        console.log('   Data length (RGBA bytes):', result.data.length);
        console.log('   Expected data length:', result.width * result.height * 4);
        console.log();

        // Example 2: Available render functions
        console.log('2. Available render functions:');
        console.log('   - renderScene(): Render to file with default multi-threading');
        console.log('   - renderSceneThreaded(): Render to file with specific thread count');
        console.log('   - renderSceneFromFile(): Load scene from JSON file and render');
        console.log('   - renderSceneFromFileThreaded(): Load from file with specific thread count');
        console.log('   - renderSceneToBuffer(): Render to memory buffer for programmatic use');
        console.log();

        console.log('✅ Demo completed successfully!');
    };
}

// Analyze plus STL file (from scripts/analyze_plus.js)
function analyzePlus() {
    return () => {
        // We can't directly get triangle count from the current API, 
        // but we can estimate from the binary STL structure
        function getTriangleCountFromBinarySTL(filePath) {
            const buffer = readFileSync(filePath);
            if (buffer.length < 84) {
                throw new Error('STL file too short');
            }
            
            // Read triangle count from bytes 80-83 (little endian)
            const triangleCount = buffer.readUInt32LE(80);
            const expectedSize = 84 + triangleCount * 50; // header + count + triangles * 50 bytes each
            
            return { triangleCount, fileSize: buffer.length, expectedSize };
        }

        console.log("📊 Plus.stl Analysis");
        const info = getTriangleCountFromBinarySTL('examples/plus.stl');
        console.log(`Triangle count: ${info.triangleCount}`);
        console.log(`File size: ${info.fileSize} bytes`);
        console.log(`Expected size: ${info.expectedSize} bytes`);
        console.log(`Size match: ${info.fileSize === info.expectedSize ? '✓' : '✗'}`);
        
        if (info.triangleCount < 1000) {
            console.log("\nThis is a small mesh, so brute force should be fast enough to compare with k-d tree.");
        } else if (info.triangleCount < 10000) {
            console.log("\nThis is a medium mesh, k-d tree should provide some speedup.");  
        } else {
            console.log("\nThis is a large mesh, k-d tree should provide significant speedup.");
        }
    };
}

// Run radial spheres example (from scripts/radial_spheres_example.js)
function runRadialSpheresExample() {
    return async () => {
        console.log('=== Radial Spheres Scene Demo ===\n');

        const rtrace = await import('./dist/index.js');

        // Function to create a sphere with given position, color, and radius
        function createSphere(center, color, radius = 0.8) {
            return {
                kind: "sphere",
                center: center,
                radius: radius,
                material: {
                    color: color,
                    ambient: 0.1,
                    diffuse: 0.8,
                    specular: 0.4,
                    shininess: 32
                }
            };
        }

        // Function to convert HSV to hex color
        function hsvToHex(h, s, v) {
            const c = v * s;
            const x = c * (1 - Math.abs((h / 60) % 2 - 1));
            const m = v - c;
            
            let r, g, b;
            if (h >= 0 && h < 60) {
                r = c; g = x; b = 0;
            } else if (h >= 60 && h < 120) {
                r = x; g = c; b = 0;
            } else if (h >= 120 && h < 180) {
                r = 0; g = c; b = x;
            } else if (h >= 180 && h < 240) {
                r = 0; g = x; b = c;
            } else if (h >= 240 && h < 300) {
                r = x; g = 0; b = c;
            } else {
                r = c; g = 0; b = x;
            }
            
            r = Math.round((r + m) * 255);
            g = Math.round((g + m) * 255);
            b = Math.round((b + m) * 255);
            
            return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
        }

        // Generate spheres in a radial pattern
        const objects = [];
        const numRings = 3;
        const spheresPerRing = 8;
        
        for (let ring = 1; ring <= numRings; ring++) {
            const radius = ring * 2.5;
            const sphereCount = spheresPerRing * ring;
            
            for (let i = 0; i < sphereCount; i++) {
                const angle = (i / sphereCount) * 2 * Math.PI;
                const x = Math.cos(angle) * radius;
                const y = Math.sin(angle) * radius;
                const z = 0;
                
                // Color based on ring and position
                const hue = (angle / (2 * Math.PI)) * 360;
                const saturation = 0.8;
                const value = 1.0 - (ring - 1) * 0.2; // Dimmer for outer rings
                const color = hsvToHex(hue, saturation, value);
                
                objects.push(createSphere([x, y, z], color, 0.4));
            }
        }

        // Add a central sphere
        objects.push(createSphere([0, 0, 0], "#FFFFFF", 0.6));

        const scene = {
            camera: {
                kind: "perspective",
                position: [0, -15, 8],
                target: [0, 0, 0],
                up: [0, 0, 1],
                fov: 45,
                width: 1.0,
                height: 1.0
            },
            scene_settings: {
                ambient_illumination: { 
                    color: "#1a1a1a",
                    intensity: 0.1 
                },
                background_color: "#000011"
            },
            objects: objects,
            lights: [
                {
                    position: [5, -10, 10],
                    color: "#FFFFFF",
                    intensity: 1.2
                },
                {
                    position: [-5, -10, 5],
                    color: "#FFAA88",
                    intensity: 0.8
                }
            ]
        };

        console.log(`Generated scene with ${objects.length} spheres`);
        console.log('Rendering radial spheres scene...');
        
        const result = rtrace.renderSceneToBuffer(JSON.stringify(scene), 400);
        console.log(`✅ Rendered ${result.width}x${result.height} image`);
        console.log('Scene demonstrates: colored spheres, radial patterns, multiple lights');
    };
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
    return async () => {
        const rtrace = require('./dist/index.js');

        async function renderScene(sceneFile, outputPrefix, size = 1000) {
            console.log(`\n=== Rendering ${sceneFile} ===`);
            
            const sceneJson = readFileSync(sceneFile, 'utf8');
            
            // Default k-d tree version
            console.log("Rendering with k-d tree (default)...");
            const kdtreeOutput = `examples/${outputPrefix}_kdtree_${size}.png`;
            const kdtreeResult = rtrace.renderScene(sceneJson, kdtreeOutput, size);
            console.log("✓", kdtreeResult);
            
            // Multi-threaded version with specific thread count
            console.log("Rendering with 4 threads...");
            const threadedOutput = `examples/${outputPrefix}_4threads_${size}.png`;
            const threadedResult = rtrace.renderSceneThreaded(sceneJson, threadedOutput, size, 4);
            console.log("✓", threadedResult);
        }

        console.log("🔧 Plus.stl Debug Renders");
        console.log("Generating optimized renders with k-d tree acceleration");
        
        // Render all three views
        await renderScene('examples/plus_front.json', 'plus_front', 1000);
        await renderScene('examples/plus_side.json', 'plus_side', 1000);
        await renderScene('examples/plus_perspective.json', 'plus_perspective', 1000);
        
        console.log("\n🎉 All renders completed!");
        console.log("\nGenerated images:");
        console.log("Front view:");
        console.log("  - K-d tree:     examples/plus_front_kdtree_1000.png");
        console.log("  - 4 threads:    examples/plus_front_4threads_1000.png");
        console.log("Side view:");
        console.log("  - K-d tree:     examples/plus_side_kdtree_1000.png");
        console.log("  - 4 threads:    examples/plus_side_4threads_1000.png");
        console.log("Perspective view:");
        console.log("  - K-d tree:     examples/plus_perspective_kdtree_1000.png");
        console.log("  - 4 threads:    examples/plus_perspective_4threads_1000.png");
        
        console.log("\nNote: renderSceneBruteForce was removed - all renders now use k-d tree optimization");
    };
}

// Render plus high resolution (from scripts/render_plus_hires.js)
function renderPlusHires() {
    return async () => {
        const rtrace = await import('./dist/index.js');

        console.log("Loading plus.stl scene for high-res rendering...");
        const sceneJson = readFileSync('examples/plus_front.json', 'utf8');
        
        // Render in high resolution with different threading options
        console.log("\nRendering plus.stl with diagonal 1000 using k-d tree acceleration...");
        const resultKdTree = rtrace.renderScene(sceneJson, 'examples/plus_kdtree_1000.png', 1000);
        console.log("K-d tree result:", resultKdTree);
        
        console.log("\nRendering plus.stl with diagonal 1000 using single thread...");
        const resultSingleThread = rtrace.renderSceneThreaded(sceneJson, 'examples/plus_single_thread_1000.png', 1000, 1);
        console.log("Single thread result:", resultSingleThread);
        
        console.log("\n✅ High-res renders completed successfully!");
        console.log("Compare these high-resolution images:");
        console.log("- Multi-threaded: examples/plus_kdtree_1000.png");
        console.log("- Single thread:  examples/plus_single_thread_1000.png");
        console.log("\nNote: renderSceneBruteForce was removed - all renders now use k-d tree optimization");
    };
}

// Run multithreaded demo (from scripts/multithreaded_demo.js)
function runMultithreadedDemo() {
    return async () => {
        console.log('🚀 rtrace Multi-threaded Rendering Demo');
        console.log('========================================\n');

        const rtrace = await import('./dist/index.js');

        // Test with an existing scene
        const sceneFile = 'examples/simple_cube.json';

        if (!existsSync(sceneFile)) {
            console.error(`❌ Scene file ${sceneFile} not found`);
            console.log('Available scenes:');
            const examples = readdirSync('examples').filter(f => f.endsWith('.json'));
            examples.forEach(f => console.log(`   examples/${f}`));
            return;
        }

        console.log('Demonstrating multi-threaded rendering capabilities...\n');
        
        // Test different thread configurations
        const tests = [
            { name: 'Single Thread', threads: 1 },
            { name: 'Dual Thread', threads: 2 }, 
            { name: 'All Cores', threads: null },
        ];
        
        for (const test of tests) {
            console.log(`📊 ${test.name} Rendering:`);
            console.time(test.name);
            
            try {
                let result;
                if (test.threads === null) {
                    // Use default multi-threading (all cores)
                    result = rtrace.renderSceneFromFile(sceneFile, `./demo_${test.name.toLowerCase().replace(' ', '_')}.png`, 500);
                } else {
                    // Use specific thread count
                    result = rtrace.renderSceneFromFileThreaded(sceneFile, `./demo_${test.name.toLowerCase().replace(' ', '_')}.png`, 500, test.threads);
                }
                console.timeEnd(test.name);
                console.log(`   ✓ ${result}\n`);
            } catch (error) {
                console.error(`   ❌ Error: ${error.message}\n`);
            }
        }
        
        console.log('🎯 Key Benefits of Multi-threaded Rendering:');
        console.log('   • Faster rendering times through parallel processing');
        console.log('   • Better utilization of modern multi-core processors');
        console.log('   • Configurable thread count for optimal performance');
        console.log('   • Identical output quality regardless of thread count');
        console.log('   • Seamless integration with existing API');
        
        console.log('\n📝 API Usage:');
        console.log('   rtrace.renderScene(json, output, size)              // Auto multi-threading');
        console.log('   rtrace.renderSceneThreaded(json, output, size, threads) // Custom threads');
        console.log('   rtrace.renderSceneFromFile(file, output, size)      // File-based rendering');
        
        // Clean up demo files
        setTimeout(() => {
            ['./demo_single_thread.png', './demo_dual_thread.png', './demo_all_cores.png'].forEach(file => {
                if (existsSync(file)) {
                    unlinkSync(file);
                }
            });
            console.log('\n🧹 Demo files cleaned up.');
        }, 1000);
    };
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
    run: exec("npx napi build --release --cargo-cwd bindings/node dist")
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