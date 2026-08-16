<script lang="ts">
	import { COLORS, regexError, tokenize } from './regex_highlight';

	interface Props {
		pattern: string | null;
		/// what to show when there is no pattern, which for a filter means it matches anything
		empty?: string;
		class?: string;
	}
	let { pattern, empty = '–', class: className = '' }: Props = $props();

	const tokens = $derived(pattern ? tokenize(pattern) : []);
	// A pattern that will not compile matches nothing, so the rule silently never fires. The
	// table is where you would look for a rule that has stopped working.
	const broken = $derived(pattern ? regexError(pattern) !== null : false);
</script>

{#if !pattern}
	<span class="text-gray-400 dark:text-gray-500">{empty}</span>
{:else if broken}
	<span class="font-mono wrap-anywhere text-red-600 dark:text-red-400" title="Ungültiger Ausdruck">
		{pattern}
	</span>
{:else}
	<span class="font-mono wrap-anywhere {className}"
		>{#each tokens as t, i (i)}<span class={COLORS[t.kind]}>{t.text}</span>{/each}</span
	>
{/if}
