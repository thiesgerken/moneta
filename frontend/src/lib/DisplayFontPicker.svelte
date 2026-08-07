<script lang="ts">
	/// A throwaway control for choosing the heading font, and nothing else.
	///
	/// It only appears with `?schrift` in the url, so the app is unchanged for anyone who is
	/// not deliberately looking at fonts, and the choice is remembered so that clicking around
	/// the app keeps comparing the same one. Delete this file, the four `@import`s in app.css
	/// and the losing packages once it is decided.
	import { page } from '$app/state';
	import { browser } from '$app/environment';

	const FONTS = [
		{ value: '', label: 'Inter (wie jetzt)' },
		{ value: 'fraunces', label: 'Fraunces' },
		{ value: 'instrument', label: 'Instrument Serif' },
		{ value: 'newsreader', label: 'Newsreader' },
		{ value: 'source', label: 'Source Serif 4' }
	];

	const KEY = 'moneta:display-font';

	const trying = $derived(page.url.searchParams.has('schrift'));

	// writable: the select assigns to it, and the url and the last choice are only where it
	// starts. `browser`, since this renders on the server too and there is no storage there.
	let chosen = $derived(
		page.url.searchParams.get('schrift') || (browser ? (localStorage.getItem(KEY) ?? '') : '')
	);

	$effect(() => {
		if (!trying) return;

		localStorage.setItem(KEY, chosen);
		if (chosen) document.documentElement.dataset.display = chosen;
		else delete document.documentElement.dataset.display;
	});
</script>

{#if trying}
	<div
		class="fixed right-4 bottom-4 z-50 rounded-lg border border-gray-300 bg-white p-2 shadow-lg dark:border-gray-600 dark:bg-gray-800"
	>
		<label class="flex items-center gap-2 text-sm">
			<span class="text-gray-500 dark:text-gray-400">Überschriften</span>
			<select bind:value={chosen} class="rounded border-gray-300 py-1 text-sm dark:bg-gray-700">
				{#each FONTS as font (font.value)}
					<option value={font.value}>{font.label}</option>
				{/each}
			</select>
		</label>
	</div>
{/if}
