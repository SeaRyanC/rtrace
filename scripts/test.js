#!/usr/bin/env node

// Test script to verify NAPI bindings work correctly
const { renderSceneToBuffer } = require('../rtrace.node');

console.log('Testing NAPI bindings...');

// Test render scene to buffer function with a minimal scene
try {
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
        console.error('✗ Expected image data, got empty buffer');
        process.exit(1);
    }
    
    if (result.width <= 0 || result.height <= 0) {
        console.error('✗ Expected positive dimensions, got:', result.width + 'x' + result.height);
        process.exit(1);
    }
} catch (error) {
    console.error('✗ renderSceneToBuffer() failed:', error.message);
    process.exit(1);
}

console.log('🎉 All tests passed! NAPI bindings are working correctly.');