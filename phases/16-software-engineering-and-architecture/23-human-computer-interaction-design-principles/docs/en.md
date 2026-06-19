# Human-Computer Interaction Design Principles

> The best algorithm in the world is useless if nobody can figure out how to use it. HCI is the bridge between computation and human cognition.

**Type:** Learn
**Languages:** TypeScript, Markdown
**Prerequisites:** Phase 16 Lessons 01-06
**Time:** ~75 minutes

## Learning Objectives

- Apply Fitts's Law, Hick's Law, and Miller's Law to predict user interaction costs.
- Design interfaces using Nielsen's 10 usability heuristics and identify violations in existing software.
- Conduct a basic usability evaluation (heuristic evaluation, think-aloud protocol).
- Explain the cognitive dimensions of notations and how they apply to API and language design.
- Connect HCI principles to CS: API design, error messages, CLI design, documentation.

## The Problem

Every piece of software has a user — even internal tools, CLIs, and APIs. Bad interface design costs real time and money:

- A confusing CLI causes developers to read man pages for 10 minutes to run a 10-second command
- A misleading error message sends someone debugging the wrong thing for hours
- An API with inconsistent naming forces users to constantly check documentation
- A visualization that violates perception principles leads to wrong conclusions

HCI isn't just about graphical interfaces. It's about how humans perceive, think, and act — and how to design systems that work with human cognition instead of against it.

## The Concept

### Fitts's Law

Time to hit a target is a function of distance and size:

```
T = a + b · log₂(D/W + 1)

T = movement time
D = distance to target
W = width of target
a, b = empirical constants
```

**Implications:**
- Make important buttons large and close to likely cursor positions
- Screen corners and edges are "infinitely large" targets (cursor stops there)
- Right-click context menus appear at cursor position — zero distance
- Touch interfaces need larger targets than mouse interfaces (finger is less precise)

### Hick's Law

Decision time increases logarithmically with the number of choices:

```
T = b · log₂(n + 1)

n = number of choices
```

**Implications:**
- Limit menu items to 7±2 (Miller's Law — working memory capacity)
- Use progressive disclosure: show common options first, advanced options behind a toggle
- Group related options to reduce perceived complexity
- Command palettes (Ctrl+P) beat nested menus for expert users

### Miller's Law

Working memory holds 7±2 chunks of information.

**Implications:**
- Break complex forms into steps (wizards)
- Use chunking: phone numbers are 555-867-5309, not 5558675309
- Limit visible options in menus and toolbars
- Use visual hierarchy to guide attention

### Nielsen's 10 Usability Heuristics

| # | Heuristic | Example Violation |
|---|-----------|-------------------|
| 1 | Visibility of system status | No progress bar on file upload |
| 2 | Match between system and real world | "Error 0x80070005" instead of "Permission denied" |
| 3 | User control and freedom | No undo for destructive actions |
| 4 | Consistency and standards | "Save" in one place, "Commit" in another for the same action |
| 5 | Error prevention | Delete button next to Save with no confirmation |
| 6 | Recognition over recall | Recent files list instead of requiring exact path |
| 7 | Flexibility and efficiency | No keyboard shortcuts for common actions |
| 8 | Aesthetic and minimalist design | Cluttered dialog with 20 options when 3 are commonly needed |
| 9 | Help users recognize errors | Red text with no explanation of what went wrong |
| 10 | Help and documentation | Man page with no examples |

### Cognitive Dimensions of Notations

For API and language design:

| Dimension | Question | Example |
|-----------|----------|---------|
| Viscosity | How hard is it to change? | Renaming a function in C requires changing all call sites |
| Premature commitment | Must you decide before you're ready? | C requires declaring variables before use; Rust infers |
| Hidden dependencies | Is the connection between parts visible? | Global variables create invisible coupling |
| Role-expressiveness | Can you see what each part does? | `x = f(a, b, c)` vs `result = compute_total(items, tax_rate, discount)` |
| Consistency | Do similar things look similar? | Python's `len()` vs `str.count()` — both measure, different syntax |

### Connection to CS

| CS Application | HCI Principle |
|----------------|---------------|
| API Design | Consistency, role-expressiveness, recognition over recall |
| Error Messages | Match real world, help recognize errors, help and documentation |
| CLI Design | Fitts's Law (short commands for common actions), Hick's Law (limit options) |
| Documentation | Progressive disclosure, examples over specifications |
| Data Visualization | Gestalt principles, pre-attentive processing, color perception |
| Programming Languages | Cognitive dimensions, learnability vs. efficiency tradeoff |

## Build It

### Step 1: Fitts's Law Calculator

```typescript
function fittsTime(distance: number, width: number, a = 0.05, b = 0.15): number {
  // T = a + b * log2(D/W + 1)
  return a + b * Math.log2(distance / width + 1);
}

// Button at 500px distance, 100px wide vs 50px wide
console.log(`Wide button: ${fittsTime(500, 100).toFixed(3)}s`);
console.log(`Narrow button: ${fittsTime(500, 50).toFixed(3)}s`);
// Wide: ~0.39s, Narrow: ~0.46s — 18% slower for half the width
```

### Step 2: Heuristic Evaluation Checklist

```typescript
interface HeuristicViolation {
  heuristic: number;      // 1-10 from Nielsen's list
  description: string;
  severity: 'cosmetic' | 'minor' | 'major' | 'catastrophic';
  location: string;
}

function evaluateInterface(violations: HeuristicViolation[]): void {
  const bySeverity = violations.reduce((acc, v) => {
    acc[v.severity] = (acc[v.severity] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  console.log('Heuristic Evaluation Summary:');
  console.log(`  Catastrophic: ${bySeverity.catastrophic || 0}`);
  console.log(`  Major: ${bySeverity.major || 0}`);
  console.log(`  Minor: ${bySeverity.minor || 0}`);
  console.log(`  Cosmetic: ${bySeverity.cosmetic || 0}`);

  const score = 10 - (bySeverity.catastrophic || 0) * 3
                    - (bySeverity.major || 0) * 2
                    - (bySeverity.minor || 0) * 0.5;
  console.log(`  Estimated usability score: ${Math.max(0, score).toFixed(1)}/10`);
}

// Example evaluation
const violations: HeuristicViolation[] = [
  { heuristic: 1, description: 'No loading indicator', severity: 'major', location: 'File upload' },
  { heuristic: 9, description: 'Generic "Error occurred" message', severity: 'major', location: 'Form submit' },
  { heuristic: 3, description: 'No undo for delete', severity: 'catastrophic', location: 'Item list' },
];
evaluateInterface(violations);
```

### Step 3: Progressive Disclosure Pattern

```typescript
interface MenuItem {
  label: string;
  shortcut?: string;
  frequency: 'common' | 'occasional' | 'rare';
  action: () => void;
}

function renderMenu(items: MenuItem[], showAdvanced: boolean): string {
  const visible = showAdvanced
    ? items
    : items.filter(i => i.frequency === 'common');

  return visible.map(i => {
    const shortcut = i.shortcut ? ` (${i.shortcut})` : '';
    return `  ${i.label}${shortcut}`;
  }).join('\n');
}

const menu: MenuItem[] = [
  { label: 'Save', shortcut: 'Ctrl+S', frequency: 'common', action: () => {} },
  { label: 'Save As...', shortcut: 'Ctrl+Shift+S', frequency: 'occasional', action: () => {} },
  { label: 'Export as PDF', frequency: 'rare', action: () => {} },
  { label: 'Print', shortcut: 'Ctrl+P', frequency: 'occasional', action: () => {} },
];

console.log('Basic menu:');
console.log(renderMenu(menu, false));
console.log('\nAdvanced menu:');
console.log(renderMenu(menu, true));
```

## Use It

HCI principles are embedded in the design of:

- **VS Code** — command palette (Ctrl+P) applies Hick's Law; progressive disclosure in settings
- **Git** — common commands are short (`git st`, `git co`), rare ones are longer (`git cherry-pick`)
- **Rust error messages** — "match real world" heuristic: errors explain what went wrong and suggest fixes
- **Stripe API** — consistency heuristic: all resources follow the same CRUD pattern
- **Unix CLI** — pipes and filters apply "visibility of system status" (output flows through pipeline)

## Read the Source

- [Don Norman: The Design of Everyday Things](https://www.amazon.com/Design-Everyday-Things-Donald-Norman/dp/0465050654) — foundational HCI text
- [Steve Krug: Don't Make Me Think](https://www.amazon.com/Dont-Make-Think-Revisited-Usability/dp/0321965515) — web usability
- [Nielsen Norman Group](https://www.nngroup.com/articles/) — research-based UX articles

## Ship It

- `code/main.ts`: Fitts's Law calculator, heuristic evaluation tool, progressive disclosure demo
- `outputs/README.md`: HCI principles quick reference

## Exercises

1. **Easy:** Evaluate a CLI tool you use daily against Nielsen's 10 heuristics. List 3 violations.
2. **Medium:** Redesign an error message in your codebase to follow "match real world" and "help recognize errors" heuristics.
3. **Hard:** Design a command palette (like VS Code's Ctrl+P) that applies Fitts's Law, Hick's Law, and progressive disclosure. Implement a prototype.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Fitts's Law | "Bigger buttons are easier to click" | T = a + b·log₂(D/W + 1); movement time depends on distance-to-width ratio |
| Hick's Law | "Too many choices slow you down" | Decision time grows logarithmically with number of options |
| Miller's Law | "7 plus or minus 2" | Working memory capacity; chunk information into groups of 5-9 |
| Affordance | "It looks clickable" | Perceived action possibilities of an object; a raised button affords pressing |
| Progressive disclosure | "Show less, reveal more" | Show common options first; advanced options behind a toggle or gesture |

## Further Reading

- [Don Norman: The Design of Everyday Things](https://www.amazon.com/Design-Everyday-Things-Donald-Norman/dp/0465050654) — the foundational text
- [Nielsen's 10 Heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/) — original list with examples
- [Cognitive Dimensions of Notations](https://www.cl.cam.ac.uk/~afb21/CognitiveDimensions/) — framework for API/language design
