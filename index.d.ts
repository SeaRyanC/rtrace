/* tslint:disable */
/* eslint-disable */

// Barrel export types for rtrace package

import * as tracer from './tracer/index';
import * as schema from './schema/schema';

export { tracer, schema };

declare const rtrace: {
  tracer: typeof tracer;
  schema: typeof schema;
};

export default rtrace;