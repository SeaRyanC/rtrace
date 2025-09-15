// Example usage of rtrace Node.js bindings
const rtrace = require('../rtrace.node');

console.log('=== rtrace Node.js Bindings Demo ===\n');

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

try {
    const result = rtrace.renderSceneToBuffer(simpleScene, 50);
    console.log('   Image dimensions:', result.width + 'x' + result.height);
    console.log('   Stride (bytes per row):', result.stride);
    console.log('   Data length (RGBA bytes):', result.data.length);
    console.log('   Expected data length:', result.width * result.height * 4);
    console.log();
} catch (error) {
    console.error('   Error:', error.message);
}

// Example 2: Available render functions
console.log('2. Available render functions:');
console.log('   - renderScene(): Render to file with default multi-threading');
console.log('   - renderSceneThreaded(): Render to file with specific thread count');
console.log('   - renderSceneFromFile(): Load scene from JSON file and render');
console.log('   - renderSceneFromFileThreaded(): Load from file with specific thread count');
console.log('   - renderSceneToBuffer(): Render to memory buffer for programmatic use');
console.log();

console.log('✅ Demo completed successfully!');