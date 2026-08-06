<script lang="ts">
	import { Card, Spinner } from 'flowbite-svelte';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import RangePicker from './RangePicker.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { overviewsStore, monthLabel, categoryColor } from './overviews';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	let span = $state('all');
	let from: Date | undefined = $state();
	let to: Date | undefined = $state();
	let sign = $state('expenses');

	// undefined until the picker knows what "Alles" means
	$effect(() => {
		if (!from || !to) return;
		void overviewsStore.fetch({ variables: { from, to } });
	});

	const overviews = $derived($overviewsStore.data?.analysis.overviews);
	const months = $derived(overviews?.months ?? []);
	const breakdown = $derived(overviews?.byCategory ?? []);

	const pick = (t: { income: number; expenses: number }) =>
		sign === 'expenses' ? t.expenses : t.income;
	const name = (c: { name: string } | null) => c?.name ?? 'Ohne Kategorie';

	// The bars are stacked per month, so every category needs a value at every month even
	// where it has none — an absent entry would shift the stack rather than leave a gap.
	const bars = $derived.by(() => {
		const keys = [...new Set(months.flatMap((m) => m.byCategory.map((c) => c.category?.id ?? '')))];

		return keys.map((key) => {
			const anywhere = months
				.flatMap((m) => m.byCategory)
				.find((c) => (c.category?.id ?? '') === key);

			return {
				name: name(anywhere?.category ?? null),
				color: categoryColor(anywhere?.category ?? null),
				data: months.map((m) => {
					const found = m.byCategory.find((c) => (c.category?.id ?? '') === key);
					return found ? pick(found) / 100 : 0;
				})
			};
		});
	});

	const barOptions = $derived({
		chart: { type: 'bar', height: 340, toolbar: { show: false }, stacked: true },
		series: bars.map((c) => ({ name: c.name, data: c.data })),
		colors: bars.map((c) => c.color),
		dataLabels: { enabled: false },
		xaxis: { categories: months.map((m) => monthLabel(m.month)) },
		yaxis: { labels: { formatter: (v: number) => Math.round(v) + ' €' } },
		// the tooltip inherits the axis formatter otherwise, which rounds away the cents
		tooltip: { y: { formatter: (v: number) => euro(v * 100) } },
		legend: { position: 'bottom' }
	} as ApexOptions);

	// a treemap rather than MoneyDB's sunburst: same category -> expense hierarchy, but
	// readable at the sizes where a sunburst's outer ring becomes unlabelable slivers
	const treemapOptions = $derived.by(() => {
		const series = breakdown
			.map((c) => {
				const children = c.biggest
					.filter((e) => (sign === 'expenses' ? e.amount < 0 : e.amount > 0))
					.map((e) => ({ x: e.title, y: Math.abs(e.amount) / 100 }));

				// what `biggest` left out, as one slice, so the areas still add up to the total
				const rest = Math.abs(sign === 'expenses' ? c.restExpenses : c.restIncome);
				if (rest > 0) children.push({ x: 'Rest', y: rest / 100 });

				return { name: name(c.category), color: categoryColor(c.category), data: children };
			})
			.filter((s) => s.data.length > 0);

		return {
			chart: { type: 'treemap', height: 420, toolbar: { show: false } },
			series: series.map((s) => ({ name: s.name, data: s.data })),
			colors: series.map((s) => s.color),
			legend: { show: true, position: 'bottom' },
			dataLabels: { enabled: true, style: { fontSize: '12px' } },
			plotOptions: { treemap: { distributed: false, enableShades: true } },
			tooltip: { y: { formatter: (v: number) => euro(v * 100) } }
		} as ApexOptions;
	});
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<RangePicker bind:value={span} bind:from bind:to />
		<ToggleGroup
			bind:value={sign}
			options={[
				{ value: 'expenses', label: 'Ausgaben' },
				{ value: 'income', label: 'Einnahmen' }
			]}
		/>
		{#if $overviewsStore.fetching}
			<Spinner size="5" />
		{/if}
	</div>

	{#if $overviewsStore.errors}
		<p class="text-sm text-red-600 dark:text-red-400">
			{$overviewsStore.errors.map((e) => e.message).join(', ')}
		</p>
	{:else}
		<Card class="max-w-none p-4">
			<h3 class="mb-2 text-lg font-semibold">Nach Kategorie je Monat</h3>
			<Chart options={barOptions} />
		</Card>

		<Card class="max-w-none p-4">
			<h3 class="text-lg font-semibold">Gesamter Zeitraum</h3>
			<p class="mb-2 text-sm text-gray-500 dark:text-gray-400">
				Eine auf mehrere Kategorien verteilte Ausgabe zählt anteilig in jede.
			</p>
			<Chart options={treemapOptions} />
		</Card>
	{/if}
</div>
