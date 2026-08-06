<script lang="ts">
	import ApexCharts, { type ApexOptions } from 'apexcharts';

	interface Props {
		options: ApexOptions;
		/// the live instance, for callers that need to drive it imperatively — showing and
		/// hiding series is not expressible through the options alone
		instance?: ApexCharts | null;
		/// after the chart has drawn, whether it was built or updated in place. Anything drawn
		/// onto the chart rather than declared in its options has to be re-applied here: apex
		/// keeps only what the options carry.
		onUpdated?: (chart: ApexCharts) => void;
	}
	let { options, instance = $bindable(null), onUpdated }: Props = $props();

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

	/// Which series are collapsed.
	///
	/// `updateOptions` deliberately keeps the collapsed set across an update, so a series that
	/// stops being `hidden` stays hidden; only `updateSeries` clears it. Worth the extra pass,
	/// because the alternative — apex's own `hideSeries`, one name at a time — redraws the
	/// whole chart per call, and a few dozen series lock the tab up for seconds.
	const hiddenSignature = (o: ApexOptions) =>
		(o.series ?? []).map((s) => ((s as { hidden?: boolean }).hidden ? 1 : 0)).join('');

	let chart: ApexCharts | null = null;
	let built = '';
	let collapsed = '';

	$effect(() => {
		// tracked before any await, so the effect re-runs when they change
		const next = options;
		const shape = signature(next);

		if (chart && shape === built) {
			// The flicker this avoids: rebuilding for a new range tears the canvas down and
			// draws it twice, once empty. Redraw the paths but skip the entry animation, or
			// every refetch replays it.
			const updating = chart;
			const wanted = hiddenSignature(next);

			// Both, in this order, and neither alone will do: `updateSeries` clears the
			// collapsed set but ignores `hidden`, which is only read where the chart draws —
			// so the pass that acts on it has to come after the one that forgot the old state.
			const cleared =
				collapsed === wanted
					? null
					: updating.updateSeries(next.series ?? [], false).then(() => undefined);
			collapsed = wanted;

			void Promise.resolve(cleared)
				.then(() => updating.updateOptions(next, true, false))
				.then(() => {
					if (chart === updating) onUpdated?.(updating);
				});
			return;
		}

		chart?.destroy();
		const created = new ApexCharts(el, next);
		chart = created;
		built = shape;
		collapsed = hiddenSignature(next);

		// Exposed only once it has drawn. Callers drive series visibility through the
		// instance, and apex reaches into DOM that `render()` has not created yet if asked
		// before then -- it throws out of their effect rather than returning an error.
		void created.render().then(() => {
			if (chart !== created) return;
			if (instance !== created) instance = created;
			onUpdated?.(created);
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
