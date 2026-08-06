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
		Button,
		Spinner
	} from 'flowbite-svelte';
	import { RefreshOutline } from 'flowbite-svelte-icons';
	import { graphql } from '$houdini';
	import type ApexCharts from 'apexcharts';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import RangePicker from './RangePicker.svelte';
	import SeriesToggle from './SeriesToggle.svelte';
	import { AVAILABILITY_LABELS, AVAILABILITY_ORDER, availabilityColor } from './availability';
	import { wealthState } from './state.svelte';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const DAY = 86400000;

	// module-level, so switching tabs and coming back does not reset the view
	const presetRange = $derived.by(() => {
		const { from, to } = wealthState.range;
		return from && to ? { min: from, max: to } : null;
	});
	const visible = $derived(wealthState.zoom ?? presetRange);

	// a preset chooses the window outright, so a zoom taken before it is stale
	let lastSpan: string | undefined;
	$effect(() => {
		if (wealthState.range.span === lastSpan) return;
		lastSpan = wealthState.range.span;
		wealthState.zoom = null;
	});

	// The backend caps the point count, and asking for more than the chart can draw only
	// makes the response bigger; ~120 is what fits across a screen.
	const TARGET_POINTS = 120;
	const stepDays = $derived(
		visible ? Math.max(1, Math.ceil((visible.max - visible.min) / DAY / TARGET_POINTS)) : 30
	);

	const zoomedIn = $derived(wealthState.zoom !== null);

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
						availability
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

	const balances = $derived($balanceStore.data?.analysis.balances ?? []);

	const total = $derived(balances.reduce((sum, b) => sum + b.amount, 0));

	const availabilityTotals = $derived(
		AVAILABILITY_ORDER.map((availability) => ({
			availability,
			label: AVAILABILITY_LABELS[availability],
			amount: balances
				.filter((b) => b.account.availability === availability)
				.reduce((sum, b) => sum + b.amount, 0)
		})).filter((a) => a.amount !== 0)
	);

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
				color: '#f05a28',
				amounts: dates.map((_, i) => loaded.byAccount.reduce((sum, s) => sum + s.amounts[i], 0))
			}
		];
	});

	let chartInstance: ApexCharts | null = $state(null);
	const seriesNames = $derived(series.map((s) => s.name));

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
		// accessibility complaint the date input below already answers
		plotEl.addEventListener('click', pickDate, true);

		return () => {
			for (const name of events) {
				window.removeEventListener(name, name.endsWith('down') ? down : up, true);
			}
			plotEl.removeEventListener('click', pickDate, true);
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
				zoomed: (_c: unknown, o: { xaxis: { min?: number; max?: number } }) => {
					if (o.xaxis.min == null || o.xaxis.max == null) {
						wealthState.zoom = null;
						return;
					}
					// A click apex read as a hairline selection would narrow the window to
					// almost nothing. Judged as a fraction of what is on screen rather than
					// by the drag flag, so this holds even where only mouse events arrive.
					const span = visible ? visible.max - visible.min : 0;
					if (span > 0 && o.xaxis.max - o.xaxis.min < span * 0.02) return;
					wealthState.zoom = { min: o.xaxis.min, max: o.xaxis.max };
				},
				// apex toggles the series itself; this only keeps our list in step with it
				legendClick: (_c: unknown, index: number) => {
					const name = series[index]?.name;
					if (!name) return;
					wealthState.hiddenSeries = wealthState.hiddenSeries.includes(name)
						? wealthState.hiddenSeries.filter((n) => n !== name)
						: [...wealthState.hiddenSeries, name];
				}
			}
		},
		series: series.map((s) => ({
			name: s.name,
			data: dates.map((d, i) => [d, s.amounts[i] / 100])
		})),
		colors: series.map((s) => s.color),
		annotations: {
			xaxis: [
				...boundaries.map((x) => ({
					x,
					borderColor: '#cbd5e1',
					strokeDashArray: 3,
					opacity: 0.5
				})),
				{
					x: new Date(wealthState.date).getTime(),
					borderColor: '#f05a28',
					strokeDashArray: 0,
					label: { text: 'Stichtag', style: { background: '#f05a28', color: '#fff' } }
				}
			]
		},
		dataLabels: { enabled: false },
		stroke: { curve: 'smooth', width: 2 },
		markers: { size: 3, strokeWidth: 0, hover: { size: 6 } },
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

<div class="space-y-4">
	<Card class="max-w-none p-4">
		<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
			<div class="flex flex-wrap items-center gap-3">
				<h3 class="text-lg font-semibold">Vermögensverlauf</h3>
				{#if $historyStore.fetching}
					<Spinner size="4" />
				{/if}
				{#if zoomedIn}
					<Button size="xs" color="alternative" onclick={() => (wealthState.zoom = null)}>
						<RefreshOutline class="mr-1 h-3 w-3" />Zoom zurücksetzen
					</Button>
				{/if}
			</div>
			<div class="flex flex-wrap items-center gap-2">
				{#if wealthState.split !== 'total'}
					<SeriesToggle
						instance={chartInstance}
						names={seriesNames}
						bind:hidden={wealthState.hiddenSeries}
					/>
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
				<p class="p-4 text-sm text-red-600 dark:text-red-400">
					{$historyStore.errors.map((e) => e.message).join(', ')}
				</p>
			{:else}
				<Chart options={chartOptions} bind:instance={chartInstance} />
			{/if}
		</div>
	</Card>

	<div class="flex flex-wrap items-end gap-4">
		<Label class="space-y-2">
			<span>Stichtag</span>
			<Input type="date" bind:value={wealthState.date} class="w-48" />
		</Label>
		<p class="pb-2 text-sm text-gray-500 dark:text-gray-400">
			gilt für die Verfügbarkeiten und die berechneten Kontostände
		</p>
	</div>

	<div class="grid gap-4 md:grid-cols-4">
		{#each availabilityTotals as a (a.availability)}
			<Card class="max-w-none p-4">
				<span class="text-sm text-gray-500 dark:text-gray-400">Verfügbar: {a.label}</span>
				<span class="text-2xl font-semibold">{euro(a.amount)}</span>
			</Card>
		{/each}
	</div>

	<Card class="max-w-none p-4">
		<h3 class="mb-2 text-lg font-semibold">Berechnete Kontostände</h3>
		<Table hoverable striped class="w-full text-left text-sm">
			<TableHead>
				<TableHeadCell class="px-4 py-3">Konto</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Stand</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Berechnet aus</TableHeadCell>
			</TableHead>
			<TableBody class="divide-y">
				{#each balances as b (b.account.id)}
					<TableBodyRow>
						<TableBodyCell class="px-4 py-3">
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
						<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
							{euro(b.reference.amount)} am {b.reference.date.toLocaleDateString(
								'de-DE'
							)}{b.expenseCount > 0
								? ` + ${b.expenseCount} ${b.expenseCount === 1 ? 'Ausgabe' : 'Ausgaben'}`
								: ''}
						</TableBodyCell>
					</TableBodyRow>
				{/each}
				<TableBodyRow>
					<TableBodyCell class="px-4 py-3 font-semibold">Gesamt</TableBodyCell>
					<TableBodyCell class="px-4 py-3 font-semibold">{euro(total)}</TableBodyCell>
					<TableBodyCell class="px-4 py-3"></TableBodyCell>
				</TableBodyRow>
			</TableBody>
		</Table>
	</Card>
</div>
