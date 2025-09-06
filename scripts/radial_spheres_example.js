// Example: Radial Array of Colored Spheres using rtrace Node.js bindings
const rtrace = require('../rtrace.node');
const path = require('path');

console.log('=== Radial Spheres Scene Demo ===\n');

// Function to create a sphere with given position, color, and radius
function createSphere(center, color, radius = 0.8) {
    return {
        kind: "sphere",
        center: center,
        radius: radius,
        material: {
            color: color,
            ambient: 0.1,
            diffuse: 0.8,
            specular: 0.4,
            shininess: 32
        }
    };
}

// Function to convert HSV to hex color
function hsvToHex(h, s, v) {
    const c = v * s;
    const x = c * (1 - Math.abs((h / 60) % 2 - 1));
    const m = v - c;
    
    let r, g, b;
    if (h >= 0 && h < 60) {
        r = c; g = x; b = 0;
    } else if (h >= 60 && h < 120) {
        r = x; g = c; b = 0;
    } else if (h >= 120 && h < 180) {
        r = 0; g = c; b = x;
    } else if (h >= 180 && h < 240) {
        r = 0; g = x; b = c;
    } else if (h >= 240 && h < 300) {
        r = x; g = 0; b = c;
    } else {
        r = c; g = 0; b = x;
    }
    
    r = Math.round((r + m) * 255);
    g = Math.round((g + m) * 255);
    b = Math.round((b + m) * 255);
    
    return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
}

// Create the scene programmatically
const scene = {
    camera: {
        kind: "ortho",
        position: [0, 2, 12],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 12,
        height: 12
    },
    objects: [],
    lights: [
        {
            position: [5, 8, 10],
            color: "#FFFFFF",
            intensity: 1.2
        },
        {
            position: [-5, 8, 10],
            color: "#FFCCAA", 
            intensity: 0.8
        }
    ],
    scene_settings: {
        ambient_illumination: {
            color: "#FFFFFF",
            intensity: 0.2
        },
        background_color: "#001133"
    }
};

// Create radial array of spheres
const numSpheres = 8;
const radius = 4.0;
const sphereRadius = 0.9;

console.log(`Creating ${numSpheres} spheres in a radial pattern...`);

for (let i = 0; i < numSpheres; i++) {
    const angle = (i / numSpheres) * 2 * Math.PI;
    const x = Math.cos(angle) * radius;
    const z = Math.sin(angle) * radius;
    const y = Math.sin(angle * 2) * 1.5; // Vary height for more interest
    
    // Create varied colors using HSV
    const hue = (i / numSpheres) * 360;
    const saturation = 0.8 + (i % 2) * 0.2; // Alternate saturation
    const value = 0.9;
    const color = hsvToHex(hue, saturation, value);
    
    // Vary sphere sizes slightly
    const currentRadius = sphereRadius + (Math.sin(angle * 3) * 0.2);
    
    scene.objects.push(createSphere([x, y, z], color, currentRadius));
    console.log(`Sphere ${i + 1}: position=[${x.toFixed(2)}, ${y.toFixed(2)}, ${z.toFixed(2)}], color=${color}, radius=${currentRadius.toFixed(2)}`);
}

// Add a central larger sphere
scene.objects.push(createSphere([0, 0, 0], "#FFFFFF", 1.2));
console.log(`Central sphere: position=[0, 0, 0], color=#FFFFFF, radius=1.2`);

// Add a ground plane for context
scene.objects.push({
    kind: "plane",
    point: [0, -3, 0],
    normal: [0, 1, 0],
    material: {
        color: "#334455",
        ambient: 0.2,
        diffuse: 0.6,
        specular: 0.1,
        shininess: 8,
        texture: {
            type: "grid",
            line_color: "#666666",
            line_width: 0.1,
            cell_size: 1.5
        }
    }
});

console.log('\nScene created with:');
console.log(`- ${scene.objects.length} objects (${scene.objects.length - 1} spheres + 1 plane)`);
console.log(`- ${scene.lights.length} lights`);
console.log('- Orthographic camera with elevated viewpoint');
console.log('- Grid texture on ground plane\n');

// Convert scene to JSON
const sceneJson = JSON.stringify(scene, null, 2);
console.log('Scene JSON created, rendering image...\n');

// Define output path in examples directory
const outputPath = path.join(__dirname, 'examples', 'radial_spheres_800x600.png');

try {
    // Render the scene using the Node.js binding
    const result = rtrace.renderScene(sceneJson, outputPath, 800, 600);
    console.log('✅ Render successful!');
    console.log(result);
    console.log(`\nOutput saved to: ${outputPath}`);
    
    // Also save the scene JSON for reference
    const jsonPath = path.join(__dirname, 'examples', 'radial_spheres.json');
    require('fs').writeFileSync(jsonPath, sceneJson);
    console.log(`Scene JSON saved to: ${jsonPath}`);
    
} catch (error) {
    console.error('❌ Render failed:', error);
    process.exit(1);
}