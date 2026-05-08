// static/main.legacy.js

/**
 * Global Initialization (ES5 compatible for legacy browsers)
 */
(function initGlobal() {
    // 1. Initial Theme Setup
    var root = document.documentElement;
    var theme = root.getAttribute('data-theme');
    var hasMatchMedia = !!window.matchMedia;
    var resolveTheme = function() {
        if (theme === 'light') {
            return 'light';
        }
        if (theme === 'dark') {
            return 'dark';
        }

        return hasMatchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    };

    var applyTheme = function(resolvedTheme) {
        if (resolvedTheme === 'dark') {
            root.classList.add('dark');
            root.classList.remove('light');
        } else {
            root.classList.add('light');
            root.classList.remove('dark');
        }
    };

    applyTheme(resolveTheme());

    // 2. Service Worker Registration
    if ("serviceWorker" in navigator) {
        navigator.serviceWorker.register("/static/sw.js");
    }

    // 3. System Theme Change Listener (Legacy compatibility)
    if (theme === 'auto' && window.matchMedia) {
        var mq = window.matchMedia('(prefers-color-scheme: dark)');
        if (mq.addListener) {
            mq.addListener(function(e) {
                applyTheme(e.matches ? 'dark' : 'light');
            });
        }
    }
})();

// Note: Dynamic imports (import()) are not supported here. Page-specific logic remains progressive enhancement.