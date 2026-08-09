<script lang="ts">
	import { graphql } from '$houdini';
	import { resolve } from '$app/paths';
	import { Card, Spinner } from 'flowbite-svelte';
	import {
		ArrowRightOutline,
		BellRingOutline,
		CalendarMonthOutline,
		PlusOutline
	} from 'flowbite-svelte-icons';
	import type { ApexOptions } from 'apexcharts';
	import Chart from '$lib/Chart.svelte';
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
	import ToggleGroup from '$lib/ToggleGroup.svelte';
	import { categoryColor } from '$lib/analysis/overviews';
	import { WEALTH_COLOR } from '$lib/analysis/cards';
	import { shortDate } from '$lib/analysis/dates';
	import QueryError from '$lib/QueryError.svelte';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', {
			style: 'currency',
			currency: 'EUR',
			maximumFractionDigits: 0
		});

	const euroExact = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const DAY = 86400000;

	/// How far back the news reaches for someone who has never been here before.
	const RECENT_DAYS = 7;

	const now = new Date();
	const monthStart = new Date(now.getFullYear(), now.getMonth(), 1);
	// The same day one month ago, which is what the wealth is held against: a fixed span reads
	// as noise, where "since this day last month" is a comparison you can keep in your head.
	// The 31st of a month whose predecessor is shorter rolls into the 1st, which is close
	// enough to a month and never skips one.
	const monthAgo = new Date(now.getFullYear(), now.getMonth() - 1, now.getDate());
	// the month before the running one, which is what the running one is compared against
	const compareFrom = new Date(now.getFullYear(), now.getMonth() - 1, 1);
	const yearAgo = new Date(now.getFullYear() - 1, now.getMonth(), now.getDate());

	/// Records this visit and hands back the one before it, which is what everything on this
	/// page dates its news from.
	const visitMutation = graphql(`
		mutation RecordVisit {
			users {
				visit {
					previous
				}
			}
		}
	`);

	/// Null until the marker is known, which is also what keeps the query below from firing
	/// with a window it would have to redo a moment later.
	let since = $state<Date | null>(null);
	let firstVisit = $state(false);

	$effect(() => {
		void visitMutation
			.mutate(null)
			.then((result) => result.data?.users.visit.previous ?? null)
			// The marker is a nicety; the page is not. `since` gates the query below, so a visit
			// that could not be recorded -- one 502 during a rollout is enough -- left a greeting
			// with nothing under it and no way out but a reload.
			.catch(() => null)
			.then((previous) => {
				firstVisit = previous === null;
				since = previous ?? new Date(now.getTime() - RECENT_DAYS * DAY);
			});
	});

	const dashboardStore = graphql(`
		query Dashboard(
			$now: DateTime!
			$since: DateTime!
			$monthAgo: DateTime!
			$monthsFrom: DateTime!
			$sparkFrom: DateTime!
		) {
			users {
				me {
					firstName
					name
				}
			}

			analysis {
				# the two ends of the delta, in one round trip
				current: balances(date: $now) {
					amount
				}
				earlier: balances(date: $monthAgo) {
					amount
				}

				# coarse on purpose: this is a shape, not a chart to read values off
				history(from: $sparkFrom, to: $now, stepDays: 12) {
					dates
					byAccount {
						amounts
					}
				}

				overviews(from: $monthsFrom, to: $now) {
					months {
						month
						income
						expenses
						byCategory {
							category {
								id
								name
								color
							}
							income
							expenses
						}
					}
				}
			}

			expenses {
				# counts only; the entries themselves are the feed's business
				changes: changelog(author: OTHERS, since: $since, limit: 1) {
					totalCount
				}
				# everyone's, including mine: whether the feed beside this has anything to show
				# for the time we were away, which decides what it can call itself
				recent: changelog(since: $since, limit: 1) {
					totalCount
				}
				# what the "pruefen" link on /expenses lands on, so the number and the list agree.
				# Oldest first, since how long the oldest one has been waiting says more about
				# whether this needs attention than the count does.
				review: list(
					limit: 1
					sortBy: ACCRUAL_START
					sortDir: ASC
					flags: {
						templates: false
						taxRelevant: false
						extraordinary: false
						unchecked: true
						incomplete: true
						normal: false
					}
				) {
					totalCount
					edges {
						node {
							id
							title
							accrualStart
						}
					}
				}
			}
		}
	`);

	$effect(() => {
		const from = since;
		if (!from) return;

		void dashboardStore.fetch({
			variables: { now, since: from, monthAgo, monthsFrom: compareFrom, sparkFrom: yearAgo }
		});
	});

	const data = $derived($dashboardStore.data);

	const greeting = $derived(data?.users.me.firstName ?? data?.users.me.name ?? '');

	const total = $derived((data?.analysis.current ?? []).reduce((sum, b) => sum + b.amount, 0));
	const earlier = $derived((data?.analysis.earlier ?? []).reduce((sum, b) => sum + b.amount, 0));
	const delta = $derived(total - earlier);

	const spark = $derived.by(() => {
		const history = data?.analysis.history;
		if (!history) return [];

		return history.dates.map((d, i) => [
			d.getTime(),
			history.byAccount.reduce((sum, s) => sum + s.amounts[i], 0) / 100
		]);
	});

	const sparkOptions = $derived({
		chart: {
			type: 'area',
			height: 120,
			sparkline: { enabled: true },
			animations: { enabled: false }
		},
		series: [{ name: 'Vermögen', data: spark }],
		colors: [WEALTH_COLOR],
		stroke: { curve: 'smooth', width: 2 },
		fill: { type: 'gradient', gradient: { opacityFrom: 0.4, opacityTo: 0 } },
		xaxis: { type: 'datetime' },
		tooltip: {
			x: { format: 'dd.MM.yyyy' },
			y: { formatter: (v: number) => euroExact(v * 100) }
		}
	} as ApexOptions);

	const months = $derived(data?.analysis.overviews.months ?? []);

	/// The running month, which is the last one the range covers, and the one before it.
	const thisMonth = $derived(months.at(-1));
	const previousMonth = $derived(months.length > 1 ? months.at(-2) : undefined);

	/// Spending and income live in the same rows with opposite signs, and a donut can only draw
	/// one of them. Everything below reads its figure through here.
	let sign = $state<'expenses' | 'income'>('expenses');
	const amount = (of: { income: number; expenses: number } | undefined) =>
		Math.abs((sign === 'income' ? of?.income : of?.expenses) ?? 0);

	/// How much of the month has run.
	///
	/// The comparison is against the previous month scaled to this, since what is known about
	/// the previous month is its whole: without the scaling the running month would report a
	/// shortfall every time, right up to its last day.
	const monthShare = $derived(now.getDate() / daysInMonth(now));

	function daysInMonth(d: Date): number {
		return new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
	}

	/// What the running month is ahead or behind by, in cents; positive is more than last month.
	const comparison = $derived(
		previousMonth ? amount(thisMonth) - amount(previousMonth) * monthShare : null
	);

	const donut = $derived.by(() => {
		const slices = (thisMonth?.byCategory ?? [])
			.map((c) => ({
				name: c.category?.name ?? 'Ohne Kategorie',
				color: categoryColor(c.category),
				value: amount(c) / 100
			}))
			.filter((s) => s.value > 0)
			.sort((a, b) => b.value - a.value);

		return {
			chart: { type: 'donut', height: 273, animations: { enabled: false } },
			series: slices.map((s) => s.value),
			labels: slices.map((s) => s.name),
			colors: slices.map((s) => s.color),
			stroke: { width: 0 },
			dataLabels: { enabled: false },
			legend: { position: 'bottom', fontSize: '12px' },
			tooltip: { y: { formatter: (v: number) => euroExact(v * 100) } },
			plotOptions: {
				pie: {
					donut: {
						size: '68%',
						labels: {
							show: true,
							// the number in the middle when a slice is hovered, which is a
							// different formatter from the total's and from the tooltip's --
							// apex hands it the value as a string off the DOM
							value: { formatter: (v: string) => euroExact(Number(v) * 100) },
							total: {
								show: true,
								label: sign === 'income' ? 'Eingenommen' : 'Ausgegeben',
								formatter: () => euro(amount(thisMonth))
							}
						}
					}
				}
			}
		} as ApexOptions;
	});

	const lastMonth = $derived(new Date(now.getFullYear(), now.getMonth() - 1, 1));

	/// Whether the feed can be about the time we were away.
	///
	/// Only if something actually happened in it, and only from the second visit on. Otherwise
	/// the column would be a heading over an empty box, which is what the feed used to be, and
	/// it falls back to the newest entries under a heading that promises no more than that.
	const sinceLastVisit = $derived(
		!firstVisit && since !== null && (data?.expenses.recent.totalCount ?? 0) > 0
	);

	const changeCount = $derived(data?.expenses.changes.totalCount ?? 0);
	const reviewCount = $derived(data?.expenses.review.totalCount ?? 0);
	const oldestReview = $derived(data?.expenses.review.edges[0]?.node);

	const monthName = (d: Date) => d.toLocaleDateString('de-DE', { month: 'long' });

	/// What the window is called. A first visit has nothing to have been away from, and a
	/// marker set today reads better as the day than as "seit deinem letzten Besuch", which
	/// sounds like weeks.
	const windowLabel = $derived.by(() => {
		if (!since || firstVisit) return `in den letzten ${RECENT_DAYS} Tagen`;

		const days = Math.floor((now.getTime() - since.getTime()) / DAY);
		if (days < 1) return 'seit heute früh';
		if (days === 1) return 'seit gestern';
		if (days < 7) return `seit ${since.toLocaleDateString('de-DE', { weekday: 'long' })}`;
		return `seit dem ${shortDate(since)}`;
	});

	const lastMonthHref = $derived(
		resolve('/analysis/[year]/[month]', {
			year: `${lastMonth.getFullYear()}`,
			month: `${lastMonth.getMonth() + 1}`
		})
	);
</script>

<div class="mx-auto max-w-7xl space-y-4 px-4 py-2 sm:px-8">
	<QueryError
		errors={$dashboardStore.errors}
		message="Die Übersicht konnte nicht geladen werden."
	/>
	<!-- Hero: the number the whole app exists to produce, and the shape it got there by. On a
	     phone the curve goes under it; from lg they share the row.

	     Dropped entirely when the query failed, rather than left standing empty: its greeting
	     runs into the figure ("Hallo Thies, du hast …"), and without one it is a sentence that
	     stops mid-way inside an otherwise blank card. The banner above has already said why. -->
	{#if !$dashboardStore.errors}
		<Card class="max-w-none border-0 bg-white p-4 sm:p-6 dark:bg-gray-800">
			<div class="grid items-center gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)]">
				<div>
					<p class="text-sm text-gray-500 dark:text-gray-400">
						Hallo{greeting ? ` ${greeting}` : ''}, du hast
					</p>
					<!-- on `data` rather than on `fetching`, which is false before the first fetch is
				     issued -- the page is rendered on the server, where there is none -->
					{#if !data}
						<!-- the banner above already says what happened; a spinner under it would go on
					     forever, since a failed query leaves `data` null for good -->
						{#if !$dashboardStore.errors}
							<div class="py-3"><Spinner size="6" /></div>
						{/if}
					{:else}
						<p class="text-4xl font-semibold tracking-tight tabular-nums sm:text-5xl">
							{euro(total)}
						</p>
						<p class="mt-1 text-sm">
							<span
								class={delta < 0
									? 'text-red-600 dark:text-red-400'
									: delta > 0
										? 'text-green-600 dark:text-green-400'
										: 'text-gray-500 dark:text-gray-400'}
							>
								{delta > 0 ? '+' : ''}{euroExact(delta)}
							</span>
							<!-- the month spelt out and the year left off: this is always a month back,
						     so the year is either obvious or the same one -->
							<span class="text-gray-500 dark:text-gray-400">
								im Vergleich zum {monthAgo.toLocaleDateString('de-DE', {
									day: 'numeric',
									month: 'long'
								})}
							</span>
						</p>
					{/if}
				</div>
				<div class="min-w-0">
					<Chart options={sparkOptions} />
				</div>
			</div>
		</Card>
	{/if}

	<!-- min-w-0 on the cell as well as on the grid: a grid item is min-content wide by default,
	     and apexcharts measures the box it is handed. The donut asked for 556px on a 390px
	     screen, the column grew to fit it, and the whole page scrolled sideways. -->
	<div class="grid min-w-0 gap-4 lg:grid-cols-2">
		<div class="min-w-0 space-y-4">
			<!-- Was ansteht: each card is a link to the thing it is talking about, and none of
			     them show up when there is nothing to say -->
			<div class="space-y-2">
				<a
					href="{resolve('/expenses')}?new"
					class="hover:border-primary-400 flex items-center gap-3 rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
				>
					<span
						class="bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300 flex h-9 w-9 shrink-0 items-center justify-center rounded-full"
					>
						<PlusOutline class="h-5 w-5" />
					</span>
					<span class="min-w-0 flex-1">
						<span class="block font-medium">Neue Ausgabe eintragen</span>
						<span class="block text-sm text-gray-500 dark:text-gray-400">
							Formular öffnet sich direkt
						</span>
					</span>
					<ArrowRightOutline class="h-4 w-4 shrink-0 text-gray-400" />
				</a>

				{#if changeCount > 0}
					<a
						href="{resolve('/analysis')}?tab=activity"
						class="hover:border-primary-400 flex items-center gap-3 rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
					>
						<span
							class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300"
						>
							<BellRingOutline class="h-5 w-5" />
						</span>
						<span class="min-w-0 flex-1">
							<span class="block font-medium">
								{changeCount}
								{changeCount === 1 ? 'Änderung' : 'Änderungen'} von anderen
							</span>
							<span class="block text-sm text-gray-500 dark:text-gray-400">
								{windowLabel} — ansehen
							</span>
						</span>
						<ArrowRightOutline class="h-4 w-4 shrink-0 text-gray-400" />
					</a>
				{/if}

				{#if reviewCount > 0}
					<a
						href="{resolve('/expenses')}?review"
						class="hover:border-primary-400 flex items-center gap-3 rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
					>
						<span
							class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300"
						>
							<BellRingOutline class="h-5 w-5" />
						</span>
						<span class="min-w-0 flex-1">
							<span class="block font-medium">
								{reviewCount}
								{reviewCount === 1 ? 'Ausgabe braucht' : 'Ausgaben brauchen'} Prüfung
							</span>
							<span class="block truncate text-sm text-gray-500 dark:text-gray-400">
								{#if oldestReview}
									<!-- the age of the oldest is what says whether this has been sitting
									     there; a count alone reads the same after a day and after a year -->
									älteste: {oldestReview.title || 'Ohne Titel'} vom
									{shortDate(oldestReview.accrualStart)}
								{:else}
									ungeprüft oder unvollständig
								{/if}
							</span>
						</span>
						<ArrowRightOutline class="h-4 w-4 shrink-0 text-gray-400" />
					</a>
				{/if}

				<a
					href={lastMonthHref}
					class="hover:border-primary-400 flex items-center gap-3 rounded-lg border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
				>
					<span
						class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300"
					>
						<CalendarMonthOutline class="h-5 w-5" />
					</span>
					<span class="min-w-0 flex-1">
						<span class="block font-medium">{monthName(lastMonth)} durchgehen</span>
						<span class="block text-sm text-gray-500 dark:text-gray-400">
							Einnahmen, Ausgaben und Kategorien des Monats
						</span>
					</span>
					<ArrowRightOutline class="h-4 w-4 shrink-0 text-gray-400" />
				</a>
			</div>

			<Card class="max-w-none border-0 bg-white p-4 dark:bg-gray-800">
				<!-- the comparison on its own line: beside the toggle it was a sentence competing
				     with a control for the same row, and it wrapped into it on the way -->
				<div class="flex items-center justify-between gap-3">
					<h2 class="text-lg font-semibold">{monthName(monthStart)}</h2>
					<ToggleGroup
						bind:value={sign}
						options={[
							{ value: 'expenses', label: 'Ausgaben' },
							{ value: 'income', label: 'Einnahmen' }
						]}
					/>
				</div>
				{#if comparison !== null}
					<p class="mt-0.5 text-sm text-gray-500 dark:text-gray-400">
						{euro(Math.abs(comparison))}
						{comparison > 0 ? 'mehr' : 'weniger'} als zu dieser Zeit im Vormonat
					</p>
				{/if}
				{#if !data}
					{#if !$dashboardStore.errors}
						<div class="flex justify-center py-12"><Spinner size="6" /></div>
					{/if}
				{:else if amount(thisMonth) > 0}
					<Chart options={donut} />
				{:else}
					<p class="py-8 text-center text-sm text-gray-500 dark:text-gray-400">
						Diesen Monat noch {sign === 'income' ? 'nichts eingenommen' : 'nichts ausgegeben'}.
					</p>
				{/if}
			</Card>
		</div>

		<!-- The feed is as long as the history is, which left the left column ending halfway up
		     the page. Absolute inside a relative cell so the row is sized by the left column
		     alone and the feed scrolls inside whatever is left; static below lg, where the two
		     are stacked and there is nothing to line up with. -->
		<div class="min-w-0 lg:relative">
			<Card
				class="flex max-w-none flex-col border-0 bg-white p-4 lg:absolute lg:inset-0 dark:bg-gray-800"
			>
				<div class="flex items-baseline justify-between gap-2">
					<h2 class="text-lg font-semibold">
						{sinceLastVisit ? 'Seit du das letzte Mal hier warst' : 'Zuletzt passiert'}
					</h2>
					<a
						href="{resolve('/analysis')}?tab=activity"
						class="text-primary-600 dark:text-primary-400 shrink-0 text-sm hover:underline"
					>
						alle anzeigen
					</a>
				</div>
				<!-- Otherwise the fallback is indistinguishable from the real thing: a list of
				     changes under a heading, with nothing saying that none of them are new. -->
				<p class="mt-0.5 mb-2 text-sm text-gray-500 dark:text-gray-400">
					{#if sinceLastVisit}
						Seit deinem letzten Besuch am {since ? shortDate(since) : ''}
					{:else if firstVisit}
						Die letzten Änderungen
					{:else}
						Seit deinem letzten Besuch ist nichts passiert — hier die letzten Änderungen
					{/if}
				</p>
				<!-- min-h-0, or the flex child refuses to shrink below its content and the card
				     grows the row after all. The scroll is only for the lg case, where the feed has
				     to fit a cell sized by the column beside it: a scrolling box inside a scrolling
				     page is a trap on a touch screen, and there is nothing to line up with anyway. -->
				<div class="min-h-0 flex-1 lg:overflow-y-auto">
					<ExpenseChangelog
						limit={25}
						controls={false}
						author="ANYONE"
						since={sinceLastVisit ? since : null}
					/>
				</div>
			</Card>
		</div>
	</div>
</div>
