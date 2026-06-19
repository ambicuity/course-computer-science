"""
Visualization — D3, deck.gl, scientific plotting
Phase 14 — Computer Graphics & Visualization

Generate SVG visualizations from raw data using only Python stdlib.
No matplotlib. No external dependencies. Pure string construction.

Outputs bar chart, scatter plot, and heatmap as SVG files.
"""

import json
import math
import os
from dataclasses import dataclass, field
from typing import Callable


# ── Data Types and Structures ────────────────────────────────────────────

@dataclass
class DataPoint:
    x: float
    y: float
    label: str = ""
    category: str = ""
    value: float = 0.0


@dataclass
class HeatmapCell:
    row: int
    col: int
    value: float


@dataclass
class BarDatum:
    label: str
    value: float


@dataclass
class Scale:
    domain_min: float
    domain_max: float
    range_min: float
    range_max: float

    def __call__(self, value: float) -> float:
        if self.domain_max == self.domain_min:
            return (self.range_min + self.range_max) / 2
        t = (value - self.domain_min) / (self.domain_max - self.domain_min)
        return self.range_min + t * (self.range_max - self.range_min)


def svg_begin(width: int, height: int) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" '
        f'width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}">\n'
    )


def svg_end() -> str:
    return "</svg>\n"


def svg_rect(x: float, y: float, w: float, h: float,
             fill: str = "#4e79a7", stroke: str = "none",
             stroke_width: float = 0, rx: float = 0) -> str:
    parts = [f'  <rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}"']
    parts.append(f' fill="{fill}"')
    if stroke != "none":
        parts.append(f' stroke="{stroke}" stroke-width="{stroke_width:.1f}"')
    if rx > 0:
        parts.append(f' rx="{rx:.1f}"')
    parts.append("/>")
    return "".join(parts)


def svg_circle(cx: float, cy: float, r: float,
               fill: str = "#4e79a7", opacity: float = 1.0) -> str:
    return (
        f'  <circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r:.1f}" '
        f'fill="{fill}" opacity="{opacity:.2f}"/>'
    )


def svg_text(x: float, y: float, text: str,
             size: int = 12, anchor: str = "middle",
             fill: str = "#333") -> str:
    escaped = text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    return (
        f'  <text x="{x:.1f}" y="{y:.1f}" font-size="{size}" '
        f'text-anchor="{anchor}" fill="{fill}">{escaped}</text>'
    )


def svg_line(x1: float, y1: float, x2: float, y2: float,
             stroke: str = "#ccc", width: float = 1.0) -> str:
    return (
        f'  <line x1="{x1:.1f}" y1="{y1:.1f}" '
        f'x2="{x2:.1f}" y2="{y2:.1f}" '
        f'stroke="{stroke}" stroke-width="{width:.1f}"/>'
    )


def svg_group(attrs: str = "") -> str:
    return f"  <g {attrs}>" if attrs else "  <g>"


def svg_group_end() -> str:
    return "  </g>"


# ── Perceptually Uniform Colormap (viridis-like) ────────────────────

def viridis_interpolate(t: float) -> str:
    t = max(0.0, min(1.0, t))
    r = int((-1.26 * t + 0.32) * t * 255 + 0.27 * 255) & 0xFF
    g = int((0.43 * t + 0.62) * 255) & 0xFF
    b = int((1.0 * t + 0.14) * 255) & 0xFF
    r = max(0, min(255, r))
    g = max(0, min(255, g))
    b = max(0, min(255, b))
    return f"#{r:02x}{g:02x}{b:02x}"


def magma_interpolate(t: float) -> str:
    t = max(0.0, min(1.0, t))
    r = int(min(255, (0.001 + 1.69 * t) * 255)) & 0xFF
    g = int(min(255, (0.0 + 0.63 * t * t) * 255)) & 0xFF
    b = int(min(255, (0.09 + 0.67 * t) * 255)) & 0xFF
    r = max(0, min(255, r))
    g = max(0, min(255, g))
    b = max(0, min(255, b))
    return f"#{r:02x}{g:02x}{b:02x}"


# ── Bar Chart ────────────────────────────────────────────────────────────

TABLEAU_10 = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f",
    "#edc948", "#b07aa1", "#ff9da7", "#9c755f", "#bab0ac",
]


def generate_bar_chart_svg(data: list[BarDatum],
                           width: int = 640,
                           height: int = 400,
                           title: str = "Bar Chart",
                           palette: list[str] | None = None) -> str:
    colors = palette or TABLEAU_10
    margin_l, margin_r, margin_t, margin_b = 60, 20, 40, 50
    plot_w = width - margin_l - margin_r
    plot_h = height - margin_t - margin_b

    if not data:
        return svg_begin(width, height) + svg_text(width // 2, height // 2, "No data") + svg_end()

    max_val = max(d.value for d in data)
    y_scale = Scale(0, max_val * 1.15, plot_h, 0)
    y_scale_inv = Scale(0, max_val * 1.15, 0, plot_h)

    n = len(data)
    band_w = plot_w / n
    bar_w = band_w * 0.7

    parts = [svg_begin(width, height)]

    for i in range(5):
        grid_val = max_val * 1.15 * (i + 1) / 5
        gy = margin_t + y_scale(grid_val)
        parts.append(svg_line(margin_l, gy, margin_l + plot_w, gy, "#ddd", 0.5))
        parts.append(svg_text(margin_l - 8, gy + 4, f"{grid_val:.0f}", 10, "end"))

    parts.append(svg_line(margin_l, margin_t + plot_h, margin_l + plot_w, margin_t + plot_h, "#333", 1.5))
    parts.append(svg_line(margin_l, margin_t, margin_l, margin_t + plot_h, "#333", 1.5))

    for i, datum in enumerate(data):
        bx = margin_l + i * band_w + (band_w - bar_w) / 2
        bar_h = y_scale_inv(datum.value)
        by = margin_t + plot_h - bar_h
        color = colors[i % len(colors)]
        parts.append(svg_rect(bx, by, bar_w, bar_h, fill=color, rx=2))
        parts.append(svg_text(
            margin_l + i * band_w + band_w / 2,
            margin_t + plot_h + 18,
            datum.label, 11
        ))
        parts.append(svg_text(bx + bar_w / 2, by - 6, f"{datum.value:.0f}", 9))

    parts.append(svg_text(width // 2, 20, title, 14, "middle", "#222"))

    desc = f"Bar chart titled '{title}' with {n} categories"
    parts.insert(1, f'  <desc>{desc}</desc>')

    parts.append(svg_end())
    return "\n".join(parts)


# ── Scatter Plot ─────────────────────────────────────────────────────────

def generate_scatter_svg(data: list[DataPoint],
                         width: int = 640,
                         height: int = 480,
                         title: str = "Scatter Plot") -> str:
    margin_l, margin_r, margin_t, margin_b = 60, 20, 40, 50
    plot_w = width - margin_l - margin_r
    plot_h = height - margin_t - margin_b

    if not data:
        return svg_begin(width, height) + svg_text(width // 2, height // 2, "No data") + svg_end()

    xs = [d.x for d in data]
    ys = [d.y for d in data]
    x_scale = Scale(min(xs), max(xs), 0, plot_w)
    y_scale = Scale(min(ys), max(ys), plot_h, 0)

    categories = sorted(set(d.category for d in data if d.category)) or ["default"]
    cat_colors = {c: TABLEAU_10[i % len(TABLEAU_10)] for i, c in enumerate(categories)}

    parts = [svg_begin(width, height)]
    parts.append(f'  <desc>Scatter plot titled "{title}" with {len(data)} points</desc>')

    for i in range(5):
        gy = margin_t + plot_h * i / 4
        parts.append(svg_line(margin_l, gy, margin_l + plot_w, gy, "#eee", 0.5))
        y_val = min(ys) + (max(ys) - min(ys)) * (1 - i / 4)
        parts.append(svg_text(margin_l - 8, gy + 4, f"{y_val:.1f}", 10, "end"))

    for i in range(5):
        gx = margin_l + plot_w * i / 4
        parts.append(svg_line(gx, margin_t, gx, margin_t + plot_h, "#eee", 0.5))
        x_val = min(xs) + (max(xs) - min(xs)) * i / 4
        parts.append(svg_text(gx, margin_t + plot_h + 18, f"{x_val:.1f}", 10))

    parts.append(svg_line(margin_l, margin_t + plot_h, margin_l + plot_w, margin_t + plot_h, "#333", 1.5))
    parts.append(svg_line(margin_l, margin_t, margin_l, margin_t + plot_h, "#333", 1.5))

    for pt in data:
        cx = margin_l + x_scale(pt.x)
        cy = margin_t + y_scale(pt.y)
        cat = pt.category or "default"
        color = cat_colors.get(cat, TABLEAU_10[0])
        parts.append(svg_circle(cx, cy, 4, fill=color, opacity=0.75))

    legend_y = margin_t + 10
    for cat, color in cat_colors.items():
        parts.append(svg_circle(margin_l + plot_w + 10, legend_y, 4, fill=color))
        parts.append(svg_text(margin_l + plot_w + 20, legend_y + 4, cat, 10, "start"))
        legend_y += 18

    parts.append(svg_text(width // 2, 20, title, 14, "middle", "#222"))
    parts.append(svg_end())
    return "\n".join(parts)


# ── Heatmap ──────────────────────────────────────────────────────────────

def generate_heatmap_svg(cells: list[HeatmapCell],
                         rows: int, cols: int,
                         row_labels: list[str] | None = None,
                         col_labels: list[str] | None = None,
                         width: int = 560,
                         height: int = 480,
                         title: str = "Heatmap") -> str:
    margin_l, margin_r, margin_t, margin_b = 80, 40, 40, 40
    plot_w = width - margin_l - margin_r
    plot_h = height - margin_t - margin_b

    if not cells:
        return svg_begin(width, height) + svg_text(width // 2, height // 2, "No data") + svg_end()

    cell_w = plot_w / cols
    cell_h = plot_h / rows
    values = [c.value for c in cells]
    v_min, v_max = min(values), max(values)
    v_scale = Scale(v_min, v_max, 0, 1)

    parts = [svg_begin(width, height)]
    parts.append(f'  <desc>Heatmap titled "{title}" with {rows}x{cols} grid</desc>')

    cell_map: dict[tuple[int, int], HeatmapCell] = {(c.row, c.col): c for c in cells}

    for r in range(rows):
        for c in range(cols):
            cell = cell_map.get((r, c))
            if cell is None:
                continue
            t = v_scale(cell.value)
            color = viridis_interpolate(t)
            x = margin_l + c * cell_w
            y = margin_t + r * cell_h
            parts.append(svg_rect(x, y, cell_w, cell_h, fill=color, stroke="#fff", stroke_width=1))
            if cell_w > 30 and cell_h > 18:
                text_color = "#fff" if t > 0.5 else "#333"
                parts.append(svg_text(
                    x + cell_w / 2, y + cell_h / 2 + 4,
                    f"{cell.value:.1f}", 9, "middle", text_color
                ))

    if row_labels:
        for r, label in enumerate(row_labels[:rows]):
            y = margin_t + r * cell_h + cell_h / 2
            parts.append(svg_text(margin_l - 8, y, label, 10, "end"))

    if col_labels:
        for c, label in enumerate(col_labels[:cols]):
            x = margin_l + c * cell_w + cell_w / 2
            parts.append(svg_text(x, margin_t + plot_h + 18, label, 9))

    bar_x = margin_l + plot_w + 8
    bar_w = 16
    for i in range(20):
        t = i / 19
        color = viridis_interpolate(t)
        by = margin_t + plot_h - t * plot_h
        bh = plot_h / 19 + 1
        parts.append(svg_rect(bar_x, by, bar_w, bh, fill=color))
    parts.append(svg_text(bar_x + bar_w + 4, margin_t + 6, f"{v_max:.1f}", 9, "start"))
    parts.append(svg_text(bar_x + bar_w + 4, margin_t + plot_h + 4, f"{v_min:.1f}", 9, "start"))

    parts.append(svg_text(width // 2, 20, title, 14, "middle", "#222"))
    parts.append(svg_end())
    return "\n".join(parts)


# ── Sample Data Generators ───────────────────────────────────────────────

def sample_bar_data() -> list[BarDatum]:
    return [
        BarDatum("North", 120),
        BarDatum("South", 80),
        BarDatum("East", 95),
        BarDatum("West", 60),
        BarDatum("Central", 135),
    ]


def sample_scatter_data() -> list[DataPoint]:
    import random
    rng = random.Random(42)
    data: list[DataPoint] = []
    for cat, cx, cy, spread in [("Cluster A", 3, 5, 1.2), ("Cluster B", 8, 9, 1.0), ("Cluster C", 5, 12, 0.8)]:
        for _ in range(25):
            x = rng.gauss(cx, spread)
            y = rng.gauss(cy, spread * 0.9)
            data.append(DataPoint(x=x, y=y, category=cat))
    return data


def sample_heatmap_data(rows: int = 8, cols: int = 10) -> tuple[list[HeatmapCell], int, int]:
    cells: list[HeatmapCell] = []
    for r in range(rows):
        for c in range(cols):
            value = 10 * math.sin(r * 0.5) * math.cos(c * 0.6) + 15 + r * 0.5
            cells.append(HeatmapCell(row=r, col=c, value=round(value, 1)))
    return cells, rows, cols


# ── Main ─────────────────────────────────────────────────────────────────

def main() -> None:
    out_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "outputs")
    os.makedirs(out_dir, exist_ok=True)

    bar_svg = generate_bar_chart_svg(sample_bar_data(), title="Regional Sales (units)")
    with open(os.path.join(out_dir, "bar_chart.svg"), "w") as f:
        f.write(bar_svg)
    print(f"Wrote bar_chart.svg ({len(bar_svg)} bytes)")

    scatter_svg = generate_scatter_svg(sample_scatter_data(), title="Cluster Distribution")
    with open(os.path.join(out_dir, "scatter_plot.svg"), "w") as f:
        f.write(scatter_svg)
    print(f"Wrote scatter_plot.svg ({len(scatter_svg)} bytes)")

    cells, rows, cols = sample_heatmap_data()
    row_labels = [f"Row {i}" for i in range(rows)]
    col_labels = [f"C{i}" for i in range(cols)]
    heatmap_svg = generate_heatmap_svg(
        cells, rows, cols,
        row_labels=row_labels,
        col_labels=col_labels,
        title="Signal Intensity Heatmap",
    )
    with open(os.path.join(out_dir, "heatmap.svg"), "w") as f:
        f.write(heatmap_svg)
    print(f"Wrote heatmap.svg ({len(heatmap_svg)} bytes)")


if __name__ == "__main__":
    main()