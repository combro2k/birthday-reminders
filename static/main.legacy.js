// static/main.legacy.js

/**
 * Global Initialization (ES5 compatible for legacy browsers)
 */
(function initGlobal() {
    // 1. Initial Theme Setup (Prevent FOUC)
    var theme = document.documentElement.getAttribute('data-theme');
    var isDark = theme === 'dark' || (theme === 'auto' && window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
    
    if (isDark) {
        document.documentElement.classList.add('dark');
    } else {
        document.documentElement.classList.remove('dark');
    }

    // 2. Service Worker Registration
    if ("serviceWorker" in navigator) {
        navigator.serviceWorker.register("/static/sw.js");
    }

    // 3. System Theme Change Listener (Legacy compatibility)
    if (theme === 'auto' && window.matchMedia) {
        var mq = window.matchMedia('(prefers-color-scheme: dark)');
        if (mq.addListener) {
            mq.addListener(function(e) {
                document.documentElement.classList.toggle('dark', e.matches);
            });
        }
    }
})();

// Note: Dynamic imports (import()) are not supported here. Page-specific logic remains progressive enhancement.