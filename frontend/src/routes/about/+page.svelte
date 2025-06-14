<script lang="ts">
	import type { PageData } from './$houdini';

	interface Props {
		data: PageData;
	}

	const { data } = $props() as Props;
	const { About } = $derived(data);
</script>

<p>{$About.data?.apiVersion}</p>

<p>{$About.data?.users.me.name || 'Not logged in'}</p>
