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
		Select,
		Spinner
	} from 'flowbite-svelte';
	import { RefreshOutline } from 'flowbite-svelte-icons';
	import { graphql } from '$houdini';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { AVAILABILITY_LABELS, AVAILABILITY_ORDER, availabilityColor } from './availability';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const DAY = 86400000;

	let date = $state(new Date().toISOString().slice(0, 10));
	let split = $state('total');
	let span = $state('all');

	const SPANS = [
		{ value: '6m', name: 'Letzte 6 Monate', months: 6 },
		{ value: '1y', name: 'Letztes Jahr', months: 12 },
		{ value: '5y', name: 'Letzte 5 Jahre', months: 60 },
		{ value: '10y', name: 'Letzte 10 Jahre', months: 120 },
		{ value: 'all', name: 'Alles', months: null }
	];

	// Where the data starts, so "Alles" means the whole history rather than a guess. Sorted by
	// booking start rather than by a transaction date, which is not a sort the list offers --
	// for an imported expense the two are the same day anyway.
	const firstExpenseStore = graphql(`
		query AnalysisFirstExpense {
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
	const dataStartKnown = $derived($firstExpenseStore.data != null);

	const presetRange = $derived.by(() => {
		const max = Date.now();
		const months = SPANS.find((s) => s.value === span)?.months ?? null;
		if (months === null) return { min: dataStart ?? max - 365 * DAY, max };

		// built rather than mutated: a negative month rolls into the previous year on its own
		const now = new Date(max);
		const from = new Date(now.getFullYear(), now.getMonth() - months, now.getDate());
		return { min: from.getTime(), max };
	});

	// Zooming overrides the preset until it is reset or the preset changes.
	let range: { min: number; max: number } | null = $state(null);
	const visible = $derived(range ?? presetRange);

	// The backend caps the point count, and asking for more than the chart can draw only
	// makes the response bigger; ~120 is what fits across a screen.
	const TARGET_POINTS = 120;
	const stepDays = $derived(
		Math.max(1, Math.ceil((visible.max - visible.min) / DAY / TARGET_POINTS))
	);

	const zoomedIn = $derived(range !== null);

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
		// "Alles" cannot ask for anything sensible until it knows where the data starts, and
		// guessing a window meanwhile means two queries racing on every page load
		if (span === 'all' && !dataStartKnown) return;

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
		void balanceStore.fetch({ variables: { date: new Date(date) } });
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
		if (split === 'account')
			return loaded.byAccount.map((s) => ({
				name: s.account.name,
				color: s.account.color ?? '#94a3b8',
				amounts: s.amounts
			}));
		if (split === 'availability')
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

	// Faint verticals on month starts to give the eye something to measure against. Beyond
	// about 18 months there are too many to stay subtle, so they fall back to year starts.
	const boundaries = $derived.by(() => {
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

	const chartOptions = $derived({
		chart: {
			type: 'area',
			height: 340,
			stacked: split !== 'total',
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
					range =
						o.xaxis.min == null || o.xaxis.max == null
							? null
							: { min: o.xaxis.min, max: o.xaxis.max };
				}
			}
		},
		series: series.map((s) => ({
			name: s.name,
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
		markers: { size: 3, strokeWidth: 0, hover: { size: 6 } },
		fill: { type: 'gradient', gradient: { opacityFrom: 0.4, opacityTo: 0.05 } },
		xaxis: { type: 'datetime', min: visible.min, max: visible.max },
		yaxis: { labels: { formatter: (v: number) => Math.round(v / 1000) + 'k €' } },
		tooltip: {
			x: { format: stepDays < 28 ? 'dd.MM.yyyy' : 'MMM yyyy' },
			// without this the tooltip inherits the axis formatter and reports "40k €"
			y: { formatter: (v: number) => euro(v * 100) }
		},
		legend: { show: split !== 'total' }
	}) as ApexOptions;
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<Label class="space-y-2">
			<span>Stichtag</span>
			<Input type="date" bind:value={date} class="w-48" />
		</Label>
	</div>

	<Card class="max-w-none p-4">
		<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
			<div class="flex flex-wrap items-center gap-3">
				<h3 class="text-lg font-semibold">Vermögensverlauf</h3>
				{#if $historyStore.fetching}
					<Spinner size="4" />
				{/if}
				{#if zoomedIn}
					<Button size="xs" color="alternative" onclick={() => (range = null)}>
						<RefreshOutline class="mr-1 h-3 w-3" />Zoom zurücksetzen
					</Button>
				{/if}
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<Select
					value={span}
					onchange={(e) => {
						// a preset picks the window outright, so a zoom that predates it is stale
						span = (e.target as HTMLSelectElement).value;
						range = null;
					}}
					items={SPANS.map((s) => ({ value: s.value, name: s.name }))}
					size="sm"
					class="w-44"
				/>
				<ToggleGroup
					bind:value={split}
					options={[
						{ value: 'total', label: 'Gesamt' },
						{ value: 'account', label: 'Nach Konto' },
						{ value: 'availability', label: 'Nach Verfügbarkeit' }
					]}
				/>
			</div>
		</div>
		{#if $historyStore.errors}
			<p class="p-4 text-sm text-red-600 dark:text-red-400">
				{$historyStore.errors.map((e) => e.message).join(', ')}
			</p>
		{:else}
			<Chart options={chartOptions} />
		{/if}
	</Card>

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
