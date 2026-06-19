# Architecture Decision Records — Ready-to-Use Template

This directory contains a ready-to-use ADR template for teams adopting Architecture Decision Records.

## How to Use

1. Create a `docs/adr/` directory in your project repository.
2. Copy the template (`adr_template.md`) into that directory as your first ADR.
3. Number each ADR sequentially: `0001-`, `0002-`, `0003-`, etc.
4. Use this template for every new architectural decision.
5. Never delete or modify an accepted ADR — supersede it with a new one.

## Files

| File | Description |
|------|-------------|
| `adr_template.md` | The blank ADR template — copy this for each new decision |

## ADR Workflow

```
1. PROPOSE   → Author writes ADR with status "Proposed" and submits via PR
2. DISCUSS   → Stakeholders review, question, and debate in PR comments
3. ACCEPT    → Change status to "Accepted"; merge the PR
4. SUPERSEDE → Write a new ADR that supersedes the old one; update old ADR status
```

## Tips

- **Write ADRs during the decision process**, not after the fact.
- **Be specific in Context**: include constraints, alternatives, and team capabilities.
- **Be honest in Consequences**: every architectural choice has downsides — document them.
- **Link ADRs to code**: reference ADR numbers in commit messages (`"Implements ADR-0005"`).
- **Keep ADRs short**: 1-2 pages. If it's longer, the decision is under-specified.
- **Don't over-document**: only write ADRs for decisions that are expensive to reverse.

## Tools

- **adr-tools**: https://github.com/npryce/adr-tools — CLI for managing ADR files
- **log4brains**: https://github.com/thomvaill/log4brains — Web UI for browsing ADRs