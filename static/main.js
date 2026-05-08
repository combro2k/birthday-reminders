// static/main.js

/**
 * Global Initialization
 */
(function initGlobal() {
    // 1. Initial Theme Setup
    const root = document.documentElement;
    const theme = document.documentElement.getAttribute('data-theme');
    const resolveTheme = () => {
        if (theme === 'light') {
            return 'light';
        }
        if (theme === 'dark') {
            return 'dark';
        }

        return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    };

    const applyTheme = (resolvedTheme) => {
        root.classList.toggle('dark', resolvedTheme === 'dark');
        root.classList.toggle('light', resolvedTheme === 'light');
    };

    applyTheme(resolveTheme());

    // 2. Service Worker Registration
    if ("serviceWorker" in navigator) {
        navigator.serviceWorker.register("/static/sw.js");
    }

    // 3. System Theme Change Listener
    if (theme === 'auto') {
        window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
            applyTheme(e.matches ? 'dark' : 'light');
        });
    }
})();

document.addEventListener('DOMContentLoaded', () => {
    const body = document.body;
    if (!body) {
        console.warn('No body element found.');
        return;
    }

    // Get the page identifier from the body's ID
    const pageId = body.id;

    // Dynamically import and initialize domain-specific scripts
    switch (pageId) {
        case 'page-birthdays-list':
            import('./birthdays/list.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading birthdays list script:', error));
            break;
        case 'page-birthdays-edit':
            import('./birthdays/edit.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading birthdays edit script:', error));
            break;
        case 'page-channels-list':
            import('./channels/list.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading channels list script:', error));
            break;
        case 'page-channels-edit':
            import('./channels/edit.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading channels edit script:', error));
            break;
        case 'page-users-profile':
            import('./users/profile.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading users profile script:', error));
            break;
        case 'page-users-settings':
            import('./users/settings.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading users settings script:', error));
            break;
        case 'page-home-dashboard':
            import('./home/dashboard.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading dashboard script:', error));
            break;
        case 'page-auth-login':
            import('./auth/login.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading login script:', error));
            break;
        case 'page-offline-index':
            import('./offline/index.js')
                .then(module => module.init())
                .catch(error => console.error('Error loading offline script:', error));
            break;
    }
});