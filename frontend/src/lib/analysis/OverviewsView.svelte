<script lang="ts">
	import { ChartOutline, TableColumnOutline } from 'flowbite-svelte-icons';
	import {
		Card,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Badge,
		Spinner,
		Tooltip
	} from 'flowbite-svelte';
	import type { ApexOptions } from 'apexcharts';
	import { resolve } from '$app/paths';
	import Chart from '$lib/Chart.svelte';
	import RangePicker from './RangePicker.svelte';
	import {
		overviewsStore,
		integrationError,
		errorClass,
		deltaColor,
		monthLabel,
		monthTicks,
		monthDrilldown,
		type OverviewsData
	} from './overviews';
	import { overviewsState } from './state.svelte';
	import { ANALYSIS_CARD } from './cards';
	import QueryError from '$lib/QueryError.svelte';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	// Kept here rather than read off the store so the chart never sees an empty intermediate
	// state: houdini clears `data` while a fetch is in flight, which drew the chart once
	// empty and once full on every range change. Only the newest request may write.
	let latestRequest = 0;
	let overviews = $state<OverviewsData | null>(null);

	$effect(() => {
		// undefined until the picker has resolved its preset
		const { from, to } = overviewsState.range;
		if (!from || !to) return;

		const request = ++latestRequest;
		void overviewsStore
			.fetch({ variables: { from: new Date(from), to: new Date(to) } })
			.then((result) => {
				if (request !== latestRequest) return;
				overviews = result.data?.analysis.overviews ?? null;
			});
	});

	const months = $derived(overviews?.months ?? []);

	const chartOptions = $derived({
		chart: {
			type: 'bar',
			height: 300,
			toolbar: { show: false },
			stacked: true,
			events: monthDrilldown(months)
		},
		series: [
			{ name: 'Einnahmen', data: months.map((o) => o.income / 100) },
			{ name: 'Ausgaben', data: months.map((o) => o.expenses / 100) }
		],
		colors: ['#16a34a', '#dc2626'],
		dataLabels: { enabled: false },
		xaxis: {
			categories: months.map((o) => monthLabel(o.month)),
			tickAmount: monthTicks(months.length),
			labels: { rotate: -45, hideOverlappingLabels: true }
		},
		yaxis: { labels: { formatter: (v: number) => Math.round(v) + ' €' } },
		// the tooltip inherits the axis formatter otherwise, which rounds away the cents
		tooltip: { y: { formatter: (v: number) => euro(v * 100) } },
		legend: { position: 'top' }
	} as ApexOptions);
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<RangePicker bind:range={overviewsState.range} dates />
		{#if $overviewsStore.fetching}
			<Spinner size="5" />
		{/if}
	</div>

	{#if $overviewsStore.errors}
		<QueryError
			errors={$overviewsStore.errors}
			message="Die Monatsübersichten konnten nicht geladen werden."
		/>
	{:else}
		<Card class={ANALYSIS_CARD}>
			<h3 class="mb-2 text-lg font-semibold">
				<ChartOutline
					class="me-2 inline h-5 w-5 align-[-0.2em] text-gray-400 dark:text-gray-500"
				/>Einnahmen und Ausgaben je Monat
			</h3>
			<Chart options={chartOptions} />
		</Card>

		<Card class={ANALYSIS_CARD}>
			<h3 class="mb-2 text-lg font-semibold">
				<TableColumnOutline
					class="me-2 inline h-5 w-5 align-[-0.2em] text-gray-400 dark:text-gray-500"
				/>Monatliche Übersichten
			</h3>
			<div class="overflow-x-auto">
				<Table hoverable striped class="w-full text-left text-sm">
					<!-- What the month did comes before what it left behind: delta, then the two
					     sides it is made of, then the balances they moved between. On a phone the
					     columns behind Ausgaben are off the screen, and what is dropped there is
					     what answers a question nobody asks on a phone -- how many expenses it
					     took, where the month started, and how far the two ways of counting
					     disagree. -->
					<TableHead>
						<TableHeadCell class="px-4 py-3">Monat</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Delta</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Einnahmen</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Ausgaben</TableHeadCell>
						<TableHeadCell class="hidden px-4 py-3 sm:table-cell">Anzahl</TableHeadCell>
						<TableHeadCell class="hidden px-4 py-3 sm:table-cell">Vermögen Beginn</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Vermögen Ende</TableHeadCell>
						<TableHeadCell class="hidden px-4 py-3 sm:table-cell">
							<span class="underline decoration-dotted">Fehler</span>
							<Tooltip class="max-w-xs text-xs font-normal normal-case">
								Die Lücke zwischen dem fortgeschriebenen und dem gemessenen Endvermögen. Eine große
								Lücke heißt nicht, dass die Rechnung driftet, sondern dass eine Ausgabe fehlt.
							</Tooltip>
						</TableHeadCell>
					</TableHead>
					<TableBody class="divide-y">
						{#each months.toReversed() as o (o.month.getTime())}
							<TableBodyRow>
								<TableBodyCell class="px-4 py-3 whitespace-nowrap">
									<a
										class="text-primary-600 dark:text-primary-500 font-medium hover:underline"
										href={resolve('/analysis/[year]/[month]', {
											year: `${o.month.getUTCFullYear()}`,
											month: `${o.month.getUTCMonth() + 1}`
										})}
									>
										{o.month.toLocaleDateString('de-DE', {
											month: 'long',
											year: 'numeric',
											timeZone: 'UTC'
										})}
									</a>
								</TableBodyCell>
								<TableBodyCell class="px-4 py-3">
									<Badge
										color={deltaColor(o.endTotal - o.startTotal)}
										class="text-sm font-semibold"
									>
										{euro(o.endTotal - o.startTotal)}
									</Badge>
								</TableBodyCell>
								<TableBodyCell class="px-4 py-3 text-green-700 dark:text-green-400"
									>{euro(o.income)}</TableBodyCell
								>
								<TableBodyCell class="px-4 py-3 text-red-600 dark:text-red-400"
									>{euro(o.expenses)}</TableBodyCell
								>
								<TableBodyCell class="hidden px-4 py-3 sm:table-cell"
									>{o.expenseCount}</TableBodyCell
								>
								<TableBodyCell class="hidden px-4 py-3 sm:table-cell"
									>{euro(o.startTotal)}</TableBodyCell
								>
								<TableBodyCell class="px-4 py-3">{euro(o.endTotal)}</TableBodyCell>
								<TableBodyCell
									class="hidden px-4 py-3 sm:table-cell {errorClass(integrationError(o))}"
								>
									{euro(integrationError(o))}
								</TableBodyCell>
							</TableBodyRow>
						{/each}
					</TableBody>
				</Table>
			</div>
		</Card>
	{/if}
</div>

<style>
	/* apex draws the axis labels as plain text; nothing says they can be clicked */
	:global(.apexcharts-xaxis-label) {
		cursor: pointer;
	}
</style>
