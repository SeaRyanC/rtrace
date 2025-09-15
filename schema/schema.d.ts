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
import { z } from "zod";
declare const MaterialSchema: z.ZodSchema<{
    color: string;
    ambient: number;
    diffuse: number;
    specular: number;
    shininess: number;
    reflectivity?: number;
    texture?: {
        type: "grid";
        line_color: string;
        line_width: number;
        cell_size: number;
    } | {
        type: "checkerboard";
        material_b: any;
    };
}>;
declare const ObjectSchema: z.ZodUnion<readonly [z.ZodObject<{
    kind: z.ZodLiteral<"sphere">;
    center: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    radius: z.ZodNumber;
    material: z.ZodType<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown, z.core.$ZodTypeInternals<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown>>;
    transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>, z.ZodObject<{
    kind: z.ZodLiteral<"plane">;
    point: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    normal: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    material: z.ZodType<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown, z.core.$ZodTypeInternals<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown>>;
    transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>, z.ZodObject<{
    kind: z.ZodLiteral<"cube">;
    center: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    size: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    material: z.ZodType<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown, z.core.$ZodTypeInternals<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown>>;
    transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>, z.ZodObject<{
    kind: z.ZodLiteral<"mesh">;
    filename: z.ZodString;
    material: z.ZodType<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown, z.core.$ZodTypeInternals<{
        color: string;
        ambient: number;
        diffuse: number;
        specular: number;
        shininess: number;
        reflectivity?: number;
        texture?: {
            type: "grid";
            line_color: string;
            line_width: number;
            cell_size: number;
        } | {
            type: "checkerboard";
            material_b: any;
        };
    }, unknown>>;
    transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
}, z.core.$strip>]>;
declare const CameraSchema: z.ZodUnion<readonly [z.ZodObject<{
    kind: z.ZodLiteral<"ortho">;
    position: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    target: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    up: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    width: z.ZodNumber;
    height: z.ZodNumber;
}, z.core.$strip>, z.ZodObject<{
    kind: z.ZodLiteral<"perspective">;
    position: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    target: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    up: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    width: z.ZodNumber;
    height: z.ZodNumber;
    fov: z.ZodNumber;
}, z.core.$strip>]>;
declare const LightSchema: z.ZodObject<{
    position: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
    color: z.ZodString;
    intensity: z.ZodNumber;
    diameter: z.ZodOptional<z.ZodNullable<z.ZodNumber>>;
}, z.core.$strip>;
declare const SceneSettingsSchema: z.ZodObject<{
    ambient_illumination: z.ZodObject<{
        color: z.ZodString;
        intensity: z.ZodNumber;
    }, z.core.$strip>;
    fog: z.ZodOptional<z.ZodObject<{
        color: z.ZodString;
        density: z.ZodNumber;
        start: z.ZodNumber;
        end: z.ZodNumber;
    }, z.core.$strip>>;
    background_color: z.ZodOptional<z.ZodString>;
    outline: z.ZodOptional<z.ZodObject<{
        enabled: z.ZodBoolean;
        depth_weight: z.ZodDefault<z.ZodNumber>;
        normal_weight: z.ZodDefault<z.ZodNumber>;
        threshold: z.ZodDefault<z.ZodNumber>;
        color: z.ZodDefault<z.ZodString>;
        thickness: z.ZodDefault<z.ZodNumber>;
        use_8_neighbors: z.ZodDefault<z.ZodBoolean>;
    }, z.core.$strip>>;
}, z.core.$strip>;
export declare const SceneSchema: z.ZodObject<{
    camera: z.ZodUnion<readonly [z.ZodObject<{
        kind: z.ZodLiteral<"ortho">;
        position: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        target: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        up: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        width: z.ZodNumber;
        height: z.ZodNumber;
    }, z.core.$strip>, z.ZodObject<{
        kind: z.ZodLiteral<"perspective">;
        position: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        target: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        up: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        width: z.ZodNumber;
        height: z.ZodNumber;
        fov: z.ZodNumber;
    }, z.core.$strip>]>;
    objects: z.ZodArray<z.ZodUnion<readonly [z.ZodObject<{
        kind: z.ZodLiteral<"sphere">;
        center: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        radius: z.ZodNumber;
        material: z.ZodType<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown, z.core.$ZodTypeInternals<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown>>;
        transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>, z.ZodObject<{
        kind: z.ZodLiteral<"plane">;
        point: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        normal: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        material: z.ZodType<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown, z.core.$ZodTypeInternals<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown>>;
        transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>, z.ZodObject<{
        kind: z.ZodLiteral<"cube">;
        center: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        size: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        material: z.ZodType<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown, z.core.$ZodTypeInternals<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown>>;
        transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>, z.ZodObject<{
        kind: z.ZodLiteral<"mesh">;
        filename: z.ZodString;
        material: z.ZodType<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown, z.core.$ZodTypeInternals<{
            color: string;
            ambient: number;
            diffuse: number;
            specular: number;
            shininess: number;
            reflectivity?: number;
            texture?: {
                type: "grid";
                line_color: string;
                line_width: number;
                cell_size: number;
            } | {
                type: "checkerboard";
                material_b: any;
            };
        }, unknown>>;
        transform: z.ZodOptional<z.ZodArray<z.ZodString>>;
    }, z.core.$strip>]>>;
    lights: z.ZodArray<z.ZodObject<{
        position: z.ZodTuple<[z.ZodNumber, z.ZodNumber, z.ZodNumber], null>;
        color: z.ZodString;
        intensity: z.ZodNumber;
        diameter: z.ZodOptional<z.ZodNullable<z.ZodNumber>>;
    }, z.core.$strip>>;
    scene_settings: z.ZodObject<{
        ambient_illumination: z.ZodObject<{
            color: z.ZodString;
            intensity: z.ZodNumber;
        }, z.core.$strip>;
        fog: z.ZodOptional<z.ZodObject<{
            color: z.ZodString;
            density: z.ZodNumber;
            start: z.ZodNumber;
            end: z.ZodNumber;
        }, z.core.$strip>>;
        background_color: z.ZodOptional<z.ZodString>;
        outline: z.ZodOptional<z.ZodObject<{
            enabled: z.ZodBoolean;
            depth_weight: z.ZodDefault<z.ZodNumber>;
            normal_weight: z.ZodDefault<z.ZodNumber>;
            threshold: z.ZodDefault<z.ZodNumber>;
            color: z.ZodDefault<z.ZodString>;
            thickness: z.ZodDefault<z.ZodNumber>;
            use_8_neighbors: z.ZodDefault<z.ZodBoolean>;
        }, z.core.$strip>>;
    }, z.core.$strip>;
}, z.core.$strip>;
export type Scene = z.infer<typeof SceneSchema>;
export type CameraType = z.infer<typeof CameraSchema>;
export type ObjectType = z.infer<typeof ObjectSchema>;
export type LightType = z.infer<typeof LightSchema>;
export type MaterialType = z.infer<typeof MaterialSchema>;
export type SceneSettingsType = z.infer<typeof SceneSettingsSchema>;
export {};
