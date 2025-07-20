<script lang="ts">
	import { Alert } from 'flowbite-svelte';
	import type { PageData } from './$houdini';

	interface Props {
		data: PageData;
	}

	const { data } = $props() as Props;
	const { About } = $derived(data);
</script>

<div class="p-8">
	<Alert>
		<span class="font-medium">Info alert!</span>
		Change a few things up and try submitting again.
		{$About.data?.apiVersion}
	</Alert>
</div>
