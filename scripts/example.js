// Example usage of rtrace Node.js bindings
const { renderSceneToBuffer } = require('../dist/index.js');
const { getMinimalTestSceneJson } = require('./test-scenes.js');

console.log('=== rtrace Node.js Bindings Demo ===\n');

// Example 1: Render scene to buffer and inspect metadata
console.log('1. Render a simple scene to buffer:');
const simpleScene = getMinimalTestSceneJson();

try {
    const result = renderSceneToBuffer(simpleScene, 50);
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