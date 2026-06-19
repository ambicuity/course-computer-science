# Threat Modeling Template

This directory contains a reusable threat modeling template produced by Phase 12, Lesson 22.

## Contents

| File | Description |
|------|-------------|
| `../code/notes.md` | Complete reference: STRIDE per-element cheat sheet, DREAD scoring sheet template, ASCII attack tree template, DFD notation guide, completed worksheet for a note-taking web app, trust boundary checklist, security requirements checklist, and glossary |

## Usage

1. **Draw your DFD** — Identify external entities, processes, data stores, and data flows. Draw trust boundaries at each security perimeter. Use the DFD notation reference in `notes.md`.
2. **Apply STRIDE per element** — For each DFD element, check the STRIDE per-element matrix and list specific threats. Use the completed worksheet as a template.
3. **Build attack trees** — Pick the highest-priority threats and build attack trees using the ASCII template. Mark AND/OR nodes, leaf conditions, and prune infeasible branches.
4. **Score with DREAD** — Score each threat path using the DREAD sheet. Produce a priority-ordered action plan.

## References

- STRIDE per-element matrix (from Microsoft SDL)
- DREAD scoring guide with 1–3 rubric per category
- Attack tree template with AND/OR semantics
- DFD notation reference (Yourdon/DeMarco style adapted for threat modeling)
- Trust boundary checklist (12 common boundaries)
- Security requirements checklist (organized by STRIDE category)
