<script lang="ts">
	import {
		Card,
		Button,
		Badge,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Alert
	} from 'flowbite-svelte';
	import { ChevronLeftOutline, ChevronRightOutline, ArrowLeftOutline } from 'flowbite-svelte-icons';
	import type { ApexOptions } from 'apexcharts';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { monthDetail } from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const year = $derived(Number(page.params.year));
	const month = $derived(Number(page.params.month));
	const detail = $derived(monthDetail(year, month));

	let sign = $state('expenses');

	const shift = (n: number) => {
		const d = new Date(year, month - 1 + n, 1);
		goto(
			resolve('/analysis/[year]/[month]', {
				year: `${d.getFullYear()}`,
				month: `${d.getMonth() + 1}`
			})
		);
	};

	const treemapOptions = $derived({
		chart: { type: 'treemap', height: 380, toolbar: { show: false } },
		series: detail.byCategory.map((c) => ({
			name: c.name,
			data: c.children.map((e) => ({
				x: e.title,
				y: (sign === 'expenses' ? e.amount : Math.round(e.amount * 0.4)) / 100
			}))
		})),
		colors: detail.byCategory.map((c) => c.color),
		legend: { show: true, position: 'bottom' },
		dataLabels: { enabled: true, style: { fontSize: '12px' } },
		plotOptions: { treemap: { distributed: false, enableShades: true } }
	}) as ApexOptions;

	const error = $derived(Math.abs(detail.endTotalApprox - detail.endTotal));
	const errorColor = (e: number) => (e >= 5000 ? 'red' : e >= 2500 ? 'yellow' : 'green');
</script>

<div class="space-y-4 pr-8 pl-8">
	<Alert color="yellow">
		<span class="font-medium">Mock:</span> alle Zahlen auf dieser Seite sind erfunden.
	</Alert>

	<div class="flex flex-wrap items-center gap-3">
		<Button color="alternative" size="sm" href={resolve('/analysis')}>
			<ArrowLeftOutline class="mr-2 h-4 w-4" />Auswertung
		</Button>
		<Button color="alternative" size="sm" onclick={() => shift(-1)}>
			<ChevronLeftOutline class="h-4 w-4" />
		</Button>
		<h2 class="text-xl font-semibold">
			{detail.date.toLocaleDateString('de-DE', { month: 'long', year: 'numeric' })}
		</h2>
		<Button color="alternative" size="sm" onclick={() => shift(1)}>
			<ChevronRightOutline class="h-4 w-4" />
		</Button>
	</div>

	<div class="grid gap-4 md:grid-cols-4">
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Einnahmen</span>
			<span class="text-2xl font-semibold text-green-700 dark:text-green-400"
				>{euro(detail.income)}</span
			>
		</Card>
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Ausgaben</span>
			<span class="text-2xl font-semibold text-red-600 dark:text-red-400"
				>{euro(detail.expenses)}</span
			>
			<span class="text-xs text-gray-500 dark:text-gray-400">{detail.expenseCount} Stück</span>
		</Card>
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Delta</span>
			<span class="text-2xl font-semibold">{euro(detail.endTotal - detail.startTotal)}</span>
		</Card>
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Vermögen am Ende</span>
			<span class="text-2xl font-semibold">{euro(detail.endTotal)}</span>
			<span class="text-xs text-gray-500 dark:text-gray-400">
				Integrationsfehler <Badge color={errorColor(error)}>{euro(error)}</Badge>
			</span>
		</Card>
	</div>

	<Card class="max-w-none p-4">
		<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
			<h3 class="text-lg font-semibold">Nach Kategorie</h3>
			<ToggleGroup
				bind:value={sign}
				options={[
					{ value: 'expenses', label: 'Ausgaben' },
					{ value: 'income', label: 'Einnahmen' }
				]}
			/>
		</div>
		<Chart options={treemapOptions} />
	</Card>

	<div class="grid gap-4 lg:grid-cols-2">
		<Card class="max-w-none p-4">
			<h3 class="mb-2 text-lg font-semibold">Konten</h3>
			<Table hoverable striped class="w-full text-left text-sm">
				<TableHead>
					<TableHeadCell class="px-4 py-3">Konto</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Beginn</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Ende</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Delta</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Fehler</TableHeadCell>
				</TableHead>
				<TableBody class="divide-y">
					{#each detail.byAccount as a (a.id)}
						<TableBodyRow>
							<TableBodyCell class="px-4 py-3">
								<span
									class="mr-2 inline-block h-3 w-3 rounded-full align-middle"
									style="background-color: {a.color}"
								></span>{a.name}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3">{euro(a.start)}</TableBodyCell>
							<TableBodyCell class="px-4 py-3">{euro(a.end)}</TableBodyCell>
							<TableBodyCell
								class="px-4 py-3 {a.delta < 0
									? 'text-red-600 dark:text-red-400'
									: 'text-green-700 dark:text-green-400'}"
							>
								{euro(a.delta)}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3">
								<Badge color={errorColor(Math.abs(a.endApprox - a.end))}>
									{euro(Math.abs(a.endApprox - a.end))}
								</Badge>
							</TableBodyCell>
						</TableBodyRow>
					{/each}
				</TableBody>
			</Table>
		</Card>

		<Card class="max-w-none p-4">
			<h3 class="mb-2 text-lg font-semibold">Größte Ausgaben</h3>
			<Table hoverable striped class="w-full text-left text-sm">
				<TableHead>
					<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Betrag</TableHeadCell>
				</TableHead>
				<TableBody class="divide-y">
					{#each detail.biggest as e (e.id)}
						<TableBodyRow>
							<TableBodyCell class="px-4 py-3">{e.title}</TableBodyCell>
							<TableBodyCell class="px-4 py-3 text-red-600 dark:text-red-400"
								>{euro(e.amount)}</TableBodyCell
							>
						</TableBodyRow>
					{/each}
				</TableBody>
			</Table>
		</Card>
	</div>
</div>
