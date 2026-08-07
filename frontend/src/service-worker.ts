/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

/// Why this exists at all: chrome will not offer to install a site without a service worker,
/// so this is what makes the app a PWA rather than a bookmark.
///
/// What it deliberately does not do is stand between the app and its data. Only the build's own
/// assets are served from the cache; everything else, `/graphql` above all, goes to the network
/// every time. A cached balance is a wrong balance.

import { build, files, version } from '$service-worker';

const sw = self as unknown as ServiceWorkerGlobalScope;

const CACHE = `moneta-${version}`;
const ASSETS = new Set([...build, ...files]);

sw.addEventListener('install', (event) => {
	// the new build takes over on the next load rather than waiting for every tab to close,
	// which is what would otherwise leave a phone on last week's app for days
	event.waitUntil(
		caches
			.open(CACHE)
			.then((cache) => cache.addAll([...ASSETS]))
			.then(() => sw.skipWaiting())
	);
});

sw.addEventListener('activate', (event) => {
	event.waitUntil(
		(async () => {
			for (const key of await caches.keys()) if (key !== CACHE) await caches.delete(key);
			await sw.clients.claim();
		})()
	);
});

sw.addEventListener('fetch', (event) => {
	if (event.request.method !== 'GET') return;

	const url = new URL(event.request.url);
	if (url.origin !== location.origin || !ASSETS.has(url.pathname)) return;

	event.respondWith(caches.match(event.request).then((cached) => cached ?? fetch(event.request)));
});
