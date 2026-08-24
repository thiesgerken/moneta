<script lang="ts">
	import { Tabs, TabItem } from 'flowbite-svelte';
	import {
		CalendarMonthOutline,
		ChartLineUpOutline,
		ObjectsColumnOutline,
		ClockArrowOutline,
		FileLinesOutline,
		HourglassOutline
	} from 'flowbite-svelte-icons';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
	import WealthView from '$lib/analysis/WealthView.svelte';
	import HistoryView from '$lib/analysis/HistoryView.svelte';
	import CalendarView from '$lib/analysis/CalendarView.svelte';
	import PrepaymentsView from '$lib/analysis/PrepaymentsView.svelte';
	import StatementView from '$lib/analysis/StatementView.svelte';

	/// The tabs, in the order they are shown. The slug is what `?tab=` carries and is English
	/// like the rest of the code; only `title` is what a reader sees.
	const TABS = [
		{ slug: 'wealth', title: 'Vermögen', icon: ChartLineUpOutline },
		{ slug: 'history', title: 'Verlauf', icon: ObjectsColumnOutline },
		{ slug: 'prepayments', title: 'Vorauszahlungen', icon: HourglassOutline },
		{ slug: 'statement', title: 'Kontoauszug', icon: FileLinesOutline },
		{ slug: 'calendar', title: 'Kalender', icon: CalendarMonthOutline },
		// Last, and no longer what the page opens on: the feed is a log, and the start page now
		// carries the part of it worth arriving at.
		{ slug: 'activity', title: 'Aktivität', icon: ClockArrowOutline }
	];

	/// What Verlauf used to be two of, so a link written before they were one still lands
	/// somewhere. The breakdown it opens on is the tab's own, not the link's -- which of the
	/// two charts you were looking at is a preference, and it outlives the URL that named it.
	const RENAMED: Record<string, string> = { months: 'history', categories: 'history' };

	/// Which tab a link may ask for. Read once: from here on the tabs own their own state, and
	/// re-deriving this would fight them for it.
	const asked = page.url.searchParams.get('tab');
	const initial = (asked && (RENAMED[asked] ?? asked)) || TABS[0].slug;

	/// So that the tab survives a reload and can be linked to -- the start page sends "alle
	/// anzeigen" straight to the feed. `replaceState` rather than `goto`, since switching tabs
	/// is not a place you want the back button to walk through one by one.
	function remember(tab: string) {
		const url = new URL(page.url);
		url.searchParams.set('tab', tab);
		// the rule wants a literal resolve() call, which cannot carry a query string; the path
		// here is the one we are already on, so there is nothing to resolve in the first place
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		replaceState(url, page.state);
	}
</script>

<div class="space-y-4 px-4 sm:px-8">
	<!-- Six tabs do not fit a phone, and without a scroller of their own they widen the page
	     instead: the last three were only reachable by dragging the whole layout sideways. -->
	<Tabs
		tabStyle="underline"
		class="flex-nowrap overflow-x-auto whitespace-nowrap"
		classes={{ content: 'mt-4 rounded-lg bg-gray-50 p-2 sm:p-4 dark:bg-gray-800' }}
	>
		{#each TABS as tab (tab.slug)}
			<TabItem open={initial === tab.slug} onclick={() => remember(tab.slug)}>
				{#snippet titleSlot()}
					{@const Icon = tab.icon}
					<span class="flex items-center gap-2"><Icon class="h-4 w-4 shrink-0" />{tab.title}</span>
				{/snippet}
				{#if tab.slug === 'wealth'}
					<WealthView />
				{:else if tab.slug === 'history'}
					<HistoryView />
				{:else if tab.slug === 'prepayments'}
					<PrepaymentsView />
				{:else if tab.slug === 'statement'}
					<StatementView />
				{:else if tab.slug === 'calendar'}
					<CalendarView />
				{:else if tab.slug === 'activity'}
					<ExpenseChangelog />
				{/if}
			</TabItem>
		{/each}
	</Tabs>
</div>
