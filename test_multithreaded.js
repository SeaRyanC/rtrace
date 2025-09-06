const rtrace = require('./rtrace.node');
const fs = require('fs');

async function testMultiThreadedRendering() {
    console.log('=== Multi-threaded Rendering Test ===\n');
    
    // Create a simple test scene
    const testScene = {
        "camera": {
            "kind": "ortho",
            "position": [0, 0, 5],
            "target": [0, 0, 0],
            "up": [0, 1, 0],
            "width": 6,
            "height": 6
        },
        "objects": [
            {
                "kind": "sphere",
                "center": [0, 0, 0],
                "radius": 1,
                "material": {
                    "color": "#FF0000",
                    "ambient": 0.1,
                    "diffuse": 0.9,
                    "specular": 0.5,
                    "shininess": 32
                }
            },
            {
                "kind": "sphere", 
                "center": [-2, 0, 0],
                "radius": 0.5,
                "material": {
                    "color": "#00FF00",
                    "ambient": 0.1,
                    "diffuse": 0.9,
                    "specular": 0.5,
                    "shininess": 32
                }
            },
            {
                "kind": "sphere",
                "center": [2, 0, 0], 
                "radius": 0.5,
                "material": {
                    "color": "#0000FF",
                    "ambient": 0.1,
                    "diffuse": 0.9,
                    "specular": 0.5,
                    "shininess": 32
                }
            },
            {
                "kind": "plane",
                "point": [0, -1, 0],
                "normal": [0, 1, 0],
                "material": {
                    "color": "#CCCCCC",
                    "ambient": 0.1,
                    "diffuse": 0.9,
                    "specular": 0.1,
                    "shininess": 16
                }
            }
        ],
        "lights": [
            {
                "position": [2, 2, 2],
                "color": "#FFFFFF",
                "intensity": 1.0
            }
        ],
        "scene_settings": {
            "background_color": "#001122",
            "ambient_illumination": {
                "color": "#222222",
                "intensity": 0.1
            }
        }
    };

    const sceneJson = JSON.stringify(testScene, null, 2);

    try {
        // Test 1: Standard multi-threaded rendering (default)
        console.log('1. Standard multi-threaded rendering:');
        console.time('Multi-threaded render');
        const result1 = rtrace.renderScene(sceneJson, './test_multithreaded.png', 400, 300);
        console.timeEnd('Multi-threaded render');
        console.log('   Result:', result1);
        console.log();

        // Test 2: Specific thread count
        console.log('2. Rendering with specific thread count (2 threads):');
        console.time('2-thread render');
        const result2 = rtrace.renderSceneThreaded(sceneJson, './test_2threads.png', 400, 300, 2);
        console.timeEnd('2-thread render');
        console.log('   Result:', result2);
        console.log();

        // Test 3: Single threaded for comparison
        console.log('3. Single-threaded rendering for comparison:');
        console.time('Single-thread render');
        const result3 = rtrace.renderSceneThreaded(sceneJson, './test_singlethread.png', 400, 300, 1);
        console.timeEnd('Single-thread render');
        console.log('   Result:', result3);
        console.log();

        // Test 4: Test with brute force (should still be multi-threaded)
        console.log('4. Brute force rendering (multi-threaded):');
        console.time('Brute force render');
        const result4 = rtrace.renderSceneBruteForce(sceneJson, './test_brute_multithreaded.png', 400, 300);
        console.timeEnd('Brute force render');
        console.log('   Result:', result4);
        console.log();

        console.log('✅ All multi-threaded rendering tests completed successfully!');

        // List generated files
        console.log('\nGenerated image files:');
        const files = ['./test_multithreaded.png', './test_2threads.png', './test_singlethread.png', './test_brute_multithreaded.png'];
        files.forEach(file => {
            if (fs.existsSync(file)) {
                const stats = fs.statSync(file);
                console.log(`   ${file} (${stats.size} bytes)`);
            }
        });

    } catch (error) {
        console.error('❌ Error:', error);
        process.exit(1);
    }
}

testMultiThreadedRendering();