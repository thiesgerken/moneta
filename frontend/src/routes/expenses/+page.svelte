<script lang="ts">
	import type { PageData } from './$houdini';

	interface Props {
		data: PageData;
	}

	const { data } = $props() as Props;
	const { Expenses } = $derived(data);
</script>

<ul>
	{#each $Expenses.data?.expenses.list.edges || [] as expense}
		<li>{expense.node.title}</li>
	{/each}
</ul>
