"use strict";
/**
 * Zod v4 schema for rtrace scene files
 *
 * This file defines a comprehensive Zod schema for validating JSON scene files
 * used by the rtrace ray tracer. It covers all scene components including:
 * - Camera configurations (orthographic)
 * - Objects (spheres, planes, cubes, meshes) with materials and transforms
 * - Lighting (point lights and area lights)
 * - Scene settings (ambient lighting, fog, backgrounds, outlines)
 * - Materials with textures and properties
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SceneSchema = void 0;
const zod_1 = require("zod");
// Utility schemas for common vector types
const Vector3Schema = zod_1.z.tuple([zod_1.z.number(), zod_1.z.number(), zod_1.z.number()]);
const HexColorSchema = zod_1.z.string().regex(/^#[0-9A-Fa-f]{6}$/, "Must be a valid hex color (e.g., #FFFFFF)");
// Base material schema (without texture to avoid recursion)
const BaseMaterialSchema = zod_1.z.object({
    color: HexColorSchema.describe("Material base color as hex string"),
    ambient: zod_1.z.number().min(0).max(1).describe("Ambient reflection coefficient"),
    diffuse: zod_1.z.number().min(0).max(1).describe("Diffuse reflection coefficient"),
    specular: zod_1.z.number().min(0).max(1).describe("Specular reflection coefficient"),
    shininess: zod_1.z.number().min(1).describe("Phong exponent for specular highlights"),
    reflectivity: zod_1.z.number().min(0).max(1).optional().describe("Optional reflectivity coefficient for mirror-like surfaces")
});
// Material schema with texture support
const MaterialSchema = BaseMaterialSchema.extend({
    texture: zod_1.z.union([
        zod_1.z.object({
            type: zod_1.z.literal("grid"),
            line_color: HexColorSchema.describe("Grid line color as hex string"),
            line_width: zod_1.z.number().min(0).describe("Grid line width in world units"),
            cell_size: zod_1.z.number().min(0).describe("Grid cell size in world units")
        }),
        zod_1.z.object({
            type: zod_1.z.literal("checkerboard"),
            material_b: zod_1.z.lazy(() => MaterialSchema).describe("Alternate material for checkerboard pattern")
        })
    ]).optional().describe("Optional texture configuration")
});
// Transform schema for object transformations
const TransformSchema = zod_1.z.array(zod_1.z.string()).optional().describe("Optional array of transform operations like 'rotate(x, y, z)', 'translate(x, y, z)', 'scale(x, y, z)'");
// Object schemas
const SphereObjectSchema = zod_1.z.object({
    kind: zod_1.z.literal("sphere"),
    center: Vector3Schema.describe("Sphere center as [x, y, z]"),
    radius: zod_1.z.number().min(0).describe("Sphere radius"),
    material: MaterialSchema,
    transform: TransformSchema
});
const PlaneObjectSchema = zod_1.z.object({
    kind: zod_1.z.literal("plane"),
    point: Vector3Schema.describe("Point on the plane as [x, y, z]"),
    normal: Vector3Schema.describe("Plane normal vector as [x, y, z]"),
    material: MaterialSchema,
    transform: TransformSchema
});
const CubeObjectSchema = zod_1.z.object({
    kind: zod_1.z.literal("cube"),
    center: Vector3Schema.describe("Cube center as [x, y, z]"),
    size: Vector3Schema.describe("Cube dimensions as [width, height, depth]"),
    material: MaterialSchema,
    transform: TransformSchema
});
const MeshObjectSchema = zod_1.z.object({
    kind: zod_1.z.literal("mesh"),
    filename: zod_1.z.string().describe("Path to STL file (binary or ASCII format)"),
    material: MaterialSchema,
    transform: TransformSchema
});
const ObjectSchema = zod_1.z.union([
    SphereObjectSchema,
    PlaneObjectSchema,
    CubeObjectSchema,
    MeshObjectSchema
]);
// Camera schema
const OrthoCameraSchema = zod_1.z.object({
    kind: zod_1.z.literal("ortho"),
    position: Vector3Schema.describe("Camera position as [x, y, z]"),
    target: Vector3Schema.describe("Camera target point as [x, y, z]"),
    up: Vector3Schema.describe("Camera up vector as [x, y, z]"),
    width: zod_1.z.number().min(0).describe("Viewport width in world units"),
    height: zod_1.z.number().min(0).describe("Viewport height in world units")
});
const PerspectiveCameraSchema = zod_1.z.object({
    kind: zod_1.z.literal("perspective"),
    position: Vector3Schema.describe("Camera position as [x, y, z]"),
    target: Vector3Schema.describe("Camera target point as [x, y, z]"),
    up: Vector3Schema.describe("Camera up vector as [x, y, z]"),
    width: zod_1.z.number().min(0).describe("Viewport width in world units"),
    height: zod_1.z.number().min(0).describe("Viewport height in world units"),
    fov: zod_1.z.number().min(0).max(180).describe("Field of view in degrees")
});
const CameraSchema = zod_1.z.union([OrthoCameraSchema, PerspectiveCameraSchema]);
// Light schema
const LightSchema = zod_1.z.object({
    position: Vector3Schema.describe("Light position as [x, y, z]"),
    color: HexColorSchema.describe("Light color as hex string (e.g., #FFFFFF)"),
    intensity: zod_1.z.number().min(0).describe("Light intensity multiplier"),
    diameter: zod_1.z.number().min(0).nullable().optional().describe("Optional diameter for diffuse (area) light sources. If null or omitted, the light behaves as a point light with sharp shadows. If specified, creates soft shadows.")
});
// Ambient illumination schema
const AmbientIlluminationSchema = zod_1.z.object({
    color: HexColorSchema.describe("Ambient light color as hex string"),
    intensity: zod_1.z.number().min(0).describe("Ambient light intensity")
});
// Fog schema
const FogSchema = zod_1.z.object({
    color: HexColorSchema.describe("Fog color as hex string"),
    density: zod_1.z.number().min(0).describe("Fog density factor (higher values create thicker fog)"),
    start: zod_1.z.number().describe("Distance where fog begins (near distance - objects closer are unaffected)"),
    end: zod_1.z.number().describe("Distance where fog calculation reaches maximum intensity (far distance)")
});
// Outline schema
const OutlineSchema = zod_1.z.object({
    enabled: zod_1.z.boolean().describe("Enable outline detection for the scene"),
    depth_weight: zod_1.z.number().min(0).default(1.0).describe("Weight for depth differences in edge detection (default: 1.0)"),
    normal_weight: zod_1.z.number().min(0).default(1.0).describe("Weight for normal differences in edge detection (default: 1.0)"),
    threshold: zod_1.z.number().min(0).max(1).default(0.1).describe("Threshold for edge detection (default: 0.1)"),
    color: HexColorSchema.default("#000000").describe("Outline color as hex string (default: #000000 - black)"),
    thickness: zod_1.z.number().min(1.0).default(1.0).describe("Line thickness factor (1.0 = no thickening, >1.0 = thicker lines, default: 1.0)"),
    use_8_neighbors: zod_1.z.boolean().default(false).describe("Use 8-neighbor sampling instead of 4-neighbor (default: false for performance)")
});
// Scene settings schema
const SceneSettingsSchema = zod_1.z.object({
    ambient_illumination: AmbientIlluminationSchema,
    fog: FogSchema.optional().describe("Optional fog configuration"),
    background_color: HexColorSchema.optional().describe("Background color as hex string"),
    outline: OutlineSchema.optional().describe("Optional outline detection configuration")
});
// Main scene schema
exports.SceneSchema = zod_1.z.object({
    camera: CameraSchema,
    objects: zod_1.z.array(ObjectSchema).describe("Array of objects in the scene"),
    lights: zod_1.z.array(LightSchema).describe("Array of light sources (point lights and diffuse area lights)"),
    scene_settings: SceneSettingsSchema
}).describe("Ray Tracer Scene - JSON schema for ray tracer scene definition files");
