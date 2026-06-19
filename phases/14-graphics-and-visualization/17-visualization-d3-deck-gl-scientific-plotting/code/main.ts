// Visualization — D3, deck.gl, scientific plotting
// Phase 14 — Computer Graphics & Visualization
//
// D3-style data join and visual encoding pipeline in pure TypeScript.
// No browser, no D3 library — shows the core patterns that make D3 powerful.

// ── Data Types ──────────────────────────────────────────────────────────

type DataType = "categorical" | "ordinal" | "quantitative" | "temporal" | "spatial";

interface Datum {
  id: string;
  values: Record<string, number | string>;
}

interface VisualEncoding {
  channel: "x" | "y" | "color" | "size" | "shape" | "opacity";
  field: string;
  dataType: DataType;
  scale: Scale;
}

interface Scale {
  kind: "linear" | "log" | "ordinal" | "band";
  domain: [number, number] | string[];
  range: [number, number] | string[];
}

// ── Scale Functions ──────────────────────────────────────────────────────

function applyScale(scale: Scale, value: number | string): number | string {
  if (scale.kind === "linear" || scale.kind === "log") {
    const val = typeof value === "string" ? parseFloat(value) : value;
    const domain = scale.domain as [number, number];
    const range = scale.range as [number, number];
    if (scale.kind === "log") {
      const logVal = Math.log(val) / Math.log(domain[1]);
      const logMin = Math.log(domain[0]) / Math.log(domain[1]);
      const t = (logVal - logMin) / (1 - logMin);
      return range[0] + t * (range[1] - range[0]);
    }
    const t = (val - domain[0]) / (domain[1] - domain[0]);
    return range[0] + t * (range[1] - range[0]);
  }
  if (scale.kind === "ordinal") {
    const domain = scale.domain as string[];
    const range = scale.range as string[];
    const idx = domain.indexOf(String(value));
    return range[((idx % range.length) + range.length) % range.length];
  }
  if (scale.kind === "band") {
    const domain = scale.domain as string[];
    const range = scale.range as [number, number];
    const idx = domain.indexOf(String(value));
    const bandWidth = (range[1] - range[0]) / domain.length;
    return range[0] + idx * bandWidth + bandWidth * 0.15;
  }
  return 0;
}

// ── Perceptually Uniform Colormap (viridis-like) ──────────────────────

function viridisInterpolate(t: number): string {
  t = Math.max(0, Math.min(1, t));
  const r = Math.max(0, Math.min(255, Math.round((-1.26 * t + 0.32) * t * 255 + 0.27 * 255)));
  const g = Math.max(0, Math.min(255, Math.round((0.43 * t + 0.62) * 255)));
  const b = Math.max(0, Math.min(255, Math.round((1.0 * t + 0.14) * 255)));
  return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

const TABLEAU_10: string[] = [
  "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f",
  "#edc948", "#b07aa1", "#ff9da7", "#9c755f", "#bab0ac",
];

// ── Data Join Engine ─────────────────────────────────────────────────────

interface JoinResult<T> {
  enter: T[];
  update: T[];
  exit: T[];
}

/**
 * D3's data join pattern: given new data and existing items (both keyed),
 * partition into three disjoint sets:
 *
 *   ENTER — data present in new but missing from existing (add new elements)
 *   UPDATE — data present in both (modify existing elements)
 *   EXIT   — data present in existing but missing from new (remove elements)
 */
function dataJoin<T extends { id: string }>(
  newData: T[],
  existingData: T[]
): JoinResult<T> {
  const existingMap = new Map<string, T>();
  for (const item of existingData) {
    existingMap.set(item.id, item);
  }

  const newMap = new Map<string, T>();
  for (const item of newData) {
    newMap.set(item.id, item);
  }

  const enter: T[] = [];
  const update: T[] = [];
  const exit: T[] = [];

  for (const item of newData) {
    if (existingMap.has(item.id)) {
      update.push(item);
    } else {
      enter.push(item);
    }
  }

  for (const item of existingData) {
    if (!newMap.has(item.id)) {
      exit.push(item);
    }
  }

  return { enter, update, exit };
}

// ── Visual Encoding Pipeline ─────────────────────────────────────────────

interface RenderElement {
  id: string;
  type: "rect" | "circle" | "text" | "line" | "path";
  attributes: Record<string, number | string>;
  children?: RenderElement[];
}

/**
 * Transform raw data through the grammar of graphics pipeline:
 *
 *   Raw Data → Mapped Data → Scaled Data → Render Elements
 *
 * Each step is a pure function, composable and testable.
 */
function pipeDataToRender(
  data: Datum[],
  encodings: VisualEncoding[]
): RenderElement[] {
  return data.map((datum) => {
    const attrs: Record<string, number | string> = {};

    for (const enc of encodings) {
      const rawValue = datum.values[enc.field];
      const scaledValue = applyScale(enc.scale, rawValue);
      attrs[enc.channel] = scaledValue;
    }

    attrs.fill = attrs.color ?? TABLEAU_10[0];
    delete attrs.color;

    return {
      id: datum.id,
      type: encodings.find((e) => e.channel === "size") ? "circle" : "rect",
      attributes: attrs,
    };
  });
}

// ── SVG Renderer ──────────────────────────────────────────────────────────

function renderToSvg(elements: RenderElement[], width: number, height: number): string {
  const lines: string[] = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`,
  ];

  for (const el of elements) {
    const attrStr = Object.entries(el.attributes)
      .map(([k, v]) => `${k}="${typeof v === "number" ? v.toFixed(1) : v}"`)
      .join(" ");
    if (el.type === "circle") {
      const cx = typeof el.attributes.x === "number" ? el.attributes.x : 0;
      const cy = typeof el.attributes.y === "number" ? el.attributes.y : 0;
      const r = typeof el.attributes.size === "number" ? el.attributes.size : 4;
      lines.push(`  <circle cx="${cx.toFixed(1)}" cy="${cy.toFixed(1)}" r="${r.toFixed(1)}" ${attrStr} />`);
    } else if (el.type === "rect") {
      const x = typeof el.attributes.x === "number" ? el.attributes.x : 0;
      const y = typeof el.attributes.y === "number" ? el.attributes.y : 0;
      const w = typeof el.attributes.width === "number" ? el.attributes.width : 20;
      const h = typeof el.attributes.height === "number" ? el.attributes.height : 20;
      lines.push(`  <rect x="${x.toFixed(1)}" y="${y.toFixed(1)}" width="${w.toFixed(1)}" height="${h.toFixed(1)}" ${attrStr} />`);
    } else if (el.type === "text") {
      lines.push(`  <text ${attrStr}>${el.id}</text>`);
    }
  }

  lines.push("</svg>");
  return lines.join("\n");
}

// ── Demonstration ────────────────────────────────────────────────────────

function demonstrateDataJoin(): void {
  console.log("=== Data Join Demo ===\n");

  const previousData: Datum[] = [
    { id: "A", values: { name: "Region A", revenue: 120 } },
    { id: "B", values: { name: "Region B", revenue: 80 } },
    { id: "C", values: { name: "Region C", revenue: 95 } },
  ];

  const newData: Datum[] = [
    { id: "A", values: { name: "Region A", revenue: 135 } },
    { id: "C", values: { name: "Region C", revenue: 100 } },
    { id: "D", values: { name: "Region D", revenue: 60 } },
    { id: "E", values: { name: "Region E", revenue: 45 } },
  ];

  const result = dataJoin(newData, previousData);

  console.log("Previous data:", previousData.map((d) => d.id).join(", "));
  console.log("New data:     ", newData.map((d) => d.id).join(", "));
  console.log("");
  console.log("ENTER  (add new elements):   ", result.enter.map((d) => `${d.id} (${d.values.name})`).join(", "));
  console.log("UPDATE (modify existing):     ", result.update.map((d) => `${d.id} (${d.values.name})`).join(", "));
  console.log("EXIT   (remove orphans):      ", result.exit.map((d) => `${d.id} (${d.values.name})`).join(", "));
  console.log("");
  console.log("This is the exact pattern D3's .data() + .enter()/.update/.exit() uses.");
  console.log("Only the minimal set of DOM operations is performed.\n");
}

function demonstrateEncodingPipeline(): void {
  console.log("=== Visual Encoding Pipeline Demo ===\n");

  const data: Datum[] = [
    { id: "North", values: { region: "North", revenue: 120, growth: 0.15 } },
    { id: "South", values: { region: "South", revenue: 80, growth: 0.08 } },
    { id: "East", values: { region: "East", revenue: 95, growth: 0.22 } },
    { id: "West", values: { region: "West", revenue: 60, growth: 0.31 } },
  ];

  const encodings: VisualEncoding[] = [
    {
      channel: "x",
      field: "region",
      dataType: "categorical",
      scale: { kind: "band", domain: ["North", "South", "East", "West"], range: [50, 590] },
    },
    {
      channel: "y",
      field: "revenue",
      dataType: "quantitative",
      scale: { kind: "linear", domain: [0, 140], range: [380, 20] },
    },
    {
      channel: "color",
      field: "region",
      dataType: "categorical",
      scale: { kind: "ordinal", domain: ["North", "South", "East", "West"], range: TABLEAU_10 },
    },
    {
      channel: "size",
      field: "growth",
      dataType: "quantitative",
      scale: { kind: "linear", domain: [0, 0.4], range: [3, 12] },
    },
  ];

  const elements = pipeDataToRender(data, encodings);

  console.log("Data → Encoded Elements:");
  for (const el of elements) {
    console.log(`  ${el.id}: x=${(el.attributes.x as number)?.toFixed?.(1) ?? "?"}, ` +
      `y=${(el.attributes.y as number)?.toFixed?.(1) ?? "?"}, ` +
      `fill=${el.attributes.fill}, size=${typeof el.attributes.size === "number" ? el.attributes.size.toFixed(1) : "?"}`);
  }
  console.log("");

  const svg = renderToSvg(elements, 640, 400);
  console.log("Generated SVG (" + svg.length + " bytes):");
  console.log(svg.substring(0, 300) + "...\n");
}

function demonstrateClevelandHierarchy(): void {
  console.log("=== Cleveland's Encoding Hierarchy ===\n");

  const hierarchy: [string, string, number][] = [
    ["Position (common scale)", "Most accurate — encode your most important variable here", 5],
    ["Position (identical scales)", "Very accurate — e.g., scatter plot dots", 5],
    ["Length", "Accurate — bar chart bars, but shorter bars look thinner", 4],
    ["Angle / Slope", "Moderate — pie charts are hard to read for this reason", 3],
    ["Area", "Poor — circles/squares encode area, but perception is non-linear", 2],
    ["Volume", "Very poor — 3D charts distort comparisons", 1],
    ["Color saturation", "Weak — useful for binning but not precision", 1],
    ["Color hue", "Weakest for quantity — great for categories, terrible for numbers", 1],
  ];

  for (const [encoding, implication, stars] of hierarchy) {
    const bar = "★".repeat(stars) + "☆".repeat(5 - stars);
    console.log(`  ${bar}  ${encoding}`);
    console.log(`         ${implication}`);
    console.log("");
  }
}

function demonstrateColorSafety(): void {
  console.log("=== Perceptually Uniform Colormaps ===\n");

  console.log("Viridis at 0%, 25%, 50%, 75%, 100%:");
  for (const t of [0, 0.25, 0.5, 0.75, 1.0]) {
    console.log(`  ${t.toFixed(2)} → ${viridisInterpolate(t)}`);
  }
  console.log("");

  console.log("Why NOT rainbow/jet:");
  console.log("  - Yellow-green band appears as a sharp boundary (luminance spike)");
  console.log("  - Blue-to-cyan gradual change looks like nothing (low luminance contrast)");
  console.log("  - Creates artificial structure where none exists in the data");
  console.log("  - Fails 3 types of colorblindness simulation");
  console.log("");

  console.log("Colorblind-safe rules:");
  console.log("  1. Never use red+green alone (use red+blue or orange+blue)");
  console.log("  2. Vary lightness, not just hue");
  console.log("  3. For quantitative: use viridis, magma, inferno, or plasma");
  console.log("  4. For categorical: use Tableau 10 or ColorBrewer qualitative");
}

// ── Entry Point ──────────────────────────────────────────────────────────

function main(): void {
  console.log("Visualization — D3, deck.gl, Scientific Plotting");
  console.log("Phase 14 — Computer Graphics & Visualization\n");

  demonstrateDataJoin();
  demonstrateEncodingPipeline();
  demonstrateClevelandHierarchy();
  demonstrateColorSafety();

  console.log("\n=== Chart Type Selection Guide ===\n");
  const guide: [string, string][] = [
    ["Categorical comparison", "Bar chart (position on common scale)"],
    ["Distribution of 1 variable", "Histogram (position + length)"],
    ["Relationship of 2 variables", "Scatter plot (position × position)"],
    ["Temporal trend", "Line chart (position over time)"],
    ["Geospatial density", "Heatmap (position + color)"],
    ["3+ variable composition", "Small multiples (facet by category)"],
    ["> 100K points on a map", "deck.gl scatter-plot layer (GPU)"],
    ["Rich interaction + < 10K points", "D3 (DOM-based, full control)"],
  ];
  for (const [scenario, chart] of guide) {
    console.log(`  ${scenario.padEnd(35)} → ${chart}`);
  }
}

main();