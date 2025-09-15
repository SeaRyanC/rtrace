// Barrel export for rtrace package
// Provides access to both tracer and schema modules

module.exports = {
  tracer: require('./tracer/rtrace.node'),
  schema: require('./schema/schema.js')
};