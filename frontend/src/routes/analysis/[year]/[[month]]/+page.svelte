<script lang="ts">
	import {
		Card,
		Button,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Spinner
	} from 'flowbite-svelte';
	import {
		ChevronLeftOutline,
		ChevronRightOutline,
		ArrowLeftOutline,
		ArrowUpRightFromSquareOutline,
		ChevronDownOutline
	} from 'flowbite-svelte-icons';
	import type { ApexOptions } from 'apexcharts';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { graphql } from '$houdini';
	import { amountColor, euro, restingAmountColor, signedEuro } from '#lib/money.ts';
	import Chart from '#lib/Chart.svelte';
	import ToggleGroup from '#lib/ToggleGroup.svelte';
	import ExpenseDetailModal from '#lib/ExpenseDetailModal.svelte';
	import { invalidate, AFTER_WRITE } from '#lib/invalidate.ts';
	import { integrationError, errorClass, categoryColor, asMoney } from '#lib/analysis/format.ts';
	import { DATE_AXES, accrued, axisState } from '#lib/analysis/state.svelte.ts';
	import { accrualShare, isoDate } from '#lib/analysis/dates.ts';
	import QueryError from '#lib/QueryError.svelte';

	const year = $derived(Number(page.params.year));

	/// One page at two grains. Without a month in the path it is the year, which is the same
	/// page over a wider window: the totals, the categories and the biggest expenses are
	/// answers about a period, and nothing about them cares how long it is. The month table is
	/// what a year has and a month does not -- its months are what it is made of.
	const month = $derived(page.params.month ? Number(page.params.month) : null);
	const byYear = $derived(month === null);

	// UTC because the backend buckets periods in UTC; building these locally would land the
	// boundary an hour off and pull a neighbouring period's first expense into this one
	const start = $derived(
		byYear ? new Date(Date.UTC(year, 0, 1)) : new Date(Date.UTC(year, month! - 1, 1))
	);
	const end = $derived(
		byYear ? new Date(Date.UTC(year + 1, 0, 1)) : new Date(Date.UTC(year, month!, 1))
	);

	let sign = $state<'expenses' | 'income'>('expenses');

	const open = (y: number, m: number | null) => {
		goto(
			resolve('/analysis/[year]/[[month]]', {
				year: `${y}`,
				month: m === null ? undefined : `${m}`
			})
		);
	};

	const shift = (n: number) => {
		if (byYear) return open(year + n, null);

		const d = new Date(Date.UTC(year, month! - 1 + n, 1));
		open(d.getUTCFullYear(), d.getUTCMonth() + 1);
	};

	/// The heading is a picker, so this page reads as one of a series rather than as a place you
	/// arrived at: the arrows only ever offer the two months next to it, and getting to March of
	/// last year meant either counting clicks or going back to the overview.
	///
	/// A select rather than `input type="month"`: firefox has no month picker and renders one as
	/// a plain text field, which put "2026-07" where the heading used to be.
	const picked = $derived(byYear ? `${year}` : `${year}-${month}`);

	/// Every period from the first expense to this one, newest first — the ones there is
	/// anything to see in. The range query is the picker's own, and cheap: one expense, sorted.
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

	/// What the period is called, wherever it is written out.
	const label = (y: number, m: number | null) =>
		m === null
			? `${y}`
			: new Date(Date.UTC(y, m - 1, 1)).toLocaleDateString('de-DE', {
					month: 'long',
					year: 'numeric',
					timeZone: 'UTC'
				});

	const choices = $derived.by(() => {
		const first = $startStore.data?.expenses.list.edges[0]?.node.accrualStart;
		const now = new Date();

		// the period being looked at is always offered, even where it is outside the data
		let y = first?.getUTCFullYear() ?? year;
		let m = byYear ? 1 : (first?.getUTCMonth() ?? month! - 1) + 1;
		const out: { value: string; label: string }[] = [];

		while (
			y < now.getFullYear() ||
			(y === now.getFullYear() && (byYear || m <= now.getMonth() + 1))
		) {
			out.push({ value: byYear ? `${y}` : `${y}-${m}`, label: label(y, byYear ? null : m) });

			if (byYear) y += 1;
			else if (m === 12) [y, m] = [y + 1, 1];
			else m += 1;
		}

		if (!out.some((o) => o.value === picked)) {
			out.push({ value: picked, label: label(year, month) });
		}

		return out.reverse();
	});

	function pick(value: string) {
		const [y, m] = value.split('-').map(Number);
		if (!y) return;
		if (byYear) open(y, null);
		else if (m) open(y, m);
	}

	const detailStore = graphql(`
		query AnalysisPeriodDetail(
			$from: DateTime!
			$to: DateTime!
			$accrued: Boolean
			$granularity: Granularity
			$byYear: Boolean!
		) {
			analysis {
				# The range is exactly one period at the grain asked for, so this is one row.
				overviews(from: $from, to: $to, accrued: $accrued, granularity: $granularity) {
					periods {
						start
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
				# The months a year is made of, for the table only a year has. A second call
				# rather than folding the twelve into one: the roll-up nets a transfer inside
				# the period it happened in, so a transfer leaving in May and landing in June
				# nets over the year and not over either month. Summing the rows would report a
				# year that never happened.
				months: overviews(from: $from, to: $to, accrued: $accrued, granularity: MONTH)
					@include(if: $byYear) {
					periods {
						start
						income
						expenses
						expenseCount
						startTotal
						endTotal
						integratedEndTotal
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
			$categoryIds: [UUID!]
		) {
			expenses {
				# the same window read from both ends, as two aliases of one field rather than
				# two documents: one round trip, one store, one fetch to keep in step
				spent: list(
					categoryIds: $categoryIds
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
							categories {
								weight
								category {
									id
									name
									color
								}
							}
						}
					}
				}
				earned: list(
					categoryIds: $categoryIds
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
					sortDir: DESC
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
							categories {
								weight
								category {
									id
									name
									color
								}
							}
						}
					}
				}
			}
		}
	`);

	/// One period wide, at the grain the path asks for. In one place because `reload` asks for
	/// the same window again after an edit, and two copies of this drifted apart once already.
	const detailWindow = $derived({
		from: start,
		to: new Date(end.getTime() - 1),
		accrued: accrued(),
		granularity: (byYear ? 'YEAR' : 'MONTH') as 'YEAR' | 'MONTH',
		byYear
	});

	$effect(() => {
		void detailStore.fetch({ variables: detailWindow });
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

	/// Which picture of the categories. The treemap goes a level deeper -- category, then the
	/// expenses inside it -- and the circle answers the question that gets asked first, which
	/// is what share of the month each category was.
	let view = $state<'donut' | 'treemap'>('treemap');

	const VIEWS = [
		{ value: 'donut', label: 'Kreis' },
		{ value: 'treemap', label: 'Treemap' }
	];

	/// The category the treemap is zoomed into, or null for the whole month.
	///
	/// Apex owns this: a click on a category header re-renders the chart around that branch and
	/// keeps the key on its own globals, with no event of its own to say so. The `updated` event
	/// fires for that re-render like any other, which is the one place the two can be brought
	/// back into step. `_key` is `<series index>:<name>`, so the index is what resolves it.
	///
	/// Only the treemap: the sunburst keeps its focus on the chart instance rather than in
	/// globals and re-lays-out without an update, so there is nothing to read there.
	let zoomed = $state<{ id: string | null; name: string } | null>(null);

	$effect(() => {
		// a new month, sign, axis or view redraws the chart from scratch, and apex's focus goes
		// with it
		void [view, sign, start, accrued()];
		zoomed = null;
	});

	// The zoom is the filter: a treemap zoomed into one category is a reader asking about that
	// category, and the two tables under it answering about the whole month is the answer to a
	// question nobody asked. `Ohne Kategorie` is the absence of one, which the list cannot be
	// asked for, so that branch leaves the tables where they are.
	const filteredBy = $derived(view === 'treemap' && zoomed?.id ? [zoomed.id] : undefined);

	const biggestWindow = $derived(
		accrued()
			? { accruedAfter: start, accruedBefore: end, limit: BIGGEST_CANDIDATES }
			: { valuedAfter: start, valuedBefore: end, limit: 10 }
	);

	$effect(() => {
		void biggestStore.fetch({ variables: { ...biggestWindow, categoryIds: filteredBy } });
	});

	/// The expense a tile was clicked on, which is what the modal is bound to.
	let opened = $state<string | undefined>(undefined);

	/// An edit through the modal moves the figures this page is made of, and none of its
	/// queries would notice on their own: the variables they were fetched with have not
	/// changed.
	async function reload() {
		invalidate();
		await Promise.all([
			detailStore.fetch({ variables: detailWindow, ...AFTER_WRITE }),
			biggestStore.fetch({
				variables: { ...biggestWindow, categoryIds: filteredBy },
				...AFTER_WRITE
			})
		]);
	}

	/// The month in the expense list, on the axis it is being read on.
	///
	/// "Ausgaben" is what the list is called and not what it holds -- income is an expense with
	/// the other sign -- so the link says Posten, which is what the two tables above it are
	/// halves of.
	const listHref = $derived.by(() => {
		const params = new URLSearchParams({
			from: isoDate(start),
			to: isoDate(new Date(end.getTime() - 86400000)),
			axis: accrued() ? 'accrual' : 'value'
		});

		return `${resolve('/expenses')}?${params}`;
	});

	const overviews = $derived($detailStore.data?.analysis.overviews);
	const detail = $derived(overviews?.periods[0]);

	/// The months of the year, newest first, like the overview table they mirror. Empty on a
	/// month, which is made of nothing smaller this page knows how to show.
	const months = $derived($detailStore.data?.analysis.months?.periods.toReversed() ?? []);

	/// What the month booked, rather than what the balances did. The same figure on the value
	/// axis, and the only one of the two that means anything on the accrual axis: filing a cost
	/// under a different month does not move money between accounts.
	const delta = $derived((detail?.income ?? 0) + (detail?.expenses ?? 0));
	const breakdown = $derived(overviews?.byCategory ?? []);
	type Row = {
		id: string;
		title: string;
		store: string;
		effectiveAmount: number;
		categories: readonly { category: { id: string; name: string; color: string | null } }[];
	};

	/// The ten biggest of one sign, and what makes one big.
	///
	/// On the value axis that is the expense itself. On the accrual axis it is the part of it
	/// this month carried, and the whole is worth saying next to it -- a hundred euros of a
	/// twelve-hundred euro contract is a different fact from a hundred euro shop.
	///
	/// The sign filter is not redundant with the sort: a month with fewer than ten expenses
	/// hands back income at the bottom of the spending list, and the other way round.
	function rank(rows: (Row & { accrualStart: Date; accrualEnd: Date })[], want: -1 | 1) {
		return rows
			.map((e) => {
				const share = accrued() ? accrualShare(e, start, end) : 1;
				return {
					...e,
					amount: e.effectiveAmount * share,
					whole: share < 1 ? e.effectiveAmount : null
				};
			})
			.filter((e) => Math.sign(e.amount) === want)
			.sort((a, b) => (b.amount - a.amount) * want)
			.slice(0, 10);
	}

	const biggest = $derived(
		rank($biggestStore.data?.expenses.spent.edges.map((e) => e.node) ?? [], -1)
	);
	const earned = $derived(
		rank($biggestStore.data?.expenses.earned.edges.map((e) => e.node) ?? [], 1)
	);

	const name = (c: { name: string } | null) => c?.name ?? 'Ohne Kategorie';

	/// One tile per category on a phone, rather than one per expense: forty tiles across 326
	/// pixels leaves none of them wide enough to carry its own name, which is the whole point of
	/// the picture. The categories stay, and with them the sense of what the month went on.
	let viewport = $state(0);
	const compact = $derived(viewport > 0 && viewport < 640);

	/// The month as a tree: every category, and the expenses inside it.
	///
	/// One shape for both pictures. Apex builds the hierarchy out of the series -- each series
	/// is a parent, its data the leaves under it -- which is what a zoom and a second ring both
	/// need, and it is also what gives every category its own colour rather than one palette
	/// entry per tile.
	const treeFor = (which: 'expenses' | 'income') =>
		breakdown
			.map((c) => {
				const leaves = c.biggest
					.filter((e) => (which === 'expenses' ? e.amount < 0 : e.amount > 0))
					.map((e) => ({
						x: e.title,
						y: Math.abs(e.amount) / 100,
						id: e.expenseId as string | null
					}));

				// what `biggest` left out, as one leaf, so the areas still add up to the category.
				// No id: it stands for a number of expenses rather than for one, so there is
				// nothing for a click to open.
				const rest = Math.abs(which === 'expenses' ? c.restExpenses : c.restIncome);
				if (rest > 0) leaves.push({ x: 'Rest', y: rest / 100, id: null });

				return {
					id: c.category?.id ?? null,
					name: name(c.category),
					color: categoryColor(c.category),
					data: leaves
				};
			})
			.filter((s) => s.data.length > 0);

	const total = (tree: ReturnType<typeof treeFor>) =>
		tree.reduce((sum, s) => sum + s.data.reduce((inner, d) => inner + d.y, 0), 0);

	/// Rings from the middle out: categories, then what is inside them.
	///
	/// One per sign, drawn side by side rather than behind a toggle -- a ring is a share of a
	/// whole, and the two wholes are different questions. The treemap keeps the toggle, where
	/// two signs of area in one picture would mean nothing.
	/// Money in the sunburst's tooltip, where apex writes a bare number -- see `asMoney`.
	///
	/// ponytail: one observer over the element apex writes into, rather than a fork of the
	/// chart library. Once apex formats it there itself the pattern stops matching and this
	/// quietly does nothing.
	const formatTooltip = (root: HTMLElement) => {
		const tooltip = root.querySelector('.apexcharts-tooltip');
		// only once per chart, however many times apex says it has been mounted or updated
		if (!tooltip || tooltip.hasAttribute('data-money')) return;
		tooltip.setAttribute('data-money', '');

		const rewrite = () => {
			for (const cell of tooltip.querySelectorAll('.apexcharts-tooltip-text-y-value')) {
				const money = asMoney(cell.textContent ?? '', euro);
				// the null is what makes this converge: the second pass finds nothing to do
				if (money !== null) cell.textContent = money;
			}
		};

		new MutationObserver(rewrite).observe(tooltip, {
			childList: true,
			subtree: true,
			characterData: true
		});
	};

	const sunburstFor = (which: 'expenses' | 'income') => {
		const tree = treeFor(which);

		return {
			chart: {
				type: 'sunburst',
				height: compact ? 320 : 420,
				// `el` is on the instance and not in apex's own types for it
				events: {
					mounted: (chart: ApexCharts) =>
						formatTooltip((chart as unknown as { el: HTMLElement }).el),
					updated: (chart: ApexCharts) =>
						formatTooltip((chart as unknown as { el: HTMLElement }).el)
				}
			},
			// One series carrying the whole tree, unlike the treemap next door: the sunburst
			// reads `series[0].data` and nothing else, so a series per category drew the first
			// category's expenses as the only ring and dropped the rest of the month.
			series: [
				{
					// `y` on the parent as well as the children: apex parses the series before it
					// ever builds the hierarchy, and a datum without one contributes nothing to
					// that pass -- every parent lacking it left the parser with no values at all,
					// which is the "no data" branch, which is a blank canvas.
					data: tree.map((c) => ({
						x: c.name,
						y: c.data.reduce((sum, d) => sum + d.y, 0),
						color: c.color,
						children: c.data
					}))
				}
			],
			legend: { show: false },
			plotOptions: {
				sunburst: {
					// the hole is where the total goes, above the chart rather than inside it:
					// apex has no centre label for this type
					innerSize: '18%',
					// enough that the expenses read as belonging to their category rather than
					// as a second set of colours
					tint: 0.18,
					spacing: 1,
					borderRadius: 2,
					// a category fills the circle on a click, and its middle takes you back out
					zoomOnClick: true,
					dataLabels: { show: true, minAngleToShow: 10, style: { fontSize: '11px' } }
				}
			},
			tooltip: { y: { formatter: (v: number) => euro(v * 100) } }
		} as ApexOptions;
	};

	const spentTree = $derived(treeFor('expenses'));
	const earnedTree = $derived(treeFor('income'));
	const spentSunburst = $derived(sunburstFor('expenses'));
	const earnedSunburst = $derived(sunburstFor('income'));

	const treemapOptions = $derived.by(() => {
		const tree = treeFor(sign);

		const focused = (key: unknown) => {
			if (typeof key !== 'string') return null;
			const index = Number(key.split(':')[0]);
			const category = Number.isInteger(index) ? tree[index] : undefined;
			return category ? { id: category.id, name: category.name } : null;
		};

		// `distributed` is what colours a single series per tile; with one series per category
		// the colour comes from the series instead, and the two cannot be mixed.
		const compactSeries = [
			{
				name: 'Kategorien',
				data: tree.map((s) => ({ x: s.name, y: s.data.reduce((sum, d) => sum + d.y, 0) }))
			}
		];

		return {
			chart: {
				type: 'treemap',
				height: compact ? 460 : 440,
				toolbar: { show: false },
				events: {
					updated: (ctx: { w?: { globals?: { treemapFocusKey?: unknown } } }) => {
						zoomed = focused(ctx?.w?.globals?.treemapFocusKey);
					},
					// A tile is one expense, and the thing anyone wants from it is the expense.
					// The category headers are what zoom, so the two gestures do not collide.
					dataPointSelection: (
						_event: unknown,
						_ctx: unknown,
						opts: { seriesIndex?: number; dataPointIndex?: number }
					) => {
						const leaf = tree[opts?.seriesIndex ?? -1]?.data[opts?.dataPointIndex ?? -1];
						if (leaf?.id) opened = leaf.id;
					}
				}
			},
			series: compact
				? compactSeries
				: tree.map((s) => ({ name: s.name, color: s.color, data: s.data })),
			colors: tree.map((s) => s.color),
			// the categories carry their own names once they are drawn as containers
			legend: { show: false },
			// the amount on the tile: a treemap says which is bigger, not by how much, and the
			// tooltip only answers for the one tile the pointer is on. Apex drops the label
			// whole where it does not fit, so a tile too small for both stays as it was.
			dataLabels: {
				enabled: true,
				style: { fontSize: compact ? '13px' : '12px' },
				// `unknown` for the second argument: apex's own types declare one opts shape for
				// every chart, and the treemap's `value` is not on it
				formatter: (text: string | number | number[], op?: unknown) =>
					`${text} (${euro(((op as { value?: number } | undefined)?.value ?? 0) * 100)})`
			},
			plotOptions: {
				treemap: {
					distributed: compact,
					enableShades: true,
					// A container per category, which is also what makes the zoom possible: apex
					// only zooms a treemap that has parents to land on. Not on a phone, where the
					// whole point of the compact shape is one tile per category and nothing to
					// zoom into.
					parents: {
						show: !compact,
						// the same figure the tiles inside it carry, so a category can be compared
						// with its neighbour without adding up what is in it
						header: {
							formatter: (name: string, op: { value: number }) =>
								`${name} (${euro(op.value * 100)})`
						}
					},
					zoom: { enabled: !compact }
				}
			},
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
		<!-- the two arrows belong to the period between them; loose in the row they wrapped
		     away from it, leaving a lone "next" on a line of its own -->
		<div class="flex items-center gap-3">
			<Button color="alternative" size="sm" onclick={() => shift(-1)}>
				<ChevronLeftOutline class="h-4 w-4" />
			</Button>
			<h2 class="text-xl font-semibold">
				<!-- `appearance-none` and a chevron of our own: the native indicator is drawn
				     inside the control's own box, and over a label as short as a year it landed
				     on the last digit. Ours has the padding to itself. -->
				<label class="relative block">
					<span class="sr-only">{byYear ? 'Jahr' : 'Monat'}</span>
					<select
						value={picked}
						onchange={(e) => pick((e.target as HTMLSelectElement).value)}
						class="focus:border-primary-500 cursor-pointer appearance-none rounded-lg border border-transparent bg-transparent py-1 ps-2 pe-8 text-xl font-semibold hover:border-gray-300 focus:ring-0 dark:hover:border-gray-600"
					>
						{#each choices as choice (choice.value)}
							<option value={choice.value}>{choice.label}</option>
						{/each}
					</select>
					<ChevronDownOutline
						class="pointer-events-none absolute end-2 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400 dark:text-gray-500"
					/>
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
		<QueryError
			errors={$detailStore.errors}
			message="{byYear ? 'Das Jahr' : 'Der Monat'} konnte nicht geladen werden."
		/>
	{:else if detail}
		<!-- three cards rather than four on the accrual axis, where the fourth is a balance:
		     see the Konten table below, and the columns the overview drops for the same reason -->
		<div class="grid gap-4 sm:grid-cols-2 {accrued() ? 'lg:grid-cols-4' : 'lg:grid-cols-5'}">
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
			</Card>
			<Card class="max-w-none p-4">
				<span class="text-sm text-gray-500 dark:text-gray-400">Delta</span>
				<span class="text-2xl font-semibold {amountColor(delta)}">{euro(delta)}</span>
			</Card>
			<!-- eslint-disable svelte/no-navigation-without-resolve -- resolve() takes a route, not a query string -->
			<a href={listHref} class="group">
				<Card class="max-w-none p-4">
					<span class="text-sm text-gray-500 dark:text-gray-400">Einträge</span>
					<span class="flex items-center gap-2 text-2xl font-semibold">
						{detail.expenseCount}
						<ArrowUpRightFromSquareOutline
							class="group-hover:text-primary-600 dark:group-hover:text-primary-500 h-4 w-4 text-gray-400 dark:text-gray-500"
						/>
					</span>
					<span class="text-xs text-gray-500 group-hover:underline dark:text-gray-400">
						in der Liste öffnen
					</span>
				</Card>
			</a>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
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
				<div class="flex flex-wrap items-center gap-2">
					<ToggleGroup bind:value={view} options={VIEWS} size="sm" />
					{#if view === 'treemap'}
						<ToggleGroup
							bind:value={sign}
							options={[
								{ value: 'expenses', label: 'Ausgaben' },
								{ value: 'income', label: 'Einnahmen' }
							]}
						/>
					{/if}
				</div>
			</div>
			{#if view === 'donut'}
				<div class="grid gap-4 md:grid-cols-2">
					{@render ring('Ausgegeben', total(spentTree), spentSunburst)}
					{@render ring('Eingenommen', total(earnedTree), earnedSunburst)}
				</div>
			{:else}
				<Chart options={treemapOptions} />
			{/if}
		</Card>

		<!-- What the year is made of, before the accounts it moved between: the months are the
		     next question after "what did the year cost", and each of them has the same page
		     this one is. Newest first, like the overview table it mirrors. -->
		{#if byYear && months.length > 0}
			<Card class="max-w-none min-w-0 p-4">
				<h3 class="mb-2 text-lg font-semibold">Monate</h3>
				<!-- The same columns at the same breakpoints as the overview table: a phone keeps
				     month, delta and closing wealth, the two sides from sm, the count from xl. -->
				<Table hoverable striped class="w-full text-left text-sm">
					<TableHead>
						<TableHeadCell class="px-2 py-3 sm:px-4">Monat</TableHeadCell>
						<TableHeadCell class="px-2 py-3 sm:px-4">Delta</TableHeadCell>
						<TableHeadCell class="hidden px-2 py-3 sm:table-cell sm:px-4">Einnahmen</TableHeadCell>
						<TableHeadCell class="hidden px-2 py-3 sm:table-cell sm:px-4">Ausgaben</TableHeadCell>
						<TableHeadCell class="hidden px-2 py-3 sm:px-4 xl:table-cell">Anzahl</TableHeadCell>
						{#if !accrued()}
							<TableHeadCell class="px-2 py-3 sm:px-4">Vermögen Ende</TableHeadCell>
						{/if}
					</TableHead>
					<TableBody class="divide-y">
						{#each months as m (m.start.getTime())}
							{@const moved = m.income + m.expenses}
							<TableBodyRow>
								<TableBodyCell class="px-2 py-3 whitespace-nowrap sm:px-4">
									<a
										class="text-primary-600 dark:text-primary-500 font-medium hover:underline"
										href={resolve('/analysis/[year]/[[month]]', {
											year: `${m.start.getUTCFullYear()}`,
											month: `${m.start.getUTCMonth() + 1}`
										})}
									>
										{m.start.toLocaleDateString('de-DE', { month: 'long', timeZone: 'UTC' })}
									</a>
								</TableBodyCell>
								<TableBodyCell class="px-2 py-3 font-semibold sm:px-4 {restingAmountColor(moved)}">
									{signedEuro(moved)}
								</TableBodyCell>
								<TableBodyCell
									class="hidden px-2 py-3 sm:table-cell sm:px-4 {restingAmountColor(m.income)}"
									>{signedEuro(m.income)}</TableBodyCell
								>
								<TableBodyCell
									class="hidden px-2 py-3 sm:table-cell sm:px-4 {restingAmountColor(m.expenses)}"
									>{signedEuro(m.expenses)}</TableBodyCell
								>
								<TableBodyCell class="hidden px-2 py-3 sm:px-4 xl:table-cell"
									>{m.expenseCount}</TableBodyCell
								>
								{#if !accrued()}
									<TableBodyCell class="px-2 py-3 sm:px-4">{euro(m.endTotal)}</TableBodyCell>
								{/if}
							</TableBodyRow>
						{/each}
					</TableBody>
				</Table>
				<!-- The months do not add up to the year above them, and saying so beats leaving
				     it to be discovered: a transfer out in May and back in June is gone from both
				     of those months and never left the year. -->
				<p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
					Umbuchungen zwischen eigenen Konten heben sich innerhalb einer Periode auf, nicht über sie
					hinweg — die Monate summieren sich daher nicht zwingend zum Jahr.
				</p>
			</Card>
		{/if}

		<!-- Balances, so the accrual axis has nothing to say here: an account holds money, and
		     money does not move between accounts because a cost was filed under another period.
		     What that period cost is the three cards above. -->
		{#if !accrued()}
			<!-- A row of its own rather than a column beside the two lists: it is as wide as the
			     accounts make it, and sharing a row pushed the lists onto separate ones, where
			     what a month took in and what it spent are meant to be read against each other. -->
			<Card class="max-w-none p-4">
				<h3 class="mb-2 text-lg font-semibold">Konten</h3>
				<!-- A phone keeps account and closing balance, what the account holds; the delta
				     comes back from sm, the opening balance and the reconciliation error from md. -->
				<Table hoverable striped class="w-full text-left text-sm">
					<TableHead>
						<TableHeadCell class="px-2 py-3 sm:px-4">Konto</TableHeadCell>
						<TableHeadCell class="hidden px-2 py-3 sm:px-4 md:table-cell">Beginn</TableHeadCell>
						<TableHeadCell class="px-2 py-3 sm:px-4">Ende</TableHeadCell>
						<TableHeadCell class="hidden px-2 py-3 sm:table-cell sm:px-4">Delta</TableHeadCell>
						<TableHeadCell class="hidden px-2 py-3 sm:px-4 md:table-cell">Fehler</TableHeadCell>
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
								<!-- a long name wraps rather than push the balance off a phone -->
								<TableBodyCell class="px-2 py-3 whitespace-normal sm:px-4">
									<span
										class="mr-2 inline-block h-3 w-3 rounded-full align-middle"
										style="background-color: {a.account.color ?? '#94a3b8'}"
									></span>{a.account.name}
								</TableBodyCell>
								<TableBodyCell class="hidden px-2 py-3 sm:px-4 md:table-cell"
									>{euro(a.startAmount)}</TableBodyCell
								>
								<TableBodyCell class="px-2 py-3 sm:px-4">{euro(a.endAmount)}</TableBodyCell>
								{@const moved = a.endAmount - a.startAmount}
								<TableBodyCell
									class="hidden px-2 py-3 font-semibold sm:table-cell sm:px-4 {restingAmountColor(
										moved
									)}"
								>
									{signedEuro(moved)}
								</TableBodyCell>
								<TableBodyCell
									class="hidden px-2 py-3 sm:px-4 md:table-cell {error === null
										? ''
										: errorClass(error)}"
								>
									{error === null ? '—' : euro(error)}
								</TableBodyCell>
							</TableBodyRow>
						{/each}
					</TableBody>
				</Table>
			</Card>
		{/if}

		<div class="grid gap-4 lg:grid-cols-2">
			{@render extremes('Größte Ausgaben', biggest)}
			{@render extremes('Größte Einnahmen', earned)}
		</div>
	{/if}
</div>

<ExpenseDetailModal bind:expenseId={opened} onsubmit={reload} />

{#snippet ring(label: string, sum: number, options: ApexOptions)}
	<div class="min-w-0">
		<div class="mb-1 text-center">
			<span class="text-sm text-gray-500 dark:text-gray-400">{label}</span>
			<span class="ml-2 font-semibold">{euro(sum * 100)}</span>
		</div>
		<Chart {options} />
	</div>
{/snippet}

{#snippet extremes(heading: string, rows: typeof biggest)}
	<Card class="max-w-none min-w-0 p-4">
		<h3 class="mb-2 text-lg font-semibold">
			{heading}
			<!-- what the treemap is zoomed into, said out loud: a shorter table is otherwise a
			     month with less in it than it has -->
			{#if filteredBy && zoomed}
				<span class="text-base font-normal text-gray-500 dark:text-gray-400">
					· {zoomed.name}
				</span>
			{/if}
		</h3>
		<QueryError
			errors={$biggestStore.errors}
			message="Die größten Posten konnten nicht geladen werden."
			class="mb-3"
		/>
		<!-- A phone keeps title and amount; the categories, a list of their own, from sm. -->
		<Table hoverable striped class="w-full text-left text-sm">
			<TableHead>
				<TableHeadCell class="px-2 py-3 sm:px-4">Titel</TableHeadCell>
				<TableHeadCell class="hidden px-2 py-3 sm:table-cell sm:px-4">Kategorie</TableHeadCell>
				<TableHeadCell class="px-2 py-3 sm:px-4">Betrag</TableHeadCell>
			</TableHead>
			<TableBody class="divide-y">
				{#each rows as e (e.id)}
					<TableBodyRow>
						<!-- takes what the amount leaves and truncates, so a long title cannot widen
						     the table past a phone -->
						<TableBodyCell class="w-full max-w-0 truncate px-2 py-3 sm:px-4">
							{e.title}
							{#if e.store}
								<span class="text-gray-500 dark:text-gray-400"> · {e.store}</span>
							{/if}
						</TableBodyCell>
						<!-- an expense can sit in several, which is why this is a list and not a
						     word; the amount beside it is the whole expense either way -->
						<TableBodyCell class="hidden px-2 py-3 sm:table-cell sm:px-4">
							<div class="flex flex-wrap gap-x-2 gap-y-1">
								{#each e.categories as c (c.category.id)}
									<span class="inline-flex items-center gap-1 text-xs whitespace-nowrap">
										<span
											class="inline-block h-2 w-2 shrink-0 rounded-full"
											style="background-color: {categoryColor(c.category)}"
										></span>{c.category.name}
									</span>
								{/each}
							</div>
						</TableBodyCell>
						<TableBodyCell
							class="px-2 py-3 whitespace-nowrap sm:px-4 {restingAmountColor(e.amount)}"
						>
							{signedEuro(e.amount)}
							<!-- under the figure, for the same reason as in the expense table: a second
							     amount on the line is what the column has no room for -->
							{#if e.whole !== null}
								<div class="text-xs font-normal text-gray-500 dark:text-gray-400">
									von {euro(Math.abs(e.whole))}
								</div>
							{/if}
						</TableBodyCell>
					</TableBodyRow>
				{/each}
			</TableBody>
		</Table>
	</Card>
{/snippet}
