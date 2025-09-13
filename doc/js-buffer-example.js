#!/usr/bin/env node

/**
 * JavaScript Example: Direct Image Buffer Manipulation with rtrace
 * 
 * This example demonstrates:
 * 1. Using the rtrace Node.js API to render directly to a buffer
 * 2. Manipulating the image buffer in JavaScript (color negation on half the image)
 * 3. Writing the processed buffer to a PNG file
 * 
 * No intermediate JSON files are used - everything is done programmatically.
 */

const fs = require('fs');
const path = require('path');
const { PNG } = require('pngjs');

// Import rtrace Node.js bindings
const rtrace = require('../rtrace.node');

console.log('=== rtrace JavaScript Buffer Manipulation Demo ===\n');

// Create an interesting scene programmatically
// This scene will have multiple colored spheres and a reflective plane
const scene = {
    camera: {
        kind: "perspective",
        position: [6, -8, 4],
        target: [0, 0, 0],
        up: [0, 0, 1],
        width: 8,
        height: 6,
        fov: 50
    },
    objects: [
        // Red sphere
        {
            kind: "sphere",
            center: [-2, 0, 1],
            radius: 1.0,
            material: {
                color: "#FF4444",
                ambient: 0.1,
                diffuse: 0.8,
                specular: 0.6,
                shininess: 64,
                reflectivity: 0.3
            }
        },
        // Green sphere
        {
            kind: "sphere", 
            center: [0, 2, 1],
            radius: 1.2,
            material: {
                color: "#44FF44",
                ambient: 0.1,
                diffuse: 0.8,
                specular: 0.4,
                shininess: 32,
                reflectivity: 0.2
            }
        },
        // Blue sphere with high reflectivity
        {
            kind: "sphere",
            center: [2, -1, 1.5],
            radius: 0.8,
            material: {
                color: "#4444FF",
                ambient: 0.1,
                diffuse: 0.5,
                specular: 0.8,
                shininess: 128,
                reflectivity: 0.7
            }
        },
        // Yellow cube
        {
            kind: "cube",
            center: [-1, -2, 0.75],
            size: [1.5, 1.5, 1.5],
            material: {
                color: "#FFFF44",
                ambient: 0.15,
                diffuse: 0.7,
                specular: 0.3,
                shininess: 16
            },
            transform: ["rotate(0, 0, 30)"]
        },
        // Reflective ground plane
        {
            kind: "plane",
            point: [0, 0, -1],
            normal: [0, 0, 1],
            material: {
                color: "#CCCCCC",
                ambient: 0.2,
                diffuse: 0.6,
                specular: 0.8,
                shininess: 100,
                reflectivity: 0.4,
                texture: {
                    type: "checkerboard",
                    material_a: {
                        color: "#FFFFFF",
                        ambient: 0.2,
                        diffuse: 0.8,
                        specular: 0.1,
                        shininess: 8,
                        reflectivity: 0.1
                    },
                    material_b: {
                        color: "#888888",
                        ambient: 0.2,
                        diffuse: 0.8,
                        specular: 0.1,
                        shininess: 8,
                        reflectivity: 0.6
                    }
                }
            }
        }
    ],
    lights: [
        // Main white light
        {
            position: [5, -5, 8],
            color: "#FFFFFF",
            intensity: 1.2,
            diameter: 1.5  // Area light for soft shadows
        },
        // Secondary warm light
        {
            position: [-3, 3, 6],
            color: "#FFCCAA",
            intensity: 0.8
        },
        // Subtle blue fill light
        {
            position: [0, -6, 3],
            color: "#AACCFF",
            intensity: 0.4
        }
    ],
    scene_settings: {
        ambient_illumination: {
            color: "#FFFFFF",
            intensity: 0.15
        },
        background_color: "#112233",
        fog: {
            color: "#334455",
            density: 0.03,
            start: 8.0,
            end: 20.0
        }
    }
};

console.log('Created scene with:');
console.log(`- ${scene.objects.length} objects (spheres, cube, and reflective checkerboard plane)`);
console.log(`- ${scene.lights.length} lights (including area light for soft shadows)`);
console.log('- Perspective camera with atmospheric fog');
console.log('- Material reflections and textures\n');

// Render scene to buffer using our new API function
console.log('Rendering scene to buffer...');
const diagonalSize = 800;
const sceneJson = JSON.stringify(scene);

let imageBuffer;
let renderTimeStart = Date.now();

try {
    imageBuffer = rtrace.renderSceneToBuffer(sceneJson, diagonalSize);
    const renderTime = Date.now() - renderTimeStart;
    console.log(`✅ Rendered to buffer in ${renderTime}ms`);
    console.log(`   Buffer size: ${imageBuffer.data.length} bytes (${imageBuffer.data.length / 4} pixels)`);
    console.log(`   Image dimensions: ${imageBuffer.width}×${imageBuffer.height} pixels`);
    console.log(`   Stride: ${imageBuffer.stride} bytes per row\n`);
} catch (error) {
    console.error('❌ Render failed:', error);
    process.exit(1);
}

// Extract dimensions from the image buffer object
const { width, height, stride, data: buffer } = imageBuffer;

// Now let's manipulate the buffer - negate colors on the left half
console.log('Applying image manipulation: negating colors on left half...');

const manipulationStart = Date.now();
const halfWidth = Math.floor(width / 2);

for (let y = 0; y < height; y++) {
    for (let x = 0; x < halfWidth; x++) {
        const pixelIndex = (y * width + x) * 4;
        
        // Negate RGB channels (leave alpha unchanged)
        buffer[pixelIndex + 0] = 255 - buffer[pixelIndex + 0]; // Red
        buffer[pixelIndex + 1] = 255 - buffer[pixelIndex + 1]; // Green  
        buffer[pixelIndex + 2] = 255 - buffer[pixelIndex + 2]; // Blue
        // Alpha remains unchanged at buffer[pixelIndex + 3]
    }
}

const manipulationTime = Date.now() - manipulationStart;
console.log(`✅ Image manipulation completed in ${manipulationTime}ms`);
console.log(`   Modified ${halfWidth * height} pixels (left half of image)\n`);

// Convert buffer to PNG and save
console.log('Converting buffer to PNG and saving...');
const saveStart = Date.now();

// Validate buffer size before creating PNG
const expectedBufferSize = width * height * 4;
if (buffer.length !== expectedBufferSize) {
    throw new Error(`Buffer size mismatch: expected ${expectedBufferSize}, got ${buffer.length}`);
}

// Create PNG with explicit configuration for RGBA
const png = new PNG({
    width: width,
    height: height,
    colorType: 6,      // RGBA (required for 4-channel data)
    bitDepth: 8,       // 8 bits per channel
    inputHasAlpha: true,
    deflateLevel: 6,   // Compression level (0-9)
    deflateStrategy: 3 // PNG-specific compression strategy
});

// Ensure PNG data buffer is properly sized
if (png.data.length !== buffer.length) {
    throw new Error(`PNG data buffer size mismatch: PNG has ${png.data.length}, buffer has ${buffer.length}`);
}

// Copy RGBA buffer data with validation
for (let i = 0; i < buffer.length; i++) {
    const value = buffer[i];
    if (value < 0 || value > 255 || !Number.isInteger(value)) {
        throw new Error(`Invalid pixel value at index ${i}: ${value} (must be 0-255 integer)`);
    }
    png.data[i] = value;
}

const outputPath = path.join(__dirname, 'images', 'js-buffer-manipulation-demo.png');
const pngBuffer = PNG.sync.write(png);
fs.writeFileSync(outputPath, pngBuffer);

const saveTime = Date.now() - saveStart;
console.log(`✅ PNG saved in ${saveTime}ms`);

const totalTime = Date.now() - renderTimeStart;
console.log(`\n🎉 Complete demo finished in ${totalTime}ms!`);
console.log(`   Output saved to: ${outputPath}`);

console.log('\n=== Demo Summary ===');
console.log('This example demonstrated:');
console.log('1. ✅ Direct JavaScript API usage (no intermediate JSON files)');
console.log('2. ✅ Rendering to buffer with renderSceneToBuffer()');
console.log('3. ✅ Direct buffer manipulation (color negation on left half)');
console.log('4. ✅ Converting buffer to PNG and saving to file');
console.log('5. ✅ Complex scene with reflections, textures, area lights, and fog');
console.log('\nThe resulting image shows:');
console.log('- RIGHT HALF: Original rendered scene');
console.log('- LEFT HALF: Color-negated version (demonstrating buffer manipulation)');