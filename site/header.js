/**
 * Shared header behaviors: live GitHub star counter.
 * Loaded by every page that includes the .header-github component.
 */
(function () {
  var REPO = 'ambicuity/course-computer-science';
  var CACHE_KEY = 'gh:stars:' + REPO;
  var CACHE_TTL_MS = 10 * 60 * 1000; // 10 minutes

  function format(n) {
    if (n >= 10000) return (n / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
    if (n >= 1000) return (n / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
    return String(n);
  }

  function paint(n) {
    var els = document.querySelectorAll('.header-github .star-count, #starCount');
    for (var i = 0; i < els.length; i++) {
      els[i].textContent = format(n);
      els[i].removeAttribute('data-loading');
    }
  }

  function paintUnavailable() {
    var els = document.querySelectorAll('.header-github .star-count, #starCount');
    for (var i = 0; i < els.length; i++) {
      els[i].textContent = '★';
      els[i].removeAttribute('data-loading');
    }
  }

  function readCache() {
    try {
      var raw = localStorage.getItem(CACHE_KEY);
      if (!raw) return null;
      var parsed = JSON.parse(raw);
      if (Date.now() - parsed.t > CACHE_TTL_MS) return null;
      return parsed.n;
    } catch (e) {
      return null;
    }
  }

  function writeCache(n) {
    try {
      localStorage.setItem(CACHE_KEY, JSON.stringify({ n: n, t: Date.now() }));
    } catch (e) {
      // localStorage may be disabled
    }
  }

  function load() {
    var cached = readCache();
    if (cached != null) {
      paint(cached);
      return;
    }
    // The star count is decorative. Avoid a network dependency so local previews,
    // private mirrors, and offline readers do not log noisy GitHub API failures.
    paintUnavailable();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', load);
  } else {
    load();
  }
})();
