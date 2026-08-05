<script lang="ts">
	import { Card, Button } from 'flowbite-svelte';
	import { ChevronLeftOutline, ChevronRightOutline } from 'flowbite-svelte-icons';
	import { calendarExpenses } from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	let month = $state(new Date(2026, 6, 1));

	const shift = (n: number) => {
		month = new Date(month.getFullYear(), month.getMonth() + n, 1);
	};

	const daysInMonth = $derived(new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate());
	// getDay() is Sunday-based, but the week starts on Monday here
	const leading = $derived((new Date(month.getFullYear(), month.getMonth(), 1).getDay() + 6) % 7);

	// one cell per day, so leading blanks and days without expenses stay part of the grid
	const days = $derived(
		Array.from({ length: daysInMonth }, (_, i) => ({
			day: i + 1,
			entries: calendarExpenses.filter((e) => e.day === i + 1)
		}))
	);

	const monthTotal = $derived(calendarExpenses.reduce((sum, e) => sum + e.amount, 0));
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
	</div>

	<Card class="max-w-none p-4">
		<div class="grid grid-cols-7 gap-px">
			{#each ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'] as d (d)}
				<div class="pb-1 text-center text-xs font-semibold text-gray-500 dark:text-gray-400">
					{d}
				</div>
			{/each}
			{#each days as { day, entries } (day)}
				<div
					class="min-h-24 rounded border border-gray-200 p-1 text-xs dark:border-gray-700 {entries.length
						? ''
						: 'bg-gray-50 dark:bg-gray-900'}"
					style={day === 1 ? `grid-column-start: ${leading + 1}` : undefined}
				>
					<div class="flex justify-between">
						<span class="font-semibold">{day}</span>
						{#if entries.length}
							<span class="text-red-600 dark:text-red-400">
								{euro(entries.reduce((sum, e) => sum + e.amount, 0))}
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
</div>
