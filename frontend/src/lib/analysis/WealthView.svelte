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
		Toggle
	} from 'flowbite-svelte';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import {
		accounts,
		availabilityTotals,
		computedBalances,
		historyByAccount,
		historyDates,
		historyTotal
	} from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	let date = $state(new Date().toISOString().slice(0, 10));
	let perAccount = $state(false);

	const total = $derived(availabilityTotals.reduce((a, b) => a + b.amount, 0));

	const chartOptions = $derived({
		chart: { type: 'area', height: 340, toolbar: { show: false }, stacked: perAccount },
		series: perAccount
			? historyByAccount.map((a) => ({
					name: a.name,
					data: historyDates.map((d, i) => [d, a.amounts[i]])
				}))
			: [{ name: 'Gesamt', data: historyDates.map((d, i) => [d, historyTotal[i]]) }],
		colors: perAccount ? accounts.map((a) => a.color) : ['#f05a28'],
		dataLabels: { enabled: false },
		stroke: { curve: 'smooth', width: 2 },
		fill: { type: 'gradient', gradient: { opacityFrom: 0.4, opacityTo: 0.05 } },
		xaxis: { type: 'datetime' },
		yaxis: { labels: { formatter: (v: number) => Math.round(v / 1000) + 'k €' } },
		tooltip: { x: { format: 'MMM yyyy' } },
		legend: { show: perAccount }
	}) as ApexOptions;
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<Label class="space-y-2">
			<span>Stichtag</span>
			<Input type="date" bind:value={date} class="w-48" />
		</Label>
		<Toggle bind:checked={perAccount}>Nach Konten aufschlüsseln</Toggle>
	</div>

	<Card class="max-w-none p-4">
		<h3 class="mb-2 text-lg font-semibold">Vermögensverlauf</h3>
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
				<TableHeadCell class="px-4 py-3">Basis</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Seitdem</TableHeadCell>
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
							{euro(b.referenceAmount)} am {b.referenceDate.toLocaleDateString('de-DE')}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
							{b.expenseCount} Ausgaben
						</TableBodyCell>
					</TableBodyRow>
				{/each}
				<TableBodyRow>
					<TableBodyCell class="px-4 py-3 font-semibold">Gesamt</TableBodyCell>
					<TableBodyCell class="px-4 py-3 font-semibold">{euro(total)}</TableBodyCell>
					<TableBodyCell class="px-4 py-3"></TableBodyCell>
					<TableBodyCell class="px-4 py-3"></TableBodyCell>
				</TableBodyRow>
			</TableBody>
		</Table>
	</Card>
</div>
