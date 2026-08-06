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

<div class="space-y-4 pr-8 pl-8">
	<Tabs tabStyle="underline">
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
