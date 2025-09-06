const rtrace = require('./rtrace.node');
const fs = require('fs');

// We can't directly get triangle count from the current API, 
// but we can estimate from the binary STL structure
function getTriangleCountFromBinarySTL(filePath) {
  const buffer = fs.readFileSync(filePath);
  if (buffer.length < 84) {
    throw new Error('STL file too short');
  }
  
  // Read triangle count from bytes 80-83 (little endian)
  const triangleCount = buffer.readUInt32LE(80);
  const expectedSize = 84 + triangleCount * 50; // header + count + triangles * 50 bytes each
  
  return { triangleCount, fileSize: buffer.length, expectedSize };
}

try {
  console.log("📊 Plus.stl Analysis");
  const info = getTriangleCountFromBinarySTL('./examples/plus.stl');
  console.log(`Triangle count: ${info.triangleCount}`);
  console.log(`File size: ${info.fileSize} bytes`);
  console.log(`Expected size: ${info.expectedSize} bytes`);
  console.log(`Size match: ${info.fileSize === info.expectedSize ? '✓' : '✗'}`);
  
  if (info.triangleCount < 1000) {
    console.log("\nThis is a small mesh, so brute force should be fast enough to compare with k-d tree.");
  } else if (info.triangleCount < 10000) {
    console.log("\nThis is a medium mesh, k-d tree should provide some speedup.");  
  } else {
    console.log("\nThis is a large mesh, k-d tree should provide significant speedup.");
  }
  
} catch (error) {
  console.error("Error analyzing STL file:", error);
}