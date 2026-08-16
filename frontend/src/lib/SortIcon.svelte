<script lang="ts">
	import { DESC } from './table_sort.svelte';

	interface Props {
		/// the direction this column is sorted in, or null when another one is
		direction: number | null;
	}
	let { direction }: Props = $props();
</script>

{#if direction === DESC}
	▲
{:else if direction !== null}
	▼
{/if}
