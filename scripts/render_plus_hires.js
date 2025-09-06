const rtrace = require('../rtrace.node');
const fs = require('fs');

try {
  console.log("Loading plus.stl scene for high-res rendering...");
  const sceneJson = fs.readFileSync('.../examples/plus_front.json', 'utf8');
  
  // Render both in high resolution for better comparison
  console.log("\nRendering plus.stl 800x600 WITH k-d tree acceleration...");
  const resultKdTree = rtrace.renderScene(sceneJson, '.../examples/plus_kdtree_800x600.png', 800, 600);
  console.log("K-d tree result:", resultKdTree);
  
  console.log("\nRendering plus.stl 800x600 WITHOUT k-d tree (brute force)...");
  const resultBruteForce = rtrace.renderSceneBruteForce(sceneJson, '.../examples/plus_brute_force_800x600.png', 800, 600);
  console.log("Brute force result:", resultBruteForce);
  
  console.log("\n✅ High-res renders completed successfully!");
  console.log("Compare these high-resolution images:");
  console.log("- K-d tree:     ../examples/plus_kdtree_800x600.png");
  console.log("- Brute force:  ../examples/plus_brute_force_800x600.png");
  
} catch (error) {
  console.error("Error:", error);
}