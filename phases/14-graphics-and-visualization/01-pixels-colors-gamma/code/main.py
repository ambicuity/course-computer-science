"""
Pixels, Colors, Gamma — Phase 14 Lesson 01

Demonstrates sRGB/linear conversion, gamma-correct blending,
pre-multiplied alpha compositing, and generates PPM images
comparing correct vs incorrect approaches.
"""

import math
import os


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


def blend_srgb_wrong(color_a: tuple, color_b: tuple, t: float) -> tuple:
    """Blend in sRGB space — produces physically incorrect darkened midtones."""
    return tuple(color_a[i] * (1.0 - t) + color_b[i] * t for i in range(3))


def blend_linear_correct(color_a: tuple, color_b: tuple, t: float) -> tuple:
    """Blend in linear space — physically correct."""
    a_lin = tuple(srgb_to_linear(c) for c in color_a)
    b_lin = tuple(srgb_to_linear(c) for c in color_b)
    mixed = tuple(a_lin[i] * (1.0 - t) + b_lin[i] * t for i in range(3))
    return tuple(linear_to_srgb(c) for c in mixed)


def composite_premul(src: tuple, dst: tuple) -> tuple:
    """Composite pre-multiplied src over dst. Each is (R, G, B, A) with RGB pre-multiplied by A."""
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
    """Composite straight-alpha src over dst. Each is (R, G, B, A) with RGB not pre-multiplied."""
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


def generate_gamma_ramp(width: int = 256, height: int = 64) -> list:
    """Generate a 2D pixel array showing linear ramp encoded in sRGB."""
    row = []
    for x in range(width):
        linear_intensity = x / (width - 1)
        byte_val = linear_to_srgb_byte(linear_intensity)
        row.append((byte_val, byte_val, byte_val))
    return [row] * height


def generate_blend_comparison(width: int = 256, height: int = 128) -> tuple:
    """Generate two images: sRGB blend (top half) vs linear blend (bottom half)."""
    color_a = (1.0, 0.0, 0.0)
    color_b = (0.0, 1.0, 0.0)

    row_wrong = []
    row_correct = []
    for x in range(width):
        t = x / (width - 1)
        blended_wrong = blend_srgb_wrong(color_a, color_b, t)
        blended_correct = blend_linear_correct(color_a, color_b, t)
        px_wrong = tuple(int(c * 255 + 0.5) for c in blended_wrong)
        px_correct = tuple(int(c * 255 + 0.5) for c in blended_correct)
        row_wrong.append(px_wrong)
        row_correct.append(px_correct)

    half = height // 2
    img_wrong = [row_wrong] * half
    img_correct = [row_correct] * (height - half)
    return img_wrong, img_correct


def generate_blend_darkness_demo(width: int = 256, height: int = 64) -> list:
    """Show sRGB-blended mid-gray vs linear-blended mid-gray side by side."""
    half = width // 2
    row = []
    for x in range(width):
        if x < half:
            linear_val = 0.5 * (srgb_to_linear(1.0) + srgb_to_linear(0.0))
            srgb_val = linear_to_srgb(linear_val)
        else:
            srgb_val = (1.0 + 0.0) / 2.0
        byte_val = int(srgb_val * 255 + 0.5)
        row.append((byte_val, byte_val, byte_val))
    return [row] * height


def write_ppm(filename: str, pixels: list) -> None:
    """Write a list-of-rows pixel array to a PPM P6 file."""
    height = len(pixels)
    width = len(pixels[0])
    with open(filename, "wb") as f:
        f.write(f"P6\n{width} {height}\n255\n".encode("ascii"))
        for row in pixels:
            for r, g, b in row:
                f.write(bytes([r, g, b]))


def print_transfer_table() -> None:
    """Print a table comparing sRGB and approximate gamma curves."""
    print("  sRGB   |  Linear (exact)  |  Linear (γ=2.2)  |  Diff")
    print("---------|------------------|--------------------|--------")
    for srgb_int in range(0, 256, 16):
        s = srgb_int / 255.0
        lin_exact = srgb_to_linear(s)
        lin_approx = srgb_to_linear_approx(s)
        diff = lin_exact - lin_approx
        print(f"  {srgb_int:3d}    |     {lin_exact:.6f}     |     {lin_approx:.6f}      | {diff:+.6f}")


def print_blend_comparison() -> None:
    """Print numerical evidence of sRGB blending darkening."""
    color_a = (0.5, 0.5, 0.5)
    color_b = (0.0, 0.0, 0.0)

    result_wrong = blend_srgb_wrong(color_a, color_b, 0.5)
    result_correct = blend_linear_correct(color_a, color_b, 0.5)

    lin_wrong = srgb_to_linear(result_wrong[0])
    lin_correct = srgb_to_linear(result_correct[0])

    print("Blend sRGB(0.5, 0.5, 0.5) with sRGB(0, 0, 0) at 50%:")
    print(f"  Wrong (sRGB blend):  sRGB={result_wrong[0]:.4f} -> linear={lin_wrong:.4f}")
    print(f"  Correct (linear blend): sRGB={result_correct[0]:.4f} -> linear={lin_correct:.4f}")
    print(f"  Expected linear:     {0.5 * srgb_to_linear(0.5):.4f} (50% of brightness)")
    print(f"  sRGB blend gives:    {lin_wrong:.4f} ({lin_wrong / (0.5 * srgb_to_linear(0.5)) * 100:.1f}% of expected)")


def print_premul_comparison() -> None:
    """Show straight vs pre-multiplied alpha blending produces same result."""
    dst = (0.0, 0.0, 0.0, 1.0)
    src_straight = (1.0, 0.0, 0.0, 0.5)
    src_premul = to_premul(src_straight)

    result_straight = composite_straight(src_straight, dst)
    result_premul = composite_premul(src_premul, dst)

    print("Compositing sRGB(1,0,0) @ alpha=0.5 over black:")
    print(f"  Straight alpha result: RGB=({result_straight[0]:.3f}, {result_straight[1]:.3f}, {result_straight[2]:.3f}), A={result_straight[3]:.3f}")
    print(f"  Pre-multiplied result: RGB=({result_premul[0]:.3f}, {result_premul[1]:.3f}, {result_premul[2]:.3f}), A={result_premul[3]:.3f}")
    print("  Same result — but pre-mul avoids a per-channel multiply during compositing.")


def main() -> None:
    print("=" * 60)
    print("Pixels, Colors, Gamma — Phase 14 Lesson 01")
    print("=" * 60)

    print("\n--- Transfer Function Table ---")
    print_transfer_table()

    print("\n--- Blending Darkening Demo ---")
    print_blend_comparison()

    print("\n--- Pre-multiplied Alpha ---")
    print_premul_comparison()

    output_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "outputs")
    os.makedirs(output_dir, exist_ok=True)

    ramp = generate_gamma_ramp()
    write_ppm(os.path.join(output_dir, "gamma_ramp.ppm"), ramp)
    print(f"\nWrote outputs/gamma_ramp.ppm")

    img_wrong, img_correct = generate_blend_comparison()
    write_ppm(os.path.join(output_dir, "blend_srgb_wrong.ppm"), img_wrong)
    write_ppm(os.path.join(output_dir, "blend_linear_correct.ppm"), img_correct)
    print(f"Wrote outputs/blend_srgb_wrong.ppm  (physically incorrect)")
    print(f"Wrote outputs/blend_linear_correct.ppm (physically correct)")

    darkness = generate_blend_darkness_demo()
    write_ppm(os.path.join(output_dir, "darkness_demo.ppm"), darkness)
    print(f"Wrote outputs/darkness_demo.ppm (left=linear blend, right=sRGB blend)")

    print("\nKey takeaway: always blend, shade, and light in LINEAR space.")
    print("Store and display in sRGB. Convert at the boundary, not in the middle.")


if __name__ == "__main__":
    main()