/**
 * Test script to validate ortho camera grid properties schema
 * This test ensures that grid properties are properly validated and prevents regression
 */

import { SceneSchema } from './schema';

// Test cases for ortho camera grid properties
const testCases = [
  {
    name: 'Valid ortho camera with complete grid properties',
    scene: {
      camera: {
        kind: "ortho",
        position: [3, 3, 8],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 8,
        height: 6,
        grid_pitch: 1.0,
        grid_color: "#444444",
        grid_thickness: 0.05
      },
      objects: [],
      lights: [],
      scene_settings: {
        ambient_illumination: {
          color: "#FFFFFF",
          intensity: 0.1
        }
      }
    },
    shouldPass: true
  },
  {
    name: 'Valid ortho camera without grid properties',
    scene: {
      camera: {
        kind: "ortho",
        position: [3, 3, 8],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 8,
        height: 6
      },
      objects: [],
      lights: [],
      scene_settings: {
        ambient_illumination: {
          color: "#FFFFFF",
          intensity: 0.1
        }
      }
    },
    shouldPass: true
  },
  {
    name: 'Valid ortho camera with partial grid properties',
    scene: {
      camera: {
        kind: "ortho",
        position: [3, 3, 8],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 8,
        height: 6,
        grid_color: "#444444"
      },
      objects: [],
      lights: [],
      scene_settings: {
        ambient_illumination: {
          color: "#FFFFFF",
          intensity: 0.1
        }
      }
    },
    shouldPass: true
  },
  {
    name: 'Invalid grid_pitch (negative value)',
    scene: {
      camera: {
        kind: "ortho",
        position: [3, 3, 8],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 8,
        height: 6,
        grid_pitch: -1.0,
        grid_color: "#444444",
        grid_thickness: 0.05
      },
      objects: [],
      lights: [],
      scene_settings: {
        ambient_illumination: {
          color: "#FFFFFF",
          intensity: 0.1
        }
      }
    },
    shouldPass: false
  },
  {
    name: 'Invalid grid_color (not hex format)',
    scene: {
      camera: {
        kind: "ortho",
        position: [3, 3, 8],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 8,
        height: 6,
        grid_pitch: 1.0,
        grid_color: "blue",
        grid_thickness: 0.05
      },
      objects: [],
      lights: [],
      scene_settings: {
        ambient_illumination: {
          color: "#FFFFFF",
          intensity: 0.1
        }
      }
    },
    shouldPass: false
  },
  {
    name: 'Invalid grid_thickness (negative value)',
    scene: {
      camera: {
        kind: "ortho",
        position: [3, 3, 8],
        target: [0, 0, 0],
        up: [0, 1, 0],
        width: 8,
        height: 6,
        grid_pitch: 1.0,
        grid_color: "#444444",
        grid_thickness: -0.05
      },
      objects: [],
      lights: [],
      scene_settings: {
        ambient_illumination: {
          color: "#FFFFFF",
          intensity: 0.1
        }
      }
    },
    shouldPass: false
  }
];

function runTests() {
  console.log('🧪 Testing Ortho Camera Grid Properties Schema\n');

  let passedTests = 0;
  let failedTests = 0;

  for (const testCase of testCases) {
    const result = SceneSchema.safeParse(testCase.scene);
    
    if (testCase.shouldPass && result.success) {
      console.log(`✅ ${testCase.name}: PASSED`);
      passedTests++;
    } else if (!testCase.shouldPass && !result.success) {
      console.log(`✅ ${testCase.name}: FAILED (as expected)`);
      console.log(`   Validation errors: ${result.error.issues.map(i => i.message).join(', ')}`);
      passedTests++;
    } else {
      console.log(`❌ ${testCase.name}: UNEXPECTED RESULT`);
      if (result.success) {
        console.log(`   Expected failure but validation passed`);
      } else {
        console.log(`   Expected success but validation failed:`);
        console.log(`   Errors: ${result.error.issues.map(i => i.message).join(', ')}`);
      }
      failedTests++;
    }
    console.log('');
  }

  console.log(`📊 Test Results:`);
  console.log(`   Passed: ${passedTests}`);
  console.log(`   Failed: ${failedTests}`);
  console.log(`   Total: ${passedTests + failedTests}`);

  if (failedTests > 0) {
    console.log('\n❌ Some tests failed');
    process.exit(1);
  } else {
    console.log('\n✅ All tests passed!');
  }
}

if (require.main === module) {
  runTests();
}

export { runTests };