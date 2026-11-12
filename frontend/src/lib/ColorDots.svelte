<script lang="ts">
	import type { Dot } from './expense_utils';

	let { items, class: cls = '' }: { items: Dot[]; class?: string } = $props();
</script>

<!--
	Accounts or categories as a dot in their own colour, the first one named.

	A dot identifies an account at a glance without competing with the amount beside it. The ones
	after the first are a bare dot with the name on hover: a second name shrunk to fit a narrow
	cell ends as a single letter, which reads like a rendering glitch rather than a name. An
	uncoloured one gets a grey dot rather than none, so the names stay in line.
-->
<span class="flex min-w-0 items-center gap-x-2 {cls}">
	{#each items as item, i (i)}
		{#if i === 0}
			<span class="flex min-w-0 items-center gap-1.5">
				<span
					class="h-2 w-2 shrink-0 rounded-full bg-gray-300 dark:bg-gray-600"
					style:background-color={item.color}
				></span>
				<span class="truncate">{item.name}</span>
			</span>
		{:else}
			<span
				role="img"
				aria-label={item.name}
				title={item.name}
				class="h-2 w-2 shrink-0 rounded-full bg-gray-300 dark:bg-gray-600"
				style:background-color={item.color}
			></span>
		{/if}
	{/each}
</span>
