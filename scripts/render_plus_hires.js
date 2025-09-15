const rtrace = require('../rtrace.node');
const fs = require('fs');

try {
  console.log("Loading plus.stl scene for high-res rendering...");
  const sceneJson = fs.readFileSync('.../examples/plus_front.json', 'utf8');
  
  // Render in high resolution with different threading options
  console.log("\nRendering plus.stl with diagonal 1000 using k-d tree acceleration...");
  const resultKdTree = rtrace.renderScene(sceneJson, '.../examples/plus_kdtree_1000.png', 1000);
  console.log("K-d tree result:", resultKdTree);
  
  console.log("\nRendering plus.stl with diagonal 1000 using single thread...");
  const resultSingleThread = rtrace.renderSceneThreaded(sceneJson, '.../examples/plus_single_thread_1000.png', 1000, 1);
  console.log("Single thread result:", resultSingleThread);
  
  console.log("\n✅ High-res renders completed successfully!");
  console.log("Compare these high-resolution images:");
  console.log("- Multi-threaded: ../examples/plus_kdtree_1000.png");
  console.log("- Single thread:  ../examples/plus_single_thread_1000.png");
  console.log("\nNote: renderSceneBruteForce was removed - all renders now use k-d tree optimization");
  
} catch (error) {
  console.error("Error:", error);
}