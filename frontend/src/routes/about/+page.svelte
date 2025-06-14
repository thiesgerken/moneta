<script lang="ts">
	import type { PageData } from './$houdini';

	interface Props {
		data: PageData;
	}

	const { data } = $props() as Props;
	const { ApiVersion } = $derived(data);
</script>

<p>{$ApiVersion.data?.apiVersion}</p>
