/**
 * Schema validation and JSON schema generation script
 * 
 * This script:
 * 1. Generates a JSON schema from the Zod schema
 * 2. Validates all scene files in the repository using the Zod schema
 * 3. Reports validation results and any errors found
 */

import { SceneSchema } from './schema';
import { zodToJsonSchema } from 'zod-to-json-schema';
import { readFileSync, writeFileSync, readdirSync, statSync } from 'fs';
import { join, extname } from 'path';

// Generate JSON schema from Zod schema
function generateJsonSchema() {
  console.log('Generating JSON schema from Zod schema...');
  
  const jsonSchema = zodToJsonSchema(SceneSchema, {
    name: "Scene"
  });
  
  // Write to schema.json file
  writeFileSync('schema.json', JSON.stringify(jsonSchema, null, 2));
  console.log('✅ JSON schema generated and saved to schema.json');
}

// Find all JSON files in specified directories
function findJsonFiles(...directories: string[]): string[] {
  const jsonFiles: string[] = [];
  
  for (const dir of directories) {
    try {
      const files = readdirSync(dir);
      for (const file of files) {
        const fullPath = join(dir, file);
        const stat = statSync(fullPath);
        
        if (stat.isFile() && extname(file) === '.json') {
          jsonFiles.push(fullPath);
        }
      }
    } catch (error: any) {
      console.log(`⚠️  Warning: Could not read directory ${dir}: ${error?.message || error}`);
    }
  }
  
  return jsonFiles;
}

// Validate a single JSON file
function validateJsonFile(filePath: string): { isValid: boolean; errors?: any[] } {
  try {
    const fileContent = readFileSync(filePath, 'utf-8');
    const jsonData = JSON.parse(fileContent);
    
    const result = SceneSchema.safeParse(jsonData);
    
    if (result.success) {
      return { isValid: true };
    } else {
      return { 
        isValid: false, 
        errors: result.error.issues.map(issue => ({
          path: issue.path.join('.'),
          message: issue.message,
          code: issue.code,
          received: issue.code === 'invalid_type' ? (issue as any).received : undefined
        }))
      };
    }
  } catch (error: any) {
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
  const validationResults: Array<{ file: string; isValid: boolean; errors?: any[] }> = [];
  
  for (const file of jsonFiles) {
    const result = validateJsonFile(file);
    validationResults.push({ file, ...result });
    
    if (result.isValid) {
      console.log(`✅ ${file}`);
      validCount++;
    } else {
      console.log(`❌ ${file}`);
      if (result.errors) {
        for (const error of result.errors) {
          if (error.path) {
            console.log(`   • ${error.path}: ${error.message}`);
          } else {
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
  } else {
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

export { generateJsonSchema, validateAllScenes, validateJsonFile };