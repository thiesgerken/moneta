<script lang="ts">
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
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
	<ExpenseChangelog />
</div>
