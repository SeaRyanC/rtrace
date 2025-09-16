#!/usr/bin/env node

// Test script to verify NAPI bindings work correctly with the new API structure
const { renderSceneToBuffer } = require('../dist/index.js');
const { getMinimalTestSceneJson } = require('./test-scenes.js');

console.log('Testing NAPI bindings with new API structure...');

// Test render scene to buffer function with a minimal scene
try {
    const minimalScene = getMinimalTestSceneJson();
    
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

console.log('🎉 All tests passed! NAPI bindings are working correctly with new API structure.');