/**
 * Supported UI locales for the site.
 *
 * Each entry: { code, native, english, dir }
 *  - code:    BCP-47-ish locale code, also the locale JSON filename (i18n/locales/<code>.json)
 *  - native:  endonym shown in the language switcher
 *  - english: English name (for aria/tooling)
 *  - dir:     'ltr' | 'rtl'
 *
 * Works in the browser (sets window.I18N_LANGUAGES) and in Node (module.exports),
 * so the test suite can validate the locale files against this single source of truth.
 */
(function (root, factory) {
  var langs = factory();
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = langs;
  }
  if (typeof window !== 'undefined') {
    window.I18N_LANGUAGES = langs;
  }
})(typeof self !== 'undefined' ? self : this, function () {
  return [
    { code: 'en',      native: 'English',             english: 'English',                dir: 'ltr' },
    { code: 'es',      native: 'Español',             english: 'Spanish',                dir: 'ltr' },
    { code: 'fr',      native: 'Français',            english: 'French',                 dir: 'ltr' },
    { code: 'de',      native: 'Deutsch',             english: 'German',                 dir: 'ltr' },
    { code: 'pt-BR',   native: 'Português (Brasil)',  english: 'Portuguese (Brazil)',    dir: 'ltr' },
    { code: 'it',      native: 'Italiano',            english: 'Italian',                dir: 'ltr' },
    { code: 'ru',      native: 'Русский',             english: 'Russian',                dir: 'ltr' },
    { code: 'zh-Hans', native: '简体中文',             english: 'Chinese (Simplified)',   dir: 'ltr' },
    { code: 'ja',      native: '日本語',               english: 'Japanese',               dir: 'ltr' },
    { code: 'ko',      native: '한국어',               english: 'Korean',                 dir: 'ltr' },
    { code: 'hi',      native: 'हिन्दी',                english: 'Hindi',                  dir: 'ltr' },
    { code: 'bn',      native: 'বাংলা',                english: 'Bengali',                dir: 'ltr' },
    { code: 'ar',      native: 'العربية',             english: 'Arabic',                 dir: 'rtl' },
    { code: 'he',      native: 'עברית',               english: 'Hebrew',                 dir: 'rtl' },
    { code: 'id',      native: 'Bahasa Indonesia',    english: 'Indonesian',             dir: 'ltr' },
    { code: 'tr',      native: 'Türkçe',              english: 'Turkish',                dir: 'ltr' }
  ];
});
