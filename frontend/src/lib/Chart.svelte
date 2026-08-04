<script lang="ts">
	import ApexCharts, { type ApexOptions } from 'apexcharts';

	interface Props {
		options: ApexOptions;
	}
	let { options }: Props = $props();

	let el: HTMLDivElement;

	// recreating on every options change is wasteful but avoids apex's updateOptions
	// merge semantics, which keep stale series around when the shape changes
	$effect(() => {
		const chart = new ApexCharts(el, options);
		chart.render();
		return () => chart.destroy();
	});
</script>

<div bind:this={el}></div>
