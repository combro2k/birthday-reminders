const APP_CACHE = 'birthday-reminders-app-v4';
const CDN_CACHE = 'birthday-reminders-cdn-v4';
const API_CACHE = 'birthday-reminders-api-v4';

const SHELL_URLS = [
    '/',
    '/offline',
    '/static/tailwind.css',
    '/static/manifest.json',
    '/static/icon-192.png',
    '/static/icon-512.png',
    '/static/icon-maskable-192.png',
    '/static/icon-maskable-512.png',
    '/static/icon-180.png'
];

function isStaticAsset(request) {
    return request.url.includes('/static/');
}

function isApiRequest(request) {
    return request.method !== 'GET' ||
        request.url.includes('/birthdays') ||
        request.url.includes('/notifications') ||
        request.url.includes('/settings') ||
        request.url.includes('/admin');
}

async function networkFirst(request, cacheName, fallbackUrl) {
    try {
        const response = await fetch(request);
        const cache = await caches.open(cacheName);
        cache.put(request, response.clone());
        return response;
    } catch {
        const cached = await caches.match(request);
        if (cached) return cached;
        if (fallbackUrl) {
            const fallback = await caches.match(fallbackUrl);
            if (fallback) return fallback;
        }
        return new Response('Offline', { status: 503, statusText: 'Offline' });
    }
}

async function cacheFirst(request, cacheName) {
    const cached = await caches.match(request);
    if (cached) return cached;

    const response = await fetch(request);
    const cache = await caches.open(cacheName);
    cache.put(request, response.clone());
    return response;
}

async function staleWhileRevalidate(request, cacheName) {
    const cache = await caches.open(cacheName);
    const cachedResponse = await cache.match(request);

    const fetchPromise = fetch(request).then((networkResponse) => {
        if (networkResponse.ok) {
            cache.put(request, networkResponse.clone());
        }
        return networkResponse;
    }).catch(() => { /* Network failure is ignored if we have cache */ });

    return cachedResponse || fetchPromise;
}

self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(APP_CACHE).then((cache) => cache.addAll(SHELL_URLS))
    );
    self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    const activeCaches = [APP_CACHE, CDN_CACHE, API_CACHE];
    event.waitUntil(
        caches.keys().then((keys) => Promise.all(
            keys
                .filter((key) => !activeCaches.includes(key))
                .map((key) => caches.delete(key))
        ))
    );
    self.clients.claim();
});

self.addEventListener('fetch', (event) => {
    const { request } = event;

    if (request.method !== 'GET') {
        return;
    }

    const url = new URL(request.url);

    if (url.origin !== self.location.origin) {
        // Cache third-party CDN scripts after first successful network fetch.
        if (url.hostname.includes('unpkg.com')) {
            event.respondWith(networkFirst(request, CDN_CACHE));
        }
        return;
    }

    if (request.mode === 'navigate') {
        event.respondWith(networkFirst(request, API_CACHE, '/offline'));
        return;
    }

    if (isStaticAsset(request)) {
        if (url.pathname.endsWith('.css')) {
            event.respondWith(staleWhileRevalidate(request, APP_CACHE));
        } else {
            event.respondWith(cacheFirst(request, APP_CACHE));
        }
        return;
    }

    if (isApiRequest(request)) {
        event.respondWith(networkFirst(request, API_CACHE));
        return;
    }

    event.respondWith(
        networkFirst(request, API_CACHE)
    );
});
