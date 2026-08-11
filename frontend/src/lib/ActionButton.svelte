<script lang="ts">
	import { Button, type ButtonProps } from 'flowbite-svelte';

	let { disabled = false, children, ...rest }: ButtonProps = $props();
</script>

<!--
	A flowbite Button whose disabled state is actually visible.

	flowbite-svelte 1.33.1 works out a Button's class exactly once. Its `attribute_effect` does
	re-run -- the `disabled` attribute updates, so the button really does become pressable -- but
	`btnCls` never recomputes with it, and the `opacity-50 cursor-not-allowed` from the moment it
	was disabled stays on for the life of the component. A button that had been disabled at mount
	stayed muted for ever while working perfectly, which is a worse state than either.

	Verified in the production build as well as in dev, so it is not an artifact of vite's
	dependency pre-bundling, and 1.33.1 is the newest release there is. Remounting is the only
	thing that produces a fresh class, and that is the whole of what this file adds.

	Only for buttons whose disabled state changes. Everything else can use `Button` directly.
-->
{#key disabled}
	<Button {disabled} {...rest}>{@render children?.()}</Button>
{/key}
