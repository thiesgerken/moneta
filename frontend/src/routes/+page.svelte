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
	import { categoryColor } from '$lib/analysis/overviews';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', {
			style: 'currency',
			currency: 'EUR',
			maximumFractionDigits: 0
		});

	const euroExact = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const DAY = 86400000;

	/// How far back "neu seit" reaches until the last visit is recorded.
	const RECENT_DAYS = 7;

	const now = new Date();
	const recentSince = new Date(now.getTime() - RECENT_DAYS * DAY);
	const monthStart = new Date(now.getFullYear(), now.getMonth(), 1);
	// six whole months before the running one, which is what the running one is compared against
	const compareFrom = new Date(now.getFullYear(), now.getMonth() - 6, 1);
	const yearAgo = new Date(now.getFullYear() - 1, now.getMonth(), now.getDate());

	const dashboardStore = graphql(`
		query Dashboard(
			$now: DateTime!
			$since: DateTime!
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
				earlier: balances(date: $since) {
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
						expenses
						byCategory {
							category {
								id
								name
								color
							}
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
				# what the "pruefen" link on /expenses lands on, so the number and the list agree
				review: list(
					limit: 1
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
				}
			}
		}
	`);

	$effect(() => {
		void dashboardStore.fetch({
			variables: {
				now,
				since: recentSince,
				monthsFrom: compareFrom,
				sparkFrom: yearAgo
			}
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
		colors: ['#f05a28'],
		stroke: { curve: 'smooth', width: 2 },
		fill: { type: 'gradient', gradient: { opacityFrom: 0.4, opacityTo: 0 } },
		xaxis: { type: 'datetime' },
		tooltip: {
			x: { format: 'dd.MM.yyyy' },
			y: { formatter: (v: number) => euroExact(v * 100) }
		}
	} as ApexOptions);

	const months = $derived(data?.analysis.overviews.months ?? []);

	/// The running month, which is the last one the range covers.
	const thisMonth = $derived(months.at(-1));

	/// What the months before it averaged, as something to hold the running one against. The
	/// running month is left out of its own comparison, and so is anything before the range.
	const average = $derived.by(() => {
		const earlier = months.slice(0, -1);
		if (earlier.length === 0) return null;
		return earlier.reduce((sum, m) => sum + m.expenses, 0) / earlier.length;
	});

	/// How the running month compares, as a share of the average. Only meaningful once enough
	/// of the month has passed for the comparison to say something -- before that it always
	/// reads as "spending less", because the month is not over.
	const monthShare = $derived(now.getDate() / daysInMonth(now));

	function daysInMonth(d: Date): number {
		return new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
	}

	const donut = $derived.by(() => {
		const slices = (thisMonth?.byCategory ?? [])
			// income sits in the same list with the opposite sign, and a donut cannot draw it
			.filter((c) => c.expenses < 0)
			.map((c) => ({
				name: c.category?.name ?? 'Ohne Kategorie',
				color: categoryColor(c.category),
				value: Math.abs(c.expenses) / 100
			}))
			.sort((a, b) => b.value - a.value);

		return {
			chart: { type: 'donut', height: 260, animations: { enabled: false } },
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
							total: {
								show: true,
								label: 'Ausgegeben',
								formatter: () => euro(Math.abs(thisMonth?.expenses ?? 0))
							}
						}
					}
				}
			}
		} as ApexOptions;
	});

	const lastMonth = $derived(new Date(now.getFullYear(), now.getMonth() - 1, 1));

	const changeCount = $derived(data?.expenses.changes.totalCount ?? 0);
	const reviewCount = $derived(data?.expenses.review.totalCount ?? 0);

	const monthName = (d: Date) => d.toLocaleDateString('de-DE', { month: 'long' });

	const feedHref = resolve('/analysis');
	const lastMonthHref = $derived(
		resolve('/analysis/[year]/[month]', {
			year: `${lastMonth.getFullYear()}`,
			month: `${lastMonth.getMonth() + 1}`
		})
	);
</script>

<div class="mx-auto max-w-7xl space-y-4 px-4 py-2 sm:px-8">
	<!-- Hero: the number the whole app exists to produce, and the shape it got there by. On a
	     phone the curve goes under it; from lg they share the row. -->
	<Card class="max-w-none border-0 bg-white p-4 sm:p-6 dark:bg-gray-800">
		<div class="grid items-center gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)]">
			<div>
				<p class="text-sm text-gray-500 dark:text-gray-400">
					Hallo{greeting ? ` ${greeting}` : ''}, dein Vermögen beträgt
				</p>
				<!-- on `data` rather than on `fetching`, which is false before the first fetch is
				     issued -- the page is rendered on the server, where there is none -->
				{#if !data}
					<div class="py-3"><Spinner size="6" /></div>
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
						<span class="text-gray-500 dark:text-gray-400">
							in den letzten {RECENT_DAYS} Tagen
						</span>
					</p>
				{/if}
			</div>
			<div class="min-w-0">
				<Chart options={sparkOptions} />
			</div>
		</div>
	</Card>

	<div class="grid gap-4 lg:grid-cols-2">
		<div class="space-y-4">
			<!-- Was ansteht: each card is a link to the thing it is talking about, and none of
			     them show up when there is nothing to say -->
			<div class="space-y-2">
				<a
					href="{resolve('/expenses')}?neu"
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
						href={feedHref}
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
								in den letzten {RECENT_DAYS} Tagen — ansehen
							</span>
						</span>
						<ArrowRightOutline class="h-4 w-4 shrink-0 text-gray-400" />
					</a>
				{/if}

				{#if reviewCount > 0}
					<a
						href="{resolve('/expenses')}?pruefen"
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
								{reviewCount === 1 ? 'Ausgabe' : 'Ausgaben'} brauchen Prüfung
							</span>
							<span class="block text-sm text-gray-500 dark:text-gray-400">
								ungeprüft oder unvollständig
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
				<div class="mb-1 flex items-baseline justify-between gap-2">
					<h2 class="text-lg font-semibold">{monthName(monthStart)}</h2>
					{#if average !== null && thisMonth}
						<!-- against the same share of the average as the month has run, or the
						     comparison would say "weniger" on every first of the month -->
						{@const expected = average * monthShare}
						{@const diff = Math.abs(thisMonth.expenses) - Math.abs(expected)}
						<span class="text-sm text-gray-500 dark:text-gray-400">
							{diff > 0 ? 'mehr' : 'weniger'} als sonst um diese Zeit: {euro(Math.abs(diff))}
						</span>
					{/if}
				</div>
				{#if !data}
					<div class="flex justify-center py-12"><Spinner size="6" /></div>
				{:else if thisMonth && thisMonth.byCategory.some((c) => c.expenses < 0)}
					<Chart options={donut} />
				{:else}
					<p class="py-8 text-center text-sm text-gray-500 dark:text-gray-400">
						Diesen Monat noch nichts ausgegeben.
					</p>
				{/if}
			</Card>
		</div>

		<Card class="max-w-none border-0 bg-white p-4 dark:bg-gray-800">
			<div class="mb-2 flex items-baseline justify-between gap-2">
				<h2 class="text-lg font-semibold">Zuletzt passiert</h2>
				<a href={feedHref} class="text-primary-600 dark:text-primary-400 text-sm hover:underline">
					alle anzeigen
				</a>
			</div>
			<ExpenseChangelog limit={6} controls={false} author="ANYONE" />
		</Card>
	</div>
</div>
