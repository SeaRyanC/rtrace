# Diffuse Light Sources

This document demonstrates the diffuse light source feature in rtrace. Diffuse lights create soft shadows and area lighting effects by simulating light sources with physical size.

## Overview

Traditional point lights cast sharp, hard shadows because they are treated as infinitesimally small points. Diffuse lights have an optional `diameter` property that creates a disk-shaped light source, resulting in soft shadows and more realistic lighting.

## JSON Schema

Add the optional `diameter` field to any light source:

```json
{
  "lights": [
    {
      "position": [2, 4, 3],
      "color": "#FFFFFF",
      "intensity": 1.0,
      "diameter": 2.0
    }
  ]
}
```

- `diameter` (optional): The diameter of the light disk. If omitted, the light behaves as a traditional point light.
- Point lights (`diameter: null` or omitted): Sharp shadows, fast rendering
- Diffuse lights (`diameter: > 0`): Soft shadows, slower rendering due to multiple shadow ray sampling

## Example Scene

See `diffuse_light_demo.json` for a demonstration scene that compares:
- Left sphere: illuminated by a point light (sharp shadows)
- Right sphere: illuminated by a diffuse light with 2.0 diameter (soft shadows)

## Technical Implementation

- Diffuse lights sample 16 random points on the light disk per pixel
- Each sample point casts a shadow ray to determine visibility
- Final lighting is averaged across all samples
- The intensity received is proportional to how many rays can reach the light source

## Performance Considerations

- Point lights: 1 shadow ray per light per pixel
- Diffuse lights: 16 shadow rays per light per pixel (16x slower)
- Use diffuse lights sparingly for best performance
- Consider using fewer samples for preview renders

## Visual Effects

Diffuse lights create several realistic lighting effects:
- **Soft shadows**: Shadow edges fade gradually from dark to light
- **Contact shadows**: Areas near contact points have sharper shadows
- **Area lighting**: Objects receive light from multiple directions
- **Realistic penumbra**: Natural shadow falloff similar to real-world lighting

This feature enables more photorealistic rendering by simulating how light behaves with physical light sources rather than mathematical points.