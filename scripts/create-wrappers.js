#!/usr/bin/env node

// Create wrapper files for the build outputs

const fs = require('fs');
const path = require('path');

function ensureDir(dirPath) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}

// Ensure dist directory exists
ensureDir('dist');
ensureDir('dist/schema');

// Find the .node file in dist/ and create index.js wrapper
const distFiles = fs.readdirSync('dist');
const nodeFile = distFiles.find(f => f.endsWith('.node'));

if (nodeFile) {
  // Create a JS wrapper that exports the native module
  const jsWrapper = `module.exports = require('./${nodeFile}');`;
  fs.writeFileSync('dist/index.js', jsWrapper);
  console.log('Created dist/index.js wrapper');
} else {
  console.error('No .node file found in dist/ directory');
  process.exit(1);
}

// Create schema entry point
const schemaEntryPoint = `module.exports = require('./schema');`;
fs.writeFileSync('dist/schema/index.js', schemaEntryPoint);
console.log('Created dist/schema/index.js entry point');

console.log('Wrapper files created successfully!');