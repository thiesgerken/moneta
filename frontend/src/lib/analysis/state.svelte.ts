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
export type Range = {
	/// which preset is selected, or `custom` when the dates were edited by hand
	span: string;
	from: number | undefined;
	to: number | undefined;
};

const range = (span: string): Range => ({ span, from: undefined, to: undefined });

export const wealthState = $state({
	range: range('all'),
	split: 'total',
	/// the Stichtag for the availability totals and the computed balances
	date: todayIso(),
	/// the zoomed window, which overrides the range until it is reset
	zoom: null as { min: number; max: number } | null,
	/// series hidden from the chart, by name; only meaningful when split by account or
	/// availability, and remembered so a tab switch does not un-hide them
	hiddenSeries: [] as string[]
});

export const overviewsState = $state({ range: range('10y') });

export const categoriesState = $state({
	range: range('10y'),
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
