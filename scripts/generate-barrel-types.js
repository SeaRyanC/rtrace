#!/usr/bin/env node

// Generate barrel export index.d.ts file directly in the right location

const fs = require('fs');
const path = require('path');

const barrelContent = `/* tslint:disable */
/* eslint-disable */

// Barrel export types for rtrace package

import * as tracer from './tracer/index';
import * as schema from './schema/schema';

export { tracer, schema };

declare const rtrace: {
  tracer: typeof tracer;
  schema: typeof schema;
};

export default rtrace;`;

// Write the barrel export to the root directory
fs.writeFileSync(path.join(__dirname, '..', 'index.d.ts'), barrelContent);

console.log('Generated index.d.ts barrel export');