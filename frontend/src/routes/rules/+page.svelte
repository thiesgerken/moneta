<script lang="ts">
	import { Tabs, TabItem } from 'flowbite-svelte';
	import PeriodicRuleTable from '$lib/PeriodicRuleTable.svelte';
	import AutomationRuleTable from '$lib/AutomationRuleTable.svelte';
</script>

<div class="px-4 sm:px-8">
	<Tabs
		tabStyle="underline"
		class="flex-nowrap overflow-x-auto whitespace-nowrap"
		classes={{ content: 'mt-4 rounded-lg bg-gray-50 p-2 sm:p-4 dark:bg-gray-800' }}
	>
		<TabItem open title="Automatisierung">
			<AutomationRuleTable />
		</TabItem>
		<TabItem title="Periodisch">
			<PeriodicRuleTable />
		</TabItem>
	</Tabs>
</div>
