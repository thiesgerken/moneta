<script lang="ts">
	import ApexCharts, { type ApexOptions } from 'apexcharts';

	interface Props {
		options: ApexOptions;
		/// the live instance, for callers that need to drive it imperatively — showing and
		/// hiding series is not expressible through the options alone
		instance?: ApexCharts | null;
	}
	let { options, instance = $bindable(null) }: Props = $props();

	let el: HTMLDivElement;

	/// Which series the chart is currently built around.
	///
	/// apex's `updateOptions` merges rather than replaces, so a chart whose series list shrinks
	/// keeps the ones that went away. Comparing the names is enough to tell the two cases
	/// apart: same names means only the numbers moved, and that it can update in place.
	const signature = (o: ApexOptions) =>
		[o.chart?.type, o.chart?.stacked, (o.series ?? []).map((s) => (s as { name?: string }).name)]
			.map((p) => JSON.stringify(p))
			.join('|');

	let chart: ApexCharts | null = null;
	let built = '';

	$effect(() => {
		// tracked before any await, so the effect re-runs when they change
		const next = options;
		const shape = signature(next);

		if (chart && shape === built) {
			// The flicker this avoids: rebuilding for a new range tears the canvas down and
			// draws it twice, once empty. Redraw the paths but skip the entry animation, or
			// every refetch replays it.
			void chart.updateOptions(next, true, false);
			return;
		}

		chart?.destroy();
		const created = new ApexCharts(el, next);
		chart = created;
		built = shape;

		// Exposed only once it has drawn. Callers drive series visibility through the
		// instance, and apex reaches into DOM that `render()` has not created yet if asked
		// before then -- it throws out of their effect rather than returning an error.
		void created.render().then(() => {
			if (chart === created && instance !== created) instance = created;
		});
	});

	$effect(() => {
		return () => {
			chart?.destroy();
			chart = null;
			if (instance !== null) instance = null;
		};
	});
</script>

<div bind:this={el}></div>
