<script lang="ts">
	import {
		Card,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Input,
		Label,
		Spinner
	} from 'flowbite-svelte';
	import { graphql } from '$houdini';
	import type ApexCharts from 'apexcharts';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import RangePicker from './RangePicker.svelte';
	import SeriesToggle from './SeriesToggle.svelte';
	import { AVAILABILITY_LABELS, availabilityColor } from './availability';
	import { CUSTOM_SPAN, wealthState } from './state.svelte';
	import { shortDate, todayIso } from './dates';
	import { ANALYSIS_CARD, WEALTH_COLOR } from './cards';
	import QueryError from '$lib/QueryError.svelte';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const DAY = 86400000;

	// module-level, so switching tabs and coming back does not reset the view
	const visible = $derived.by(() => {
		const { from, to } = wealthState.range;
		return from && to ? { min: from, max: to } : null;
	});

	// The backend caps the point count, and asking for more than the chart can draw only
	// makes the response bigger; ~120 is what fits across a screen. A phone has a fifth of
	// that width, where 120 samples land two pixels apart and the line reads as a row of dots.
	let viewport = $state(0);
	const compact = $derived(viewport > 0 && viewport < 640);
	const targetPoints = $derived(compact ? 40 : 120);

	const stepDays = $derived(
		visible ? Math.max(1, Math.ceil((visible.max - visible.min) / DAY / targetPoints)) : 30
	);

	type HistoryData = NonNullable<
		NonNullable<(typeof $historyStore)['data']>
	>['analysis']['history'];

	const historyStore = graphql(`
		query AnalysisHistory($from: DateTime!, $to: DateTime!, $stepDays: Int) {
			analysis {
				history(from: $from, to: $to, stepDays: $stepDays) {
					dates
					byAccount {
						account {
							id
							name
							color
							# only to decide which series start collapsed
							closed
						}
						amounts
					}
					byAvailability {
						availability
						amounts
					}
				}
			}
		}
	`);

	const balanceStore = graphql(`
		query AnalysisComputedBalances($date: DateTime!) {
			analysis {
				balances(date: $date) {
					amount
					expenseCount
					account {
						id
						name
						color
						# a closed account keeps its last balance for ever; the table hides those
						closed
					}
					reference {
						id
						date
						amount
					}
				}
			}
		}
	`);

	// houdini 2 has no @load, so every query needs its own effect; referencing the variables
	// inside it is what makes zooming and the Stichtag re-run them.
	//
	// The result is kept here rather than read off the store, because two fetches can be in
	// flight at once — picking a preset while a zoom is still loading, say. houdini writes
	// whichever answers last into the store, so a slow narrow query can land on top of the
	// wide one that was asked for after it. Only the newest request may write.
	let latestRequest = 0;
	let history = $state<HistoryData | null>(null);

	$effect(() => {
		// undefined until the picker knows what "Alles" means
		if (!visible) return;

		const request = ++latestRequest;
		const variables = {
			from: new Date(visible.min),
			to: new Date(visible.max),
			stepDays
		};

		void historyStore.fetch({ variables }).then((result) => {
			if (request !== latestRequest) return;
			history = result.data?.analysis.history ?? null;
		});
	});

	$effect(() => {
		void balanceStore.fetch({ variables: { date: new Date(wealthState.date) } });
	});

	/// A closed account held money once, so it belongs in the history — but a decade of accounts
	/// you have since given up is not what the chart is opened for, and with thirty-odd series
	/// the legend alone fills a phone. They start collapsed, once per session: after that the
	/// choice is the reader's, including the choice to bring them back.
	$effect(() => {
		const loaded = history;
		if (!loaded || wealthState.accountsSeeded) return;

		wealthState.accountsSeeded = true;
		const closed = loaded.byAccount.filter((s) => s.account.closed).map((s) => s.account.name);
		if (closed.length > 0) wealthState.hiddenAccounts = closed;
	});

	/// which list the toggle and the chart read, since each split hides its own names
	const hidden = $derived(
		wealthState.split === 'account' ? wealthState.hiddenAccounts : wealthState.hiddenAvailability
	);

	const balances = $derived($balanceStore.data?.analysis.balances ?? []);

	const closedCount = $derived(balances.filter((b) => b.account.closed).length);

	const shownBalances = $derived(
		wealthState.balanceAccounts === 'all' ? balances : balances.filter((b) => !b.account.closed)
	);

	/// Of what is shown: a total that counts rows you cannot see is a total of nothing in
	/// particular, and a closed account contributes the same figure for ever.
	const total = $derived(shownBalances.reduce((sum, b) => sum + b.amount, 0));

	const dates = $derived(history?.dates.map((d) => d.getTime()) ?? []);

	const series = $derived.by(() => {
		// a local const, so the narrowing survives into the callbacks below
		const loaded = history;
		if (!loaded) return [];
		if (wealthState.split === 'account')
			return loaded.byAccount.map((s) => ({
				name: s.account.name,
				color: s.account.color ?? '#94a3b8',
				amounts: s.amounts
			}));
		if (wealthState.split === 'availability')
			return loaded.byAvailability.map((s) => ({
				name: AVAILABILITY_LABELS[s.availability],
				color: availabilityColor(s.availability),
				amounts: s.amounts
			}));
		return [
			{
				name: 'Gesamt',
				color: WEALTH_COLOR,
				amounts: dates.map((_, i) => loaded.byAccount.reduce((sum, s) => sum + s.amounts[i], 0))
			}
		];
	});

	let chartInstance: ApexCharts | null = $state(null);
	const seriesNames = $derived(series.map((s) => s.name));

	/// The Stichtag marker, drawn onto the chart rather than declared in its options.
	///
	/// Changed options are applied by handing apex the whole set again, which rebuilds the
	/// series — so moving this line would blink every marker on the chart, for a line that has
	/// nothing to do with the data. (`redrawPaths: false` does not help; the markers go either
	/// way.) The cost is that apex keeps only the annotations its options carry, so this has
	/// to be re-drawn after anything that makes it re-render.
	const STICHTAG = 'stichtag';
	function drawStichtag(chart: ApexCharts | null) {
		const at = new Date(wealthState.date).getTime();
		if (!chart || Number.isNaN(at)) return;

		try {
			chart.removeAnnotation(STICHTAG);
			// Today is where the chart already ends, so the line would sit on its right edge
			// and label the axis' own last point. It is worth drawing once it means something
			// other than "now", which is to say once it has been moved.
			if (wealthState.date === todayIso()) return;

			chart.addXaxisAnnotation(
				{
					id: STICHTAG,
					x: at,
					borderColor: '#f05a28',
					strokeDashArray: 0,
					label: { text: 'Stichtag', style: { background: '#f05a28', color: '#fff' } }
				},
				false
			);
		} catch {
			// the chart can be torn down between the instance being handed over and this
			// running; apex throws rather than returning when it is
		}
	}

	$effect(() => {
		const chart = chartInstance;
		void wealthState.date;
		// hiding or showing a series re-renders the chart, which drops this line with it
		void hidden.length;

		// a frame late, so it lands after whatever redraw prompted it rather than under it
		const timer = setTimeout(() => drawStichtag(chart));
		return () => clearTimeout(timer);
	});

	// Faint verticals on month starts to give the eye something to measure against. Beyond
	// about 18 months there are too many to stay subtle, so they fall back to year starts.
	const boundaries = $derived.by(() => {
		if (!visible) return [];
		const yearly = visible.max - visible.min > 550 * DAY;
		const from = new Date(visible.min);
		const year = from.getFullYear();
		const month = yearly ? 0 : from.getMonth();
		const out: number[] = [];
		// the month argument may overflow past 11; Date rolls it into the next year
		for (let k = 0; k < 400; k++) {
			const t = yearly
				? new Date(year + k, 0, 1).getTime()
				: new Date(year, month + k, 1).getTime();
			if (t > visible.max) break;
			if (t >= visible.min) out.push(t);
		}
		return out;
	});

	// Apex reports a drag that selects a range and a plain click through the same `click`
	// event, so without this, selecting a range also moved the Stichtag to wherever the
	// pointer came to rest. How far the pointer travelled is what tells them apart.
	const DRAG_SLOP = 5; // px: a click with a shaky hand, not a selection
	let pressedAt: { x: number; y: number } | null = null;
	let wasDrag = false;

	$effect(() => {
		const down = (e: MouseEvent) => {
			pressedAt = { x: e.clientX, y: e.clientY };
			wasDrag = false;
		};
		const up = (e: MouseEvent) => {
			if (!pressedAt) return;
			wasDrag = Math.hypot(e.clientX - pressedAt.x, e.clientY - pressedAt.y) > DRAG_SLOP;
			pressedAt = null;
		};

		// Both event families: pointer events cover touch, but anything synthesising plain
		// mouse events would otherwise never set the flag. Capture phase, so they have run
		// before apex's own handlers inside the chart.
		const events = ['pointerdown', 'mousedown', 'pointerup', 'mouseup'] as const;
		for (const name of events) {
			window.addEventListener(name, name.endsWith('down') ? down : up, true);
		}
		// on the element rather than in the markup: a click handler on a plain div is an
		// accessibility complaint the date input below already answers. Held in a local, since
		// svelte clears `bind:this` before it runs the teardown.
		const plot = plotEl;
		plot.addEventListener('click', pickDate, true);

		return () => {
			for (const name of events) {
				window.removeEventListener(name, name.endsWith('down') ? down : up, true);
			}
			plot.removeEventListener('click', pickDate, true);
		};
	});

	let plotEl: HTMLDivElement;

	/// Clicking the plot picks that day as the Stichtag for the simulation below.
	///
	/// Read off the x position rather than from apex's own click event, which reports a data
	/// point only while the pointer is over one — everywhere else in the plot it reports -1,
	/// and its range-selection layer covers the plot besides. The position also beats the
	/// point it would have named: the series is sampled at ~120 dates, so over a decade the
	/// nearest one can be weeks from where the click landed.
	function pickDate(e: MouseEvent) {
		if (wasDrag || !visible) return;
		if ((e.target as Element).closest('.apexcharts-toolbar, .apexcharts-legend')) return;

		const grid = plotEl.querySelector('.apexcharts-grid')?.getBoundingClientRect();
		if (!grid || grid.width === 0) return;

		const at = visible.min + ((e.clientX - grid.left) / grid.width) * (visible.max - visible.min);
		if (at < visible.min || at > visible.max) return;

		const d = new Date(at);
		// local parts, so the date input does not shift a day against the displayed one
		wealthState.date = `${d.getFullYear()}-${`${d.getMonth() + 1}`.padStart(2, '0')}-${`${d.getDate()}`.padStart(2, '0')}`;
	}

	const chartOptions = $derived({
		chart: {
			type: 'area',
			height: 340,
			stacked: wealthState.split !== 'total',
			zoom: { enabled: true, type: 'x', autoScaleYaxis: true },
			toolbar: {
				show: true,
				// apex's own reset is deliberately off: it bails out unless *this chart instance*
				// was zoomed interactively, and changing the resolution rebuilds the chart, so the
				// instance you would be resetting has no zoom recorded and the button does nothing.
				// The labelled button next to the title drives our own range state instead.
				tools: {
					download: false,
					selection: true,
					zoom: true,
					zoomin: true,
					zoomout: true,
					pan: false,
					reset: false
				}
			},
			events: {
				// A zoom is a window like any other, so it becomes the range rather than
				// something layered over it — which is what lets the dropdown say so.
				zoomed: (_c: unknown, o: { xaxis: { min?: number; max?: number } }) => {
					if (o.xaxis.min == null || o.xaxis.max == null) return;
					// A click apex read as a hairline selection would narrow the window to
					// almost nothing. Judged as a fraction of what is on screen rather than
					// by the drag flag, so this holds even where only mouse events arrive.
					const span = visible ? visible.max - visible.min : 0;
					if (span > 0 && o.xaxis.max - o.xaxis.min < span * 0.02) return;
					wealthState.range = { span: CUSTOM_SPAN, from: o.xaxis.min, to: o.xaxis.max };
				},
				// apex toggles the series itself; this only keeps our list in step with it
				legendClick: (_c: unknown, index: number) => {
					const name = series[index]?.name;
					if (!name) return;

					const next = hidden.includes(name) ? hidden.filter((n) => n !== name) : [...hidden, name];

					if (wealthState.split === 'account') wealthState.hiddenAccounts = next;
					else wealthState.hiddenAvailability = next;
				}
			}
		},
		// apex collapses a series marked `hidden` when it draws, which is how the whole set can
		// be hidden in one redraw. Asking it series by series costs one full redraw each — a few
		// dozen accounts froze the tab for seconds.
		series: series.map((s) => ({
			name: s.name,
			hidden: hidden.includes(s.name),
			data: dates.map((d, i) => [d, s.amounts[i] / 100])
		})),
		colors: series.map((s) => s.color),
		annotations: {
			xaxis: boundaries.map((x) => ({
				x,
				borderColor: '#cbd5e1',
				strokeDashArray: 3,
				opacity: 0.5
			}))
		},
		dataLabels: { enabled: false },
		stroke: { curve: 'smooth', width: 2 },
		// no dots on a phone: even at forty samples they sit close enough to thicken the line
		// rather than mark anything, and the hover marker still appears with the tooltip
		markers: { size: compact ? 0 : 3, strokeWidth: 0, hover: { size: 6 } },
		fill: { type: 'gradient', gradient: { opacityFrom: 0.4, opacityTo: 0.05 } },
		xaxis: { type: 'datetime', min: visible?.min, max: visible?.max },
		yaxis: { labels: { formatter: (v: number) => Math.round(v / 1000) + 'k €' } },
		tooltip: {
			x: { format: stepDays < 28 ? 'dd.MM.yyyy' : 'MMM yyyy' },
			// without this the tooltip inherits the axis formatter and reports "40k €"
			y: { formatter: (v: number) => euro(v * 100) }
		},
		legend: { show: wealthState.split !== 'total' }
	}) as ApexOptions;
</script>

<svelte:window bind:innerWidth={viewport} />

<div class="space-y-4">
	<Card class={ANALYSIS_CARD}>
		<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
			<div class="flex flex-wrap items-center gap-3">
				<h3 class="text-lg font-semibold">Vermögensverlauf</h3>
				{#if $historyStore.fetching}
					<Spinner size="4" />
				{/if}
			</div>
			<div class="flex flex-wrap items-center gap-2">
				{#if wealthState.split === 'account'}
					<SeriesToggle names={seriesNames} bind:hidden={wealthState.hiddenAccounts} />
				{:else if wealthState.split === 'availability'}
					<SeriesToggle names={seriesNames} bind:hidden={wealthState.hiddenAvailability} />
				{/if}
				<RangePicker bind:range={wealthState.range} />
				<ToggleGroup
					bind:value={wealthState.split}
					options={[
						{ value: 'total', label: 'Gesamt' },
						{ value: 'account', label: 'Nach Konto' },
						{ value: 'availability', label: 'Nach Verfügbarkeit' }
					]}
				/>
			</div>
		</div>
		<div bind:this={plotEl}>
			{#if $historyStore.errors}
				<QueryError
					errors={$historyStore.errors}
					message="Der Verlauf konnte nicht geladen werden."
				/>
			{:else}
				<Chart options={chartOptions} bind:instance={chartInstance} onUpdated={drawStichtag} />
			{/if}
		</div>
	</Card>

	<Label class="space-y-2">
		<span>Stichtag</span>
		<Input type="date" bind:value={wealthState.date} class="w-48" />
	</Label>

	<Card class={ANALYSIS_CARD}>
		<QueryError
			errors={$balanceStore.errors}
			message="Die Kontostände konnten nicht berechnet werden."
			class="mb-3"
		/>
		<!-- Everything below is hidden rather than shown empty: the total row sums whatever the
		     table has, so with no balances it asserts a wealth of 0,00 EUR. A number that is
		     wrong is worse than a number that is missing. -->
		{#if !$balanceStore.errors}
			<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
				<h3 class="text-lg font-semibold">Berechnete Kontostände</h3>
				<!-- A closed account keeps its last balance for ever, so it sits in this table at
			     whatever it held on the day it was closed and never moves again. Open ones only
			     by default; the total follows whichever is shown. -->
				{#if closedCount > 0}
					<ToggleGroup
						bind:value={wealthState.balanceAccounts}
						size="sm"
						options={[
							{ value: 'open', label: 'Aktive Konten' },
							{ value: 'all', label: 'Alle' }
						]}
					/>
				{/if}
			</div>
			<Table hoverable striped class="w-full text-left text-sm">
				<TableHead>
					<TableHeadCell class="px-4 py-3">Konto</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Stand</TableHeadCell>
					<!-- what the number was worked out from is the footnote, not the reading, and it is
				     what made the table wider than a phone -->
					<TableHeadCell class="hidden px-4 py-3 sm:table-cell">Berechnet aus</TableHeadCell>
				</TableHead>
				<TableBody class="divide-y">
					<!-- First, because it is the number the table exists to produce; the rows below are
				     how it was arrived at. The rule under it is what keeps `striped` from making
				     this read as an ordinary row that happens to be bold. -->
					<TableBodyRow class="border-b-2 border-gray-300 dark:border-gray-600">
						<TableBodyCell class="px-4 py-3 font-semibold">Gesamt</TableBodyCell>
						<TableBodyCell class="px-4 py-3 font-semibold">{euro(total)}</TableBodyCell>
						<TableBodyCell class="hidden px-4 py-3 sm:table-cell"></TableBodyCell>
					</TableBodyRow>
					{#each shownBalances as b (b.account.id)}
						<TableBodyRow>
							<!-- long account names wrap rather than widen the table; on a phone that is
						     the difference between two lines and a sideways scroll -->
							<TableBodyCell class="max-w-40 px-4 py-3 whitespace-normal sm:max-w-none">
								<span
									class="mr-2 inline-block h-3 w-3 rounded-full align-middle"
									style="background-color: {b.account.color ?? '#94a3b8'}"
								></span>{b.account.name}
							</TableBodyCell>
							<TableBodyCell
								class="px-4 py-3 {b.amount < 0
									? 'text-red-600 dark:text-red-400'
									: 'text-green-700 dark:text-green-400'}"
							>
								{euro(b.amount)}
							</TableBodyCell>
							<TableBodyCell
								class="hidden px-4 py-3 text-gray-500 sm:table-cell dark:text-gray-400"
							>
								{euro(b.reference.amount)} am {shortDate(b.reference.date)}{b.expenseCount > 0
									? ` + ${b.expenseCount} ${b.expenseCount === 1 ? 'Ausgabe' : 'Ausgaben'}`
									: ''}
							</TableBodyCell>
						</TableBodyRow>
					{/each}
				</TableBody>
			</Table>
		{/if}
	</Card>
</div>
