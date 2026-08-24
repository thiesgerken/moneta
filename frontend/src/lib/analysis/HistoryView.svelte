<script lang="ts">
	import {
		ChartOutline,
		ChartMixedOutline,
		ChartPieOutline,
		TableColumnOutline
	} from 'flowbite-svelte-icons';
	import {
		Card,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Badge,
		Spinner,
		Tooltip
	} from 'flowbite-svelte';
	import type { ApexOptions } from 'apexcharts';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Chart from '$lib/Chart.svelte';
	import RangePicker from './RangePicker.svelte';
	import SeriesToggle from './SeriesToggle.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { overviewsStore, periodDrilldown, type OverviewsData } from './overviews';
	import {
		integrationError,
		errorClass,
		deltaColor,
		periodLabel,
		periodTicks,
		categoryColor
	} from './format';
	import {
		CUSTOM_SPAN,
		DATE_AXES,
		accrued,
		axisState,
		grainFor,
		historyState,
		type Granularity
	} from './state.svelte';
	import { ANALYSIS_CARD } from './cards';
	import { isoDate } from './dates';
	import QueryError from '$lib/QueryError.svelte';
	import { euro, signedEuro } from '$lib/money';

	// Kept here rather than read off the store so the chart never sees an empty intermediate
	// state: houdini clears `data` while a fetch is in flight, which drew the chart once
	// empty and once full on every range change. Only the newest request may write.
	let latestRequest = 0;
	let overviews = $state<OverviewsData | null>(null);

	/// How many months the range covers, both ends inclusive -- the backend snaps it outwards
	/// to whole periods anyway, so a fragment of a month counts as one.
	const span = $derived.by(() => {
		const { from, to } = historyState.range;
		if (!from || !to) return 0;

		const a = new Date(from);
		const b = new Date(to);
		return (b.getFullYear() - a.getFullYear()) * 12 + (b.getMonth() - a.getMonth()) + 1;
	});

	/// The length of the range picks the grain, and the toggle overrides it until the length
	/// changes again. Asking for ten years is itself the request for years, and a grain chosen
	/// for one window is not an answer about the next one.
	$effect(() => {
		if (span === 0 || historyState.grainDecidedFor === span) return;
		historyState.grainDecidedFor = span;
		historyState.granularity = grainFor(span);
	});

	const granularity = $derived((historyState.granularity || 'MONTH') as Granularity);
	const byYear = $derived(granularity === 'YEAR');

	$effect(() => {
		// undefined until the picker has resolved its preset
		const { from, to } = historyState.range;
		if (!from || !to || !historyState.granularity) return;

		const request = ++latestRequest;
		void overviewsStore
			.fetch({
				variables: { from: new Date(from), to: new Date(to), accrued: accrued(), granularity }
			})
			.then((result) => {
				if (request !== latestRequest) return;
				overviews = result.data?.analysis.overviews ?? null;
			});
	});

	const periods = $derived(overviews?.periods ?? []);

	/// A year opens into its own months rather than jumping to a month page: the bar stands for
	/// twelve of them and there is no one month it means. The range picker follows, so what
	/// happened is visible in the control that did it.
	///
	/// Local midnights, because that is what the picker's own date fields read back: they
	/// render through `isoDate`, which is local so that nobody is shown the day before the one
	/// they typed. A `Date.UTC` here would put 31.12. in the field for anyone west of UTC.
	const intoYear = (start: Date) => {
		// the backend dates a period at midnight UTC; reading it locally can land in the year
		// before
		const year = start.getUTCFullYear();
		historyState.range = {
			span: CUSTOM_SPAN,
			from: new Date(year, 0, 1).getTime(),
			to: new Date(year, 11, 31).getTime()
		};
		historyState.granularity = 'MONTH';
		// the range is twelve months now, so the rule above would land on MONTH as well --
		// this only saves it a redraw
		historyState.grainDecidedFor = 12;
	};

	/// Whether the chart splits a month into categories instead of into its two sides. The
	/// table below is the same table either way -- a balance and an integration error are facts
	/// about a month, and a category has neither.
	const split = $derived(historyState.breakdown === 'categories');

	/// The magnitude, not the sign: spending is stored negative, which drew the stack downwards
	/// from zero for the one of the two views that is read most. Which side of zero it is on is
	/// what the toggle above the chart says, so the bars are free to grow the way bars do.
	const pick = (t: { income: number; expenses: number }) =>
		Math.abs(historyState.sign === 'expenses' ? t.expenses : t.income);
	const name = (c: { name: string } | null) => c?.name ?? 'Ohne Kategorie';

	// The bars are stacked per month, so every category needs a value at every month even
	// where it has none — an absent entry would shift the stack rather than leave a gap.
	const bars = $derived.by(() => {
		if (!split) return [];

		const keys = [
			...new Set(periods.flatMap((p) => p.byCategory.map((c) => c.category?.id ?? '')))
		];

		return keys.map((key) => {
			const anywhere = periods
				.flatMap((p) => p.byCategory)
				.find((c) => (c.category?.id ?? '') === key);

			return {
				name: name(anywhere?.category ?? null),
				color: categoryColor(anywhere?.category ?? null),
				data: periods.map((p) => {
					const found = p.byCategory.find((c) => (c.category?.id ?? '') === key);
					return found ? pick(found) / 100 : 0;
				})
			};
		});
	});

	const barNames = $derived(bars.map((c) => c.name));

	/// One chart, two readings of the same periods: the totals of each, or one of the two sides
	/// of each broken into what it was spent on. Same bars, same x axis, same drilldown -- the
	/// toggle above it says which question is being asked, not which chart is being drawn.
	const chartOptions = $derived({
		chart: {
			type: 'bar',
			height: split ? 340 : 300,
			toolbar: { show: false },
			stacked: true,
			events: {
				...periodDrilldown(periods, granularity, intoYear),
				// apex toggles the series itself; this only keeps our list in step with it
				legendClick: (_c: unknown, index: number) => {
					const clicked = bars[index]?.name;
					if (!clicked) return;
					historyState.hiddenSeries = historyState.hiddenSeries.includes(clicked)
						? historyState.hiddenSeries.filter((n) => n !== clicked)
						: [...historyState.hiddenSeries, clicked];
				}
			}
		},
		// see WealthView: `hidden` is what lets the whole set collapse in a single redraw
		series: split
			? bars.map((c) => ({
					name: c.name,
					hidden: historyState.hiddenSeries.includes(c.name),
					data: c.data
				}))
			: [
					{ name: 'Einnahmen', data: periods.map((o) => o.income / 100) },
					{ name: 'Ausgaben', data: periods.map((o) => o.expenses / 100) }
				],
		colors: split ? bars.map((c) => c.color) : ['#16a34a', '#dc2626'],
		dataLabels: { enabled: false },
		xaxis: {
			categories: periods.map((o) => periodLabel(o.start, granularity)),
			tickAmount: periodTicks(periods.length),
			labels: { rotate: -45, hideOverlappingLabels: true }
		},
		yaxis: { labels: { formatter: (v: number) => Math.round(v) + ' €' } },
		// the tooltip inherits the axis formatter otherwise, which rounds away the cents
		tooltip: { y: { formatter: (v: number) => euro(v * 100) } },
		// two series fit above the chart; thirty categories are a block of their own below it
		legend: { position: split ? 'bottom' : 'top' }
	} as ApexOptions);

	const wholePeriod = $derived(overviews?.byCategory ?? []);

	// a treemap rather than MoneyDB's sunburst: same category -> expense hierarchy, but
	// readable at the sizes where a sunburst's outer ring becomes unlabelable slivers
	const treemapSeries = $derived(
		wholePeriod
			.map((c) => {
				const children = c.biggest
					.filter((e) => (historyState.sign === 'expenses' ? e.amount < 0 : e.amount > 0))
					.map((e) => ({ x: e.title, y: Math.abs(e.amount) / 100 }));

				// what `biggest` left out, as one slice, so the areas still add up to the total
				const rest = Math.abs(historyState.sign === 'expenses' ? c.restExpenses : c.restIncome);
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
			.filter((s) => s.data.length > 0 && !historyState.hiddenSeries.includes(s.name))
	);

	/// The expenses behind a tile, in the list that can actually show them.
	///
	/// Which is what the "Rest" tile is for: it stands for everything the treemap had no room
	/// to name, and there is no bound on how much that is, so it belongs in a list rather than
	/// in a tooltip. The ends are snapped to whole months, which is what the backend does to
	/// the range before it answers -- a list ending mid-month would not add up to the tile. A
	/// year is twelve of them, so the same snapping covers both grains.
	function openCategory(seriesIndex: number) {
		const id = treemapSeries[seriesIndex]?.categoryId;
		const { from, to } = historyState.range;
		// nothing to filter on: "Ohne Kategorie" is the absence of one, which the expense list
		// cannot ask for
		if (!id || !from || !to) return;

		const start = new Date(from);
		const end = new Date(to);
		const params = new URLSearchParams({
			category: id.toString(),
			// the tile is filed under whichever axis the tab is reading, and so is the list
			axis: accrued() ? 'accrual' : 'value',
			from: isoDate(new Date(start.getFullYear(), start.getMonth(), 1)),
			to: isoDate(new Date(end.getFullYear(), end.getMonth() + 1, 0))
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

	const sideLabel = $derived(historyState.sign === 'expenses' ? 'Ausgaben' : 'Einnahmen');
	const periodWord = $derived(byYear ? 'Jahr' : 'Monat');
</script>

<div class="space-y-4">
	<!-- Aufschlüsselung first, since it is what the two cards below it are of. The sign only
	     appears with it: the totals chart draws both sides at once, so there is nothing there
	     to choose between. Two short toggles rather than one that spells out all three
	     combinations -- `ButtonGroup` does not wrap, and "Einnahmen nach Kategorie" beside two
	     others is wider than a phone. -->
	<div class="flex flex-wrap items-end gap-4">
		<RangePicker bind:range={historyState.range} dates />
		<ToggleGroup
			bind:value={historyState.breakdown}
			size="sm"
			options={[
				{ value: 'total', label: 'Gesamt' },
				{ value: 'categories', label: 'Nach Kategorie' }
			]}
		/>
		{#if split}
			<ToggleGroup
				bind:value={historyState.sign}
				size="sm"
				options={[
					{ value: 'expenses', label: 'Ausgaben' },
					{ value: 'income', label: 'Einnahmen' }
				]}
			/>
		{/if}
		<!-- The range picks this and the toggle overrides it, so it mostly does not have to be
		     found at all: it is here for the ten-year window you want the months of, and the
		     two-year one you want summed. -->
		<ToggleGroup
			bind:value={historyState.granularity}
			size="sm"
			options={[
				{ value: 'MONTH', label: 'Monate' },
				{ value: 'YEAR', label: 'Jahre' }
			]}
		/>
		<ToggleGroup bind:value={axisState.axis} options={DATE_AXES} size="sm" />
		{#if $overviewsStore.fetching}
			<Spinner size="5" class="self-center" />
		{/if}
	</div>

	{#if $overviewsStore.errors}
		<QueryError
			errors={$overviewsStore.errors}
			message="Die Auswertung konnte nicht geladen werden."
		/>
	{:else}
		<Card class={ANALYSIS_CARD}>
			<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
				<h3 class="text-lg font-semibold">
					{#if split}
						<ChartMixedOutline
							class="me-2 inline h-5 w-5 align-[-0.2em] text-gray-400 dark:text-gray-500"
						/>{sideLabel} nach Kategorie je {periodWord}
					{:else}
						<ChartOutline
							class="me-2 inline h-5 w-5 align-[-0.2em] text-gray-400 dark:text-gray-500"
						/>Einnahmen und Ausgaben je {periodWord}
					{/if}
				</h3>
				{#if split}
					<SeriesToggle names={barNames} bind:hidden={historyState.hiddenSeries} />
				{/if}
			</div>
			<Chart options={chartOptions} />
		</Card>

		{#if split}
			<Card class={ANALYSIS_CARD}>
				<!-- the same list of hidden categories as the bars above, so the tab filters as one
				     rather than asking twice which categories are of interest -->
				<div class="flex flex-wrap items-center justify-between gap-2">
					<h3 class="text-lg font-semibold">
						<ChartPieOutline
							class="me-2 inline h-5 w-5 align-[-0.2em] text-gray-400 dark:text-gray-500"
						/>Gesamter Zeitraum
					</h3>
					<SeriesToggle names={barNames} bind:hidden={historyState.hiddenSeries} />
				</div>
				<p class="mb-2 text-sm text-gray-500 dark:text-gray-400">
					Eine auf mehrere Kategorien verteilte Ausgabe zählt anteilig in jede.
				</p>
				<Chart options={treemapOptions} />
			</Card>
		{/if}

		<Card class={ANALYSIS_CARD}>
			<h3 class="mb-2 text-lg font-semibold">
				<TableColumnOutline
					class="me-2 inline h-5 w-5 align-[-0.2em] text-gray-400 dark:text-gray-500"
				/>Übersichten je {periodWord}
			</h3>
			<div class="overflow-x-auto">
				<Table hoverable striped class="w-full text-left text-sm">
					<!-- What the period did comes before what it left behind: delta, then the two
					     sides it is made of, then the balances they moved between. On a phone the
					     columns behind Ausgaben are off the screen, and what is dropped there is
					     what answers a question nobody asks on a phone -- how many expenses it
					     took, where the period started, and how far the two ways of counting
					     disagree.

					     The balances are gone entirely on the accrual axis, along with the error
					     that reconciles against them: they are money in an account, which does
					     not move when a cost is filed under a different month, so a column that
					     reads the same either way sitting next to four that do not is a column
					     inviting the wrong subtraction. -->
					<TableHead>
						<TableHeadCell class="px-4 py-3">{periodWord}</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Delta</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Einnahmen</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Ausgaben</TableHeadCell>
						<TableHeadCell class="hidden px-4 py-3 sm:table-cell">Anzahl</TableHeadCell>
						{#if !accrued()}
							<TableHeadCell class="hidden px-4 py-3 sm:table-cell">Vermögen Beginn</TableHeadCell>
							<TableHeadCell class="px-4 py-3">Vermögen Ende</TableHeadCell>
						{/if}
						<!-- Only on the value axis. The error is the gap between the balances and what the
						     period booked, and on the accrual axis those are answers to different
						     questions: a sixth of a subscription is a cost the period carried, not money
						     that left an account, so there is nothing to reconcile. -->
						{#if !accrued()}
							<TableHeadCell class="hidden px-4 py-3 sm:table-cell">
								<span class="underline decoration-dotted">Fehler</span>
								<Tooltip class="max-w-xs text-xs font-normal normal-case">
									Die Lücke zwischen dem fortgeschriebenen und dem gemessenen Endvermögen. Eine
									große Lücke heißt nicht, dass die Rechnung driftet, sondern dass eine Ausgabe
									fehlt.
								</Tooltip>
							</TableHeadCell>
						{/if}
					</TableHead>
					<TableBody class="divide-y">
						{#each periods.toReversed() as o (o.start.getTime())}
							<TableBodyRow>
								<!-- A year has no page of its own, so it opens into its own months the
								     way its bar does. A month keeps its link, which is what made the
								     month pages reachable in the first place. -->
								<TableBodyCell class="px-4 py-3 whitespace-nowrap">
									{#if byYear}
										<button
											type="button"
											class="text-primary-600 dark:text-primary-500 font-medium hover:underline"
											onclick={() => intoYear(o.start)}
										>
											{o.start.getUTCFullYear()}
										</button>
									{:else}
										<a
											class="text-primary-600 dark:text-primary-500 font-medium hover:underline"
											href={resolve('/analysis/[year]/[month]', {
												year: `${o.start.getUTCFullYear()}`,
												month: `${o.start.getUTCMonth() + 1}`
											})}
										>
											{o.start.toLocaleDateString('de-DE', {
												month: 'long',
												year: 'numeric',
												timeZone: 'UTC'
											})}
										</a>
									{/if}
								</TableBodyCell>
								<!-- what the period booked, not what the balances did: on the accrual axis
								     the balances stand still while the two columns beside this one move,
								     and a delta that disagreed with the sum of them read as an error -->
								{@const delta = o.income + o.expenses}
								<TableBodyCell class="px-4 py-3">
									<Badge color={deltaColor(delta)} class="text-sm font-semibold">
										{signedEuro(delta)}
									</Badge>
								</TableBodyCell>
								<TableBodyCell class="px-4 py-3 text-green-700 dark:text-green-400"
									>{euro(o.income)}</TableBodyCell
								>
								<TableBodyCell class="px-4 py-3 text-red-600 dark:text-red-400"
									>{euro(o.expenses)}</TableBodyCell
								>
								<TableBodyCell class="hidden px-4 py-3 sm:table-cell"
									>{o.expenseCount}</TableBodyCell
								>
								{#if !accrued()}
									<TableBodyCell class="hidden px-4 py-3 sm:table-cell"
										>{euro(o.startTotal)}</TableBodyCell
									>
									<TableBodyCell class="px-4 py-3">{euro(o.endTotal)}</TableBodyCell>
								{/if}
								{@const error = integrationError(o)}
								{#if error !== null}
									<TableBodyCell class="hidden px-4 py-3 sm:table-cell {errorClass(error)}">
										{signedEuro(error)}
									</TableBodyCell>
								{/if}
							</TableBodyRow>
						{/each}
					</TableBody>
				</Table>
			</div>
		</Card>
	{/if}
</div>

<style>
	/* apex draws the axis labels as plain text; nothing says they can be clicked */
	:global(.apexcharts-xaxis-label) {
		cursor: pointer;
	}
</style>
