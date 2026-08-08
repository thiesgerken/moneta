<script lang="ts">
	import { Select, Input } from 'flowbite-svelte';
	import { graphql } from '$houdini';
	import { CUSTOM_SPAN, type Range } from './state.svelte';
	import { isoDate, monthsBefore } from './dates';

	/// A window chosen from a handful of presets, or typed out by hand.
	///
	/// "Alles" reaches back to the first expense, which is why this owns a query: the analysis
	/// views default to a preset, and a range picked without knowing where the data starts
	/// either misses most of it or asks for empty years.
	let { range = $bindable(), dates = false }: { range: Range; dates?: boolean } = $props();

	const DAY = 86400000;

	const SPANS = [
		{ value: '6m', name: 'Letzte 6 Monate', months: 6 },
		{ value: '1y', name: 'Letztes Jahr', months: 12 },
		{ value: '5y', name: 'Letzte 5 Jahre', months: 60 },
		{ value: '10y', name: 'Letzte 10 Jahre', months: 120 },
		{ value: 'all', name: 'Alles', months: null },
		{ value: CUSTOM_SPAN, name: 'Eigener Zeitraum', months: null }
	];

	// Sorted by accrual start rather than by a transaction date, which is not a sort the list
	// offers -- for an imported expense the two are the same day anyway.
	// No error surface, deliberately: this only finds how far back the data goes, and a failure
	// falls back to the last year (see `ready`). The views it feeds report their own.
	const firstExpenseStore = graphql(`
		query AnalysisRangeStart {
			expenses {
				list(limit: 1, sortBy: ACCRUAL_START, sortDir: ASC) {
					edges {
						node {
							id
							accrualStart
						}
					}
				}
			}
		}
	`);

	$effect(() => {
		void firstExpenseStore.fetch();
	});

	const dataStart = $derived(
		$firstExpenseStore.data?.expenses.list.edges[0]?.node.accrualStart?.getTime()
	);
	// Resolved, whether or not it found anything — an empty database has no first expense, and a
	// failed lookup knows no more than that. Waiting on it would be worse than guessing: `ready`
	// gates the range, the range gates every query on the tab, so a lookup that never arrives
	// leaves the whole page blank with nothing on it to explain why.
	const ready = $derived($firstExpenseStore.data != null || $firstExpenseStore.errors != null);

	// `from`/`to` stay undefined until a preset can be resolved, which is how a caller knows
	// not to query yet: a guessed window would be replaced a moment later and the two races.
	let applied: string | undefined;
	$effect(() => {
		if (range.span === CUSTOM_SPAN) return;

		const max = Date.now();
		const months = SPANS.find((s) => s.value === range.span)?.months ?? null;

		let start: number;
		if (range.span === 'all') {
			if (!ready) return;
			start = dataStart ?? max - 365 * DAY;
		} else if (months !== null) {
			start = monthsBefore(max, months);
		} else {
			return;
		}

		// only write on a real change: `to` is Date.now(), so assigning unconditionally would
		// re-trigger whatever the caller queries on every run of this effect
		if (range.from === start && applied === range.span) return;

		// Both ends, every time. Every preset runs up to now, and the window may have been
		// narrowed since — zoomed into, or typed over — leaving an end that is not now.
		applied = range.span;
		range.from = start;
		range.to = max;
	});

	/// Typing a date means the preset no longer describes the window, so the dropdown follows.
	function edit(which: 'from' | 'to', value: string) {
		if (!value) return;
		range.span = CUSTOM_SPAN;
		range[which] = new Date(value).getTime();
	}
</script>

<div class="flex flex-wrap items-center gap-2">
	<!-- an explicit height on both: flowbite's `sm` is padding, and a select and a date input
	     do not arrive at the same height from the same padding.
	     The preset takes the whole line on a phone, which leaves the next one to the two dates:
	     sharing a line with it squeezed them to five characters apiece. -->
	<Select
		value={range.span}
		onchange={(e) => (range.span = (e.target as HTMLSelectElement).value)}
		items={SPANS.map((s) => ({ value: s.value, name: s.name }))}
		size="sm"
		class="w-full sm:w-44"
		selectClass="h-9 py-0"
		placeholder=""
	/>
	{#if dates}
		<!-- The two ends wrap as one: loose in the row, a narrow screen broke the line between
		     them and left the end date underneath the start looking like a control of its own.
		     They share what is left of the row there, and keep their fixed width from `sm` up. -->
		<div class="flex min-w-0 flex-1 items-center gap-2 sm:flex-none">
			<!-- flowbite's Input fills its container, so the width lives on a wrapper -->
			<div class="min-w-0 flex-1 sm:w-40 sm:flex-none">
				<Input
					type="date"
					size="sm"
					class="h-9 py-0"
					value={range.from ? isoDate(new Date(range.from)) : ''}
					onchange={(e) => edit('from', (e.target as HTMLInputElement).value)}
				/>
			</div>
			<span class="text-gray-500 dark:text-gray-400">–</span>
			<div class="min-w-0 flex-1 sm:w-40 sm:flex-none">
				<Input
					type="date"
					size="sm"
					class="h-9 py-0"
					value={range.to ? isoDate(new Date(range.to)) : ''}
					onchange={(e) => edit('to', (e.target as HTMLInputElement).value)}
				/>
			</div>
		</div>
	{/if}
</div>
