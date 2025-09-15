#!/usr/bin/env node

// Comprehensive test for the new API structure
console.log('=== Comprehensive API Structure Test ===\n');

// Test the API structure directly from the repository 
console.log('Testing internal API structure...');

try {
    // Test 1: Barrel export
    const rtrace = require('../dist/index.js');
    console.log('✓ Barrel export works: require("../dist/index.js")');
    console.log('✓ rtrace.tracer available:', !!rtrace.tracer);
    console.log('✓ rtrace.schema available:', !!rtrace.schema);
    
    // Test 2: Direct tracer import
    const tracer = require('../tracer/rtrace.node');
    console.log('✓ Direct tracer import works');
    console.log('✓ tracer functions available:', [
        'renderScene',
        'renderSceneThreaded', 
        'renderSceneFromFile',
        'renderSceneFromFileThreaded',
        'renderSceneToBuffer'
    ].every(fn => typeof tracer[fn] === 'function'));
    
    // Test 3: Direct schema import
    const schema = require('../dist/schema/schema.js');
    console.log('✓ Direct schema import works');
    console.log('✓ schema.SceneSchema available:', !!schema.SceneSchema);
    
    // Test 4: Functional test with schema validation
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
    const validatedScene = schema.SceneSchema.parse(testScene);
    console.log('✓ Schema validation works');
    
    // Render with tracer  
    const result = tracer.renderSceneToBuffer(JSON.stringify(testScene), 30);
    console.log('✓ Rendering works - dimensions:', result.width + 'x' + result.height);
    
    // Test cross-compatibility: rtrace.tracer should work the same as direct tracer
    const result2 = rtrace.tracer.renderSceneToBuffer(JSON.stringify(testScene), 30);
    console.log('✓ Barrel export tracer works - dimensions:', result2.width + 'x' + result2.height);
    
    // Test cross-compatibility: rtrace.schema should work the same as direct schema
    const validatedScene2 = rtrace.schema.SceneSchema.parse(testScene);
    console.log('✓ Barrel export schema works');
    
    console.log('\n🎉 All internal API structure tests passed!');
    
} catch (error) {
    console.error('✗ Test failed:', error.message);
    process.exit(1);
}

console.log('\n=== API Structure Test Complete ===');