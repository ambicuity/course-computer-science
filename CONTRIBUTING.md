# Contributing

Thanks for helping improve Course: Computer Science. This repository is a curriculum, so contributions should preserve the course shape: each lesson is readable on its own, builds a concrete artifact, and fits the phase it belongs to.

## What to Contribute

Good contributions include:

- Fixing factual errors, broken links, typos, and unclear explanations.
- Improving a lesson's runnable code or quiz explanations.
- Adding missing glossary references to `glossary/terms.md`.
- Filling an incomplete lesson using `LESSON_TEMPLATE.md`.
- Improving generated site data by updating the source Markdown, then rebuilding the site.

Avoid unrelated rewrites. If a change affects the curriculum structure, update `README.md`, `ROADMAP.md`, and the relevant phase `README.md` together.

## Lesson Standard

Every lesson should include:

- `docs/en.md` with the standard lesson sections.
- Runnable code in `code/` when the lesson requires implementation.
- `quiz.json` with explanations for each answer.
- `outputs/` when the lesson ships a reusable artifact.

Use `LESSON_TEMPLATE.md` as the canonical structure. The lesson should replace placeholders with real content and should not leave `TODO`, `scaffold-stub`, `NotImplementedError`, or `unimplemented!` markers behind.

## Workflow

1. Fork the repository and create a focused branch.
2. Make the smallest coherent change that completes the contribution.
3. Run the relevant checks:

```bash
python3 scripts/scaffold_course.py
node site/build.js
```

4. If you changed Rust workspace code, also run:

```bash
cargo test --workspace
```

5. Open a pull request with a short summary, the files changed, and the checks you ran.

## Style

- Write directly and concretely.
- Prefer runnable examples over abstract description.
- Link terms to `glossary/terms.md` when a concept is reused across lessons.
- Keep code comments for non-obvious constraints and invariants.
- Use ASCII in new files unless the surrounding file already uses non-ASCII notation.

## Required Project Docs

| Goal | Read |
|---|---|
| Contribute a lesson or fix | `CONTRIBUTING.md` |
| Fork for your team or school | `FORKING.md` |
| Lesson template | `LESSON_TEMPLATE.md` |
| Track progress | `ROADMAP.md` |
| Glossary | `glossary/terms.md` |
| Code of conduct | `CODE_OF_CONDUCT.md` |
