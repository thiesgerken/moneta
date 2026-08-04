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
		Button
	} from 'flowbite-svelte';
	import { RefreshOutline } from 'flowbite-svelte-icons';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import {
		availabilityTotals,
		computedBalances,
		historyByAccount,
		historyByAvailability,
		historyDates,
		historyTotal
	} from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	let date = $state(new Date().toISOString().slice(0, 10));
	let split = $state('total');

	// null = full range. Set from apex's zoom event so the resolution can follow it.
	let range: { min: number; max: number } | null = $state(null);

	const total = $derived(availabilityTotals.reduce((a, b) => a + b.amount, 0));

	// The visible window decides the step, so zooming in exposes finer detail instead of
	// magnifying the same monthly points. The real endpoint gets asked for this resolution
	// rather than shipping every day and thinning it here.
	const TARGET_POINTS = 120;

	const visible = $derived.by(() => {
		const zoom: { min: number; max: number } | null = range;
		const min = zoom ? zoom.min : historyDates[0];
		const max = zoom ? zoom.max : historyDates[historyDates.length - 1];
		const indices = historyDates
			.map((d, i) => ({ d, i }))
			.filter(({ d }) => d >= min && d <= max)
			.map(({ i }) => i);
		const step = Math.max(1, Math.ceil(indices.length / TARGET_POINTS));
		// always keep the last point, otherwise the line stops short of the range end
		const picked = indices.filter((_, k) => k % step === 0);
		const last = indices.at(-1);
		if (last !== undefined && picked.at(-1) !== last) picked.push(last);
		return { indices: picked, step, min, max };
	});

	// Faint verticals on month starts to give the eye something to measure against. Beyond
	// about 18 months there are too many to stay subtle, so they fall back to year starts.
	const boundaries = $derived.by(() => {
		const yearly = visible.max - visible.min > 550 * 86400000;
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

	const series = $derived.by(() => {
		if (split === 'account')
			return historyByAccount.map((a) => ({
				name: a.name,
				color: a.color,
				amounts: a.amounts
			}));
		if (split === 'availability')
			return historyByAvailability.map((a) => ({
				name: a.label,
				// availability has no colour of its own; a ramp keeps the stack ordered
				// from liquid at the bottom to illiquid at the top
				color: ['#f05a28', '#f0a428', '#7db35b', '#4a90c2', '#8e6cbf', '#bf6c8e'][
					historyByAvailability.indexOf(a)
				],
				amounts: a.amounts
			}));
		return [{ name: 'Gesamt', color: '#f05a28', amounts: historyTotal }];
	});

	const chartOptions = $derived({
		chart: {
			type: 'area',
			height: 340,
			stacked: split !== 'total',
			zoom: { enabled: true, type: 'x', autoScaleYaxis: true },
			toolbar: {
				show: true,
				tools: {
					download: false,
					selection: true,
					zoom: true,
					zoomin: true,
					zoomout: true,
					pan: false,
					reset: true
				}
			},
			events: {
				zoomed: (_c: unknown, o: { xaxis: { min?: number; max?: number } }) => {
					range =
						o.xaxis.min == null || o.xaxis.max == null
							? null
							: { min: o.xaxis.min, max: o.xaxis.max };
				},
				beforeResetZoom: () => {
					range = null;
				}
			}
		},
		series: series.map((s) => ({
			name: s.name,
			data: visible.indices.map((i) => [historyDates[i], s.amounts[i]])
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
			x: { format: visible.step < 28 ? 'dd.MM.yyyy' : 'MMM yyyy' },
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
				{#if range}
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
		<Chart options={chartOptions} />
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
				{#each computedBalances as b (b.id)}
					<TableBodyRow>
						<TableBodyCell class="px-4 py-3">
							<span
								class="mr-2 inline-block h-3 w-3 rounded-full align-middle"
								style="background-color: {b.color}"
							></span>{b.name}
						</TableBodyCell>
						<TableBodyCell
							class="px-4 py-3 {b.amount < 0
								? 'text-red-600 dark:text-red-400'
								: 'text-green-700 dark:text-green-400'}"
						>
							{euro(b.amount)}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
							{euro(b.referenceAmount)} am {b.referenceDate.toLocaleDateString(
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
