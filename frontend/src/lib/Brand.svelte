<script lang="ts">
	let { class: className = '' }: { class?: string } = $props();
</script>

<picture class="block shrink-0">
	<source media="(prefers-color-scheme: dark)" srcset="/brand/lockup-dark.svg" />
	<img src="/brand/lockup.svg" alt="Moneta" width="296" height="64" class={className} />
</picture>
