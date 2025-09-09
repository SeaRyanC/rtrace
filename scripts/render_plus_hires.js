const rtrace = require('../rtrace.node');
const fs = require('fs');

try {
  console.log("Loading plus.stl scene for high-res rendering...");
  const sceneJson = fs.readFileSync('.../examples/plus_front.json', 'utf8');
  
  // Render both in high resolution for better comparison
  console.log("\nRendering plus.stl with diagonal 1000 WITH k-d tree acceleration...");
  const resultKdTree = rtrace.renderScene(sceneJson, '.../examples/plus_kdtree_1000.png', 1000);
  console.log("K-d tree result:", resultKdTree);
  
  console.log("\nRendering plus.stl with diagonal 1000 WITHOUT k-d tree (brute force)...");
  const resultBruteForce = rtrace.renderSceneBruteForce(sceneJson, '.../examples/plus_brute_force_1000.png', 1000);
  console.log("Brute force result:", resultBruteForce);
  
  console.log("\n✅ High-res renders completed successfully!");
  console.log("Compare these high-resolution images:");
  console.log("- K-d tree:     ../examples/plus_kdtree_1000.png");
  console.log("- Brute force:  ../examples/plus_brute_force_1000.png");
  
} catch (error) {
  console.error("Error:", error);
}