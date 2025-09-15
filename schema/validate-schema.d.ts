/**
 * Schema validation and JSON schema generation script
 *
 * This script:
 * 1. Generates a JSON schema from the Zod schema
 * 2. Validates all scene files in the repository using the Zod schema
 * 3. Reports validation results and any errors found
 */
declare function generateJsonSchema(): void;
declare function validateJsonFile(filePath: string): {
    isValid: boolean;
    errors?: any[];
};
declare function validateAllScenes(): boolean;
export { generateJsonSchema, validateAllScenes, validateJsonFile };
