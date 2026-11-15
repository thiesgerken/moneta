<script lang="ts">
	import { ASC, DESC } from './table_sort.svelte';

	interface Props {
		/// the direction this column is sorted in, or null when another one is
		direction: number | null;
	}
	let { direction }: Props = $props();
</script>

<!-- the point of the arrow is where the small end is: up for ascending, as everywhere else -->
{#if direction === ASC}
	▲
{:else if direction === DESC}
	▼
{/if}
