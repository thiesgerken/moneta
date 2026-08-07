<script lang="ts">
	import { Tabs, TabItem } from 'flowbite-svelte';
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
	import WealthView from '$lib/analysis/WealthView.svelte';
	import OverviewsView from '$lib/analysis/OverviewsView.svelte';
	import CategoriesView from '$lib/analysis/CategoriesView.svelte';
	import CalendarView from '$lib/analysis/CalendarView.svelte';
	import PrepaymentsView from '$lib/analysis/PrepaymentsView.svelte';
	import StatementView from '$lib/analysis/StatementView.svelte';
</script>

<div class="space-y-4 px-4 sm:px-8">
	<!-- Seven tabs do not fit a phone, and without a scroller of their own they widen the page
	     instead: the last four were only reachable by dragging the whole layout sideways. -->
	<Tabs
		tabStyle="underline"
		class="flex-nowrap overflow-x-auto whitespace-nowrap"
		classes={{ content: 'mt-4 rounded-lg bg-gray-50 p-2 sm:p-4 dark:bg-gray-800' }}
	>
		<TabItem open title="Aktivität">
			<ExpenseChangelog />
		</TabItem>
		<TabItem title="Vermögen">
			<WealthView />
		</TabItem>
		<TabItem title="Monate">
			<OverviewsView />
		</TabItem>
		<TabItem title="Kategorien">
			<CategoriesView />
		</TabItem>
		<TabItem title="Kontoauszug">
			<StatementView />
		</TabItem>
		<TabItem title="Kalender">
			<CalendarView />
		</TabItem>
		<TabItem title="Vorauszahlungen">
			<PrepaymentsView />
		</TabItem>
	</Tabs>
</div>
