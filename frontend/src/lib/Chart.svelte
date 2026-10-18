<script lang="ts">
	import ApexCharts, { type ApexOptions } from 'apexcharts';
	import 'apexcharts/sunburst';

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

	/// The same chart with its entry animation taken out.
	///
	/// `animate: false` on `updateOptions` is not enough, and the third argument is not the one
	/// that decides this. Apex animates when `chart.animations.enabled && !globals.resized`, and
	/// it only sets `resized` on the branch it takes when `redrawPaths` is false -- which is the
	/// branch we do not want, because it morphs the old paths into the new ones. So the redraw
	/// we ask for looks like a first draw to apex and replays the entry animation, and the only
	/// way to say no is to hand it a config that has none.
	const still = (o: ApexOptions): ApexOptions => ({
		...o,
		chart: { ...o.chart, animations: { enabled: false } }
	});

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
			const quiet = still(next);
			const wanted = hiddenSignature(next);

			// Both, in this order, and neither alone will do: `updateSeries` clears the
			// collapsed set but ignores `hidden`, which is only read where the chart draws —
			// so the pass that acts on it has to come after the one that forgot the old state.
			const cleared =
				collapsed === wanted
					? null
					: updating.updateSeries(quiet.series ?? [], false).then(() => undefined);
			collapsed = wanted;

			void Promise.resolve(cleared)
				.then(() => updating.updateOptions(quiet, true, false))
				.then(() => {
					if (chart === updating) onUpdated?.(updating);
				});
			return;
		}

		// Nothing that replaces a chart already on screen animates its way in. Whoever is
		// looking has the old picture in their head and is about to compare it with this one --
		// the axis toggle over the analysis charts is exactly that -- and bars growing out of
		// the floor is what makes the two hard to hold side by side. Only a chart drawn where
		// there was none, which is a page load or a tab being opened, still animates.
		const replacing = chart !== null;
		chart?.destroy();
		const created = new ApexCharts(el, replacing ? still(next) : next);
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
