<script lang="ts">
	import { Card, Spinner } from 'flowbite-svelte';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import RangePicker from './RangePicker.svelte';
	import SeriesToggle from './SeriesToggle.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import {
		overviewsStore,
		monthLabel,
		monthTicks,
		monthDrilldown,
		categoryColor,
		type OverviewsData
	} from './overviews';
	import { categoriesState } from './state.svelte';
	import { ANALYSIS_CARD } from './cards';
	import { isoDate } from './dates';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	// see OverviewsView: the store is cleared mid-fetch, which flickers the charts
	let latestRequest = 0;
	let overviews = $state<OverviewsData | null>(null);

	$effect(() => {
		const { from, to } = categoriesState.range;
		if (!from || !to) return;

		const request = ++latestRequest;
		void overviewsStore
			.fetch({ variables: { from: new Date(from), to: new Date(to) } })
			.then((result) => {
				if (request !== latestRequest) return;
				overviews = result.data?.analysis.overviews ?? null;
			});
	});

	const months = $derived(overviews?.months ?? []);

	const breakdown = $derived(overviews?.byCategory ?? []);

	/// The magnitude, not the sign: spending is stored negative, which drew the stack downwards
	/// from zero for the one of the two views that is read most. Which side of zero it is on is
	/// what the toggle above the chart says, so the bars are free to grow the way bars do.
	const pick = (t: { income: number; expenses: number }) =>
		Math.abs(categoriesState.sign === 'expenses' ? t.expenses : t.income);
	const name = (c: { name: string } | null) => c?.name ?? 'Ohne Kategorie';

	// The bars are stacked per month, so every category needs a value at every month even
	// where it has none — an absent entry would shift the stack rather than leave a gap.
	const bars = $derived.by(() => {
		const keys = [...new Set(months.flatMap((m) => m.byCategory.map((c) => c.category?.id ?? '')))];

		return keys.map((key) => {
			const anywhere = months
				.flatMap((m) => m.byCategory)
				.find((c) => (c.category?.id ?? '') === key);

			return {
				name: name(anywhere?.category ?? null),
				color: categoryColor(anywhere?.category ?? null),
				data: months.map((m) => {
					const found = m.byCategory.find((c) => (c.category?.id ?? '') === key);
					return found ? pick(found) / 100 : 0;
				})
			};
		});
	});

	const barNames = $derived(bars.map((c) => c.name));

	const barOptions = $derived({
		chart: {
			type: 'bar',
			height: 340,
			toolbar: { show: false },
			stacked: true,
			events: {
				...monthDrilldown(months),
				// apex toggles the series itself; this only keeps our list in step with it
				legendClick: (_c: unknown, index: number) => {
					const name = bars[index]?.name;
					if (!name) return;
					categoriesState.hiddenSeries = categoriesState.hiddenSeries.includes(name)
						? categoriesState.hiddenSeries.filter((n) => n !== name)
						: [...categoriesState.hiddenSeries, name];
				}
			}
		},
		// see WealthView: `hidden` is what lets the whole set collapse in a single redraw
		series: bars.map((c) => ({
			name: c.name,
			hidden: categoriesState.hiddenSeries.includes(c.name),
			data: c.data
		})),
		colors: bars.map((c) => c.color),
		dataLabels: { enabled: false },
		xaxis: {
			categories: months.map((m) => monthLabel(m.month)),
			tickAmount: monthTicks(months.length),
			labels: { rotate: -45, hideOverlappingLabels: true }
		},
		yaxis: { labels: { formatter: (v: number) => Math.round(v) + ' €' } },
		// the tooltip inherits the axis formatter otherwise, which rounds away the cents
		tooltip: { y: { formatter: (v: number) => euro(v * 100) } },
		legend: { position: 'bottom' }
	} as ApexOptions);

	// a treemap rather than MoneyDB's sunburst: same category -> expense hierarchy, but
	// readable at the sizes where a sunburst's outer ring becomes unlabelable slivers
	const treemapSeries = $derived(
		breakdown
			.map((c) => {
				const children = c.biggest
					.filter((e) => (categoriesState.sign === 'expenses' ? e.amount < 0 : e.amount > 0))
					.map((e) => ({ x: e.title, y: Math.abs(e.amount) / 100 }));

				// what `biggest` left out, as one slice, so the areas still add up to the total
				const rest = Math.abs(categoriesState.sign === 'expenses' ? c.restExpenses : c.restIncome);
				if (rest > 0) children.push({ x: 'Rest', y: rest / 100 });

				return {
					name: name(c.category),
					categoryId: c.category?.id,
					color: categoryColor(c.category),
					data: children
				};
			})
			// Filtered out rather than declared `hidden`: a treemap has no collapsed state to
			// put a series into, and the tiles are laid out from whatever is left.
			.filter((s) => s.data.length > 0 && !categoriesState.hiddenSeries.includes(s.name))
	);

	/// The expenses behind a tile, in the list that can actually show them.
	///
	/// Which is what the "Rest" tile is for: it stands for everything the treemap had no room
	/// to name, and there is no bound on how much that is, so it belongs in a list rather than
	/// in a tooltip. The ends are snapped to whole months, which is what the backend does to
	/// the range before it answers -- a list ending mid-month would not add up to the tile.
	function openCategory(seriesIndex: number) {
		const id = treemapSeries[seriesIndex]?.categoryId;
		const { from, to } = categoriesState.range;
		// nothing to filter on: "Ohne Kategorie" is the absence of one, which the expense list
		// cannot ask for
		if (!id || !from || !to) return;

		const start = new Date(from);
		const end = new Date(to);
		const params = new URLSearchParams({
			kategorie: id.toString(),
			von: isoDate(new Date(start.getFullYear(), start.getMonth(), 1)),
			bis: isoDate(new Date(end.getFullYear(), end.getMonth() + 1, 0))
		});

		// the rule wants a literal resolve() call, which cannot carry a query string
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		void goto(`${resolve('/expenses')}?${params}`);
	}

	const treemapOptions = $derived.by(() => {
		const series = treemapSeries;

		return {
			chart: {
				type: 'treemap',
				height: 420,
				toolbar: { show: false },
				events: {
					dataPointSelection: (...args: unknown[]) =>
						openCategory((args[2] as { seriesIndex?: number } | undefined)?.seriesIndex ?? -1)
				}
			},
			series: series.map((s) => ({ name: s.name, data: s.data })),
			colors: series.map((s) => s.color),
			legend: { show: true, position: 'bottom' },
			dataLabels: { enabled: true, style: { fontSize: '12px' } },
			plotOptions: { treemap: { distributed: false, enableShades: true } },
			tooltip: { y: { formatter: (v: number) => euro(v * 100) } }
		} as ApexOptions;
	});
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<RangePicker bind:range={categoriesState.range} dates />
		<ToggleGroup
			bind:value={categoriesState.sign}
			options={[
				{ value: 'expenses', label: 'Ausgaben' },
				{ value: 'income', label: 'Einnahmen' }
			]}
		/>
		{#if $overviewsStore.fetching}
			<Spinner size="5" />
		{/if}
	</div>

	{#if $overviewsStore.errors}
		<p class="text-sm text-red-600 dark:text-red-400">
			{$overviewsStore.errors.map((e) => e.message).join(', ')}
		</p>
	{:else}
		<Card class={ANALYSIS_CARD}>
			<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
				<h3 class="text-lg font-semibold">Nach Kategorie je Monat</h3>
				<SeriesToggle names={barNames} bind:hidden={categoriesState.hiddenSeries} />
			</div>
			<Chart options={barOptions} />
		</Card>

		<Card class={ANALYSIS_CARD}>
			<!-- the same list of hidden categories as the bars above, so the tab filters as one
			     rather than asking twice which categories are of interest -->
			<div class="flex flex-wrap items-center justify-between gap-2">
				<h3 class="text-lg font-semibold">Gesamter Zeitraum</h3>
				<SeriesToggle names={barNames} bind:hidden={categoriesState.hiddenSeries} />
			</div>
			<p class="mb-2 text-sm text-gray-500 dark:text-gray-400">
				Eine auf mehrere Kategorien verteilte Ausgabe zählt anteilig in jede.
			</p>
			<Chart options={treemapOptions} />
		</Card>
	{/if}
</div>

<style>
	/* apex draws the axis labels as plain text; nothing says they can be clicked */
	:global(.apexcharts-xaxis-label) {
		cursor: pointer;
	}
</style>
