// Example usage of rtrace Node.js bindings
const rtrace = require('./rtrace.node');

console.log('=== rtrace Node.js Bindings Demo ===\n');

// Basic usage
console.log('1. Basic hello world:');
console.log('   helloWorld():', rtrace.helloWorld());
console.log();

// Advanced usage with parameters
console.log('2. Greeting with custom name:');
console.log('   greetWithName("Bob"):', rtrace.greetWithName("Bob"));
console.log('   greetWithName("World"):', rtrace.greetWithName("World"));
console.log();

// Test edge cases
console.log('3. Edge cases:');
console.log('   greetWithName(""):', rtrace.greetWithName(""));
console.log('   greetWithName("🚀"):', rtrace.greetWithName("🚀"));
console.log();

console.log('✅ Demo completed successfully!');