<script lang="ts">
	import '../app.css';
	import Nav from '$lib/Nav.svelte';
	import { graphql } from '$houdini';
	import { page } from '$app/state';
	import { pageTitle } from '$lib/navigation';
	import type { Snippet } from 'svelte';

	interface Props {
		children: Snippet;
	}

	const { children } = $props() as Props;

	const Session = graphql(`
		query Session {
			users {
				me {
					name
				}
			}
		}
	`);

	// Guarding here rather than per page: a route that queries nothing (or only data the
	// backend serves anonymously) otherwise renders an empty shell to a logged-out visitor
	// instead of sending them to the login form.
	$effect(() => {
		const onLoginPage = page.url.pathname === '/login';

		// decide on the resolved result rather than on store state: `users.me` errors out
		// when logged out, so the pre-fetch and logged-out states look identical
		Session.fetch().then((result) => {
			if (result.data?.users.me) {
				if (onLoginPage) window.location.href = '/';
				return;
			}

			// only an ended session belongs on the login form. Anything else that makes the query
			// fail -- backend down, database gone, a bug -- used to look identical from here and
			// bounced you to a login you did not need.
			const loggedOut = result.errors?.some((e) => e.extensions?.code === 'UNAUTHENTICATED');
			if (loggedOut && !onLoginPage) window.location.href = '/login';
		});
	});
</script>

<svelte:head>
	<title>{pageTitle(page.url.pathname)}</title>
</svelte:head>

<Nav />

{@render children()}
