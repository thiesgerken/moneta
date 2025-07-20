<script lang="ts">
	import type { PageData } from './$houdini';

	interface Props {
		data: PageData;
	}

	const { data } = $props() as Props;
	const { About } = $derived(data);

	$effect(() => {
		if (!$About.data?.users.me) {
			window.location.href = '/login';
		}
	});
</script>

<div class="p-8">
	<h1 class="text-2xl font-bold">Dashboard</h1>
	<p>Welcome to the dashboard</p>
	<p>API Version: {$About.data?.apiVersion}</p>
</div>
