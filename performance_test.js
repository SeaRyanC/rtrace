const rtrace = require('./rtrace.node');
const fs = require('fs');

async function testPerformanceComparison() {
    console.log('=== Multi-threaded Performance Comparison ===\n');
    
    const sceneFile = './examples/radial_spheres.json';
    const width = 800;
    const height = 600;

    if (!fs.existsSync(sceneFile)) {
        console.error(`Scene file ${sceneFile} not found`);
        process.exit(1);
    }

    console.log(`Testing with: ${sceneFile} (${width}x${height})`);
    console.log('This test will compare single-threaded vs multi-threaded rendering performance.\n');

    try {
        // Test 1: Single-threaded rendering
        console.log('1. Single-threaded rendering:');
        console.time('Single-threaded');
        const result1 = rtrace.renderSceneFromFileThreaded(sceneFile, './perf_test_single.png', width, height, 1);
        console.timeEnd('Single-threaded');
        console.log('   Result:', result1);
        console.log();

        // Test 2: Multi-threaded rendering (default - all cores)
        console.log('2. Multi-threaded rendering (all cores):');
        console.time('Multi-threaded');
        const result2 = rtrace.renderSceneFromFile(sceneFile, './perf_test_multi.png', width, height);
        console.timeEnd('Multi-threaded');
        console.log('   Result:', result2);
        console.log();

        // Test 3: 2-thread rendering
        console.log('3. 2-thread rendering:');
        console.time('2-threads');
        const result3 = rtrace.renderSceneFromFileThreaded(sceneFile, './perf_test_2thread.png', width, height, 2);
        console.timeEnd('2-threads');
        console.log('   Result:', result3);
        console.log();

        // Test 4: 4-thread rendering
        console.log('4. 4-thread rendering:');
        console.time('4-threads');
        const result4 = rtrace.renderSceneFromFileThreaded(sceneFile, './perf_test_4thread.png', width, height, 4);
        console.timeEnd('4-threads');
        console.log('   Result:', result4);
        console.log();

        console.log('✅ Performance comparison completed!');
        console.log('\nGenerated images for verification:');
        const files = [
            './perf_test_single.png', 
            './perf_test_multi.png', 
            './perf_test_2thread.png',
            './perf_test_4thread.png'
        ];
        
        files.forEach(file => {
            if (fs.existsSync(file)) {
                const stats = fs.statSync(file);
                console.log(`   ${file} (${stats.size} bytes)`);
            }
        });
        
        console.log('\n📊 The timing results above show the performance improvement from multi-threading.');

    } catch (error) {
        console.error('❌ Error:', error);
        process.exit(1);
    }
}

testPerformanceComparison();