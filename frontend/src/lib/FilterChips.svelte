<script lang="ts">
	import { tint } from './expense_utils';

	export type ActiveFilter = { label: string; color?: string | null; clear: () => void };

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
			<!-- tinted where what is being filtered on has a colour of its own, so the chip and the
			     cells it left standing look like each other -->
			<span
				class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 {filter.color
					? ''
					: 'bg-gray-200 dark:bg-gray-700'}"
				style={tint(filter.color)}
			>
				{filter.label}
				<!-- dimmed rather than grey: on a tinted chip the text colour is whatever the tint
				     needs, and a fixed grey × was invisible on the dark ones -->
				<button
					type="button"
					aria-label="{filter.label} entfernen"
					class="opacity-60 hover:opacity-100"
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
