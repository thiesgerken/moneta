<script lang="ts">
	import { Tabs, TabItem, Alert } from 'flowbite-svelte';
	import WealthView from '$lib/analysis/WealthView.svelte';
	import OverviewsView from '$lib/analysis/OverviewsView.svelte';
	import CategoriesView from '$lib/analysis/CategoriesView.svelte';
	import CalendarView from '$lib/analysis/CalendarView.svelte';
</script>

<div class="space-y-4 pr-8 pl-8">
	<Alert color="yellow">
		<span class="font-medium">Mock:</span> alle Zahlen auf dieser Seite sind erfunden. Die Backend-Endpunkte
		fehlen noch.
	</Alert>

	<Tabs tabStyle="underline">
		<TabItem open title="Vermögen">
			<WealthView />
		</TabItem>
		<TabItem title="Monate">
			<OverviewsView />
		</TabItem>
		<TabItem title="Kategorien">
			<CategoriesView />
		</TabItem>
		<TabItem title="Kalender">
			<CalendarView />
		</TabItem>
	</Tabs>
</div>
