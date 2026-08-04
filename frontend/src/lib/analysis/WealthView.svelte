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
		Label
	} from 'flowbite-svelte';
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

	const total = $derived(availabilityTotals.reduce((a, b) => a + b.amount, 0));

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
		chart: { type: 'area', height: 340, toolbar: { show: false }, stacked: split !== 'total' },
		series: series.map((s) => ({
			name: s.name,
			data: historyDates.map((d, i) => [d, s.amounts[i]])
		})),
		colors: series.map((s) => s.color),
		dataLabels: { enabled: false },
		stroke: { curve: 'smooth', width: 2 },
		fill: { type: 'gradient', gradient: { opacityFrom: 0.4, opacityTo: 0.05 } },
		xaxis: { type: 'datetime' },
		yaxis: { labels: { formatter: (v: number) => Math.round(v / 1000) + 'k €' } },
		tooltip: { x: { format: 'MMM yyyy' } },
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
			<h3 class="text-lg font-semibold">Vermögensverlauf</h3>
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
							{euro(b.referenceAmount)} am {b.referenceDate.toLocaleDateString('de-DE')},
							{b.expenseCount === 1 ? '1 Ausgabe' : `${b.expenseCount} Ausgaben`} seitdem
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
