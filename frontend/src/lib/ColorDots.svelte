<script lang="ts">
	import type { Dot } from './expense_utils';

	let { items, class: cls = '' }: { items: Dot[]; class?: string } = $props();
</script>

<!--
	Accounts and categories as a dot in their own colour plus the name, on one line.

	The colour used to fill the whole cell, striped by share for a shared expense. Three tinted
	cells side by side read as one band, and the account and category colours drowned out the
	amount, which is what a row is about. A dot keeps the colour as a way to recognise an account
	without letting it decide what the eye lands on. An uncoloured one gets a grey dot rather than
	none, so the names stay in line.
-->
<span class="flex min-w-0 items-center gap-x-3 {cls}">
	<!-- The ones after the first give way a hundred times faster, down to their dot: a shared
	     expense's main account by name and the other as a dot says more than both cut to three
	     letters. -->
	{#each items as item, i (i)}
		<span class="flex min-w-0 items-center gap-1.5 {i === 0 ? 'shrink' : 'shrink-[100]'}">
			<span
				class="h-2 w-2 shrink-0 rounded-full bg-gray-300 dark:bg-gray-600"
				style:background-color={item.color}
			></span>
			<span class="truncate">{item.name}</span>
		</span>
	{/each}
</span>
