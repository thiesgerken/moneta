<script lang="ts">
	import { Card } from 'flowbite-svelte';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { categoryTree, monthlyByCategory, overviews } from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	let sign = $state('expenses');

	const barOptions = $derived({
		chart: { type: 'bar', height: 340, toolbar: { show: false }, stacked: true },
		series: monthlyByCategory.map((c) => ({
			name: c.name,
			data: c.amounts.map((a) => (sign === 'expenses' ? a : Math.round(a * 0.4)) / 100)
		})),
		colors: monthlyByCategory.map((c) => c.color),
		dataLabels: { enabled: false },
		xaxis: {
			categories: overviews.map((o) =>
				o.month.toLocaleDateString('de-DE', { month: 'short', year: '2-digit' })
			)
		},
		yaxis: { labels: { formatter: (v: number) => Math.round(v) + ' €' } },
		// the tooltip inherits the axis formatter otherwise, which rounds away the cents
		tooltip: { y: { formatter: (v: number) => euro(v * 100) } },
		legend: { position: 'bottom' }
	}) as ApexOptions;

	// a treemap rather than MoneyDB's sunburst: same category -> expense hierarchy, but
	// readable at the sizes where a sunburst's outer ring becomes unlabelable slivers
	const treemapOptions = $derived({
		chart: { type: 'treemap', height: 420, toolbar: { show: false } },
		series: categoryTree.map((c) => ({
			name: c.name,
			data: c.children.map((e) => ({
				x: e.title,
				y: (sign === 'expenses' ? e.amount : Math.round(e.amount * 0.4)) / 100
			}))
		})),
		colors: categoryTree.map((c) => c.color),
		legend: { show: true, position: 'bottom' },
		dataLabels: { enabled: true, style: { fontSize: '12px' } },
		plotOptions: { treemap: { distributed: false, enableShades: true } },
		tooltip: { y: { formatter: (v: number) => euro(v * 100) } }
	}) as ApexOptions;
</script>

<div class="space-y-4">
	<ToggleGroup
		bind:value={sign}
		options={[
			{ value: 'expenses', label: 'Ausgaben' },
			{ value: 'income', label: 'Einnahmen' }
		]}
	/>

	<Card class="max-w-none p-4">
		<h3 class="mb-2 text-lg font-semibold">Nach Kategorie je Monat</h3>
		<Chart options={barOptions} />
	</Card>

	<Card class="max-w-none p-4">
		<h3 class="mb-2 text-lg font-semibold">Gesamter Zeitraum</h3>
		<Chart options={treemapOptions} />
	</Card>
</div>
