// Shared test scene definitions to avoid duplication across test and example files

/**
 * Standard minimal test scene used across multiple test files
 * Features: single red sphere, basic lighting, perspective camera
 */
function getMinimalTestScene() {
    return {
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
}

/**
 * Returns the minimal test scene as a JSON string
 */
function getMinimalTestSceneJson() {
    return JSON.stringify(getMinimalTestScene());
}

module.exports = {
    getMinimalTestScene,
    getMinimalTestSceneJson
};