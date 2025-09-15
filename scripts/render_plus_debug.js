const rtrace = require('../rtrace.node');
const fs = require('fs');

async function renderScene(sceneFile, outputPrefix, size = 1000) {
  console.log(`\n=== Rendering ${sceneFile} ===`);
  
  const sceneJson = fs.readFileSync(sceneFile, 'utf8');
  
  // Default k-d tree version
  console.log("Rendering with k-d tree (default)...");
  const kdtreeOutput = `../examples/${outputPrefix}_kdtree_${size}.png`;
  const kdtreeResult = rtrace.renderScene(sceneJson, kdtreeOutput, size);
  console.log("✓", kdtreeResult);
  
  // Multi-threaded version with specific thread count
  console.log("Rendering with 4 threads...");
  const threadedOutput = `../examples/${outputPrefix}_4threads_${size}.png`;
  const threadedResult = rtrace.renderSceneThreaded(sceneJson, threadedOutput, size, 4);
  console.log("✓", threadedResult);
}

async function main() {
  try {
    console.log("🔧 Plus.stl Debug Renders");
    console.log("Generating optimized renders with k-d tree acceleration");
    
    // Render all three views
    await renderScene('.../examples/plus_front.json', 'plus_front', 1000);
    await renderScene('.../examples/plus_side.json', 'plus_side', 1000);
    await renderScene('.../examples/plus_perspective.json', 'plus_perspective', 1000);
    
    console.log("\n🎉 All renders completed!");
    console.log("\nGenerated images:");
    console.log("Front view:");
    console.log("  - K-d tree:     ../examples/plus_front_kdtree_1000.png");
    console.log("  - 4 threads:    ../examples/plus_front_4threads_1000.png");
    console.log("Side view:");
    console.log("  - K-d tree:     ../examples/plus_side_kdtree_1000.png");
    console.log("  - 4 threads:    ../examples/plus_side_4threads_1000.png");
    console.log("Perspective view:");
    console.log("  - K-d tree:     ../examples/plus_perspective_kdtree_1000.png");
    console.log("  - 4 threads:    ../examples/plus_perspective_4threads_1000.png");
    
    console.log("\nNote: renderSceneBruteForce was removed - all renders now use k-d tree optimization");
    
  } catch (error) {
    console.error("❌ Error:", error);
  }
}

main();