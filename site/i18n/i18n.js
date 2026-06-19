/**
 * Lightweight, dependency-free i18n runtime for the static site.
 *
 * Responsibilities:
 *  - Resolve the active locale (?lang= → localStorage → navigator → 'en').
 *  - Set <html lang> and <html dir> (RTL for Arabic/Hebrew) as early as possible.
 *  - Load the active locale JSON plus English as a fallback.
 *  - Translate static markup tagged with data-i18n / data-i18n-attr / data-i18n-html.
 *  - Inject an accessible language switcher into the page header.
 *  - Expose window.t(key, vars) and window.I18N.ready for JS-rendered content.
 *
 * Usage in markup:
 *   <a data-i18n="nav.catalog">Catalog</a>
 *   <input data-i18n-attr="placeholder:catalog.searchPlaceholder">
 *   <p data-i18n-html="home.howP1">…</p>   (innerHTML; use only for trusted locale text)
 *
 * Usage in JS (wait for dictionaries, then render):
 *   I18N.ready.then(function () { render(); });   // t() is reliable inside
 */
(function () {
  'use strict';

  var STORAGE_KEY = 'cscs:locale';
  var DEFAULT_LOCALE = 'en';
  var LANGUAGES = (typeof window !== 'undefined' && window.I18N_LANGUAGES) || [];
  var CODES = LANGUAGES.map(function (l) { return l.code; });

  // Resolve the directory this script lives in so locale files load regardless of page path.
  function localesBase() {
    var scripts = document.getElementsByTagName('script');
    for (var i = 0; i < scripts.length; i++) {
      var src = scripts[i].getAttribute('src') || '';
      if (/i18n\.js(\?|$)/.test(src)) {
        return src.replace(/i18n\.js(\?.*)?$/, '') + 'locales/';
      }
    }
    return 'i18n/locales/';
  }

  function langInfo(code) {
    for (var i = 0; i < LANGUAGES.length; i++) {
      if (LANGUAGES[i].code === code) return LANGUAGES[i];
    }
    return null;
  }

  // Map a browser/user code (e.g. "pt", "zh-CN", "iw") to a supported locale.
  function matchSupported(raw) {
    if (!raw) return null;
    var code = String(raw).replace('_', '-');
    if (CODES.indexOf(code) !== -1) return code;

    var lower = code.toLowerCase();
    // Exact case-insensitive
    for (var i = 0; i < CODES.length; i++) {
      if (CODES[i].toLowerCase() === lower) return CODES[i];
    }
    var primary = lower.split('-')[0];
    // Aliases / script preferences
    var ALIAS = { iw: 'he', in: 'id', pt: 'pt-BR', zh: 'zh-Hans' };
    if (ALIAS[primary]) return ALIAS[primary];
    if (primary === 'zh') return 'zh-Hans';
    // Primary-subtag match (e.g. "es-419" → "es")
    for (var j = 0; j < CODES.length; j++) {
      if (CODES[j].toLowerCase().split('-')[0] === primary) return CODES[j];
    }
    return null;
  }

  function detectLocale() {
    try {
      var params = new URLSearchParams(window.location.search);
      var fromUrl = matchSupported(params.get('lang'));
      if (fromUrl) return fromUrl;
    } catch (e) { /* ignore */ }

    try {
      var stored = matchSupported(localStorage.getItem(STORAGE_KEY));
      if (stored) return stored;
    } catch (e) { /* ignore */ }

    var navLangs = (navigator.languages && navigator.languages.length)
      ? navigator.languages
      : [navigator.language || navigator.userLanguage];
    for (var i = 0; i < navLangs.length; i++) {
      var m = matchSupported(navLangs[i]);
      if (m) return m;
    }
    return DEFAULT_LOCALE;
  }

  function applyDir(code) {
    var info = langInfo(code) || { dir: 'ltr' };
    var html = document.documentElement;
    html.setAttribute('lang', code);
    html.setAttribute('dir', info.dir);
  }

  function fetchJSON(url) {
    return fetch(url, { cache: 'no-cache' }).then(function (r) {
      if (!r.ok) throw new Error('Failed to load ' + url + ' (' + r.status + ')');
      return r.json();
    });
  }

  // Simple {placeholder} interpolation.
  function interpolate(str, vars) {
    if (!vars) return str;
    return str.replace(/\{(\w+)\}/g, function (m, k) {
      return (vars[k] !== undefined && vars[k] !== null) ? String(vars[k]) : m;
    });
  }

  var state = {
    locale: DEFAULT_LOCALE,
    dict: {},      // active locale strings
    fallback: {}   // English strings
  };

  function t(key, vars) {
    var val = state.dict[key];
    if (val === undefined) val = state.fallback[key];
    if (val === undefined) return key; // last-resort: surface the key, never throw
    return interpolate(val, vars);
  }

  function translateElement(el) {
    var key = el.getAttribute('data-i18n');
    if (key) {
      var txt = t(key);
      if (txt !== key) el.textContent = txt;
    }
    var htmlKey = el.getAttribute('data-i18n-html');
    if (htmlKey) {
      var html = t(htmlKey);
      if (html !== htmlKey) el.innerHTML = html;
    }
    var attrSpec = el.getAttribute('data-i18n-attr');
    if (attrSpec) {
      attrSpec.split(';').forEach(function (pair) {
        var parts = pair.split(':');
        if (parts.length !== 2) return;
        var attr = parts[0].trim();
        var k = parts[1].trim();
        var v = t(k);
        if (attr && v !== k) el.setAttribute(attr, v);
      });
    }
  }

  function translateTree(root) {
    var scope = root || document;
    var nodes = scope.querySelectorAll('[data-i18n],[data-i18n-attr],[data-i18n-html]');
    for (var i = 0; i < nodes.length; i++) translateElement(nodes[i]);
  }

  function setLocale(code) {
    var next = matchSupported(code) || DEFAULT_LOCALE;
    try { localStorage.setItem(STORAGE_KEY, next); } catch (e) { /* ignore */ }
    // Reload so JS-rendered content (catalog, modal, lesson body) re-renders cleanly.
    var url = new URL(window.location.href);
    url.searchParams.set('lang', next);
    window.location.href = url.toString();
  }

  function buildSwitcher() {
    var nav = document.querySelector('.header-nav');
    if (!nav || nav.querySelector('.lang-switcher')) return;

    var wrap = document.createElement('label');
    wrap.className = 'lang-switcher';
    wrap.setAttribute('aria-label', t('a11y.language'));

    var globe = document.createElement('span');
    globe.className = 'lang-switcher-icon';
    globe.setAttribute('aria-hidden', 'true');
    globe.textContent = '🌐';

    var select = document.createElement('select');
    select.className = 'lang-select';
    select.setAttribute('aria-label', t('a11y.language'));

    LANGUAGES.forEach(function (l) {
      var opt = document.createElement('option');
      opt.value = l.code;
      opt.textContent = l.native;
      if (l.code === state.locale) opt.selected = true;
      select.appendChild(opt);
    });

    select.addEventListener('change', function () { setLocale(select.value); });

    wrap.appendChild(globe);
    wrap.appendChild(select);
    // Place before the GitHub link if present, else append.
    var gh = nav.querySelector('.header-github');
    if (gh) nav.insertBefore(wrap, gh);
    else nav.appendChild(wrap);
  }

  function addHreflangTags() {
    var head = document.head;
    if (!head) return;
    var base = window.location.origin + window.location.pathname;
    LANGUAGES.forEach(function (l) {
      var link = document.createElement('link');
      link.setAttribute('rel', 'alternate');
      link.setAttribute('hreflang', l.code);
      link.setAttribute('href', base + '?lang=' + encodeURIComponent(l.code));
      head.appendChild(link);
    });
    var xdef = document.createElement('link');
    xdef.setAttribute('rel', 'alternate');
    xdef.setAttribute('hreflang', 'x-default');
    xdef.setAttribute('href', base);
    head.appendChild(xdef);
  }

  // ── Bootstrap ──────────────────────────────────────────────────────────
  state.locale = detectLocale();
  applyDir(state.locale); // set lang/dir before paint to limit RTL flash

  var base = localesBase();
  var loads = [fetchJSON(base + 'en.json').then(function (d) { state.fallback = d; })];
  if (state.locale !== DEFAULT_LOCALE) {
    loads.push(
      fetchJSON(base + state.locale + '.json')
        .then(function (d) { state.dict = d; })
        .catch(function () { state.dict = {}; /* fall back to English */ })
    );
  }

  var ready = Promise.all(loads).then(function () {
    if (state.locale === DEFAULT_LOCALE) state.dict = state.fallback;
    function paint() {
      translateTree(document);
      buildSwitcher();
      addHreflangTags();
    }
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', paint);
    } else {
      paint();
    }
    return state.locale;
  });

  // Public API
  window.t = t;
  window.I18N = {
    ready: ready,
    locale: function () { return state.locale; },
    languages: LANGUAGES,
    setLocale: setLocale,
    translate: translateTree,
    t: t
  };
})();
