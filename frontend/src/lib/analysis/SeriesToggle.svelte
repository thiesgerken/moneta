<script lang="ts">
	import { Button, ButtonGroup } from 'flowbite-svelte';
	import type ApexCharts from 'apexcharts';

	/// Alle / Keine / Umkehren for a chart's series.
	///
	/// Visibility is not part of apex's options, so this drives the instance directly. Which
	/// series are hidden is tracked here rather than read back out of apex, whose
	/// `collapsedSeries` internals differ between stacked and unstacked charts — and the list
	/// has to be remembered anyway, so it survives a tab switch.
	let {
		instance,
		names,
		hidden = $bindable([])
	}: {
		instance: ApexCharts | null;
		names: string[];
		hidden?: string[];
	} = $props();

	// A rebuilt chart starts with everything visible, so the hidden list has to be re-applied
	// rather than assumed to still hold. Cheap: apex ignores hiding what is already hidden.
	$effect(() => {
		if (!instance || names.length === 0) return;
		for (const name of names) {
			try {
				if (hidden.includes(name)) instance.hideSeries(name);
				else instance.showSeries(name);
			} catch {
				// apex has no way to ask whether a series is still there, and it throws rather
				// than returning when the chart has been torn down under us -- which happens
				// on a tab switch. Nothing to do about a series that no longer exists.
			}
		}
	});

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
