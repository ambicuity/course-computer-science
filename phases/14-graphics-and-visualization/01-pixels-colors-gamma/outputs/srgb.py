"""
srgb.py — Reusable sRGB / linear conversion and compositing utilities.

Import in later phases of the path tracer and rasterizer:
    from srgb import srgb_to_linear, linear_to_srgb, blend_linear_correct
"""


def srgb_to_linear(srgb_value: float) -> float:
    """Convert a single sRGB-encoded channel [0,1] to linear light (IEC 61966-2-1)."""
    if srgb_value <= 0.04045:
        return srgb_value / 12.92
    return ((srgb_value + 0.055) / 1.055) ** 2.4


def linear_to_srgb(linear_value: float) -> float:
    """Convert a linear-light channel [0,1] to sRGB encoding (IEC 61966-2-1)."""
    if linear_value <= 0.0031308:
        return 12.92 * linear_value
    return 1.055 * (linear_value ** (1.0 / 2.4)) - 0.055


def srgb_to_linear_approx(srgb_value: float, gamma: float = 2.2) -> float:
    """Approximate sRGB decode using a simple power law (no linear toe)."""
    return max(0.0, srgb_value) ** gamma


def linear_to_srgb_approx(linear_value: float, gamma: float = 2.2) -> float:
    """Approximate sRGB encode using a simple power law (no linear toe)."""
    return max(0.0, linear_value) ** (1.0 / gamma)


def srgb_byte_to_linear(byte_val: int) -> float:
    """Convert an 8-bit sRGB channel value (0-255) to linear light [0,1]."""
    return srgb_to_linear(byte_val / 255.0)


def linear_to_srgb_byte(linear_val: float) -> int:
    """Convert a linear-light channel value [0,1] to 8-bit sRGB (0-255)."""
    clamped = max(0.0, min(1.0, linear_val))
    return int(linear_to_srgb(clamped) * 255.0 + 0.5)


def blend_linear_correct(color_a: tuple, color_b: tuple, t: float) -> tuple:
    """Blend two sRGB colors in linear space — physically correct."""
    a_lin = tuple(srgb_to_linear(c) for c in color_a)
    b_lin = tuple(srgb_to_linear(c) for c in color_b)
    mixed = tuple(a_lin[i] * (1.0 - t) + b_lin[i] * t for i in range(3))
    return tuple(linear_to_srgb(c) for c in mixed)


def blend_srgb_wrong(color_a: tuple, color_b: tuple, t: float) -> tuple:
    """Blend in sRGB space — produces physically incorrect darkened midtones."""
    return tuple(color_a[i] * (1.0 - t) + color_b[i] * t for i in range(3))


def composite_premul(src: tuple, dst: tuple) -> tuple:
    """Composite pre-multiplied src over dst. Each is (R, G, B, A)."""
    src_r, src_g, src_b, src_a = src
    dst_r, dst_g, dst_b, dst_a = dst
    out_a = src_a + dst_a * (1.0 - src_a)
    if out_a <= 0.0:
        return (0.0, 0.0, 0.0, 0.0)
    out_r = src_r + dst_r * (1.0 - src_a)
    out_g = src_g + dst_g * (1.0 - src_a)
    out_b = src_b + dst_b * (1.0 - src_a)
    return (out_r, out_g, out_b, out_a)


def composite_straight(src: tuple, dst: tuple) -> tuple:
    """Composite straight-alpha src over dst. Each is (R, G, B, A)."""
    src_r, src_g, src_b, src_a = src
    dst_r, dst_g, dst_b, dst_a = dst
    out_a = src_a + dst_a * (1.0 - src_a)
    if out_a <= 0.0:
        return (0.0, 0.0, 0.0, 0.0)
    out_r = (src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a
    out_g = (src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a
    out_b = (src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a
    return (out_r, out_g, out_b, out_a)


def to_premul(color: tuple) -> tuple:
    """Convert straight-alpha (R, G, B, A) to pre-multiplied alpha."""
    r, g, b, a = color
    return (r * a, g * a, b * a, a)


def from_premul(color: tuple) -> tuple:
    """Convert pre-multiplied alpha to straight-alpha (R, G, B, A)."""
    r, g, b, a = color
    if a <= 0.0:
        return (0.0, 0.0, 0.0, 0.0)
    return (r / a, g / a, b / a, a)