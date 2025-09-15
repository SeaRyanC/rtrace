#!/usr/bin/env node

// Test script to verify the new API structure works correctly
console.log('Testing new API structure...\n');

// Test 1: Import tracer directly
console.log('Test 1: Import tracer directly');
try {
    const tracer = require('../tracer/rtrace.node');
    console.log('✓ tracer = require("../tracer/rtrace.node") works');
    console.log('✓ tracer.renderSceneToBuffer available:', typeof tracer.renderSceneToBuffer === 'function');
} catch (error) {
    console.error('✗ Direct tracer import failed:', error.message);
}

// Test 2: Import schema directly
console.log('\nTest 2: Import schema directly');
try {
    const schema = require('../dist/schema/schema.js');
    console.log('✓ schema = require("../dist/schema/schema.js") works');
    console.log('✓ schema.SceneSchema available:', !!schema.SceneSchema);
} catch (error) {
    console.error('✗ Direct schema import failed:', error.message);
}

// Test 3: Import via barrel export
console.log('\nTest 3: Import via barrel export');
try {
    const rtrace = require('../dist/index.js');
    console.log('✓ rtrace = require("../dist/index.js") works');
    console.log('✓ rtrace available, type:', typeof rtrace);
} catch (error) {
    console.error('✗ Barrel export import failed:', error.message);
}

// Test 4: Test actual functionality with tracer
console.log('\nTest 4: Test actual functionality');
try {
    const tracer = require('../tracer/rtrace.node');
    const schema = require('../dist/schema/schema.js');
    
    // Create a minimal scene
    const minimalScene = {
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
    };
    
    // Validate with schema
    const parseResult = schema.SceneSchema.safeParse(minimalScene);
    if (parseResult.success) {
        console.log('✓ Schema validation works');
    } else {
        console.error('✗ Schema validation failed:', parseResult.error.issues[0]);
        return;
    }
    
    // Render with tracer
    const result = tracer.renderSceneToBuffer(JSON.stringify(minimalScene), 50);
    console.log('✓ Rendering works - image dimensions:', result.width + 'x' + result.height);
    
} catch (error) {
    console.error('✗ Functionality test failed:', error.message);
}

console.log('\n🎉 All API structure tests completed!');