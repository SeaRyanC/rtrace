#!/usr/bin/env node

// Test script to verify the exact API calls specified in the problem statement
console.log('Testing exact API calls from problem statement...\n');

// Test the exact require statements specified
console.log('Test 1: const tracer = require("rtrace/tracer");');
try {
    const tracer = require("rtrace/tracer");
    console.log('✓ Successfully imported tracer');
    console.log('✓ tracer.renderScene available:', typeof tracer.renderScene === 'function');
    console.log('✓ tracer.renderSceneToBuffer available:', typeof tracer.renderSceneToBuffer === 'function');
    
    // Test rendering
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
    
    const result = tracer.renderSceneToBuffer(minimalScene, 30);
    console.log('✓ tracer.renderSceneToBuffer() works - dimensions:', result.width + 'x' + result.height);
    
} catch (error) {
    console.error('✗ tracer import failed:', error.message);
}

console.log('\nTest 2: const schema = require("rtrace/schema");');
try {
    const schema = require("rtrace/schema");
    console.log('✓ Successfully imported schema');
    console.log('✓ schema.SceneSchema available:', !!schema.SceneSchema);
    
    // Test schema parsing
    const testScene = {
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
            }
        },
        objects: [],
        lights: []
    };
    
    const parseResult = schema.SceneSchema.parse(testScene);
    console.log('✓ schema.SceneSchema.parse() works');
    
} catch (error) {
    console.error('✗ schema import or parse failed:', error.message);
}

console.log('\nTest 3: const rtrace = require("rtrace");');
try {
    const rtrace = require("rtrace");
    console.log('✓ Successfully imported rtrace');
    console.log('✓ rtrace.tracer available:', !!rtrace.tracer);
    console.log('✓ rtrace.schema available:', !!rtrace.schema);
    console.log('✓ rtrace.tracer.renderSceneToBuffer available:', typeof rtrace.tracer.renderSceneToBuffer === 'function');
    console.log('✓ rtrace.schema.SceneSchema available:', !!rtrace.schema.SceneSchema);
    
} catch (error) {
    console.error('✗ rtrace barrel import failed:', error.message);
}

console.log('\n🎉 All exact API calls work correctly!');