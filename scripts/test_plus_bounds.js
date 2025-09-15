const rtrace = require('../tracer/rtrace.node');
const fs = require('fs');

try {
  console.log("Loading plus.stl scene...");
  const sceneJson = fs.readFileSync('.../examples/plus_front.json', 'utf8');
  
  // Render with k-d tree (default, optimized)
  console.log("\nRendering plus.stl with k-d tree acceleration...");
  const resultKdTree = rtrace.renderScene(sceneJson, '.../examples/plus_kdtree_500.png', 500);
  console.log("K-d tree result:", resultKdTree);
  
  // Render with threading control
  console.log("\nRendering plus.stl with specific thread count (4 threads)...");
  const resultThreaded = rtrace.renderSceneThreaded(sceneJson, '.../examples/plus_threaded_500.png', 500, 4);
  console.log("Threaded result:", resultThreaded);
  
  console.log("\n✅ Both renders completed successfully!");
  console.log("Compare these images:");
  console.log("- K-d tree:     ../examples/plus_kdtree_500.png");
  console.log("- 4 threads:    ../examples/plus_threaded_500.png");
  console.log("\nNote: renderSceneBruteForce was removed - all renders now use k-d tree optimization");
  
} catch (error) {
  console.error("Error:", error);
}