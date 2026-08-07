<script lang="ts">
	import { Tabs, TabItem } from 'flowbite-svelte';
	import PeriodicRuleTable from '$lib/PeriodicRuleTable.svelte';
	import AutomationRuleTable from '$lib/AutomationRuleTable.svelte';
</script>

<div class="px-4 sm:px-8">
	<Tabs tabStyle="underline" class="flex-nowrap overflow-x-auto whitespace-nowrap">
		<TabItem open title="Periodisch">
			<PeriodicRuleTable />
		</TabItem>
		<TabItem title="Automatisierung">
			<AutomationRuleTable />
		</TabItem>
	</Tabs>
</div>
