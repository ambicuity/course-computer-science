# Pixels, Colors, Gamma

> If you blend colors in the wrong space, your midtones go dark — and you won't know why.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 13
**Time:** ~45 minutes

## Learning Objectives

- Convert fluently between sRGB-encoded values and linear-light values using both the exact piecewise function and the approximate power law.
- Explain why gamma correction exists (CRT physics, perceptual uniformity) and predict what goes wrong when you skip it.
- Blend two colors in both sRGB space and linear space, observe the darkening artifact, and explain it mathematically.
- Implement pre-multiplied alpha compositing and articulate why it avoids fringing along transparent edges.

## The Problem

This lesson sits in **Phase 14 — Computer Graphics & Visualization**. The phase capstone is a path tracer plus a triangle rasterizer. Both produce colors that must be correct on screen. If you write color values straight to a pixel buffer without understanding gamma, your rendered images will have darkened midtones, banding in gradients, and fringing around transparent edges — and you'll have no idea why.

Here's the concrete scenario: you write a rasterizer that fills two halves of a quad. The left half is red `(1.0, 0.0, 0.0)` and the right half is green `(0.0, 1.0, 0.0)`. At the boundary, you blend 50/50. You expect a pleasant midtone — a dark yellow. Instead you get a muddy, dim olive. The reason: you blended in sRGB space (what's stored in the framebuffer) instead of linear light space (what physics requires). This lesson teaches you why that happens and how to fix it.

## The Concept

### Pixels and Coordinates

A **pixel** is the smallest addressable unit on a display. Pixel coordinates are integers — pixel `(0, 0)` covers the area from `(0.0, 0.0)` to `(1.0, 1.0)` in continuous space. This is the **point-sampling** vs **area-averaging** distinction, and it matters for rendering: a triangle that covers half of pixel `(3, 7)` should contribute 0.5 of its color to that pixel, not all or nothing.

**Sub-pixel rendering** uses the fact that each pixel has RGB sub-pixels physically offset, which lets LCDs simulate higher horizontal resolution for text. But for graphics, a pixel is one sample — one color triple.

**Display resolution** is the grid: 1920×1080 means 2,073,600 pixels, each with an R, G, B triple stored as 8-bit integers (0–255). That's 6.2 MB per frame in raw form. The framebuffer *is* this grid in memory.

### Color Spaces: Why "Just RGB" Isn't Enough

RGB values are meaningless without a color space. The most common is **sRGB** (IEC 61966-2-1), which defines:

1. The chromaticities of the red, green, and blue primaries and the white point (D65).
2. The **transfer function** (gamma curve) that maps linear light intensity to encoded values.

When you write `(128, 128, 128)` to a pixel, you're writing an sRGB-encoded value, not a linear value. The display hardware applies the inverse transfer function (the "gamma decode") to produce linear light. This distinction is the source of most color bugs in graphics.

### Gamma: The Power Law

**CRT monitors** had a physical property: the electron gun output was proportional to the input voltage raised to the power of ~2.2. If you sent voltage `v`, the light output was `v^2.2`. For a mid-gray input of 0.5, you'd get `0.5^2.2 ≈ 0.22` — only 22% brightness. Images looked far too dark.

```
Linear input:     0.0  0.2  0.4  0.6  0.8  1.0
CRT output:      0.0  0.03 0.13 0.30 0.56 1.0   (γ=2.2)
                                         ↑ way too dim at midtones
```

The fix: pre-distort the signal. If the hardware applies `γ=2.2`, encode with `1/2.2 ≈ 0.45`:

```
Encoded:          0.0  0.2^0.45  0.4^0.45  0.6^0.45  0.8^0.45  1.0
                  ≈0   0.48      0.67      0.79      0.89      1.0
CRT decode:      ≈0   0.48^2.2  0.67^2.2  ...                            ≈ 0.2, 0.4, 0.6, 0.8, 1.0 ✅
```

The encoding curve compensates the display's decoding curve. The result: linear relationship between intended and displayed intensity. This is **gamma correction**.

**Lucky coincidence:** the gamma ~2.2 CRT curve and the human visual system's ~0.42 perceptual curve are nearly inverses. This means sRGB encoding allocates more values to darker tones where our eyes are more sensitive, reducing visible banding in 8-bit channels. Gamma was initially a bug in CRTs; it turned out to be a feature for perceptual uniformity.

### sRGB Transfer Function

Modern sRGB isn't just a simple power law. The official spec (IEC 61966-2-1) defines a **piecewise** function:

**Linear→sRGB (encode):**

```
if linear ≤ 0.0031308:
    srgb = 12.92 × linear
else:
    srgb = 1.055 × linear^(1/2.4) − 0.055
```

**sRGB→Linear (decode):**

```
if srgb ≤ 0.04045:
    linear = srgb / 12.92
else:
    linear = ((srgb + 0.055) / 1.055)^2.4
```

The linear segment near zero avoids a derivative discontinuity at the origin (a pure power law has infinite slope at zero). The transition points (0.0031308 / 0.04045) are chosen so the piecewise function is C¹-continuous.

**Approximation:** For many practical purposes, `srgb ≈ linear^(1/2.2)` and `linear ≈ srgb^2.2`. The error is small except near black, where the piecewise linear toe prevents infinite slope.

### The Blending Error: sRGB vs Linear

**The classic bug:** blend two colors at 50% in sRGB space:

```
sRGB_blend = (sRGB_a + sRGB_b) / 2
```

This is wrong because sRGB is nonlinear. The correct process:

```
1. Decode both colors to linear: L_a = srgb_to_linear(sRGB_a), L_b = srgb_to_linear(sRGB_b)
2. Blend in linear space:       L_mix = (L_a + L_b) / 2
3. Encode back to sRGB:        sRGB_mix = linear_to_srgb(L_mix)
```

**Numerical example:** Blending red `(1.0, 0, 0)` and green `(0, 1.0, 0)` at 50%:

- Wrong (sRGB blend): `(0.5, 0.5, 0)` — displayed brightness ~0.22 per channel
- Correct (linear blend): decode R=1.0, G=1.0, blend in linear = (0.5, 0.5, 0), encode = `(0.735, 0.735, 0)` — displayed brightness matches 50% of each

Wait — that example is simple because the primaries are at full intensity. The real pain shows at mid-values:

- Blending sRGB `(0.5, 0.5, 0.5)` with `(0.0, 0.0, 0.0)` at 50%:
  - Wrong (sRGB blend): `(0.25, 0.25, 0.25)` → linear ≈ 0.050 → 5% brightness instead of expected 25%
  - Correct (linear blend): decode 0.5 → linear 0.214, average with 0 → 0.107, encode → sRGB ≈ 0.382 → ~38% of 0.5's brightness. Wait, let's be precise:
  - Linear blend of (0.214, 0.214, 0.214) with (0, 0, 0) = (0.107, 0.107, 0.107), encode → sRGB ≈ 0.382
  - sRGB blend: (0.25, 0.25, 0.25) → linear ≈ 0.050 → **only 5% of max brightness, not the 25% you'd expect**

The sRGB blend produces an image that's visually far too dark in midtones. This is why every serious renderer works in linear space.

### Additive vs Subtractive Color Mixing

**Additive** (light): Red + Green = Yellow. Used in displays, rendering. More light = brighter.
**Subtractive** (pigment/ink): Cyan + Magenta = Blue. Used in print. More ink = darker (absorbs more light).

In graphics we use additive mixing exclusively. The "RGB" in your framebuffer means: this pixel emits this much red, green, and blue light.

### Pre-multiplied Alpha

When compositing (layering a semi-transparent source over a destination), you compute:

```
result = src_color × src_alpha + dst_color × (1 − src_alpha)
```

**Straight alpha** stores `(R, G, B, A)` with R/G/B as the full-intensity color regardless of transparency.
**Pre-multiplied alpha** stores `(R×A, G×A, B×A, A)` — the color components already multiplied by alpha.

Pre-multiplied alpha has two advantages:

1. **Correct compositing** with a single operation per channel: `result = src + dst × (1 − src_alpha)`. No per-channel multiply by alpha during blend.
2. **No fringing.** With straight alpha, if you scale or filter before compositing, the RGB channels bleed past the alpha boundary, causing dark or colored halos. Pre-multiplication clips this: where alpha is zero, color is zero by construction.

**Concrete example:**

```
Straight:    color=(1.0, 0.0, 0.0) alpha=0.5 → stored as (1.0, 0.0, 0.0, 0.5)
Pre-multiplied: color=(0.5, 0.0, 0.0) alpha=0.5 → stored as (0.5, 0.0, 0.0, 0.5)

Compositing over dst=(0.0, 0.0, 0.0):
  Straight: 1.0×0.5 + 0.0×0.5 = 0.5 ✅ (must multiply during blend)
  Pre-mul: 0.5 + 0.0×0.5 = 0.5 ✅ (no per-channel multiply)
```

But when you *scale* the image (e.g., for mipmapping or resizing), straight alpha causes problems:

```
Scale 2× → mix two texels at (1.0, 0.0, 0.0, 0.5) and (0.0, 0.0, 0.0, 0.0)
Straight average: (0.5, 0.0, 0.0, 0.25) → composited: 0.5/0.25 = 2.0 per channel?! CLAMP.
Pre-mul average: (0.25, 0.0, 0.0, 0.25) → composited: 0.25/0.25 = 1.0 ✅
```

The straight-alpha version produces a color value that exceeds 1.0 after un-premultiplying — a **fringing artifact** visible as white or colored edges around transparent regions.

### Banding in 8-bit

sRGB's perceptual encoding allocates more codes to dark values, reducing visible banding. In 8-bit linear space, you only get ~10 perceptually distinct steps below intensity 0.1, vs ~50 steps in sRGB. This is why textures are stored in sRGB and converted to linear on the GPU — the higher precision lives where your eyes need it.

## Build It

### Step 1: sRGB ↔ Linear Conversion

```python
def srgb_to_linear(srgb_value: float) -> float:
    """Convert a single sRGB-encoded channel [0,1] to linear light."""
    if srgb_value <= 0.04045:
        return srgb_value / 12.92
    return ((srgb_value + 0.055) / 1.055) ** 2.4

def linear_to_srgb(linear_value: float) -> float:
    """Convert a linear-light channel [0,1] to sRGB encoding."""
    if linear_value <= 0.0031308:
        return 12.92 * linear_value
    return 1.055 * (linear_value ** (1.0 / 2.4)) - 0.055
```

### Step 2: Blending in sRGB vs Linear — Showing the Darkening Bug

```python
def blend_srgb_wrong(a: tuple, b: tuple, t: float) -> tuple:
    """Blend in sRGB space — produces darkened midtones."""
    return tuple(
        a[i] * (1 - t) + b[i] * t for i in range(3)
    )

def blend_linear_correct(a: tuple, b: tuple, t: float) -> tuple:
    """Blend in linear space — physically correct."""
    a_lin = tuple(srgb_to_linear(c) for c in a)
    b_lin = tuple(srgb_to_linear(c) for c in b)
    mixed = tuple(a_lin[i] * (1 - t) + b_lin[i] * t for i in range(3))
    return tuple(linear_to_srgb(c) for c in mixed)
```

### Step 3: Pre-multiplied Alpha Compositing

```python
def composite_premul(src: tuple, dst: tuple) -> tuple:
    """Composite pre-multiplied src over dst. Each is (R,G,B,A)."""
    src_r, src_g, src_b, src_a = src
    dst_r, dst_g, dst_b, dst_a = dst
    out_a = src_a + dst_a * (1 - src_a)
    if out_a == 0:
        return (0.0, 0.0, 0.0, 0.0)
    out_r = src_r + dst_r * (1 - src_a)
    out_g = src_g + dst_g * (1 - src_a)
    out_b = src_b + dst_b * (1 - src_a)
    return (out_r, out_g, out_b, out_a)

def composite_straight(src: tuple, dst: tuple) -> tuple:
    """Composite straight-alpha src over dst. Each is (R,G,B,A)."""
    src_r, src_g, src_b, src_a = src
    dst_r, dst_g, dst_b, dst_a = dst
    out_a = src_a + dst_a * (1 - src_a)
    if out_a == 0:
        return (0.0, 0.0, 0.0, 0.0)
    out_r = (src_r * src_a + dst_r * dst_a * (1 - src_a)) / out_a
    out_g = (src_g * src_a + dst_g * dst_a * (1 - src_a)) / out_a
    out_b = (src_b * src_a + dst_b * dst_a * (1 - src_a)) / out_a
    return (out_r, out_g, out_b, out_a)
```

### Step 4: Gamma Ramp and PPM Image Generation

```python
def generate_gamma_ramp(width: int = 256, height: int = 128) -> list:
    """Generate a row of pixels showing linear ramp in sRGB encoding."""
    row = []
    for x in range(width):
        linear_intensity = x / (width - 1)
        srgb_val = linear_to_srgb(linear_intensity)
        byte_val = int(srgb_val * 255 + 0.5)
        row.append((byte_val, byte_val, byte_val))
    rows = [row] * height
    return rows
```

### Step 5: Full Program with PPM Output

The complete program in `code/main.py` demonstrates all of the above and writes PPM images so you can see the difference visually.

## Use It

**OpenGL/WebGPU:** Textures loaded from disk are sRGB-encoded. You must create them with the `SRGB8` / `RGBA8_SRGB` format so the GPU decodes them to linear on sampling. The framebuffer is sRGB-encoded, so the GPU re-encodes on write. If you forget the sRGB format flag, you get the dark-midtone bug exactly as shown above.

**CSS:** `linear-gradient(to right, red, green)` blends in sRGB by default. Use `color(interpolate-in-srgb ...)` for sRGB or `color(interpolate-in-oklab ...)` for perceptually uniform. There's no "linear-light" blend in CSS — you'd need `@property` hacks or WebGL.

**Python/Pillow:** `Image.blend(img_a, img_b, alpha)` blends sRGB values directly. For correct blending, you must manually convert to linear, blend, then convert back. `skimage` and `colour-science` libraries provide proper sRGB/linear conversion.

**Key production source:**

- Chromium's sRGB conversion: `third_party/blink/renderer/platform/graphics/color_space.cc`
- Blender's color management: `source/blender/blenlib/intern/math_color.cc` (function `srgb_to_linearrgb`)

## Read the Source

- `skia/src/core/SkColorSpaceXform.cpp` — Skia's color-space conversion. Look for `srgb_to_linear` and the piecewise function. Note how they handle the linear toe for numerical stability.
- `blender/source/blender/blenlib/intern/math_color.cc` — Blender's `srgb_to_linearrgb` and `linearrgb_to_srgb`. Clean, well-documented reference implementation.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **`srgb.py`** — A self-contained Python module with sRGB↔linear conversion, correct linear blending, pre-multiplied alpha compositing, and gamma ramp generation. Import it in later phases of the path tracer and rasterizer.

## Exercises

1. **Easy** — Write a function that generates a 256×1 gradient image where the leftmost pixel is sRGB 0 and the rightmost is sRGB 255, with linear interpolation in between. Then generate the same gradient but with linear interpolation in *linear* space (encode each step). Compare the two images side by side. Which one looks perceptually uniform?

2. **Medium** — Implement a checkerboard pattern where each cell is a 50/50 blend of two colors. Generate two PPM images: one blending in sRGB space and one blending in linear space. Measure the average luminance of each image using the formula `L = 0.2126*R + 0.7152*G + 0.0722*B` (in linear space). How much darker is the sRGB-blended version?

3. **Hard** — Implement a simple resize filter (e.g., box filter averaging 2×2 pixels) for RGBA images. Show that resizing a straight-alpha image produces fringing artifacts along transparent edges, while resizing a pre-multiplied-alpha image does not. Write a PPM comparison. For extra challenge: demonstrate why converting *back* from pre-multiplied to straight alpha after resizing can still produce valid results, and identify the one case where it cannot.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Gamma | "Make it brighter" | A power-law transfer function relating encoded values to linear light intensity. Encoding gamma < 1 (compand for display), decoding gamma > 1 (display physics). |
| sRGB | "Regular RGB" | A specific color space with defined primaries (D65 white point) and a piecewise transfer function (linear toe + power curve) that is the de facto standard for the web and most displays. |
| Linear light | "The real brightness" | Physical light intensity, proportional to photon count. Blending and lighting calculations must happen in this space to be physically correct. |
| Gamma correction | "Gamma adjustment" | Applying ~1/2.2 power to linear values before storage, compensating for the display's ~2.2 power curve, so the net result is identity (intended = displayed). |
| Pre-multiplied alpha | "Alpha times color" | Storing (R×A, G×A, B×A, A) instead of (R, G, B, A). Eliminates fringing artifacts during filtering and simplifies compositing to one multiply per destination channel. |
| Banding | "Steppy gradients" | Visible quantization steps in smooth gradients. Worse in linear 8-bit because sRGB's perceptual encoding packs more codes into dark regions where eyes are most sensitive. |

## Further Reading

- [What Every Coder Should Know About Gamma](https://blog.johnnovak.net/2016/09/21/what-every-coder-should-know-about-gamma/) — John Novak's deep walkthrough of the gamma problem with visuals. Best single resource on the topic.
- [The sRGB Color Space](https://www.w3.org/Graphics/Color/sRGB) — The W3C's normative reference for sRGB. Read for the exact transfer function parameters.
- [Premultiplied Alpha](https://developer.nvidia.com/content/alpha-blending-premultiplied-alpha) — NVIDIA's Tom Forsyth on why pre-multiplied alpha is the right default. Short and practical.
- [Colour-Science Python Library](https://colour-science.readthedocs.io/) — Production-grade color-science library. Compare your `srgb_to_linear` against theirs. Look at `colour.models.eotf_sRGB` and `colour.models.eotf_inverse_sRGB`.
- [GPU Gems 3: Compositing](https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-space-techniques/chapter-24-advanced-volumetric-lighting) — Chapter on compositing that covers pre-multiplied alpha in production, with discussion of alpha fringe artifacts.