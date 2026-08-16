<script lang="ts">
	import { Button, Checkbox, Dropdown } from 'flowbite-svelte';
	import { ChevronDownOutline } from 'flowbite-svelte-icons';
	import type { Snippet } from 'svelte';
	import { toolbarControl } from './table_styles';

	interface Props {
		label: string;
		options: { name: string; value: string; color?: string | null }[];
		selected: string[];
		toggle: (id: string) => void;
		/// what the filter is about, in the trigger — a row of triggers is read by shape first
		icon?: Snippet;
		class?: string;
	}
	let { label, options, selected, toggle, icon, class: className = 'md:w-40' }: Props = $props();
</script>

<!-- Accounts and categories carry a colour, and that colour is how a reader recognises them --
     it is what the cells in the table below are tinted with. flowbite's MultiSelect prints its
     options as bare text and offers no hook to change that, so the filters are a checkbox list
     with the swatch in front of each name. -->
<!-- The count only when the filter is actually narrowing something: every option ticked is the
     same list as none of them, and "(2)" on an untouched filter is noise that also wrapped the
     label onto a second line. -->
<Button color="alternative" class="{toolbarControl} justify-between whitespace-nowrap {className}">
	<span class="flex items-center gap-2">
		{@render icon?.()}
		{selected.length && selected.length < options.length ? `${label} (${selected.length})` : label}
	</span>
	<ChevronDownOutline class="ml-2 h-3 w-3 shrink-0" />
</Button>
<Dropdown class="max-h-80 w-64 divide-y-0 overflow-y-auto p-3 text-sm">
	<div class="space-y-2">
		{#each options as option (option.value)}
			<Checkbox checked={selected.includes(option.value)} onchange={() => toggle(option.value)}>
				{#if option.color !== undefined}
					<span
						class="mr-2 inline-block h-3 w-3 shrink-0 rounded-sm"
						style="background-color: {option.color}"
					></span>
				{/if}
				{option.name}
			</Checkbox>
		{:else}
			<span class="text-gray-500 dark:text-gray-400">Nichts zum Filtern da.</span>
		{/each}
	</div>
</Dropdown>
