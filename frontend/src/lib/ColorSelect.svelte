<script lang="ts">
	import { Dropdown } from 'flowbite-svelte';
	import { ChevronDownOutline } from 'flowbite-svelte-icons';

	interface Option {
		value: string;
		name: string;
		color?: string | null;
	}

	interface Props {
		options: Option[];
		value: string;
		placeholder?: string;
		/// what to say when there is nothing to choose from, which is not the same as choosing
		/// nothing -- an account list filtered down to nothing is a different problem
		empty?: string;
	}

	let {
		options,
		value = $bindable(),
		placeholder = 'Auswählen',
		empty = 'Nichts da'
	}: Props = $props();

	let isOpen = $state(false);
	const selected = $derived(options.find((option) => option.value === value));

	function pick(option: Option) {
		value = option.value;
		isOpen = false;
	}
</script>

<!--
	A select that can show what colour a thing is.

	A native `<select>` cannot: the option list is drawn by the operating system and takes no
	styling at all, so the one thing that identifies an account or a category in every other
	place in this app -- its colour -- was the one thing missing from the list where you pick
	it. Dozens of accounts as identical black text is a list you read rather than recognise.

	The trigger and each row carry the dot the lists draw in front of the name.
-->
<button
	type="button"
	onclick={() => (isOpen = !isOpen)}
	class="flex w-full cursor-pointer items-center justify-between gap-2 rounded-lg border border-gray-300 bg-gray-50 px-2.5 py-2 text-left text-sm dark:border-gray-600 dark:bg-gray-700"
>
	<span class="flex min-w-0 items-center gap-2">
		{#if selected?.color}
			<span class="h-2 w-2 shrink-0 rounded-full" style:background-color={selected.color}></span>
		{/if}
		<span class="truncate {selected ? '' : 'text-gray-400 dark:text-gray-500'}">
			{selected?.name ?? placeholder}
		</span>
	</span>
	<ChevronDownOutline class="h-3 w-3 shrink-0 opacity-60" />
</button>

<Dropdown bind:isOpen simple class="max-h-72 w-72 overflow-y-auto p-1">
	{#each options as option (option.value)}
		<button
			type="button"
			onclick={() => pick(option)}
			class="flex w-full cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-600 {option.value ===
			value
				? 'font-medium'
				: ''}"
		>
			<!-- a dot rather than a tinted row: forty rows of full-strength colour is a paint chart -->
			<span
				class="h-2 w-2 shrink-0 rounded-full"
				style="background-color: {option.color ?? 'transparent'}"
			></span>
			<span class="truncate">{option.name}</span>
		</button>
	{:else}
		<span class="block px-2 py-1.5 text-sm text-gray-500 dark:text-gray-400">{empty}</span>
	{/each}
</Dropdown>
