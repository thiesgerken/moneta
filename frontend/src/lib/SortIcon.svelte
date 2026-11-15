<script lang="ts">
	import { ASC, DESC } from './table_sort.svelte';

	interface Props {
		/// the direction this column is sorted in, or null when another one is
		direction: number | null;
	}
	let { direction }: Props = $props();
</script>

<!-- the tip is the small end: up for ascending, as in spreadsheets and file managers -->
{#if direction === ASC}
	▲
{:else if direction === DESC}
	▼
{/if}
