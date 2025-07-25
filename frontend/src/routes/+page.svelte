<script lang="ts">
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
	import { Tabs, TabItem } from 'flowbite-svelte';
	import type { PageData } from './$houdini';

	interface Props {
		data: PageData;
	}

	const { data } = $props() as Props;
	const { About } = $derived(data);

	$effect(() => {
		if (!$About.data?.users.me) {
			window.location.href = '/login';
		}
	});
</script>

<div class="pl-8 pr-8">
	<Tabs tabStyle="underline">
		<TabItem open title="Aktivität">
			<ExpenseChangelog />
		</TabItem>
	</Tabs>
</div>
