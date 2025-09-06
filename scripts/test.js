#!/usr/bin/env node

// Test script to verify NAPI bindings work correctly
const { helloWorld, greetWithName } = require('../rtrace.node');

console.log('Testing NAPI bindings...');

// Test basic hello world function
try {
    const result = helloWorld();
    console.log('✓ helloWorld():', result);
    
    if (result !== 'hello world') {
        console.error('✗ Expected "hello world", got:', result);
        process.exit(1);
    }
} catch (error) {
    console.error('✗ helloWorld() failed:', error.message);
    process.exit(1);
}

// Test greet with name function
try {
    const result = greetWithName('Alice');
    console.log('✓ greetWithName("Alice"):', result);
    
    if (result !== 'hello world, Alice') {
        console.error('✗ Expected "hello world, Alice", got:', result);
        process.exit(1);
    }
} catch (error) {
    console.error('✗ greetWithName() failed:', error.message);
    process.exit(1);
}

console.log('🎉 All tests passed! NAPI bindings are working correctly.');