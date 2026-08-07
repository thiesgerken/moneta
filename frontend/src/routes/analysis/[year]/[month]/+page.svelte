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
		Spinner
	} from 'flowbite-svelte';
	import { ChevronLeftOutline, ChevronRightOutline, ArrowLeftOutline } from 'flowbite-svelte-icons';
	import type { ApexOptions } from 'apexcharts';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { graphql } from '$houdini';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { integrationError, errorClass, deltaColor, categoryColor } from '$lib/analysis/overviews';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const year = $derived(Number(page.params.year));
	const month = $derived(Number(page.params.month));

	// UTC because the backend buckets months in UTC; building these locally would land the
	// boundary an hour off and pull a neighbouring month's first expense into this one
	const start = $derived(new Date(Date.UTC(year, month - 1, 1)));
	const end = $derived(new Date(Date.UTC(year, month, 1)));

	let sign = $state('expenses');

	const open = (y: number, m: number) => {
		goto(resolve('/analysis/[year]/[month]', { year: `${y}`, month: `${m}` }));
	};

	const shift = (n: number) => {
		const d = new Date(Date.UTC(year, month - 1 + n, 1));
		open(d.getUTCFullYear(), d.getUTCMonth() + 1);
	};

	/// The heading is a month input, so this page reads as one of a series rather than as a
	/// place you arrived at: the arrows only ever offer the two next to it, and getting to
	/// March of last year meant either counting clicks or going back to the overview.
	const picked = $derived(`${year}-${`${month}`.padStart(2, '0')}`);

	function pick(value: string) {
		// an emptied input, which the picker allows
		if (!value) return;
		const [y, m] = value.split('-').map(Number);
		if (y && m) open(y, m);
	}

	const detailStore = graphql(`
		query AnalysisMonthDetail($from: DateTime!, $to: DateTime!) {
			analysis {
				overviews(from: $from, to: $to) {
					months {
						month
						startTotal
						endTotal
						integratedEndTotal
						income
						expenses
						expenseCount
						byAccount {
							account {
								id
								name
								color
							}
							startAmount
							endAmount
							integratedEndAmount
						}
					}
					byCategory {
						category {
							id
							name
							color
						}
						income
						expenses
						restIncome
						restExpenses
						biggest {
							expenseId
							title
							amount
						}
					}
				}
			}
		}
	`);

	const biggestStore = graphql(`
		query AnalysisMonthBiggest($after: DateTime!, $before: DateTime!) {
			expenses {
				list(
					valuedAfter: $after
					valuedBefore: $before
					flags: {
						templates: false
						taxRelevant: true
						extraordinary: true
						unchecked: true
						incomplete: true
						normal: true
					}
					sortBy: EFFECTIVE_AMOUNT
					sortDir: ASC
					limit: 10
				) {
					edges {
						node {
							id
							title
							store
							effectiveAmount
						}
					}
				}
			}
		}
	`);

	$effect(() => {
		void detailStore.fetch({ variables: { from: start, to: new Date(end.getTime() - 1) } });
	});

	$effect(() => {
		void biggestStore.fetch({ variables: { after: start, before: end } });
	});

	const overviews = $derived($detailStore.data?.analysis.overviews);
	const detail = $derived(overviews?.months[0]);
	const breakdown = $derived(overviews?.byCategory ?? []);
	const biggest = $derived($biggestStore.data?.expenses.list.edges.map((e) => e.node) ?? []);

	const name = (c: { name: string } | null) => c?.name ?? 'Ohne Kategorie';

	/// One tile per category on a phone, rather than one per expense: forty tiles across 326
	/// pixels leaves none of them wide enough to carry its own name, which is the whole point of
	/// the picture. The categories stay, and with them the sense of what the month went on.
	let viewport = $state(0);
	const compact = $derived(viewport > 0 && viewport < 640);

	const treemapOptions = $derived.by(() => {
		const series = breakdown
			.map((c) => {
				const children = c.biggest
					.filter((e) => (sign === 'expenses' ? e.amount < 0 : e.amount > 0))
					.map((e) => ({ x: e.title, y: Math.abs(e.amount) / 100 }));

				const rest = Math.abs(sign === 'expenses' ? c.restExpenses : c.restIncome);
				if (rest > 0) children.push({ x: 'Rest', y: rest / 100 });

				return { name: name(c.category), color: categoryColor(c.category), data: children };
			})
			.filter((s) => s.data.length > 0);

		// `distributed` is what colours a single series per tile; with one series per category
		// the colour comes from the series instead, and the two cannot be mixed.
		const compactSeries = [
			{
				name: 'Kategorien',
				data: series.map((s) => ({ x: s.name, y: s.data.reduce((sum, d) => sum + d.y, 0) }))
			}
		];

		return {
			chart: { type: 'treemap', height: compact ? 460 : 380, toolbar: { show: false } },
			series: compact ? compactSeries : series.map((s) => ({ name: s.name, data: s.data })),
			colors: series.map((s) => s.color),
			// the tiles carry their own names when there is one per category
			legend: { show: !compact, position: 'bottom' },
			dataLabels: { enabled: true, style: { fontSize: compact ? '13px' : '12px' } },
			plotOptions: { treemap: { distributed: compact, enableShades: true } },
			tooltip: { y: { formatter: (v: number) => euro(v * 100) } }
		} as ApexOptions;
	});
</script>

<svelte:window bind:innerWidth={viewport} />

<div class="space-y-4 px-4 sm:px-8">
	<div class="flex flex-wrap items-center gap-3">
		<Button color="alternative" size="sm" href={resolve('/analysis')}>
			<ArrowLeftOutline class="mr-2 h-4 w-4" />Auswertung
		</Button>
		<!-- the two arrows belong to the month between them; loose in the row they wrapped
		     away from it, leaving a lone "next" on a line of its own -->
		<div class="flex items-center gap-3">
			<Button color="alternative" size="sm" onclick={() => shift(-1)}>
				<ChevronLeftOutline class="h-4 w-4" />
			</Button>
			<!-- a month input rather than a heading: the browser's own picker is a year and a
			     month grid, which is exactly the choice on offer here -->
			<h2 class="text-xl font-semibold">
				<label>
					<span class="sr-only">Monat</span>
					<input
						type="month"
						value={picked}
						onchange={(e) => pick((e.target as HTMLInputElement).value)}
						class="focus:border-primary-500 cursor-pointer rounded-lg border border-transparent bg-transparent px-2 py-1 text-xl font-semibold hover:border-gray-300 focus:ring-0 dark:hover:border-gray-600"
					/>
				</label>
			</h2>
			<Button color="alternative" size="sm" onclick={() => shift(1)}>
				<ChevronRightOutline class="h-4 w-4" />
			</Button>
		</div>
		{#if $detailStore.fetching}
			<Spinner size="4" />
		{/if}
	</div>

	{#if $detailStore.errors}
		<p class="text-sm text-red-600 dark:text-red-400">
			{$detailStore.errors.map((e) => e.message).join(', ')}
		</p>
	{:else if detail}
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
				<span
					class="text-2xl font-semibold {detail.endTotal - detail.startTotal < 0
						? 'text-red-600 dark:text-red-400'
						: 'text-green-700 dark:text-green-400'}"
				>
					{euro(detail.endTotal - detail.startTotal)}
				</span>
			</Card>
			<Card class="max-w-none p-4">
				<span class="text-sm text-gray-500 dark:text-gray-400">Vermögen am Ende</span>
				<span class="text-2xl font-semibold">{euro(detail.endTotal)}</span>
				<span class="text-xs text-gray-500 dark:text-gray-400">
					Integrationsfehler <span class={errorClass(integrationError(detail))}>
						{euro(integrationError(detail))}
					</span>
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
			<!-- min-w-0 twice over: a grid track and a flex child both size to their content
			     unless told otherwise, so without it the table widens the page rather than
			     scrolling inside the card -->
			<Card class="max-w-none min-w-0 p-4">
				<h3 class="mb-2 text-lg font-semibold">Konten</h3>
				<div class="min-w-0 overflow-x-auto">
					<Table hoverable striped class="w-full text-left text-sm">
						<TableHead>
							<TableHeadCell class="px-4 py-3">Konto</TableHeadCell>
							<TableHeadCell class="px-4 py-3">Beginn</TableHeadCell>
							<TableHeadCell class="px-4 py-3">Ende</TableHeadCell>
							<TableHeadCell class="px-4 py-3">Delta</TableHeadCell>
							<TableHeadCell class="px-4 py-3">Fehler</TableHeadCell>
						</TableHead>
						<TableBody class="divide-y">
							{#each detail.byAccount as a (a.account.id)}
								{@const error = Math.abs(a.endAmount - a.integratedEndAmount)}
								<TableBodyRow>
									<TableBodyCell class="px-4 py-3">
										<span
											class="mr-2 inline-block h-3 w-3 rounded-full align-middle"
											style="background-color: {a.account.color ?? '#94a3b8'}"
										></span>{a.account.name}
									</TableBodyCell>
									<TableBodyCell class="px-4 py-3">{euro(a.startAmount)}</TableBodyCell>
									<TableBodyCell class="px-4 py-3">{euro(a.endAmount)}</TableBodyCell>
									<TableBodyCell class="px-4 py-3">
										<Badge
											color={deltaColor(a.endAmount - a.startAmount)}
											class="text-sm font-semibold"
										>
											{euro(a.endAmount - a.startAmount)}
										</Badge>
									</TableBodyCell>
									<TableBodyCell class="px-4 py-3 {errorClass(error)}">{euro(error)}</TableBodyCell>
								</TableBodyRow>
							{/each}
						</TableBody>
					</Table>
				</div>
			</Card>

			<Card class="max-w-none min-w-0 p-4">
				<h3 class="mb-2 text-lg font-semibold">Größte Ausgaben</h3>
				<Table hoverable striped class="w-full text-left text-sm">
					<TableHead>
						<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Betrag</TableHeadCell>
					</TableHead>
					<TableBody class="divide-y">
						{#each biggest as e (e.id)}
							<TableBodyRow>
								<TableBodyCell class="px-4 py-3">
									{e.title}
									{#if e.store}
										<span class="text-gray-500 dark:text-gray-400"> · {e.store}</span>
									{/if}
								</TableBodyCell>
								<TableBodyCell class="px-4 py-3 text-red-600 dark:text-red-400">
									{euro(e.effectiveAmount)}
								</TableBodyCell>
							</TableBodyRow>
						{/each}
					</TableBody>
				</Table>
			</Card>
		</div>
	{/if}
</div>
