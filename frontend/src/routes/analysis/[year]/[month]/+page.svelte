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
	import { amountColor } from '$lib/money';
	import Chart from '$lib/Chart.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { integrationError, errorClass, deltaColor, categoryColor } from '$lib/analysis/format';
	import { DATE_AXES, accrued, axisState } from '$lib/analysis/state.svelte';
	import { accrualShare } from '$lib/analysis/dates';
	import QueryError from '$lib/QueryError.svelte';

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

	/// The heading is a picker, so this page reads as one of a series rather than as a place you
	/// arrived at: the arrows only ever offer the two months next to it, and getting to March of
	/// last year meant either counting clicks or going back to the overview.
	///
	/// A select rather than `input type="month"`: firefox has no month picker and renders one as
	/// a plain text field, which put "2026-07" where the heading used to be.
	const picked = $derived(`${year}-${month}`);

	/// Every month from the first expense to this one, newest first — the ones there is anything
	/// to see in. The range query is the picker's own, and cheap: one expense, sorted.
	// No error surface, deliberately: this only widens the month picker to the earliest data,
	// and the month being viewed is offered regardless. A failure shortens a list, it empties
	// nothing.
	const startStore = graphql(`
		query MonthPickerStart {
			expenses {
				list(limit: 1, sortBy: ACCRUAL_START, sortDir: ASC) {
					edges {
						node {
							id
							accrualStart
						}
					}
				}
			}
		}
	`);

	$effect(() => {
		void startStore.fetch();
	});

	const choices = $derived.by(() => {
		const first = $startStore.data?.expenses.list.edges[0]?.node.accrualStart;
		const now = new Date();

		// the month being looked at is always offered, even where it is outside the data
		let y = first?.getUTCFullYear() ?? year;
		let m = (first?.getUTCMonth() ?? month - 1) + 1;
		const out: { value: string; label: string }[] = [];

		while (y < now.getFullYear() || (y === now.getFullYear() && m <= now.getMonth() + 1)) {
			out.push({
				value: `${y}-${m}`,
				label: new Date(Date.UTC(y, m - 1, 1)).toLocaleDateString('de-DE', {
					month: 'long',
					year: 'numeric',
					timeZone: 'UTC'
				})
			});
			if (m === 12) [y, m] = [y + 1, 1];
			else m += 1;
		}

		if (!out.some((o) => o.value === picked)) {
			out.push({
				value: picked,
				label: start.toLocaleDateString('de-DE', {
					month: 'long',
					year: 'numeric',
					timeZone: 'UTC'
				})
			});
		}

		return out.reverse();
	});

	function pick(value: string) {
		const [y, m] = value.split('-').map(Number);
		if (y && m) open(y, m);
	}

	const detailStore = graphql(`
		query AnalysisMonthDetail($from: DateTime!, $to: DateTime!, $accrued: Boolean) {
			analysis {
				overviews(from: $from, to: $to, accrued: $accrued) {
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
		query AnalysisMonthBiggest(
			$valuedAfter: DateTime
			$valuedBefore: DateTime
			$accruedAfter: DateTime
			$accruedBefore: DateTime
			$limit: Int
		) {
			expenses {
				list(
					valuedAfter: $valuedAfter
					valuedBefore: $valuedBefore
					accruedAfter: $accruedAfter
					accruedBefore: $accruedBefore
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
					limit: $limit
				) {
					edges {
						node {
							id
							title
							store
							effectiveAmount
							accrualStart
							accrualEnd
						}
					}
				}
			}
		}
	`);

	$effect(() => {
		void detailStore.fetch({
			variables: { from: start, to: new Date(end.getTime() - 1), accrued: accrued() }
		});
	});

	// One pair of the two, never both: they would be ANDed, and an expense paid in June for
	// August is in one of the windows and not the other. On the accrual axis the amount shown
	// is still the whole expense rather than the share this month carried -- what the list
	// answers is which of the things the month was paying for were the big ones.
	// ponytail: 60 rows to rank 10 by. The backend can only sort by the whole amount, and on
	// the accrual axis the ranking is by the share that fell in this month -- a year-long
	// contract outranks a weekly shop on the first and not on the second. 60 is a guess at how
	// far down the list a month's biggest share can hide, not a bound anything derives; raise
	// it if a month ever looks like it is missing one.
	const BIGGEST_CANDIDATES = 60;

	$effect(() => {
		void biggestStore.fetch({
			variables: accrued()
				? { accruedAfter: start, accruedBefore: end, limit: BIGGEST_CANDIDATES }
				: { valuedAfter: start, valuedBefore: end, limit: 10 }
		});
	});

	const overviews = $derived($detailStore.data?.analysis.overviews);
	const detail = $derived(overviews?.months[0]);

	/// What the month booked, rather than what the balances did. The same figure on the value
	/// axis, and the only one of the two that means anything on the accrual axis: filing a cost
	/// under a different month does not move money between accounts.
	const delta = $derived((detail?.income ?? 0) + (detail?.expenses ?? 0));
	const breakdown = $derived(overviews?.byCategory ?? []);
	/// The ten biggest, and what makes one big.
	///
	/// On the value axis that is the expense itself. On the accrual axis it is the part of it
	/// this month carried, and the whole is worth saying next to it -- a hundred euros of a
	/// twelve-hundred euro contract is a different fact from a hundred euro shop.
	const biggest = $derived.by(() => {
		const rows = $biggestStore.data?.expenses.list.edges.map((e) => e.node) ?? [];
		if (!accrued()) return rows.map((e) => ({ ...e, amount: e.effectiveAmount, whole: null }));

		return rows
			.map((e) => {
				const share = accrualShare(e, start, end);
				return {
					...e,
					amount: e.effectiveAmount * share,
					whole: share < 1 ? e.effectiveAmount : null
				};
			})
			.sort((a, b) => a.amount - b.amount)
			.slice(0, 10);
	});

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
			<h2 class="text-xl font-semibold">
				<label>
					<span class="sr-only">Monat</span>
					<select
						value={picked}
						onchange={(e) => pick((e.target as HTMLSelectElement).value)}
						class="focus:border-primary-500 cursor-pointer rounded-lg border border-transparent bg-transparent px-2 py-1 text-xl font-semibold hover:border-gray-300 focus:ring-0 dark:hover:border-gray-600"
					>
						{#each choices as choice (choice.value)}
							<option value={choice.value}>{choice.label}</option>
						{/each}
					</select>
				</label>
			</h2>
			<Button color="alternative" size="sm" onclick={() => shift(1)}>
				<ChevronRightOutline class="h-4 w-4" />
			</Button>
		</div>
		<!-- the whole page rather than one card: the totals, the treemap and the list below are
		     all filed under whichever date this names -->
		<ToggleGroup bind:value={axisState.axis} options={DATE_AXES} size="sm" />
		{#if $detailStore.fetching}
			<Spinner size="4" />
		{/if}
	</div>

	{#if $detailStore.errors}
		<QueryError errors={$detailStore.errors} message="Der Monat konnte nicht geladen werden." />
	{:else if detail}
		<!-- three cards rather than four on the accrual axis, where the fourth is a balance:
		     see the Konten table below, and the columns the overview drops for the same reason -->
		<div class="grid gap-4 {accrued() ? 'md:grid-cols-3' : 'md:grid-cols-4'}">
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
				<span class="text-2xl font-semibold {amountColor(delta)}">{euro(delta)}</span>
			</Card>
			{#if !accrued()}
				<Card class="max-w-none p-4">
					<span class="text-sm text-gray-500 dark:text-gray-400">Vermögen am Ende</span>
					<span class="text-2xl font-semibold">{euro(detail.endTotal)}</span>
					{@const error = integrationError(detail)}
					{#if error !== null}
						<span class="text-xs text-gray-500 dark:text-gray-400">
							Integrationsfehler <span class={errorClass(error)}>
								{euro(error)}
							</span>
						</span>
					{/if}
				</Card>
			{/if}
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

		<!-- one column on the accrual axis, where the Konten table is not shown and the list
		     beside it would otherwise keep half the page to itself -->
		<div class="grid gap-4 {accrued() ? '' : 'lg:grid-cols-2'}">
			<!-- Balances, so the accrual axis has nothing to say here: an account holds money,
			     and money does not move between accounts because a cost was filed under
			     another month. What that month cost is the three cards above. -->
			{#if !accrued()}
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
									<!-- null on the accrual axis: a share of a subscription is a cost the month
								     carried, not money that left this account, so there is nothing to
								     reconcile it against -->
									{@const error =
										a.integratedEndAmount === null
											? null
											: Math.abs(a.endAmount - a.integratedEndAmount)}
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
										<TableBodyCell class="px-4 py-3 {error === null ? '' : errorClass(error)}">
											{error === null ? '—' : euro(error)}
										</TableBodyCell>
									</TableBodyRow>
								{/each}
							</TableBody>
						</Table>
					</div>
				</Card>
			{/if}

			<Card class="max-w-none min-w-0 p-4">
				<h3 class="mb-2 text-lg font-semibold">Größte Ausgaben</h3>
				<QueryError
					errors={$biggestStore.errors}
					message="Die größten Ausgaben konnten nicht geladen werden."
					class="mb-3"
				/>
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
								<TableBodyCell class="px-4 py-3 whitespace-nowrap text-red-600 dark:text-red-400">
									{euro(e.amount)}
									{#if e.whole !== null}
										<span class="text-xs text-gray-500 dark:text-gray-400">
											von {euro(e.whole)}
										</span>
									{/if}
								</TableBodyCell>
							</TableBodyRow>
						{/each}
					</TableBody>
				</Table>
			</Card>
		</div>
	{/if}
</div>
