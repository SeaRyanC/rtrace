const rtrace = require('../rtrace.node');
const fs = require('fs');

async function renderScene(sceneFile, outputPrefix, size = 1000) {
  console.log(`\n=== Rendering ${sceneFile} ===`);
  
  const sceneJson = fs.readFileSync(sceneFile, 'utf8');
  
  // K-d tree version
  console.log("Rendering WITH k-d tree...");
  const kdtreeOutput = `../examples/${outputPrefix}_kdtree_${size}.png`;
  const kdtreeResult = rtrace.renderScene(sceneJson, kdtreeOutput, size);
  console.log("✓", kdtreeResult);
  
  // Brute force version  
  console.log("Rendering WITHOUT k-d tree (brute force)...");
  const bruteOutput = `../examples/${outputPrefix}_brute_force_${size}.png`;
  const bruteResult = rtrace.renderSceneBruteForce(sceneJson, bruteOutput, size);
  console.log("✓", bruteResult);
}

async function main() {
  try {
    console.log("🔧 Plus.stl Debugging Renders");
    console.log("Comparing k-d tree vs brute force triangle intersection");
    
    // Render all three views
    await renderScene('.../examples/plus_front.json', 'plus_front', 1000);
    await renderScene('.../examples/plus_side.json', 'plus_side', 1000);
    await renderScene('.../examples/plus_perspective.json', 'plus_perspective', 1000);
    
    console.log("\n🎉 All renders completed!");
    console.log("\nGenerated images for comparison:");
    console.log("Front view:");
    console.log("  - K-d tree:     ../examples/plus_front_kdtree_800x600.png");
    console.log("  - Brute force:  ../examples/plus_front_brute_force_800x600.png");
    console.log("Side view:");
    console.log("  - K-d tree:     ../examples/plus_side_kdtree_800x600.png");
    console.log("  - Brute force:  ../examples/plus_side_brute_force_800x600.png");
    console.log("Perspective view:");
    console.log("  - K-d tree:     ../examples/plus_perspective_kdtree_800x600.png");
    console.log("  - Brute force:  ../examples/plus_perspective_brute_force_800x600.png");
    
    console.log("\nCompare these pairs to identify if the issue is in k-d tree or triangle logic.");
    
  } catch (error) {
    console.error("❌ Error:", error);
  }
}

main();