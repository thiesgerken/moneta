<script lang="ts">
	import { ArrowRightOutline } from 'flowbite-svelte-icons';
	import { ABSENT, describeChanges, type Change } from './changelog';

	let {
		changes,
		names = {},
		class: klass = ''
	}: {
		changes: readonly Change[];
		/// account and category ids to what they are called, so a change of account reads as
		/// one; ids the caller does not know stay ids
		names?: Record<string, string>;
		class?: string;
	} = $props();

	const rows = $derived(describeChanges(changes, names));
</script>

<!--
	What one changelog entry changed.

	This used to be the unified diff the backend produced, printed as it came. It was exact and
	nobody read it: one changed number arrived as a dozen lines of JSON context, with the number
	itself in cents. A field per row says the same thing in the words the rest of the app uses.

	Not a monospace font, which is the other way to line up a column of figures: it would make
	this the one place in the app that looks like a terminal, and the labels beside the numbers
	are prose. `tabular-nums` buys the alignment on its own.
-->
{#snippet side(text: string, color: string)}
	<span class={text === ABSENT ? 'text-gray-400 dark:text-gray-500' : color}>{text}</span>
{/snippet}

<!-- A floor under the label column rather than `max-content` alone: each entry is its own
     grid, and without one a list of two entries has its arrows in two different places. -->
<dl
	class="grid grid-cols-[minmax(9rem,max-content)_1fr] items-baseline gap-x-3 gap-y-1 text-xs {klass}"
>
	{#each rows as row, i (i)}
		<dt class="text-gray-500 dark:text-gray-400">{row.label}</dt>
		<dd class="min-w-0 tabular-nums">
			{@render side(row.old, 'text-red-700 dark:text-red-400')}
			<ArrowRightOutline class="mx-1 inline h-3 w-3 shrink-0 text-gray-400 dark:text-gray-500" />
			{@render side(row.new, 'text-green-700 dark:text-green-400')}
		</dd>
	{/each}
</dl>
