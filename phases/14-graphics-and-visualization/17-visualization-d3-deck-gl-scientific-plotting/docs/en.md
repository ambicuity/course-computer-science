# Visualization — D3, deck.gl, Scientific Plotting

> The last mile of the graphics pipeline: turning data into something humans can see, understand,
> and act on. Every pixel you rasterized, every color you interpolated, every vector you transformed
> exists so that a human eye can extract meaning from numbers.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 14 lessons 01–16
**Time:** ~45 minutes

## Learning Objectives

- Classify data by type (categorical, ordinal, quantitative, temporal, spatial) and match each to effective visual encodings.
- Explain Cleveland's hierarchy of visual encodings and why position > length > angle > area > color.
- Describe the D3 data-join pattern (enter/update/exit) and how it differs from imperative drawing.
- Contrast D3's flexibility with deck.gl's GPU-accelerated, layer-based design for large-scale geospatial data.
- Generate SVG visualizations from raw data using only string construction (no matplotlib dependency).
- Choose perceptually uniform colormaps for quantitative data and avoid rainbow maps.
- Apply the grammar of graphics philosophy to compose plots from orthogonal components.

## The Problem

You have a CSV with 100,000 earthquake records — latitude, longitude, depth, magnitude. Staring at
the numbers tells you nothing. You need to *see* where earthquakes cluster, how depth relates to
magnitude, and whether patterns emerge over time. This is the visualization problem: mapping data
attributes to visual attributes so that pattern recognition — the thing human vision does
extraordinarily well — can operate on abstract information.

Without visualization, you're lost. With bad visualization, you're misled. This lesson teaches you
how to build good ones.

## Data Types and Visual Encodings

### Data Types

Every column in your dataset falls into one of five types:

| Type | Example | Properties |
|------|---------|------------|
| Categorical | Country names, species | No inherent order |
| Ordinal | Education level, ranks | Ordered but gaps unequal |
| Quantitative | Temperature, revenue | Equal-interval numbers |
| Temporal | Timestamps, dates | Ordered, periodic |
| Spatial | Lat/Lon, pixel coords | Position in space |

The data type determines which visual encodings work. Mapping a categorical variable to position on
a number line makes no sense. Mapping a quantitative variable to color hue wastes precision.

### Cleveland's Hierarchy

William Cleveland and Robert McGill's experiments (1984, 1985) ranked visual encodings by how
accurately people perceive them:

```
 MOST ACCURATE
 ─────────────────────────────────
  Position on common scale     ★★★★★
  Position on identical scales ★★★★★
  Length                       ★★★★
  Angle / Slope                ★★★
  Area                         ★★
  Volume / Curvature           ★★
  Color saturation             ★
  Color hue                    ★
 LEAST ACCURATE
 ─────────────────────────────────
```

**The rule:** Encode the most important variable with position. Use color for categories, not
precision quantities. Never encode three quantitative variables in a pie chart — that's angle +
area, the two worst encodings combined.

**Worked example:** You have sales data by region and quarter.

```
Region  Q1    Q2    Q3    Q4
North   120   145   160   175
South    80    90   110   130
East     95   110   125   140
West     60    75    85   100
```

A grouped bar chart uses *position on identical scales* for revenue (y-axis) and *position on
common scale* for quarter (x-axis). This is good — both important variables use position.

A pie chart uses *angle* for revenue and *color hue* for region. Both are near the bottom of
Cleveland's hierarchy. You'll struggle to see that West grew 67% while North grew 46%.

### Visual Encoding Selection Guide

```
 Data Type        Best Encoding          Avoid
 ───────────────  ────────────────────   ────────────────
 Categorical      Position (bar chart),  Angle, Area
                  Color hue
 Ordinal          Position, Color      Length (unequal bins)
                  (ordered palette)
 Quantitative     Position (scatter,    Hue (unordered)
                  line chart)
 Temporal         Position (x-axis),    Area
                  line chart
 Spatial          Position (map),       Table
                  color (heatmap)
```

## D3.js: Data-Driven Documents

### Philosophy

D3 is not a chart library. It's a *data join* over the DOM. You don't call `barChart(data)`.
You say: "bind this data array to these SVG elements, create new ones for entering data,
update existing ones, remove ones whose data left."

This separation — data binding vs. rendering — is what makes D3 composable and powerful.

### The Data Join Pattern

```
 ┌──────────────────────────────────────────────────┐
 │            DATA ARRAY                            │
 │   [A, B, C, D, E]                               │
 │                                                  │
 │   Key function maps each datum to a unique ID    │
 │                                                  │
 │           ┌──────────────┐                       │
 │           │  SELECTION    │                       │
 │           │  (DOM nodes)  │                       │
 │           │  [A_old, C_old│                       │
 │           │   F_old]      │                       │
 │           └──────┬───────┘                       │
 │                  │                                │
 │     ┌────────────┼────────────┐                  │
 │     │            │            │                  │
 │  ENTER       UPDATE       EXIT                  │
 │  (B, D, E    (A, C)       (F)                   │
 │   new)       changed)     removed)              │
 │     │            │            │                  │
 │  append()    transition   remove()              │
 │  each new    attributes    orphaned              │
 │  element     smoothly     node                   │
 │                                                  │
 └──────────────────────────────────────────────────┘
```

**Concrete example:**

```javascript
const data = [10, 20, 30, 40, 50];

// JOIN: bind data to <circle> elements
const circles = d3.select("svg")
  .selectAll("circle")
  .data(data, d => d);

// ENTER: new data points that don't have elements yet
circles.enter()
  .append("circle")
  .attr("r", 0)
  .attr("cx", (d, i) => i * 40 + 20)
  .attr("cy", d => 200 - d * 3)
  .transition()
  .attr("r", 5);

// UPDATE: existing elements whose data changed
circles.transition()
  .attr("cx", (d, i) => i * 40 + 20)
  .attr("cy", d => 200 - d * 3);

// EXIT: elements whose data left the dataset
circles.exit()
  .transition()
  .attr("r", 0)
  .remove();
```

**Why this matters:** When your data updates (new earthquake data arrives, user filters by date),
you don't redraw everything. You surgically create, modify, and remove only what changed. This
is efficient and enables smooth animated transitions.

### Scales and Axes

D3 scales map data values to visual values:

```
 Data domain:  [0, 100]         Visual range:  [0, 500] pixels

 scaleLinear:  50  →  250px     (proportional)
 scaleLog:     10  →  ~167px    (logarithmic)
 scaleOrdinal: "A" →  #4e79a7   (category → color)
 scaleBand:    "Q1"→  0–80px    (category → pixel band)
```

## deck.gl: GPU-Accelerated Geospatial Visualization

### When D3 Isn't Enough

D3 renders SVG elements. At ~10,000 elements, SVG DOM manipulation becomes sluggish. At 1 million
points, it's unusable. deck.gl solves this by:

1. Rendering on the GPU via WebGL
2. Using a layer-based API (not DOM-level selection)
3. Supporting map projections and geospatial coordinate systems natively

### Layer Architecture

```
 ┌─────────────────────────────────────────┐
 │              deck.gl                     │
 │                                         │
 │  ┌─────────┐  ┌─────────┐  ┌────────┐  │
 │  │ Scatter- │  │  Column │  │  Hex-  │  │
 │  │ plotLayer│  │  Layer  │  │ agon   │  │
 │  └────┬────┘  └────┬────┘  │ Layer  │  │
 │       │            │       └───┬────┘  │
 │       └────────────┴───────────┘       │
 │                    │                    │
 │              Layer Manager             │
 │              (viewport, pick-          │
 │               ing, z-fighting)         │
 │                    │                    │
 │              WebGL Renderer             │
 │              (GPU shaders)              │
 └─────────────────────────────────────────┘
```

**D3 vs. deck.gl decision guide:**

```
 Question                        Use
 ──────────────────────────────   ────────
 < 10K points, rich interaction?  D3
 > 100K points, need performance?  deck.gl
 Geospatial / map-based?          deck.gl
 Custom animated transitions?     D3
 Both size + interactivity?       deck.gl + D3 overlay
```

## Scientific Plotting: The Grammar of Graphics

### The Grammar

ggplot2 and matplotlib share a philosophy: a plot is the *composition* of orthogonal components.

```
 Plot = Data + Mapping + Geometry + Scale + Coordinate + Facet
         │       │          │         │         │          │
         │       │          │         │         │          └─ split by
         │       │          │         │         │             variable
         │       │          │         │         └─ map projection,
         │       │          │         │            polar, etc.
         │       │          │         └─ perceptual uniform
         │       │          │            colormaps
         │       │          └─ bar, point, line,
         │       │             polygon, contour
         │       └─ data column → visual channel
         └─ rows and columns
```

### Chart Selection Guide

```
 ┌───────────────────────────────────────────────────────────────┐
 │                     What do you want to show?                 │
 │                                                               │
 │   Comparison      Distribution     Relationship   Composition │
 │       │                │                │              │     │
 │  ┌────┴────┐     ┌─────┴─────┐    ┌─────┴─────┐   ┌───┴───┐  │
 │  │Bar chart│     │Histogram  │    │Scatter    │   │Stacked│  │
 │  │(categorical)   │(1 var)   │    │plot      │   │bar    │  │
 │  │         │     │Box plot  │    │(2 vars)  │   │chart  │  │
 │  │Dot plot │     │(summary) │    │Line chart│   │       │  │
 │  │(Cleveland)     │Violin    │    │(temporal)│   │Pie?NO │  │
 │  └─────────┘     └──────────┘    └──────────┘   └───────┘  │
 │                                                               │
 │   Geospatial        Network        Temporal                  │
 │   ┌──────┐        ┌─────────┐     ┌──────────┐              │
 │   │Choro-│        │Force-   │     │Time-     │              │
 │   │pleth │        │directed │     │series    │              │
 │   │Heat- │        │graph   │     │Slope-    │              │
 │   │map   │        │Sankey  │     │graph    │              │
 │   └──────┘        └─────────┘     └──────────┘              │
 └───────────────────────────────────────────────────────────────┘
```

## Color in Visualization

### Perceptual Uniformity

Your eye perceives changes in luminance non-linearly. A colormap that maps data[0]=blue,
data[0.5]=green, data[1.0]=yellow looks "equal" in RGB space but your brain sees the
green-yellow transition as 3× larger than the blue-green transition.

Perceptually uniform colormaps (viridis, magma, inferno, plasma) were designed so that equal
steps in data produce equal steps in perceived brightness:

```
 BAD (jet/rainbow):           GOOD (viridis):
 Data: 0.0  0.25  0.5  0.75  1.0     Data: 0.0  0.25  0.5  0.75  1.0
 Color: ████ ████ ████ ████ ████     Color: ████ ████ ████ ████ ████
        dark blue-green-yellow-red           dark purple-teal-yellow
        ↑ appears abrupt here ↑              ↑ smooth luminance gradient ↑
        cyan→green looks like a              no artificial boundaries
        sharp boundary
```

**Rules:**
1. **Quantitative data** → sequential single-hue or perceptually uniform (viridis, magma)
2. **Diverging data** → diverging colormap (blue-white-red, purple-orange)
3. **Categorical data** → qualitative palette (Tableau 10, ColorBrewer qualitative)
4. **Never use rainbow/jet** for quantitative data — it creates artificial boundaries

### Colorblind-Safe Palettes

~8% of men and ~0.5% of women have some form of color vision deficiency. Safe choices:

```
 Tableau 10 (colorblind-safe):
 #4e79a7  #f28e2b  #e15759  #76b7b2  #59a14f
 #edc948  #b07aa1  #ff9da7  #9c755f  #bab0ac

 Rules:
 - Never use red+green alone (use red+blue or orange+blue)
 - Vary lightness, not just hue
 - Test with Coblis or SimDaltonism simulators
```

## Interactive Visualization

### Techniques

```
 ┌─────────────────────────────────────────────────────┐
 │  Technique        What it does           When to use│
 │─────────────────────────────────────────────────────│
 │  Brushing         Select a region with mouse/filter  │
 │                   to highlight subset                │
 │                                                   │
 │  Linking          Brush in one view → highlight     │
 │                   in all views (Scatterplot Matrix) │
 │                                                   │
 │  Zoom/Pan         Scale + translate viewport for     │
 │                   detail or context                 │
 │                                                   │
 │  Filter/Dynamic   Adjust what data is visible       │
 │  queries          via sliders, checkboxes           │
 │                                                   │
 │  Tooltip          Hover → show detail for one      │
 │                   data point                        │
 │                                                   │
 │  Small multiples  Show same chart for each          │
 │  (faceting)       category side by side              │
 └─────────────────────────────────────────────────────┘
```

### Accessibility

- Provide `alt` text for every chart (describe the trend, not "a chart")
- Use pattern fills + color for fill/stroke (never color alone)
- Ensure 4.5:1 contrast ratio for text on backgrounds
- Test with a screen reader — SVG `<title>` and `<desc>` elements
- Structure tabular data alongside charts for screen readers

## Build It

### Python: SVG Visualization from Scratch

We'll generate three chart types as SVG strings — bar chart, scatter plot, and heatmap —
using only the Python standard library. No matplotlib, no external packages. This teaches
you what matplotlib does under the hood.

### TypeScript: D3 Data-Join Pipeline

We'll implement the data-join and visual-encoding pipeline as pure TypeScript — no browser,
no D3 library. This teaches you the pattern that makes D3 powerful.

## Read the Source

- **D3 source:** `d3/d3-selection/src/selection/data.js` — the 30-line function that
  implements enter/update/exit. It's shorter than you'd expect.
- **deck.gl layers:** `visgl/deck.gl/modules/layers/src/scatter-plot-layer/` — see how a GPU
  layer encodes data into a WebGL buffer.
- **matplotlib colormaps:** `matplotlib/matplotlib/lib/matplotlib/_cm.py` — the viridis
  perceptually uniform cmap definition.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **viz_reference.md** — A reference card with chart type selection guide, color palette
  recommendations, D3 data join pattern, and encoding hierarchy.

## Exercises

1. **Easy** — Modify the bar chart SVG generator to accept a color palette parameter and produce
   a grouped bar chart (two series per category).
2. **Medium** — Implement a contour plot generator: given a 2D function f(x,y), sample it on a
   grid and output SVG `<path>` elements for contour lines. Hint: use marching squares.
3. **Hard** — Build a complete data join engine in TypeScript that supports keyed joins, nested
   selections, and transition interpolation (lerp between old and new positions over N frames).

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|------------------------|
| Data join | "D3's update pattern" | Binding an array of data objects to a selection of DOM elements, then handling three disjoint cases: entering data (new elements), updating data (modify existing), exiting data (remove orphans) |
| Visual encoding | "How you show the data" | Mapping a data attribute (column) to a visual channel (x-position, color, size) |
| Perceptual uniformity | "Good colormap" | Equal steps in data value produce equal steps in perceived brightness — no artificial boundaries |
| Grammar of graphics | "ggplot philosophy" | Decomposing a plot into orthogonal components: data, mapping, geometry, scale, coordinate system, facet |
| Scale | "Axis scale" | A function that maps from data domain to visual range (e.g., [0,100] → [0,500px]) |
| Layer (deck.gl) | "A chart layer" | A GPU-rendered visual encoding applied to a dataset, composited in z-order with other layers |
| Brushing | "Select by dragging" | Interactive selection of a subset of data points by drawing a region; used with linking across views |

## Further Reading

- Cleveland, W.S. & McGill, R. (1984). "Graphical Perception: Theory, Experimentation, and Application to the Development of Graphical Methods." *Journal of the ASA*, 79(387), 531–554.
- Bostock, M., Ogievetsky, V. & Heer, J. (2011). "D3: Data-Driven Documents." *IEEE Trans. Visualization & Computer Graphics*, 17(12), 2301–2309.
- Wickham, H. (2010). "A Layered Grammar of Graphics." *Journal of Computational and Graphical Statistics*, 19(1), 3–28.
- Nathaniel, S. & Heer, J. (2024). "The Mosaic Framework." — D3 + sql-based interaction.
- Perceptually uniform colormaps: https://www.youtube.com/watch?v=xAoljeRJ3lU (Stéfan van der Walt, SciPy 2015)