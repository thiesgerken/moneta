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
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { AVAILABILITY_LABELS, AVAILABILITY_ORDER, availabilityColor } from './availability';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const DAY = 86400000;

	let date = $state(new Date().toISOString().slice(0, 10));
	let split = $state('total');

	// The window the chart is showing. Zooming rewrites it, which is what makes the query
	// come back at a different resolution rather than magnifying the points already drawn.
	const defaultRange = { min: Date.now() - 730 * DAY, max: Date.now() };
	let range: { min: number; max: number } | null = $state(null);
	const visible = $derived(range ?? defaultRange);

	// The backend caps the point count, and asking for more than the chart can draw only
	// makes the response bigger; ~120 is what fits across a screen.
	const TARGET_POINTS = 120;
	const stepDays = $derived(
		Math.max(1, Math.ceil((visible.max - visible.min) / DAY / TARGET_POINTS))
	);

	const zoomedIn = $derived(range !== null);

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
	// inside it is what makes zooming and the Stichtag re-run them
	$effect(() => {
		void historyStore.fetch({
			variables: {
				from: new Date(visible.min),
				to: new Date(visible.max),
				stepDays
			}
		});
	});

	$effect(() => {
		void balanceStore.fetch({ variables: { date: new Date(date) } });
	});

	const history = $derived($historyStore.data?.analysis.history);
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
		if (!history) return [];
		if (split === 'account')
			return history.byAccount.map((s) => ({
				name: s.account.name,
				color: s.account.color ?? '#94a3b8',
				amounts: s.amounts
			}));
		if (split === 'availability')
			return history.byAvailability.map((s) => ({
				name: AVAILABILITY_LABELS[s.availability],
				color: availabilityColor(s.availability),
				amounts: s.amounts
			}));
		return [
			{
				name: 'Gesamt',
				color: '#f05a28',
				amounts: dates.map((_, i) => history.byAccount.reduce((sum, s) => sum + s.amounts[i], 0))
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
			<ToggleGroup
				bind:value={split}
				options={[
					{ value: 'total', label: 'Gesamt' },
					{ value: 'account', label: 'Nach Konto' },
					{ value: 'availability', label: 'Nach Verfügbarkeit' }
				]}
			/>
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
