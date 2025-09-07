# Orthographic Grid Camera Backgrounds

This document describes the orthographic grid camera background feature in rtrace, which provides world coordinate-based grid lines as backgrounds for orthographic cameras.

## Overview

When an orthographic camera ray misses all objects in the scene, rtrace can display grid lines based on world coordinates instead of a solid background color. The grid lines are centered at origin planes (XY, XZ, or YZ) and provide visual reference for scale and positioning.

## Configuration

Grid backgrounds are configured through optional parameters in the camera configuration:

```json
{
  "camera": {
    "kind": "ortho",
    "position": [0, 0, 10],
    "target": [0, 0, 0],
    "up": [0, 1, 0],
    "width": 20,
    "height": 20,
    "grid_pitch": 2.0,
    "grid_color": "#666666",
    "grid_thickness": 0.2
  }
}
```

### Grid Parameters

- **grid_pitch** (optional): Distance between grid lines in world units. If not specified, no grid is displayed.
- **grid_color** (optional): Color of grid lines in hex format (e.g., "#FF0000" for red). Required when grid_pitch is specified.
- **grid_thickness** (optional): Thickness of grid lines in world units. Defaults to 0.1 if not specified.

## How It Works

1. **Plane Selection**: The system automatically selects which coordinate plane to use for the grid based on the camera's view direction:
   - If looking mostly along Z-axis: Uses XY plane (Z=0)
   - If looking mostly along Y-axis: Uses XZ plane (Y=0)  
   - If looking mostly along X-axis: Uses YZ plane (X=0)

2. **Ray-Plane Intersection**: When a ray misses all objects, the system calculates where it intersects the selected coordinate plane.

3. **Grid Line Detection**: At the intersection point, the system checks if the point falls on a grid line based on the configured pitch and thickness.

4. **Color Selection**: Returns grid color if on a grid line, otherwise returns the background color.

## Example Usage

### Basic Grid Background

```json
{
  "camera": {
    "kind": "ortho",
    "position": [0, 0, 10],
    "target": [0, 0, 0],
    "up": [0, 1, 0],
    "width": 20,
    "height": 20,
    "grid_pitch": 1.0,
    "grid_color": "#808080",
    "grid_thickness": 0.1
  },
  "scene_settings": {
    "background_color": "#F0F0F0"
  }
}
```

This creates a light gray grid with 1-unit spacing on a light background.

### Fine Grid with Color

```json
{
  "camera": {
    "kind": "ortho",
    "position": [10, 0, 0],
    "target": [0, 0, 0],
    "up": [0, 0, 1],
    "width": 20,
    "height": 20,
    "grid_pitch": 0.5,
    "grid_color": "#FF6600",
    "grid_thickness": 0.05
  },
  "scene_settings": {
    "background_color": "#000033"
  }
}
```

This creates an orange grid with 0.5-unit spacing and thin lines on a dark blue background, viewed from the side (YZ plane).

## Important Notes

- Grid backgrounds only work with orthographic cameras (`"kind": "ortho"`). Perspective cameras ignore grid configuration.
- Grid lines are always centered at the world origin (0, 0, 0).
- The grid is infinite and extends in both directions along each axis.
- Grid thickness is measured in world units, so very thin lines may not be visible at low resolutions.

## Example Scenes

See the following example scenes in the `examples/` directory:

- `grid_background_test.json`: Basic grid background demonstration
- `grid_background_side.json`: Side view with different grid configuration

## Technical Implementation

The grid background system:

1. Only activates when all three grid parameters are configured on an orthographic camera
2. Integrates seamlessly with the existing ray tracing pipeline
3. Maintains performance by only calculating grid intersections for rays that miss all objects
4. Works with all other rtrace features including lighting, fog, and reflections

## Performance Considerations

Grid background calculation adds minimal overhead as it only runs for rays that miss all objects. The calculation involves:
- One ray-plane intersection test
- Simple modular arithmetic for grid line detection
- No additional memory allocation

The performance impact is negligible for typical scenes.