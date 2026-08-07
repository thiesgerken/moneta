<script lang="ts">
	import { Card, Button, Spinner } from 'flowbite-svelte';
	import { ChevronLeftOutline, ChevronRightOutline } from 'flowbite-svelte-icons';
	import { graphql } from '$houdini';
	import { categoryColor } from './overviews';
	import { calendarState } from './state.svelte';
	import { shiftMonth } from './dates';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const month = $derived(new Date(calendarState.month));
	const shift = (n: number) => {
		calendarState.month = shiftMonth(calendarState.month, n);
	};

	const nextMonth = $derived(new Date(month.getFullYear(), month.getMonth() + 1, 1));

	// Filtered on the transaction dates rather than the accrual period: the calendar answers
	// "what happened on this day", and a prepayment's period spans a year while its payment
	// lands on one of them.
	const calendarStore = graphql(`
		query AnalysisCalendar($after: DateTime!, $before: DateTime!) {
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
					limit: 1000
					sortBy: ACCRUAL_START
				) {
					edges {
						node {
							id
							title
							effectiveAmount
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
		void calendarStore.fetch({ variables: { after: month, before: nextMonth } });
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

	// One cell per day, so days without expenses stay part of the grid. An expense lands on
	// each day it has a transaction in this month, with only that day's share.
	const days = $derived.by(() =>
		Array.from({ length: daysInMonth }, (_, i) => {
			const day = i + 1;
			const entries = expenses
				.map((e) => {
					const legs = e.transactions.filter(
						(t) =>
							t.date >= month &&
							t.date < nextMonth &&
							t.date.getDate() === day &&
							t.date.getMonth() === month.getMonth()
					);
					const amount = legs.reduce((sum, t) => sum + t.effectiveAmount, 0);
					return { id: e.id, title: e.title, amount, color: chipColor(e), legs: legs.length };
				})
				.filter((e) => e.legs > 0);

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
		<span class="w-48 text-center text-lg font-semibold">
			{month.toLocaleDateString('de-DE', { month: 'long', year: 'numeric' })}
		</span>
		<Button color="alternative" size="sm" onclick={() => shift(1)}>
			<ChevronRightOutline class="h-4 w-4" />
		</Button>
		<span class="ml-4 text-gray-500 dark:text-gray-400">Summe: {euro(monthTotal)}</span>
		{#if $calendarStore.fetching}
			<Spinner size="4" />
		{/if}
	</div>

	{#if $calendarStore.errors}
		<p class="text-sm text-red-600 dark:text-red-400">
			{$calendarStore.errors.map((e) => e.message).join(', ')}
		</p>
	{:else}
		<Card class="max-w-none p-4">
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
								<span
									class={total < 0
										? 'text-red-600 dark:text-red-400'
										: 'text-green-700 dark:text-green-400'}
								>
									{euro(total)}
								</span>
							{/if}
						</div>
						{#each entries.slice(0, 3) as e (e.id)}
							<div class="mt-0.5 truncate rounded px-1" style="background-color: {e.color}33">
								{e.title}
							</div>
						{/each}
						{#if entries.length > 3}
							<div class="mt-0.5 text-gray-500 dark:text-gray-400">
								+{entries.length - 3} weitere
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</Card>
	{/if}
</div>
