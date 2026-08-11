<script lang="ts">
	import { ArrowRightOutline } from 'flowbite-svelte-icons';
	import { describeChanges, type Change } from './changelog';

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
-->
<dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs {klass}">
	{#each rows as row, i (i)}
		<dt class="text-gray-500 dark:text-gray-400">{row.label}</dt>
		<dd class="min-w-0">
			<!-- Red and green as everywhere else a before and an after sit next to each other,
			     and no strikethrough: the arrow already says which of the two is current. -->
			{#if row.old !== null && row.new !== null}
				<span class="text-red-700 dark:text-red-400">{row.old}</span>
				<ArrowRightOutline class="mx-0.5 inline h-3 w-3 shrink-0 text-gray-400" />
				<span class="text-green-700 dark:text-green-400">{row.new}</span>
			{:else if row.new !== null}
				<span class="text-green-700 dark:text-green-400">{row.new}</span>
				<span class="text-gray-400 dark:text-gray-500">(neu)</span>
			{:else}
				<span class="text-red-700 line-through dark:text-red-400">{row.old}</span>
				<span class="text-gray-400 dark:text-gray-500">(entfernt)</span>
			{/if}
		</dd>
	{/each}
</dl>
