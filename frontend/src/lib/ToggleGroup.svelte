<script lang="ts">
	import { Button, ButtonGroup } from 'flowbite-svelte';

	interface Props {
		options: { value: string; label: string }[];
		value: string;
		size?: 'xs' | 'sm';
	}
	let { options, value = $bindable(), size = 'xs' }: Props = $props();
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
				onclick={() => (value = o.value)}
			>
				{o.label}
			</Button>
		{/each}
	</ButtonGroup>
{/key}
