/**
 * i18n integrity tests — run with: node --test
 *
 * Guarantees, for every supported locale:
 *   - the locale JSON file exists and parses,
 *   - it has exactly the same key set as the canonical English file
 *     (no missing keys, no extra keys),
 *   - values are non-empty strings,
 *   - {placeholder} tokens are preserved from the source,
 *   - HTML-bearing values keep their tags.
 * Also checks the registry (languages.js) and the locale files agree.
 */
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.join(__dirname, '..');
const LOCALES_DIR = path.join(ROOT, 'site', 'i18n', 'locales');
const LANGUAGES = require(path.join(ROOT, 'site', 'i18n', 'languages.js'));

function readLocale(code) {
  const file = path.join(LOCALES_DIR, code + '.json');
  const raw = fs.readFileSync(file, 'utf8');
  return JSON.parse(raw); // throws on invalid JSON → test fails
}

const en = readLocale('en');
const enKeys = Object.keys(en).sort();
const PLACEHOLDER = /\{(\w+)\}/g;

function placeholders(str) {
  return (String(str).match(PLACEHOLDER) || []).sort();
}

test('registry includes English and has unique, well-formed entries', () => {
  const codes = LANGUAGES.map((l) => l.code);
  assert.ok(codes.includes('en'), 'registry must include en');
  assert.equal(new Set(codes).size, codes.length, 'locale codes must be unique');
  assert.ok(LANGUAGES.length >= 16, `expected >=16 locales, got ${LANGUAGES.length}`);
  for (const l of LANGUAGES) {
    assert.ok(l.code && l.native && l.english, `entry missing fields: ${JSON.stringify(l)}`);
    assert.ok(['ltr', 'rtl'].includes(l.dir), `bad dir for ${l.code}: ${l.dir}`);
  }
});

test('RTL languages are flagged correctly', () => {
  const rtl = LANGUAGES.filter((l) => l.dir === 'rtl').map((l) => l.code).sort();
  assert.deepEqual(rtl, ['ar', 'he'], 'exactly Arabic and Hebrew should be RTL');
});

test('every registry locale has a JSON file and vice versa', () => {
  const files = fs
    .readdirSync(LOCALES_DIR)
    .filter((f) => f.endsWith('.json'))
    .map((f) => f.replace(/\.json$/, ''))
    .sort();
  const codes = LANGUAGES.map((l) => l.code).sort();
  assert.deepEqual(files, codes, 'locale files and registry codes must match exactly');
});

for (const { code } of LANGUAGES) {
  test(`locale "${code}" matches the English key set exactly`, () => {
    const dict = readLocale(code);
    const keys = Object.keys(dict).sort();
    const missing = enKeys.filter((k) => !keys.includes(k));
    const extra = keys.filter((k) => !enKeys.includes(k));
    assert.deepEqual(missing, [], `${code} is missing keys: ${missing.join(', ')}`);
    assert.deepEqual(extra, [], `${code} has unexpected keys: ${extra.join(', ')}`);
  });

  test(`locale "${code}" has non-empty string values and preserves placeholders`, () => {
    const dict = readLocale(code);
    for (const k of enKeys) {
      const v = dict[k];
      assert.equal(typeof v, 'string', `${code}.${k} must be a string`);
      assert.ok(v.trim().length > 0, `${code}.${k} must not be empty`);
      assert.deepEqual(
        placeholders(v),
        placeholders(en[k]),
        `${code}.${k} must preserve placeholders from English`
      );
    }
  });
}

test('HTML-bearing keys keep their tags in every locale', () => {
  const htmlKeys = enKeys.filter((k) => /<[a-z]/i.test(en[k]));
  for (const { code } of LANGUAGES) {
    const dict = readLocale(code);
    for (const k of htmlKeys) {
      assert.ok(/<em>.*<\/em>/.test(dict[k]), `${code}.${k} should retain <em> tags`);
    }
  }
});
