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

/// `version` is a build timestamp, so the keys sort oldest-first as strings.
const PREFIX = 'moneta-';
const CACHE = `${PREFIX}${version}`;
const ASSETS = new Set([...build, ...files]);

/// Content-hashed, so a hit from an older build's cache is the file it asked for and not a
/// stale one. This is what a page from before the last deploy is served from.
const IMMUTABLE = '/_app/immutable/';

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
			// The build before this one is kept. A tab that was already open is still running it
			// and still asks for its chunks -- which the server stopped serving the moment the
			// image was replaced, so the cache is the only place left that has them. Deleting
			// them here on every deploy is what turned an open tab into a dead one.
			const keys = await caches.keys();
			const keep = new Set([
				CACHE,
				...keys
					.filter((k) => k.startsWith(PREFIX))
					.sort()
					.slice(-2)
			]);

			for (const key of keys) if (!keep.has(key)) await caches.delete(key);
			await sw.clients.claim();
		})()
	);
});

sw.addEventListener('fetch', (event) => {
	if (event.request.method !== 'GET') return;

	const url = new URL(event.request.url);
	if (url.origin !== location.origin) return;
	if (!ASSETS.has(url.pathname) && !url.pathname.startsWith(IMMUTABLE)) return;

	// This build's cache first, and only then the generation behind it -- and that one only for
	// content-hashed paths. Everything under `files` keeps its name between builds, so an older
	// cache answering for the favicon or the manifest would be answering with a stale one; an
	// older cache answering for a hashed chunk is answering with the only copy left anywhere.
	event.respondWith(
		caches
			.open(CACHE)
			.then((cache) => cache.match(event.request))
			.then(
				(hit) =>
					hit ?? (url.pathname.startsWith(IMMUTABLE) ? caches.match(event.request) : undefined)
			)
			.then((cached) => cached ?? fetch(event.request))
	);
});
