const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const SITE = path.join(__dirname, '..', 'site');
const ORIGIN = 'https://course-computer-science.riteshrana.engineer';
const IMAGE = `${ORIGIN}/og-image-1200x630.png`;

for (const page of ['index.html', 'catalog.html', 'glossary.html', 'prereqs.html', 'lesson.html']) {
  test(`${page} shares the raster preview image`, () => {
    const html = fs.readFileSync(path.join(SITE, page), 'utf8');
    assert.match(html, /<meta property="og:title" content="[^"]+">/);
    assert.ok(html.includes(`<meta property="og:image" content="${IMAGE}">`));
    assert.ok(html.includes(`<meta name="twitter:image" content="${IMAGE}">`));
  });
}

for (const [page, pathname] of [
  ['index.html', '/'],
  ['catalog.html', '/catalog.html'],
  ['glossary.html', '/glossary.html'],
  ['prereqs.html', '/prereqs.html'],
]) {
  test(`${page} uses its absolute canonical URL`, () => {
    const html = fs.readFileSync(path.join(SITE, page), 'utf8');
    const expected = `${ORIGIN}${pathname}`;
    assert.ok(html.includes(`<link rel="canonical" href="${expected}">`));
    assert.ok(html.includes(`<meta property="og:url" content="${expected}">`));
  });
}

test('social preview is a 1200 by 630 PNG', () => {
  const png = fs.readFileSync(path.join(SITE, 'og-image-1200x630.png'));
  assert.equal(png.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  assert.equal(png.readUInt32BE(16), 1200);
  assert.equal(png.readUInt32BE(20), 630);
});
