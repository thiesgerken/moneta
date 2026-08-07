<script lang="ts">
	import { Tabs, TabItem } from 'flowbite-svelte';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
	import WealthView from '$lib/analysis/WealthView.svelte';
	import OverviewsView from '$lib/analysis/OverviewsView.svelte';
	import CategoriesView from '$lib/analysis/CategoriesView.svelte';
	import CalendarView from '$lib/analysis/CalendarView.svelte';
	import PrepaymentsView from '$lib/analysis/PrepaymentsView.svelte';
	import StatementView from '$lib/analysis/StatementView.svelte';

	/// Which tab a link may ask for. Read once: from here on the tabs own their own state, and
	/// re-deriving this would fight them for it.
	const initial = page.url.searchParams.get('tab') ?? 'vermoegen';

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
	<!-- Seven tabs do not fit a phone, and without a scroller of their own they widen the page
	     instead: the last four were only reachable by dragging the whole layout sideways. -->
	<Tabs
		tabStyle="underline"
		class="flex-nowrap overflow-x-auto whitespace-nowrap"
		classes={{ content: 'mt-4 rounded-lg bg-gray-50 p-2 sm:p-4 dark:bg-gray-800' }}
	>
		<TabItem open={initial === 'vermoegen'} onclick={() => remember('vermoegen')} title="Vermögen">
			<WealthView />
		</TabItem>
		<TabItem open={initial === 'monate'} onclick={() => remember('monate')} title="Monate">
			<OverviewsView />
		</TabItem>
		<TabItem
			open={initial === 'kategorien'}
			onclick={() => remember('kategorien')}
			title="Kategorien"
		>
			<CategoriesView />
		</TabItem>
		<TabItem
			open={initial === 'vorauszahlungen'}
			onclick={() => remember('vorauszahlungen')}
			title="Vorauszahlungen"
		>
			<PrepaymentsView />
		</TabItem>
		<TabItem
			open={initial === 'kontoauszug'}
			onclick={() => remember('kontoauszug')}
			title="Kontoauszug"
		>
			<StatementView />
		</TabItem>
		<TabItem open={initial === 'kalender'} onclick={() => remember('kalender')} title="Kalender">
			<CalendarView />
		</TabItem>
		<!-- Last, and no longer what the page opens on: the feed is a log, and the start page
		     now carries the part of it worth arriving at. -->
		<TabItem
			open={initial === 'aktivitaet'}
			onclick={() => remember('aktivitaet')}
			title="Aktivität"
		>
			<ExpenseChangelog />
		</TabItem>
	</Tabs>
</div>
