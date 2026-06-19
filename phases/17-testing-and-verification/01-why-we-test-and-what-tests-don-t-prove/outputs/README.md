# Output Artifact: Testing Confidence Map Template

This lesson ships a reusable artifact pattern you can copy into any feature
folder or ADR package.

## Template Sections

1. Risk register (top failure modes)
2. Claim definitions (falsifiable statements)
3. Claim-to-evidence matrix
4. Assumption ledger
5. Stop-ship gates
6. Production verification metrics

## How to Use

- Create a `confidence-map.md` for each high-risk feature.
- Reference claim IDs in test names and incident reports.
- Update the map when assumptions or architecture change.

## Why It Matters

The template prevents "tests passed" from becoming an empty signal. It enforces
traceability between business risk, technical claims, automated checks, and
runtime observability.
