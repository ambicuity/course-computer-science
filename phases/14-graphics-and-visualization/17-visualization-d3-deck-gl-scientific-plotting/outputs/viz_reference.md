# Visualization Reference Card

## Chart Type Selection Guide

| What you want to show | Best chart type | Encoding used |
|---|---|---|
| Compare categories | Bar chart (vertical or horizontal) | Position on common scale |
| Distribution of one variable | Histogram or box plot | Position + length |
| Relationship between two variables | Scatter plot | Position × position |
| Trend over time | Line chart | Position over time axis |
| Composition / part-to-whole | Stacked bar (not pie) | Length segments |
| Multi-variable comparison | Small multiples (faceting) | Position per panel |
| Geospatial density | Choropleth or heatmap | Color on spatial position |
| Network relationships | Force-directed graph | Position (computed) |
| > 100K points on a map | deck.gl scatter-plot layer | GPU position + color |

## Data Type → Encoding Mapping

| Data type | Best encoding | Worst encoding |
|---|---|---|
| Categorical | Position (bar), hue | Area, angle |
| Ordinal | Position, ordered color palette | Unordered hue |
| Quantitative | Position (scatter, line) | Hue (unordered) |
| Temporal | Position on x-axis | Area |
| Spatial | Position (map), color (heatmap) | Table, text |

## Cleveland's Hierarchy (Most → Least Accurate)

1. Position on common scale ★★★★★
2. Position on identical scales ★★★★★
3. Length ★★★★
4. Angle / Slope ★★★
5. Area ★★
6. Volume / Curvature ★★
7. Color saturation ★
8. Color hue ★

**Rule:** Encode your most important variable with the highest-ranking channel.

## D3 Data Join Pattern

```
DATA (new array) ──┐
                   ├─► JOIN ──► ENTER (new keys) ──► append()
SELECTION (DOM) ───┘          ├─► UPDATE (matched) ──► transition()
                              └─► EXIT (orphaned) ──► remove()
```

```javascript
const join = d3.select("svg")
  .selectAll("circle")
  .data(data, d => d.id);  // key function for object constancy

join.enter()    // new data → create elements
  .append("circle")
  .attr("r", 0)
  .transition().attr("r", 5);

join.update()   // existing data → modify
  .transition()
  .attr("cx", d => x(d.value))
  .attr("cy", d => y(d.value));

join.exit()     // removed data → destroy
  .transition().attr("r", 0)
  .remove();
```

## Color Palette Recommendations

### Quantitative (Sequential) — Perceptually Uniform
- **viridis** — Purple → teal → yellow (default for matplotlib, best general-purpose)
- **magma** — Black → purple → orange → yellow (dark background friendly)
- **inferno** — Black → red → orange → yellow (high contrast)
- **plasma** — Purple → pink → orange → yellow (warm tones)

### Quantitative (Diverging)
- Blue → white → red (cool/warm for data with meaningful center)
- Purple → orange

### Categorical — Colorblind-Safe
- **Tableau 10:** #4e79a7, #f28e2b, #e15759, #76b7b2, #59a14f, #edc948, #b07aa1, #ff9da7, #9c755f, #bab0ac
- **ColorBrewer Qualitative:** Set1, Set2, Paired (max 3-8 categories)

### Never Use
- Rainbow / jet — artificial boundaries, not perceptually uniform
- Red + green alone — fails deuteranopia and protanopia
- Equal-hue gradients — hard to distinguish categories

## D3 vs. deck.gl Decision Guide

| Question | Choose |
|---|---|
| < 10K points, rich interaction needed? | D3 |
| > 100K points, need performance? | deck.gl |
| Geospatial / map-based data? | deck.gl |
| Custom animated transitions? | D3 |
| Both scale + interactivity? | deck.gl + D3 overlay |

## Accessibility Checklist

- [ ] Every chart has `<title>` and `<desc>` (SVG) or alt text
- [ ] Information is conveyed by more than color alone (use shape, pattern, position)
- [ ] 4.5:1 contrast ratio for text on background
- [ ] Color palette passes colorblind simulation (Coblis, SimDaltonism)
- [ ] Tabular data alternative available for screen readers
- [ ] Interactive features are keyboard-accessible

## Grammar of Graphics Composition

```
Plot = Data          (rows, columns)
     + Mapping       (column → visual channel)
     + Geometry      (bar, point, line, polygon, contour)
     + Scale         (data domain → visual range)
     + Coordinate    (Cartesian, polar, map projection)
     + Facet         (split panels by category)
```

Each component is independent and composable — change the geometry from bar to point
without touching the scale; change the scale from linear to log without touching the data.