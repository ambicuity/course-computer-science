# Output: Pipeline Cheatsheet

This directory contains the reusable artifact from Lesson 02: The Graphics Pipeline at 30,000 ft.

## Artifact

**`pipeline_cheatsheet.md`** — A one-page reference card that maps each graphics pipeline stage to its GPU API equivalent in Vulkan, Metal, and WebGPU. Also includes:

- Key data flowing at each stage (input → output → operation)
- Transform quick reference (object → world → view → clip → NDC → screen)
- Perspective-correct interpolation formula
- Blend modes quick reference
- Fixed-function vs. programmable stage classification

## Usage

Keep this cheatsheet open when:
- Setting up a render pipeline in any graphics API
- Debugging which stage is producing incorrect output
- Remembering which stages are programmable vs. fixed-function
- Looking up the API struct name for a specific pipeline stage