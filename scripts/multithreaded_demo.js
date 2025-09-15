// Multi-threaded Rendering Demo for rtrace
const rtrace = require('../tracer/rtrace.node');
const fs = require('fs');

console.log('🚀 rtrace Multi-threaded Rendering Demo');
console.log('========================================\n');

// Test with an existing scene
const sceneFile = '../examples/simple_cube.json';

if (!fs.existsSync(sceneFile)) {
    console.error(`❌ Scene file ${sceneFile} not found`);
    console.log('Available scenes:');
    const examples = fs.readdirSync('../examples').filter(f => f.endsWith('.json'));
    examples.forEach(f => console.log(`   ../examples/${f}`));
    process.exit(1);
}

async function demonstrateMultiThreading() {
    console.log('Demonstrating multi-threaded rendering capabilities...\n');
    
    // Test different thread configurations
    const tests = [
        { name: 'Single Thread', threads: 1 },
        { name: 'Dual Thread', threads: 2 }, 
        { name: 'All Cores', threads: null },
    ];
    
    for (const test of tests) {
        console.log(`📊 ${test.name} Rendering:`);
        console.time(test.name);
        
        try {
            let result;
            if (test.threads === null) {
                // Use default multi-threading (all cores)
                result = rtrace.renderSceneFromFile(sceneFile, `./demo_${test.name.toLowerCase().replace(' ', '_')}.png`, 500);
            } else {
                // Use specific thread count
                result = rtrace.renderSceneFromFileThreaded(sceneFile, `./demo_${test.name.toLowerCase().replace(' ', '_')}.png`, 500, test.threads);
            }
            console.timeEnd(test.name);
            console.log(`   ✓ ${result}\n`);
        } catch (error) {
            console.error(`   ❌ Error: ${error.message}\n`);
        }
    }
    
    console.log('🎯 Key Benefits of Multi-threaded Rendering:');
    console.log('   • Faster rendering times through parallel processing');
    console.log('   • Better utilization of modern multi-core processors');
    console.log('   • Configurable thread count for optimal performance');
    console.log('   • Identical output quality regardless of thread count');
    console.log('   • Seamless integration with existing API');
    
    console.log('\n📝 API Usage:');
    console.log('   rtrace.renderScene(json, output, size)              // Auto multi-threading');
    console.log('   rtrace.renderSceneThreaded(json, output, size, threads) // Custom threads');
    console.log('   rtrace.renderSceneFromFile(file, output, size)      // File-based rendering');
    
    // Clean up demo files
    setTimeout(() => {
        ['./demo_single_thread.png', './demo_dual_thread.png', './demo_all_cores.png'].forEach(file => {
            if (fs.existsSync(file)) {
                fs.unlinkSync(file);
            }
        });
        console.log('\n🧹 Demo files cleaned up.');
    }, 1000);
}

demonstrateMultiThreading().catch(console.error);