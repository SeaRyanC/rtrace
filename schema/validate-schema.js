"use strict";
/**
 * Schema validation and JSON schema generation script
 *
 * This script:
 * 1. Generates a JSON schema from the Zod schema
 * 2. Validates all scene files in the repository using the Zod schema
 * 3. Reports validation results and any errors found
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateJsonSchema = generateJsonSchema;
exports.validateAllScenes = validateAllScenes;
exports.validateJsonFile = validateJsonFile;
const schema_1 = require("./schema");
const fs_1 = require("fs");
const path_1 = require("path");
const z = __importStar(require("zod"));
// Generate JSON schema from Zod schema
function generateJsonSchema() {
    console.log('Generating JSON schema from Zod schema...');
    const jsonSchema = z.toJSONSchema(schema_1.SceneSchema);
    // Write to schema.json file
    (0, fs_1.writeFileSync)('schema.json', JSON.stringify(jsonSchema, null, 2));
    console.log('✅ JSON schema generated and saved to schema.json');
}
// Find all JSON files in specified directories
function findJsonFiles(...directories) {
    const jsonFiles = [];
    for (const dir of directories) {
        try {
            const files = (0, fs_1.readdirSync)(dir);
            for (const file of files) {
                const fullPath = (0, path_1.join)(dir, file);
                const stat = (0, fs_1.statSync)(fullPath);
                if (stat.isFile() && (0, path_1.extname)(file) === '.json') {
                    jsonFiles.push(fullPath);
                }
            }
        }
        catch (error) {
            console.log(`⚠️  Warning: Could not read directory ${dir}: ${error?.message || error}`);
        }
    }
    return jsonFiles;
}
// Validate a single JSON file
function validateJsonFile(filePath) {
    try {
        const fileContent = (0, fs_1.readFileSync)(filePath, 'utf-8');
        const jsonData = JSON.parse(fileContent);
        const result = schema_1.SceneSchema.safeParse(jsonData);
        if (result.success) {
            return { isValid: true };
        }
        else {
            return {
                isValid: false,
                errors: result.error.issues.map(issue => ({
                    path: issue.path.join('.'),
                    message: issue.message,
                    code: issue.code,
                    received: issue.code === 'invalid_type' ? issue.received : undefined
                }))
            };
        }
    }
    catch (error) {
        return {
            isValid: false,
            errors: [{
                    path: 'parse',
                    message: `Failed to parse JSON: ${error?.message || error}`,
                    code: 'invalid_json'
                }]
        };
    }
}
// Main validation function
function validateAllScenes() {
    console.log('Validating all scene files in the repository...\n');
    // Find all JSON files in examples and doc/scenes directories
    const jsonFiles = findJsonFiles('examples', 'doc/scenes');
    if (jsonFiles.length === 0) {
        console.log('❌ No JSON files found to validate');
        return false;
    }
    console.log(`Found ${jsonFiles.length} JSON files to validate:\n`);
    let validCount = 0;
    let invalidCount = 0;
    const validationResults = [];
    for (const file of jsonFiles) {
        const result = validateJsonFile(file);
        validationResults.push({ file, ...result });
        if (result.isValid) {
            console.log(`✅ ${file}`);
            validCount++;
        }
        else {
            console.log(`❌ ${file}`);
            if (result.errors) {
                for (const error of result.errors) {
                    if (error.path) {
                        console.log(`   • ${error.path}: ${error.message}`);
                    }
                    else {
                        console.log(`   • ${error.message}`);
                    }
                    if (error.received) {
                        console.log(`     Received: ${error.received}`);
                    }
                }
            }
            invalidCount++;
        }
    }
    console.log(`\n📊 Validation Summary:`);
    console.log(`   Valid files: ${validCount}`);
    console.log(`   Invalid files: ${invalidCount}`);
    console.log(`   Total files: ${jsonFiles.length}`);
    if (invalidCount > 0) {
        console.log(`\n❌ Schema validation failed for ${invalidCount} files`);
        return false;
    }
    else {
        console.log(`\n✅ All files passed schema validation!`);
        return true;
    }
}
// Main execution
function main() {
    const args = process.argv.slice(2);
    if (args.includes('--generate-schema') || args.includes('-g')) {
        generateJsonSchema();
    }
    if (args.includes('--validate') || args.includes('-v') || args.length === 0) {
        const success = validateAllScenes();
        if (!success) {
            process.exit(1);
        }
    }
    if (args.includes('--help') || args.includes('-h')) {
        console.log(`
Usage: node validate-schema.js [options]

Options:
  -g, --generate-schema    Generate JSON schema from Zod schema
  -v, --validate          Validate all scene files (default)
  -h, --help              Show this help message

Examples:
  node validate-schema.js                    # Validate all scene files
  node validate-schema.js --generate-schema  # Generate JSON schema
  node validate-schema.js -g -v              # Generate schema and validate
`);
    }
}
if (require.main === module) {
    main();
}
