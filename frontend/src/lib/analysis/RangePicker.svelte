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

	// Sorted by booking start rather than by a transaction date, which is not a sort the list
	// offers -- for an imported expense the two are the same day anyway.
	const firstExpenseStore = graphql(`
		query AnalysisRangeStart {
			expenses {
				list(limit: 1, sortBy: BOOKING_START, sortDir: ASC) {
					edges {
						node {
							id
							bookingStart
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
		$firstExpenseStore.data?.expenses.list.edges[0]?.node.bookingStart?.getTime()
	);
	// resolved, whether or not it found anything — an empty database has no first expense
	const ready = $derived($firstExpenseStore.data != null);

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
	     do not arrive at the same height from the same padding -->
	<Select
		value={range.span}
		onchange={(e) => (range.span = (e.target as HTMLSelectElement).value)}
		items={SPANS.map((s) => ({ value: s.value, name: s.name }))}
		size="sm"
		class="w-44"
		selectClass="h-9 py-0"
		placeholder=""
	/>
	{#if dates}
		<!-- flowbite's Input fills its container, so the width lives on a wrapper -->
		<div class="w-40">
			<Input
				type="date"
				size="sm"
				class="h-9 py-0"
				value={range.from ? isoDate(new Date(range.from)) : ''}
				onchange={(e) => edit('from', (e.target as HTMLInputElement).value)}
			/>
		</div>
		<span class="text-gray-500 dark:text-gray-400">–</span>
		<div class="w-40">
			<Input
				type="date"
				size="sm"
				class="h-9 py-0"
				value={range.to ? isoDate(new Date(range.to)) : ''}
				onchange={(e) => edit('to', (e.target as HTMLInputElement).value)}
			/>
		</div>
	{/if}
</div>
