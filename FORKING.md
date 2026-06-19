# Forking

You may fork this course for a team, school, reading group, or self-study cohort. Keep the license and attribution intact, then adapt the pacing and assignments to your audience.

## What to Keep

- Keep `LICENSE` and the original attribution in `README.md`.
- Keep links to the upstream project unless your fork intentionally becomes a separate curriculum.
- Keep the lesson folder shape from `LESSON_TEMPLATE.md` so future upstream changes are easy to compare.
- Keep `ROADMAP.md` as the progress source of truth.

## What to Change

Common fork-specific changes include:

- Add local deadlines, office hours, or cohort notes in a separate folder.
- Mark selected phases as required or optional for your group.
- Add local tooling instructions for school machines or lab environments.
- Add assignments that wrap existing lessons instead of rewriting the lesson body.

Avoid editing generated site output directly. Update Markdown sources first and rebuild with:

```bash
node site/build.js
```

## Staying Close to Upstream

For long-running forks:

1. Keep local additions in clearly named folders or commits.
2. Pull upstream regularly.
3. Resolve conflicts in source files, not generated output.
4. Rebuild the site after curriculum or glossary changes.

## Publishing a Fork

If your fork is public, make clear what changed from upstream. A short section in your fork's `README.md` is enough:

- Who the fork is for.
- Which phases are included.
- Any changed prerequisites, deadlines, or tooling.
- How learners should report issues.
