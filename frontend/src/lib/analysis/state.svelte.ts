/// What the analysis tabs remember when you leave them.
///
/// flowbite's `TabItem` unmounts the tab you switch away from, so anything a view keeps in its
/// own `$state` is gone by the time you come back — you would land on the default range again
/// every time. Module-level runes outlive the components that read them.
///
/// Each view keeps its own range on purpose. `Monate` and `Kategorien` share one query store,
/// which only works because a single tab is mounted at a time; giving them one shared range
/// as well would mean changing the period in one silently changed the other.

import { daysAgoIso, startOfMonth, todayIso } from './dates';

/// Epoch milliseconds rather than `Date`, because these live in reactive state: a `Date` there
/// would have to be a `SvelteDate` to stay reactive when mutated, and a plain number cannot be
/// mutated by accident in the first place.
/// the span of a window nothing preset describes: typed into the date fields, or zoomed into
export const CUSTOM_SPAN = 'custom';

export type Range = {
	/// which preset is selected, or `CUSTOM_SPAN`
	span: string;
	from: number | undefined;
	to: number | undefined;
};

const range = (span: string): Range => ({ span, from: undefined, to: undefined });

/// Whose changes the activity feed shows, and how far back. A feed that opens on an author
/// filter reads as an empty feed when nobody else has been active, and leaves you looking for
/// your own entry — where a month of history is plenty to arrive on, and `span` is a number of
/// days or the empty string for all of it.
export const feedState = $state({ author: 'ANYONE', span: '30' });

export const wealthState = $state({
	range: range('all'),
	split: 'total',
	/// the Stichtag for the availability totals and the computed balances
	date: todayIso(),
	/// A closed account keeps its last balance for ever, so it sits in the table at whatever it
	/// held on its last day and never moves again -- a row that answers nothing.
	showClosedBalances: false,
	/// Series hidden from the chart, by name, remembered so a tab switch does not un-hide them.
	///
	/// One list per split rather than one shared: the toggle drops names the chart no longer
	/// draws, so a shared list lost the account choices the moment you looked at availability.
	hiddenAccounts: [] as string[],
	hiddenAvailability: [] as string[],
	/// whether the closed accounts have been collapsed for this session yet
	accountsSeeded: false
});

export const overviewsState = $state({ range: range('1y') });

export const categoriesState = $state({
	range: range('1y'),
	sign: 'expenses',
	hiddenSeries: [] as string[]
});

export const statementState = $state({
	accountId: '',
	from: daysAgoIso(90),
	to: todayIso()
});

export const prepaymentsState = $state({ date: todayIso() });

/// the first of the displayed month, as epoch milliseconds
export const calendarState = $state({ month: startOfMonth(Date.now()) });
