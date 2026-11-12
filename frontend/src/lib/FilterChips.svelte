<script lang="ts">
	import type { ActiveFilter } from './filters';

	interface Props {
		filters: ActiveFilter[];
		reset: () => void;
	}
	let { filters, reset }: Props = $props();
</script>

<!-- What is currently being filtered on, and how to undo each of them.
     The filters live behind a button on a phone and in a row of empty-looking selects on a
     desktop, so a list arriving pre-filtered from somewhere else looked like the whole list,
     only shorter. Saying it out loud is also the only way to notice a filter you forgot. -->
{#if filters.length > 0}
	<div class="flex flex-wrap items-center gap-2 px-4 pb-3 text-sm sm:px-0">
		<span class="text-gray-500 dark:text-gray-400">Gefiltert nach</span>
		{#each filters as filter (filter.label)}
			<!-- the dot where what is being filtered on has a colour of its own, so the chip and the
			     rows it left standing look like each other -->
			<span
				class="inline-flex items-center gap-1.5 rounded-full border border-gray-300 px-2 py-0.5 dark:border-gray-600"
			>
				{#if filter.color}
					<span class="h-2 w-2 shrink-0 rounded-full" style:background-color={filter.color}></span>
				{/if}
				{filter.label}
				<button
					type="button"
					aria-label="{filter.label} entfernen"
					class="text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white"
					onclick={filter.clear}
				>
					×
				</button>
			</span>
		{/each}
		<button
			type="button"
			class="text-primary-600 dark:text-primary-500 hover:underline"
			onclick={reset}
		>
			alle entfernen
		</button>
	</div>
{/if}
