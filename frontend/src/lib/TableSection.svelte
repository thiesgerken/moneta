<script lang="ts">
	import type { Snippet } from 'svelte';
	const { children }: { children: Snippet } = $props();
</script>

<section class="bg-gray-50 p-0 sm:p-5 dark:bg-gray-900">
	<div class="mx-auto max-w-screen-xl px-4 lg:px-12">
		{@render children()}
	</div>
</section>
