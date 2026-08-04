<script lang="ts">
	import '../app.css';
	import Nav from '$lib/Nav.svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		children: Snippet;
	}

	const { children } = $props() as Props;
</script>

<Nav />

{@render children()}
