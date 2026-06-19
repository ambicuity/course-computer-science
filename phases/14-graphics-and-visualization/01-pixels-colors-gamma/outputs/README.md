# Outputs — Pixels, Colors, Gamma

## Artifacts

### `srgb.py` — Reusable Gamma & Color Module

A self-contained Python module providing:

- `srgb_to_linear(s)` / `linear_to_srgb(l)` — Exact piecewise sRGB transfer function (IEC 61966-2-1)
- `srgb_to_linear_approx(s, gamma)` / `linear_to_srgb_approx(l, gamma)` — Simplified power-law approximation
- `srgb_byte_to_linear(b)` / `linear_to_srgb_byte(l)` — 8-bit integer convenience wrappers
- `blend_linear_correct(a, b, t)` — Physically correct linear-space blending
- `blend_srgb_wrong(a, b, t)` — Incorrect sRGB-space blending (for comparison)
- `composite_premul(src, dst)` / `composite_straight(src, dst)` — Alpha compositing functions
- `generate_gamma_ramp()` — Gamma ramp visualization data

Import in later phases:
```python
from srgb import srgb_to_linear, linear_to_srgb, blend_linear_correct
```

### PPM Images

Running `python code/main.py` generates:

| File | Description |
|------|-------------|
| `gamma_ramp.ppm` | 256-wide grayscale ramp encoded in sRGB — appears perceptually uniform |
| `blend_srgb_wrong.ppm` | Red-to-green gradient blended in sRGB space — dark midtones |
| `blend_linear_correct.ppm` | Red-to-green gradient blended in linear space — physically correct |
| `darkness_demo.ppm` | Left half: linear blend of mid-gray with black. Right half: sRGB blend. Shows the darkening artifact. |

## Cheatsheet: Gamma Correction in Rendering

```
   Texture (sRGB on disk)
        │
        ▼  srgb_to_linear()  ← at texture sample time
   Linear light values
        │
        ▼  shading, lighting, blending, compositing  ← ALL computation here
   Linear result
        │
        ▼  linear_to_srgb()  ← at framebuffer write time
   Framebuffer (sRGB for display)
```

**Rule of thumb:** Convert at the boundary, compute in the middle.