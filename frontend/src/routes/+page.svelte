<script lang="ts">
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
	import { Tabs, TabItem } from 'flowbite-svelte';
</script>

<div class="pr-8 pl-8">
	<Tabs tabStyle="underline">
		<TabItem open title="Aktivität">
			<ExpenseChangelog />
		</TabItem>
	</Tabs>
</div>
