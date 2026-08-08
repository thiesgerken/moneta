<script lang="ts">
	import '../app.css';
	import Nav from '$lib/Nav.svelte';
	import { graphql } from '$houdini';
	import { beforeNavigate } from '$app/navigation';
	import { page, updated } from '$app/state';
	import { pageTitle } from '$lib/navigation';
	import type { Snippet } from 'svelte';

	interface Props {
		children: Snippet;
	}

	const { children } = $props() as Props;

	// No error surface, deliberately: this one is *read* for its errors below, and every page
	// under it already reports its own. A banner here would double each of them.
	const Session = graphql(`
		query Session {
			users {
				me {
					name
				}
			}
		}
	`);

	/// Leave for the new build on the way out of a page.
	///
	/// The service worker calls `skipWaiting()` and `clients.claim()`, so a deploy takes over a
	/// page that is already running -- and that page still holds imports to chunks the new build
	/// does not ship. A phone keeps the app on its home screen across days of deploys, which is
	/// where "nothing loads any more" comes from. A navigation is the one moment where a full
	/// page load costs the reader nothing, so that is where it happens rather than under them.
	beforeNavigate(({ willUnload, to }) => {
		if (updated.current && !willUnload && to?.url) window.location.href = to.url.href;
	});

	// Guarding here rather than per page: a route that queries nothing (or only data the
	// backend serves anonymously) otherwise renders an empty shell to a logged-out visitor
	// instead of sending them to the login form.
	$effect(() => {
		const onLoginPage = page.url.pathname === '/login';

		// decide on the resolved result rather than on store state: `users.me` errors out
		// when logged out, so the pre-fetch and logged-out states look identical
		Session.fetch()
			.then((result) => {
				if (result.data?.users.me) {
					if (onLoginPage) window.location.href = '/';
					return;
				}

				// only an ended session belongs on the login form. Anything else that makes the
				// query fail -- backend down, database gone, a bug -- used to look identical from
				// here and bounced you to a login you did not need.
				const loggedOut = result.errors?.some((e) => e.extensions?.code === 'UNAUTHENTICATED');
				if (loggedOut && !onLoginPage) window.location.href = '/login';
			})
			// a request that never came back is a backend that is down, not a session that ended
			.catch(() => {});
	});
</script>

<svelte:head>
	<title>{pageTitle(page.url.pathname)}</title>
</svelte:head>

<Nav />

{@render children()}
