# AGENTS.md

Guidance for AI coding agents (and humans) working in this repository. Keep
changes consistent with the conventions below. For the contributor process and
the translation workflow, see [`CONTRIBUTING.md`](CONTRIBUTING.md) and
[`docs/TRANSLATING.md`](docs/TRANSLATING.md) — this file does not duplicate them.

## What this project is

**Course: Computer Science** — an open-source curriculum of **421 lessons across
20 phases**, plus a **static website** that presents it. There is **no backend,
database, or user accounts**. The site is plain HTML/CSS/vanilla JS (no
framework, no bundler). Some lessons ship runnable code in C, C++, Rust, Go,
Python, Haskell, SQL, RISC-V assembly, and TLA+.

Live site: `https://course-computer-science.riteshrana.engineer` (GitHub Pages).

## Repository layout

```
phases/<NN-phase>/<NN-lesson>/
  docs/en.md          # lesson body (English source; docs/<locale>.md = translations)
  code/               # runnable code / artifacts for the lesson
  quiz.json           # quiz questions + explanations
  outputs/            # reusable artifact the lesson produces (when applicable)
site/                 # the static website
  *.html              # index, catalog, glossary, prereqs (roadmap), lesson
  app.js, header.js, progress.js, style.css
  build.js            # generates site/data.js from README/ROADMAP/glossary
  data.js             # GENERATED — do not hand-edit
  i18n/               # internationalization (see "Internationalization")
scripts/scaffold_course.py   # scaffolds/validates the course, injects README phases
glossary/terms.md     # CS glossary (source for the glossary page)
tests/                # node --test suites
.github/              # workflows, issue/PR templates, CODEOWNERS, dependabot
Cargo.toml            # Rust workspace (7 lesson crates as members)
```

## Setup & commands

No install step is required for the site or tests (Node built-ins only).

```bash
# Scaffold/validate course structure and refresh the README phases block
python3 scripts/scaffold_course.py

# Build the static site data (regenerates site/data.js)
node site/build.js

# Run the test suite (i18n parity + integrity)
node --test

# Rust lesson crates
cargo build --workspace
cargo test --workspace
```

Local preview: serve the repo root over HTTP (e.g. `python3 -m http.server`)
and open `/site/index.html`. The lesson page fetches Markdown by path, so it
needs HTTP (not `file://`).

## Conventions

- **Keep changes focused.** One logical change per branch/PR. Match the style of
  surrounding code; don't reformat unrelated lines.
- **Commits:** Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`,
  `test:`, `ci:`, `refactor:`, `perf:`). Attribution trailers are disabled.
- **Files:** prefer many small files over large ones. Use ASCII in new files
  unless the surrounding file already uses non-ASCII notation.
- **No stub markers** left behind: `TODO`, `scaffold-stub`, `NotImplementedError`,
  `unimplemented!`, `todo!()`.
- **CSS:** animate compositor-friendly properties; design tokens live at the top
  of `site/style.css` (`--ink`, `--blueprint`, `--bg`, …) — reuse them, don't
  hardcode the palette.

## Lesson authoring standard

Use [`LESSON_TEMPLATE.md`](LESSON_TEMPLATE.md) as the canonical structure. Every
lesson is readable on its own, builds a concrete artifact, and fits its phase.
A lesson includes `docs/en.md` (standard sections), runnable `code/` when it
requires implementation, `quiz.json` with an explanation per answer, and
`outputs/` when it ships an artifact. If a change affects curriculum structure,
update `README.md`, `ROADMAP.md`, and the phase `README.md` **together** — the
README phases block between the `AUTO-GENERATED PHASES` markers is produced by
`scripts/scaffold_course.py` and parsed by `site/build.js`; keep both in sync.

## Internationalization (read before touching i18n)

The site ships **16 UI locales** (English + 15, incl. RTL Arabic/Hebrew). The
single source of truth for locales is
[`site/i18n/languages.js`](site/i18n/languages.js).

- UI strings live in `site/i18n/locales/<code>.json`, keyed identically to the
  canonical [`en.json`](site/i18n/locales/en.json). **Every locale must have the
  exact same key set** — `node --test` enforces parity and will fail CI otherwise.
- Preserve placeholders (`{n}`, `{command}`) and any HTML tags inside values.
- **Never put a raw `"` inside a value** — use the language's typographic quotes
  (this broke several files during the initial import).
- Markup is tagged with `data-i18n` / `data-i18n-attr` / `data-i18n-html`;
  JS-rendered strings use `t('key')` and must wait for `I18N.ready`.
- Lessons translate per file: add `docs/<locale>.md` next to `docs/en.md`. The
  lesson page loads it when present and falls back to English with a notice — no
  registration or build step.

## Testing

`node --test` runs the suites in `tests/`. The i18n suite checks valid JSON,
exact key parity with English, placeholder preservation, and correct RTL flags.
CI runs `node --test` **and** `node site/build.js` on every PR. Add tests for new
behavior; model them on `tests/i18n.test.cjs` (CommonJS, `.cjs` extension — the
test files must stay CommonJS).

## CI / deployment

- **CI** (`.github/workflows/ci.yml`): tests + site build on every push/PR.
  `main` is protected and requires the `test` check to be green before merge.
- **Pages** (`.github/workflows/pages.yml`): on push to `main`, runs the build
  and deploys `site/` to GitHub Pages on the custom domain. `site/CNAME` pins the
  domain; don't remove it.

## Pull requests

Open PRs through the templates; fill the checklist in
[`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md). State what
changed and how you tested it (real command output, not "it builds"). Link the
issue with `Closes #N`.

## Agent do / don't

- **Do** run `node --test` and `node site/build.js` before claiming success.
- **Do** keep `README.md`, `ROADMAP.md`, and phase READMEs in sync for structural
  changes.
- **Don't** hand-edit generated files (`site/data.js`); regenerate them.
- **Don't** add a locale key to one file without adding it to **all** locale files.
- **Don't** introduce dependencies for the site or tests — they intentionally use
  only platform/Node built-ins.
- **Don't** reintroduce references to the previous Vercel deployment as the
  canonical URL; the production host is the GitHub Pages custom domain.
