<script lang="ts">
	import type { Snippet } from 'svelte';
	const { children }: { children: Snippet } = $props();
</script>

<section class="bg-gray-50 p-0 sm:p-5 dark:bg-gray-900">
	<div class="px-4">
		{@render children()}
	</div>
</section>
