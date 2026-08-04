<script lang="ts">
	import { Tabs, TabItem } from 'flowbite-svelte';
	import PeriodicRuleTable from '$lib/PeriodicRuleTable.svelte';
	import AutomationRuleTable from '$lib/AutomationRuleTable.svelte';
</script>

<div class="pr-8 pl-8">
	<Tabs tabStyle="underline">
		<TabItem open title="Periodisch">
			<PeriodicRuleTable />
		</TabItem>
		<TabItem title="Automatisierung">
			<AutomationRuleTable />
		</TabItem>
	</Tabs>
</div>
