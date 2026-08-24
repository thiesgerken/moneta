<script lang="ts">
	import { Button, ButtonGroup } from 'flowbite-svelte';

	interface Props {
		options: { value: string; label: string }[];
		value: string;
		size?: 'xs' | 'sm';
		/// Run after the new value is set, for a choice that has to move something besides
		/// itself. An `$effect` on the bound value cannot tell a click from whatever else
		/// writes to it.
		onchange?: (value: string) => void;
	}
	let { options, value = $bindable(), size = 'xs', onchange }: Props = $props();
</script>

<!--
	flowbite-svelte's Button destructures its $derived theme object
	(`const { base, ... } = $derived(button({ color, ... }))`), which freezes the styling at
	the color it had on mount. A `color` that changes later updates nothing, so the active
	button stays whichever one was active first. Remounting on every change is the cheap
	way out; drop the {#key} once flowbite stops destructuring.
-->
{#key value}
	<ButtonGroup>
		{#each options as o (o.value)}
			<Button
				{size}
				color={value === o.value ? 'primary' : 'alternative'}
				onclick={() => {
					value = o.value;
					onchange?.(o.value);
				}}
			>
				{o.label}
			</Button>
		{/each}
	</ButtonGroup>
{/key}
