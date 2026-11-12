<script lang="ts">
	import type { Dot } from './expense_utils';

	let { items, class: cls = '' }: { items: Dot[]; class?: string } = $props();

	const main = $derived(items[0]);
	const summary = $derived(
		items.map((item) => `${item.name} ${Math.round(item.share * 100)} %`).join(' · ')
	);
</script>

<!--
	The accounts or categories of one expense, the main one named.

	One is a dot in its colour and the name: enough to recognise it without competing with the
	amount beside it. Several are a small capsule split by share, the main name and "+N": an
	expense shared three to one says so at a glance, and the colour stays inside a glyph rather
	than filling the cell. Every segment keeps two pixels so a small share does not vanish, and an
	uncoloured one is grey like an uncoloured dot. The shares in words are the title and the label.
-->
{#if main}
	<span
		class="flex min-w-0 items-center gap-1.5 {cls}"
		role={items.length > 1 ? 'group' : undefined}
		title={items.length > 1 ? summary : undefined}
		aria-label={items.length > 1 ? summary : undefined}
	>
		{#if items.length === 1}
			<span
				class="h-2 w-2 shrink-0 rounded-full bg-gray-300 dark:bg-gray-600"
				style:background-color={main.color}
			></span>
		{:else}
			<span class="flex h-2 w-6 shrink-0 gap-px overflow-hidden rounded-full">
				{#each items as item, i (i)}
					<span
						class="h-full min-w-0.5 basis-0 bg-gray-300 dark:bg-gray-600"
						style:background-color={item.color}
						style:flex-grow={item.share}
					></span>
				{/each}
			</span>
		{/if}
		<span class="truncate">{main.name}</span>
		{#if items.length > 1}
			<span class="shrink-0 text-gray-500 dark:text-gray-400">+{items.length - 1}</span>
		{/if}
	</span>
{/if}
