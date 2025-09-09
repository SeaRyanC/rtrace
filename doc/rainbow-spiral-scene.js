// Example: Rainbow Spiral Scene using rtrace Node.js bindings
// Creates arcs of cubes in rainbow colors, each cube rotated to be tangent to its arc
// and arranged in a spiral pattern for a stunning 3D visualization

const rtrace = require('../rtrace.node');
const path = require('path');
const fs = require('fs');

console.log('=== Rainbow Spiral Scene Demo ===\n');

// Rainbow colors (ROYGBIV)
const rainbowColors = [
    { name: 'Red',    color: '#FF0000' },
    { name: 'Orange', color: '#FF8000' },
    { name: 'Yellow', color: '#FFFF00' },
    { name: 'Green',  color: '#00FF00' },
    { name: 'Blue',   color: '#0080FF' },
    { name: 'Indigo', color: '#4000FF' },
    { name: 'Violet', color: '#8000FF' }
];

// Function to create a cube with given position, rotation, and color
function createCube(center, rotationDegrees, color, size = [0.3, 0.3, 0.3]) {
    return {
        kind: "cube",
        center: [0, 0, 0], // Will be transformed
        size: size,
        material: {
            color: color,
            ambient: 0.15,
            diffuse: 0.8,
            specular: 0.6,
            shininess: 64,
            reflectivity: 0.2
        },
        transform: [
            `rotate(${rotationDegrees[0]}, ${rotationDegrees[1]}, ${rotationDegrees[2]})`,
            `translate(${center[0]}, ${center[1]}, ${center[2]})`
        ]
    };
}

// Calculate position along an arc with spiral effect
function calculateArcPosition(baseRadius, arcAngle, spiralHeight, spiralRadius) {
    const x = Math.cos(arcAngle) * (baseRadius + spiralRadius);
    const y = Math.sin(arcAngle) * (baseRadius + spiralRadius);
    const z = spiralHeight;
    return [x, y, z];
}

// Calculate tangent rotation for cube to align with arc direction
function calculateTangentRotation(arcAngle, tiltAngle = 0) {
    // Rotate around Z-axis to be tangent to the arc
    const zRotation = (arcAngle * 180 / Math.PI) + 90; // +90 to be tangent
    return [tiltAngle, 0, zRotation];
}

// Create the scene programmatically
const scene = {
    camera: {
        kind: "perspective",
        position: [12, 12, 15],
        target: [0, 0, 3],
        up: [0, 0, 1],
        width: 12,
        height: 9,
        fov: 50
    },
    objects: [],
    lights: [
        {
            position: [15, 15, 20],
            color: "#FFFFFF",
            intensity: 1.5,
            diameter: 3.0  // Area light for soft shadows
        },
        {
            position: [-10, 8, 12],
            color: "#FFEECC",
            intensity: 0.8
        },
        {
            position: [8, -10, 8],
            color: "#CCDDFF",
            intensity: 0.6
        }
    ],
    scene_settings: {
        ambient_illumination: {
            color: "#FFFFFF",
            intensity: 0.2
        },
        background_color: "#1a1a2e",
        fog: {
            color: "#2d2d4f",
            density: 0.02,
            start: 15.0,
            end: 30.0
        }
    }
};

console.log('Creating rainbow spiral arcs...');

// Parameters for the spiral arcs
const numCubesPerArc = 16;           // Number of cubes in each arc
const baseRadius = 4.0;              // Base radius of the arcs
const arcSpanAngle = Math.PI * 1.5;  // Each arc spans 270 degrees
const spiralHeight = 6.0;            // Maximum height variation
const spiralRadius = 2.0;            // Maximum radius variation

let totalCubes = 0;

rainbowColors.forEach((colorInfo, colorIndex) => {
    console.log(`Creating ${colorInfo.name} arc...`);
    
    // Offset each color's arc to create a staggered spiral effect
    const arcStartAngle = (colorIndex / rainbowColors.length) * Math.PI * 2;
    
    for (let i = 0; i < numCubesPerArc; i++) {
        // Progress along this arc (0 to 1)
        const arcProgress = i / (numCubesPerArc - 1);
        
        // Current angle along the arc
        const currentAngle = arcStartAngle + (arcProgress * arcSpanAngle);
        
        // Spiral effects
        const spiralPhase = (colorIndex * 0.5) + (arcProgress * 4); // Phase offset per color
        const heightVariation = Math.sin(spiralPhase) * spiralHeight;
        const radiusVariation = Math.cos(spiralPhase * 1.3) * spiralRadius;
        
        // Calculate position
        const position = calculateArcPosition(
            baseRadius,
            currentAngle,
            3 + heightVariation,  // Base height + variation
            radiusVariation
        );
        
        // Calculate rotation to be tangent to arc, with spiral tilt
        const tiltAngle = Math.sin(spiralPhase) * 30; // Up to 30 degrees tilt
        const rotation = calculateTangentRotation(currentAngle, tiltAngle);
        
        // Vary cube sizes slightly for more visual interest
        const sizeVariation = 1.0 + Math.sin(spiralPhase * 2) * 0.3;
        const cubeSize = [0.3 * sizeVariation, 0.3 * sizeVariation, 0.3 * sizeVariation];
        
        // Create and add the cube
        const cube = createCube(position, rotation, colorInfo.color, cubeSize);
        scene.objects.push(cube);
        totalCubes++;
    }
});

console.log(`Created ${totalCubes} cubes across ${rainbowColors.length} rainbow arcs`);

// Add a central decorative element
const centralSphere = {
    kind: "sphere",
    center: [0, 0, 3],
    radius: 0.8,
    material: {
        color: "#FFFFFF",
        ambient: 0.1,
        diffuse: 0.3,
        specular: 0.9,
        shininess: 128,
        reflectivity: 0.8
    }
};
scene.objects.push(centralSphere);

// Add a ground plane for context and reflections
const groundPlane = {
    kind: "plane",
    point: [0, 0, -1],
    normal: [0, 0, 1],
    material: {
        color: "#333366",
        ambient: 0.15,
        diffuse: 0.6,
        specular: 0.4,
        shininess: 32,
        reflectivity: 0.4,
        texture: {
            type: "grid",
            line_color: "#444477",
            line_width: 0.1,
            cell_size: 2.0
        }
    }
};
scene.objects.push(groundPlane);

console.log('\nScene created with:');
console.log(`- ${scene.objects.length} objects (${totalCubes} cubes + 1 sphere + 1 plane)`);
console.log(`- ${scene.lights.length} lights (1 area light + 2 point lights)`);
console.log(`- ${rainbowColors.length} rainbow colors in spiral arcs`);
console.log('- Perspective camera for 3D depth');
console.log('- Atmospheric fog for depth perception');
console.log('- Reflective ground plane with grid texture\n');

// Convert scene to JSON
const sceneJson = JSON.stringify(scene, null, 2);
console.log('Scene JSON created, rendering image...\n');

// Define output paths
const outputImagePath = path.join(__dirname, 'images', 'rainbow-spiral-scene.png');
const outputScenePath = path.join(__dirname, 'scenes', 'rainbow-spiral-scene.json');

// Ensure directories exist
const imagesDir = path.dirname(outputImagePath);
const scenesDir = path.dirname(outputScenePath);
if (!fs.existsSync(imagesDir)) {
    fs.mkdirSync(imagesDir, { recursive: true });
}
if (!fs.existsSync(scenesDir)) {
    fs.mkdirSync(scenesDir, { recursive: true });
}

try {
    // Render the scene using the Node.js binding
    console.log('Rendering rainbow spiral scene...');
    const result = rtrace.renderScene(sceneJson, outputImagePath, 1200, 900);
    console.log('✅ Render successful!');
    console.log(result);
    console.log(`\nImage saved to: ${outputImagePath}`);
    
    // Save the scene JSON for reference
    fs.writeFileSync(outputScenePath, sceneJson);
    console.log(`Scene JSON saved to: ${outputScenePath}`);
    
    console.log('\n=== Rainbow Spiral Scene Complete ===');
    console.log('This scene demonstrates:');
    console.log('• Programmatic generation of complex geometric patterns');
    console.log('• Mathematical arc and spiral calculations');
    console.log('• Object transformations (rotation and translation)');
    console.log('• Rainbow color progression (ROYGBIV)');
    console.log('• Tangent rotation alignment with arc curves');
    console.log('• Spiral height and radius variations');
    console.log('• Advanced lighting with area lights and soft shadows');
    console.log('• Atmospheric fog effects for depth');
    console.log('• Reflective materials and surfaces');
    console.log('• Perspective camera with optimal framing\n');
    
} catch (error) {
    console.error('❌ Render failed:', error);
    process.exit(1);
}