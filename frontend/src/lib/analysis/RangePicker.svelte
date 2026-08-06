<script lang="ts">
	import { Select } from 'flowbite-svelte';
	import { graphql } from '$houdini';

	/// A window ending today, chosen from a handful of presets rather than typed as two dates.
	///
	/// "Alles" reaches back to the first expense, which is why this owns a query: the analysis
	/// views default to it, and a range picked without knowing where the data starts either
	/// misses most of it or asks for empty years.
	let {
		value = $bindable('all'),
		from = $bindable(),
		to = $bindable(),
		size = 'sm'
	}: {
		value?: string;
		from?: Date;
		to?: Date;
		size?: 'sm' | 'md';
	} = $props();

	const DAY = 86400000;

	const SPANS = [
		{ value: '6m', name: 'Letzte 6 Monate', months: 6 },
		{ value: '1y', name: 'Letztes Jahr', months: 12 },
		{ value: '5y', name: 'Letzte 5 Jahre', months: 60 },
		{ value: '10y', name: 'Letzte 10 Jahre', months: 120 },
		{ value: 'all', name: 'Alles', months: null }
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

	// `from`/`to` stay undefined until there is an answer, which is how a caller knows not to
	// query yet: a guessed window would be replaced a moment later and the two would race.
	$effect(() => {
		const max = Date.now();
		const months = SPANS.find((s) => s.value === value)?.months ?? null;

		let start: Date;
		if (months === null) {
			if (!ready) return;
			start = new Date(dataStart ?? max - 365 * DAY);
		} else {
			// built rather than mutated: a negative month rolls into the previous year on its own
			const now = new Date(max);
			start = new Date(now.getFullYear(), now.getMonth() - months, now.getDate());
		}

		// only write on a real change: `to` is Date.now(), so assigning unconditionally would
		// hand the caller a fresh object on every run and re-trigger whatever it queries
		if (from?.getTime() !== start.getTime()) from = start;
		if (to === undefined) to = new Date(max);
	});
</script>

<Select
	{value}
	onchange={(e) => (value = (e.target as HTMLSelectElement).value)}
	items={SPANS.map((s) => ({ value: s.value, name: s.name }))}
	{size}
	class="w-44"
/>
