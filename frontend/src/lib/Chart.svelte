<script lang="ts">
	import ApexCharts, { type ApexOptions } from 'apexcharts';

	interface Props {
		options: ApexOptions;
	}
	let { options }: Props = $props();

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
		chart = new ApexCharts(el, next);
		built = shape;
		void chart.render();
	});

	$effect(() => {
		return () => {
			chart?.destroy();
			chart = null;
		};
	});
</script>

<div bind:this={el}></div>
