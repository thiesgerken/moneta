<script lang="ts">
	import { Card, Button, Spinner } from 'flowbite-svelte';
	import { ChevronLeftOutline, ChevronRightOutline } from 'flowbite-svelte-icons';
	import { graphql } from '$houdini';
	import { categoryColor } from './format';
	import { DATE_AXES, accrued, axisState, calendarState } from './state.svelte';
	import ToggleGroup from '#lib/ToggleGroup.svelte';
	import { ANALYSIS_CARD } from './cards';
	import { shiftMonth } from './dates';
	import { resolve } from '$app/paths';
	import QueryError from '#lib/QueryError.svelte';
	import { amountColor, euro } from '#lib/money.ts';

	const month = $derived(new Date(calendarState.month));

	/// Days whose chips are all showing rather than the first three. By day number, and cleared
	/// on the way out of the month, because day 12 of the next one is a different day.
	let expandedDays = $state<number[]>([]);
	const toggleDay = (day: number) => {
		expandedDays = expandedDays.includes(day)
			? expandedDays.filter((d) => d !== day)
			: [...expandedDays, day];
	};

	const shift = (n: number) => {
		calendarState.month = shiftMonth(calendarState.month, n);
		expandedDays = [];
	};

	const nextMonth = $derived(new Date(month.getFullYear(), month.getMonth() + 1, 1));

	// Which of the two questions the grid answers: what left an account on this day, or what
	// this day cost. A prepayment's period spans a year while its payment lands on one of
	// them, so the two months look nothing alike.
	const calendarStore = graphql(`
		query AnalysisCalendar(
			$valuedAfter: DateTime
			$valuedBefore: DateTime
			$accruedAfter: DateTime
			$accruedBefore: DateTime
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
					limit: 1000
					sortBy: ACCRUAL_START
				) {
					edges {
						node {
							id
							title
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
							transactions {
								date
								effectiveAmount
							}
						}
					}
				}
			}
		}
	`);

	$effect(() => {
		// Both pairs every time, the unused one explicitly null: houdini merges variables into
		// the ones the last fetch used, so leaving them out kept the previous axis's window
		// alive and asked for expenses accruing in this month that were paid in the last one.
		//
		// The accrual pair overlaps rather than contains, which is what brings in a period that
		// runs through this month without starting or ending in it.
		const on = accrued();
		void calendarStore.fetch({
			variables: {
				valuedAfter: on ? null : month,
				valuedBefore: on ? null : nextMonth,
				accruedAfter: on ? month : null,
				accruedBefore: on ? nextMonth : null
			}
		});
	});

	const expenses = $derived($calendarStore.data?.expenses.list.edges.map((e) => e.node) ?? []);

	const daysInMonth = $derived(new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate());
	// getDay() is Sunday-based, but the week starts on Monday here
	const leading = $derived((new Date(month.getFullYear(), month.getMonth(), 1).getDay() + 6) % 7);

	/// The heaviest category decides the colour of an expense's chip; a split expense has to
	/// pick one, and the biggest share is the least arbitrary choice.
	const chipColor = (e: (typeof expenses)[number]) => {
		const heaviest = [...e.categories].sort((a, b) => b.weight - a.weight)[0];
		return categoryColor(heaviest?.category ?? null);
	};

	/// Midnight local, so two timestamps can be compared as calendar days. The accrual bounds
	/// carry a time of day nobody chose -- midday UTC for an imported expense, 02:00 for a
	/// generated one -- and comparing those directly would drop a day at either end.
	const midnight = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
	const DAY_MS = 86400000;
	/// Rounded rather than divided exactly: a period crossing a DST switch is 23 or 25 hours
	/// longer than the whole number of days it is.
	const daysBetween = (a: Date, b: Date) => Math.round((midnight(b) - midnight(a)) / DAY_MS) + 1;

	// One cell per day, so days without expenses stay part of the grid.
	//
	// On the value axis an expense lands on each day it has a transaction, with that day's
	// share. On the accrual axis it lands on every day of its Leistungszeitraum, each carrying
	// an equal share of the cost -- the same linear consumption the monthly totals and the
	// Vorauszahlungen view use. A yearly insurance is therefore a small chip every day rather
	// than one big one, which is what the axis is for.
	const days = $derived.by(() =>
		Array.from({ length: daysInMonth }, (_, i) => {
			const day = i + 1;
			const date = new Date(month.getFullYear(), month.getMonth(), day);

			const entries = expenses
				.map((e) => {
					if (accrued()) {
						const inside =
							midnight(e.accrualStart) <= date.getTime() &&
							date.getTime() <= midnight(e.accrualEnd);
						if (!inside) return null;

						const share = e.effectiveAmount / daysBetween(e.accrualStart, e.accrualEnd);
						return { id: e.id, title: e.title, amount: share, color: chipColor(e) };
					}

					const legs = e.transactions.filter(
						(t) =>
							t.date >= month &&
							t.date < nextMonth &&
							t.date.getDate() === day &&
							t.date.getMonth() === month.getMonth()
					);
					if (legs.length === 0) return null;

					const amount = legs.reduce((sum, t) => sum + t.effectiveAmount, 0);
					return { id: e.id, title: e.title, amount, color: chipColor(e) };
				})
				.filter((e) => e !== null);

			return { day, entries, total: entries.reduce((sum, e) => sum + e.amount, 0) };
		})
	);

	const monthTotal = $derived(days.reduce((sum, d) => sum + d.total, 0));
</script>

<div class="space-y-4">
	<div class="flex items-center gap-3">
		<Button color="alternative" size="sm" onclick={() => shift(-1)}>
			<ChevronLeftOutline class="h-4 w-4" />
		</Button>
		<!-- the month you are looking at is also a page of its own, and this is the only place
		     that already knows which one -->
		<a
			class="hover:text-primary-600 dark:hover:text-primary-500 w-48 text-center text-lg font-semibold hover:underline"
			href={resolve('/analysis/[year]/[[month]]', {
				year: `${month.getFullYear()}`,
				month: `${month.getMonth() + 1}`
			})}
		>
			{month.toLocaleDateString('de-DE', { month: 'long', year: 'numeric' })}
		</a>
		<Button color="alternative" size="sm" onclick={() => shift(1)}>
			<ChevronRightOutline class="h-4 w-4" />
		</Button>
		<span class="ml-4 text-gray-500 dark:text-gray-400">Summe: {euro(monthTotal)}</span>
		<ToggleGroup bind:value={axisState.axis} options={DATE_AXES} size="sm" />
		{#if $calendarStore.fetching}
			<Spinner size="4" />
		{/if}
	</div>

	{#if $calendarStore.errors}
		<QueryError
			errors={$calendarStore.errors}
			message="Der Kalender konnte nicht geladen werden."
		/>
	{:else}
		<Card class={ANALYSIS_CARD}>
			<div class="grid grid-cols-7 gap-px">
				{#each ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'] as d (d)}
					<div class="pb-1 text-center text-xs font-semibold text-gray-500 dark:text-gray-400">
						{d}
					</div>
				{/each}
				{#each days as { day, entries, total } (day)}
					<div
						class="min-h-24 rounded border border-gray-200 p-1 text-xs dark:border-gray-700 {entries.length
							? ''
							: 'bg-gray-50 dark:bg-gray-900'}"
						style={day === 1 ? `grid-column-start: ${leading + 1}` : undefined}
					>
						<div class="flex justify-between">
							<span class="font-semibold">{day}</span>
							{#if entries.length}
								<span class={amountColor(total)}>
									{euro(total)}
								</span>
							{/if}
						</div>
						{#each expandedDays.includes(day) ? entries : entries.slice(0, 3) as e (e.id)}
							<div class="mt-0.5 truncate rounded px-1" style="background-color: {e.color}33">
								{e.title}
							</div>
						{/each}
						{#if entries.length > 3}
							<!-- On the accrual axis every subscription is on every day of its period, so
							     a cell routinely has a dozen. The count was the only thing saying so and
							     there was no way to read them. -->
							<button
								type="button"
								class="mt-0.5 w-full text-left text-gray-500 hover:underline dark:text-gray-400"
								onclick={() => toggleDay(day)}
							>
								{expandedDays.includes(day) ? 'weniger' : `+${entries.length - 3} weitere`}
							</button>
						{/if}
					</div>
				{/each}
			</div>
		</Card>
	{/if}
</div>
