<script lang="ts">
	import {
		Card,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Badge
	} from 'flowbite-svelte';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import { overviews } from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const chartOptions = {
		chart: { type: 'bar', height: 300, toolbar: { show: false }, stacked: true },
		series: [
			{ name: 'Einnahmen', data: overviews.map((o) => o.income / 100) },
			{ name: 'Ausgaben', data: overviews.map((o) => o.expenses / 100) }
		],
		colors: ['#16a34a', '#dc2626'],
		dataLabels: { enabled: false },
		xaxis: {
			categories: overviews.map((o) =>
				o.month.toLocaleDateString('de-DE', { month: 'short', year: '2-digit' })
			)
		},
		yaxis: { labels: { formatter: (v: number) => Math.round(v) + ' €' } },
		legend: { position: 'top' }
	} as ApexOptions;

	// how far the integrated end balance drifts from the measured one; a large gap means
	// an expense is missing rather than that the arithmetic is wrong
	const error = (o: (typeof overviews)[number]) => Math.abs(o.endTotalApprox - o.endTotal);
	const errorColor = (e: number) => (e >= 5000 ? 'red' : e >= 2500 ? 'yellow' : 'green');
</script>

<div class="space-y-4">
	<Card class="max-w-none p-4">
		<h3 class="mb-2 text-lg font-semibold">Einnahmen und Ausgaben je Monat</h3>
		<Chart options={chartOptions} />
	</Card>

	<Card class="max-w-none p-4">
		<h3 class="mb-2 text-lg font-semibold">Monatliche Übersichten</h3>
		<Table hoverable striped class="w-full text-left text-sm">
			<TableHead>
				<TableHeadCell class="px-4 py-3">Monat</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Vermögen Beginn</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Vermögen Ende</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Einnahmen</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Ausgaben</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Delta</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Anzahl</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Fehler</TableHeadCell>
			</TableHead>
			<TableBody class="divide-y">
				{#each overviews.toReversed() as o (o.month.getTime())}
					<TableBodyRow>
						<TableBodyCell class="px-4 py-3">
							{o.month.toLocaleDateString('de-DE', { month: 'long', year: 'numeric' })}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3">{euro(o.startTotal)}</TableBodyCell>
						<TableBodyCell class="px-4 py-3">{euro(o.endTotal)}</TableBodyCell>
						<TableBodyCell class="px-4 py-3 text-green-700 dark:text-green-400"
							>{euro(o.income)}</TableBodyCell
						>
						<TableBodyCell class="px-4 py-3 text-red-600 dark:text-red-400"
							>{euro(o.expenses)}</TableBodyCell
						>
						<TableBodyCell
							class="px-4 py-3 {o.endTotal - o.startTotal < 0
								? 'text-red-600 dark:text-red-400'
								: 'text-green-700 dark:text-green-400'}"
						>
							{euro(o.endTotal - o.startTotal)}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3">{o.expenseCount}</TableBodyCell>
						<TableBodyCell class="px-4 py-3">
							<Badge color={errorColor(error(o))}>{euro(error(o))}</Badge>
						</TableBodyCell>
					</TableBodyRow>
				{/each}
			</TableBody>
		</Table>
	</Card>
</div>
