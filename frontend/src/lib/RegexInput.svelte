<script lang="ts">
	import { Helper } from 'flowbite-svelte';
	import { COLORS, regexError, tokenize } from './regex_highlight';

	interface Props {
		value: string;
		placeholder?: string;
		id?: string;
	}
	let { value = $bindable(), placeholder = '', id = undefined }: Props = $props();

	const error = $derived(regexError(value));

	const tokens = $derived(tokenize(value));

	// The colours live on a copy of the text behind a transparent input, so the two have to
	// agree on every dimension that moves a glyph: font, size, padding, border width.
	let mirror = $state<HTMLPreElement | undefined>(undefined);
	const sync = (e: Event) => {
		if (mirror) mirror.scrollLeft = (e.currentTarget as HTMLInputElement).scrollLeft;
	};

	const BOX = 'block w-full rounded-lg border p-2.5 font-mono text-sm leading-tight whitespace-pre';
</script>

<div class="relative">
	<pre
		bind:this={mirror}
		aria-hidden="true"
		class="{BOX} pointer-events-none absolute inset-0 m-0 overflow-hidden border-transparent bg-gray-50 dark:bg-gray-700">{#each tokens as t, i (i)}<span
				class={COLORS[t.kind]}>{t.text}</span
			>{/each}</pre>

	<input
		type="text"
		{id}
		bind:value
		{placeholder}
		autocomplete="off"
		spellcheck="false"
		onscroll={sync}
		class="{BOX} relative bg-transparent text-transparent caret-gray-900 placeholder:text-gray-400 dark:caret-white dark:placeholder:text-gray-500 {error
			? 'border-red-500 focus:border-red-500 focus:ring-red-500 dark:border-red-500'
			: 'focus:border-primary-500 focus:ring-primary-500 border-gray-300 dark:border-gray-600'}"
	/>
</div>

{#if error}
	<Helper color="red" class="mt-1">{error}</Helper>
{/if}
