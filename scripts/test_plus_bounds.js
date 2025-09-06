const rtrace = require('../rtrace.node');
const fs = require('fs');

try {
  console.log("Loading plus.stl scene...");
  const sceneJson = fs.readFileSync('.../examples/plus_front.json', 'utf8');
  
  // Render with k-d tree
  console.log("\nRendering plus.stl WITH k-d tree acceleration...");
  const resultKdTree = rtrace.renderScene(sceneJson, '.../examples/plus_kdtree_400x300.png', 400, 300);
  console.log("K-d tree result:", resultKdTree);
  
  // Render without k-d tree (brute force)
  console.log("\nRendering plus.stl WITHOUT k-d tree (brute force)...");
  const resultBruteForce = rtrace.renderSceneBruteForce(sceneJson, '.../examples/plus_brute_force_400x300.png', 400, 300);
  console.log("Brute force result:", resultBruteForce);
  
  console.log("\n✅ Both renders completed successfully!");
  console.log("Compare these images:");
  console.log("- K-d tree:     ../examples/plus_kdtree_400x300.png");
  console.log("- Brute force:  ../examples/plus_brute_force_400x300.png");
  
} catch (error) {
  console.error("Error:", error);
}