# Outputs — Code Review Practice

The shippable artifact for this lesson is:

**`review_checklist.md`** — A practical code review checklist that teams can adopt directly. It covers:

- Pre-review, correctness, design, security, performance, testing, and documentation checks
- Comment label conventions ([blocking], [nit], [question], [suggestion])
- Size guidelines mapping change size to expected review depth and time
- Anti-pattern reference table with quick fixes

### How to Use

1. Copy `review_checklist.md` into your repo's `.github/` directory or your team's engineering wiki
2. Integrate the checklist into your PR template (GitHub supports PR templates in `.github/PULL_REQUEST_TEMPLATE.md`)
3. Customize the checklist items to match your stack and team's most common failure modes
4. Keep it under 30 items — a checklist that's too long won't be used