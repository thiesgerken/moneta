<script lang="ts">
	import '../app.css';
	import Nav from '$lib/Nav.svelte';
	import { graphql } from '$houdini';
	import { page } from '$app/state';
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
		if (page.url.pathname === '/login') return;

		// decide on the resolved result rather than on store state: `users.me` errors out
		// when logged out, so the pre-fetch and logged-out states look identical
		Session.fetch().then((result) => {
			if (!result.data?.users.me) window.location.href = '/login';
		});
	});
</script>

<Nav />

{@render children()}
