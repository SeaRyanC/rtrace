#!/usr/bin/env node

// Copy build outputs to dist/ folder structure

const fs = require('fs');
const path = require('path');

function ensureDir(dirPath) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}

function copyFile(src, dest) {
  ensureDir(path.dirname(dest));
  fs.copyFileSync(src, dest);
  console.log(`Copied ${src} -> ${dest}`);
}

// Ensure dist directory exists
ensureDir('dist');
ensureDir('dist/schema');

// Copy napi output to dist/index.js and dist/index.d.ts
if (fs.existsSync('tracer/index.d.ts')) {
  copyFile('tracer/index.d.ts', 'dist/index.d.ts');
}

// Find the .node file and copy it as index.js (this is how napi works)
const tracerFiles = fs.readdirSync('tracer');
const nodeFile = tracerFiles.find(f => f.endsWith('.node'));
if (nodeFile) {
  copyFile(path.join('tracer', nodeFile), path.join('dist', nodeFile));
  
  // Create a JS wrapper that exports the native module
  const jsWrapper = `module.exports = require('./${nodeFile}');`;
  fs.writeFileSync('dist/index.js', jsWrapper);
  console.log('Created dist/index.js wrapper');
}

// The schema files are already built to dist/schema/ by tsc
// Create a simple CommonJS entry point
const schemaEntryPoint = `module.exports = require('./schema');`;

fs.writeFileSync('dist/schema/index.js', schemaEntryPoint);
console.log('Created dist/schema/index.js entry point');

console.log('Build outputs copied successfully!');