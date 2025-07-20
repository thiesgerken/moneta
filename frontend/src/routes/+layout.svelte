<script lang="ts">
	import '../app.css';
	import Nav from '$lib/Nav.svelte';

	interface Props {
		children: any;
	}

	const { children } = $props() as Props;
</script>

<Nav />

{@render children()}
