<script lang="ts">
	import { graphql } from '$houdini';

	import { Timeline, TimelineItem } from 'flowbite-svelte';
	import { CalendarWeekSolid } from 'flowbite-svelte-icons';
	import { ChevronDownOutline, ChevronRightOutline } from 'flowbite-svelte-icons';

	let expandedDiffs = $state<string[]>([]);

	const changelogStore = graphql(`
		query ExpenseChangelog {
			users {
				me {
					name
				}
			}

			expenses {
				changelog(limit: 100, skip: 0) {
					edges {
						node {
							id
							expenseId
							date
							tool
							diff
							user {
								name
								firstName
							}
							oldState {
								__typename # just to find out if it was just created or not
							}
							newState {
								title
								bookingStart
							}
						}
					}
				}
			}
		}
	`);

	$effect(() => void changelogStore.fetch());

	const changelogEntries = $derived(
		($changelogStore.data?.expenses.changelog.edges || []).sort(
			(a, b) => b.node.date.getTime() - a.node.date.getTime()
		)
	);

	const toggleDiff = (id: string) => {
		if (expandedDiffs.includes(id)) expandedDiffs = expandedDiffs.filter((rowId) => rowId !== id);
		else expandedDiffs = [...expandedDiffs, id];
	};
</script>

<Timeline order="vertical">
	{#each changelogEntries as entry (entry.node.id)}
		<TimelineItem
			title={(entry.node.newState?.title || '') +
				(' (' + entry.node.newState?.bookingStart.toLocaleDateString() + ')')}
			dateFormat="full-date"
			date={`${entry.node.date.getFullYear()}-${String(entry.node.date.getMonth() + 1).padStart(2, '0')}-${String(entry.node.date.getDate()).padStart(2, '0')}`}
		>
			{#snippet orientationSlot()}
				<span
					class="bg-primary-200 dark:bg-primary-900 absolute -start-3 flex h-6 w-6 items-center justify-center rounded-full ring-8 ring-white dark:ring-gray-900"
				>
					<CalendarWeekSolid class="text-primary-600 dark:text-primary-400 h-4 w-4" />
				</span>
			{/snippet}
			<p class="mb-4 text-base font-normal text-gray-500 dark:text-gray-400">
				{entry.node.user?.name == $changelogStore.data?.users.me?.name
					? 'Du hast'
					: entry.node.user?.firstName + ' hat'}
				die Ausgabe
				{entry.node.oldState ? 'verändert' : 'erstellt'}

				{#if entry.node.tool}
					(via {entry.node.tool})
				{/if}
			</p>

			{#if entry.node.oldState && entry.node.diff}
				<div class="mb-4">
					<button
						class="flex items-center gap-2 text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
						onclick={() => toggleDiff(entry.node.id.toString())}
					>
						{#if expandedDiffs.includes(entry.node.id.toString())}
							<ChevronDownOutline class="h-3 w-3" />
						{:else}
							<ChevronRightOutline class="h-3 w-3" />
						{/if}
						<span>Show diff</span>
					</button>
					{#if expandedDiffs.includes(entry.node.id.toString())}
						<pre
							class="mt-2 font-mono text-xs whitespace-pre-wrap text-gray-500 dark:text-gray-400">{entry
								.node.diff}</pre>
					{/if}
				</div>
			{/if}
		</TimelineItem>
	{/each}
</Timeline>
