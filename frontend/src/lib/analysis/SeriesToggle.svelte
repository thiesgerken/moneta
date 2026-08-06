<script lang="ts">
	import { Button, ButtonGroup } from 'flowbite-svelte';

	/// Alle / Keine / Umkehren for a chart's series.
	///
	/// Only the list of hidden names — the chart reads it back out of its own options, where a
	/// series marked `hidden` is collapsed as it draws. Driving apex's `hideSeries` per name
	/// instead costs a full redraw each, which for a few dozen series locks up the tab.
	let {
		names,
		hidden = $bindable([])
	}: {
		names: string[];
		hidden?: string[];
	} = $props();

	// A series that is gone from the chart should not stay in the list, or "Umkehren" would
	// keep resurrecting names nothing can show.
	$effect(() => {
		const stale = hidden.filter((n) => !names.includes(n));
		if (stale.length > 0) hidden = hidden.filter((n) => names.includes(n));
	});
</script>

<ButtonGroup size="sm">
	<Button size="xs" color="alternative" onclick={() => (hidden = [])}>Alle</Button>
	<Button size="xs" color="alternative" onclick={() => (hidden = [...names])}>Keine</Button>
	<Button
		size="xs"
		color="alternative"
		onclick={() => (hidden = names.filter((n) => !hidden.includes(n)))}
	>
		Umkehren
	</Button>
</ButtonGroup>
