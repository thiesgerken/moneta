<script lang="ts">
	import { graphql } from '$houdini';

	import { Timeline, TimelineItem } from 'flowbite-svelte';
	import { CalendarWeekSolid } from 'flowbite-svelte-icons';

	const changelogStore = graphql(`
		query ExpenseChangelog @load {
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

	const changelogEntries = $derived(
		($changelogStore.data?.expenses.changelog.edges || []).sort(
			(a, b) => b.node.date.getTime() - a.node.date.getTime()
		)
	);
</script>

<Timeline order="vertical">
	{#each changelogEntries as entry}
		<TimelineItem
			title={(entry.node.newState?.title || '') +
				(' (' + entry.node.newState?.bookingStart.toLocaleDateString() + ')')}
			dateFormat={'full-date'}
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
				<p class="mb-4 text-base font-normal font-mono text-gray-500 dark:text-gray-400">
					{entry.node.diff}
				</p>
			{/if}
		</TimelineItem>
	{/each}
</Timeline>
