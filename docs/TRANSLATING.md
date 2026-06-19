# Translating the course

The site and course support multiple languages. There are two layers:

1. **Website UI** — navigation, buttons, headings, and labels. These live in
   small JSON files and are fully translated for every supported locale.
2. **Lesson content** — the `docs/*.md` body of each lesson. These are English
   by default; translations are added per lesson, per language, and the site
   falls back to English when a translation is missing.

## Supported languages

The locale registry is the single source of truth:
[`site/i18n/languages.js`](../site/i18n/languages.js). It currently ships 16
locales (English plus 15 translations), including right-to-left support for
Arabic and Hebrew.

## How locale resolution works

On every page the active locale is chosen in this order:

1. `?lang=<code>` query parameter (shareable links),
2. the visitor's saved choice (`localStorage`),
3. the browser's `Accept-Language` / `navigator.languages`,
4. English (`en`) as the final fallback.

The language switcher (🌐) in the header lets visitors change it; the choice is
remembered. `<html lang>` and `<html dir>` are set automatically, so RTL
locales render right-to-left.

## Translating the UI

UI strings live in [`site/i18n/locales/`](../site/i18n/locales/), one JSON file
per locale, keyed identically to the canonical English file
[`en.json`](../site/i18n/locales/en.json).

To add or fix a UI translation:

1. Open the locale file (e.g. `fr.json`). Keep **every key** identical to
   `en.json` — translate only the values.
2. Preserve placeholders such as `{n}` and `{command}` exactly.
3. Preserve any HTML tags inside a value (e.g. `<em>…</em>`).
4. Do **not** translate technical tokens or proper nouns: language names
   (C, C++, Rust, Go, Python, Haskell, SQL, TLA+), `RISC-V`, `B-tree`, `Raft`,
   `GitHub`, `MIT`, `ChatGPT`, and `Ritesh Rana`.
5. Never put a raw `"` inside a value — use the language's typographic quotes.

To add a **new** language: add an entry to `site/i18n/languages.js` and create
`site/i18n/locales/<code>.json` with the full key set.

### Marking new UI strings for translation

In HTML, tag an element and add the key to every locale file:

```html
<a data-i18n="nav.catalog">Catalog</a>
<input data-i18n-attr="placeholder:catalog.searchPlaceholder">
<p data-i18n-html="glossary.subtitleHtml">…</p>
```

In JavaScript-rendered markup, call `t('key')` (optionally `t('key', { n: 5 })`).

## Translating a lesson

Each lesson lives at `phases/<phase>/<lesson>/docs/`. The English source is
`en.md`. To translate a lesson into French, add `fr.md` alongside it:

```
phases/03-data-structures/01-arrays-and-slices/docs/
├── en.md        # English source (required)
└── fr.md        # French translation (optional)
```

When a visitor reads a lesson in French, the page loads `fr.md` if present and
otherwise shows `en.md` with a small "not translated yet" notice. No build step
or registration is required — drop the file in and it is picked up.

## Tests

`node --test` validates that every locale file is valid JSON, has the exact
same keys as `en.json`, keeps placeholders, and flags RTL correctly. CI runs
this on every pull request, so a broken or incomplete locale fails the build.
