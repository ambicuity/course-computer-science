# ADR-NNNN: <one-line decision>

**Date:** YYYY-MM-DD
**Status:** Proposed | Accepted | Superseded by ADR-XXXX | Deprecated
**Deciders:** name, name, name

## Context

<2–4 paragraphs. What is the situation that's forcing a decision? What
constraints exist (technical, organizational, deadline, regulatory)? What
information do we have?>

## Decision

<One paragraph. State the chosen option clearly and concretely. "We will
…" — not "We propose to consider …".>

## Consequences

What changes because of this decision:

- **Positive:** <item>
- **Positive:** <item>
- **Negative:** <item>
- **Neutral / follow-up:** <item>

## Alternatives considered

- **Option A:** <one-line>. Rejected because <reason>.
- **Option B:** <one-line>. Rejected because <reason>.
- **Option C:** <one-line>. Rejected because <reason>.

## Notes / References

- Link to the discussion thread, RFC, or design doc.
- Link to relevant code or data.

---

### How to use this template

1. Copy this file as `docs/adr/<NNNN>-<short-slug>.md`.
2. Fill in the sections. Keep it short — under one screen ideally.
3. PR for review. Once merged with `Status: Accepted`, *don't edit it again*.
4. If a later decision overrides it, write a new ADR and set this one's status to
   `Superseded by [ADR-XXXX](…)`.
5. The chronological list of ADRs in `docs/adr/` is the history of why the system
   looks the way it does.
