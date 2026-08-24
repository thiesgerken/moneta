/// What the analysis tabs remember when you leave them.
///
/// flowbite's `TabItem` unmounts the tab you switch away from, so anything a view keeps in its
/// own `$state` is gone by the time you come back — you would land on the default range again
/// every time. Module-level runes outlive the components that read them.
///
/// Each view keeps its own range on purpose: a range is which window you are looking at, and
/// two views looking at different windows is the normal case rather than a bug.

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
	/// Which accounts the computed balances list: `used`, `open` or `all`. A closed account
	/// keeps its last balance for ever, so it sits in the table at whatever it held on its
	/// final day and never moves again -- a row that answers nothing. An open account standing
	/// at zero answers just as little, which is why `used` rather than `open` is the default:
	/// what is holding money is the shorter and more useful list.
	balanceAccounts: 'used',
	/// Which date the *curve* is read on: `value` or `accrued`, the same two words the monthly
	/// figures use. Its own field rather than the shared `axisState`, because only the Gesamt
	/// split can answer on the accrual axis — a toggle in the page-wide bar that silently does
	/// nothing in two of three splits is worse than one that appears when it applies.
	axis: 'value',
	/// Series hidden from the chart, by name, remembered so a tab switch does not un-hide them.
	///
	/// One list per split rather than one shared: the toggle drops names the chart no longer
	/// draws, so a shared list lost the account choices the moment you looked at availability.
	hiddenAccounts: [] as string[],
	hiddenAvailability: [] as string[],
	/// whether the closed accounts have been collapsed for this session yet
	accountsSeeded: false
});

/// Which date the figures are filed under: `value` for the day the money moved, `accrued` for
/// the period the cost was for, spread evenly over its days.
///
/// Its own state rather than a field of the view that shows it, unlike the range: a range is
/// which window you are looking at, this is what the numbers in it mean, and one figure meaning
/// two different things in two places on the same page is the confusion it exists to remove.
export const axisState = $state({ axis: 'value' });

export const accrued = () => axisState.axis === 'accrued';

/// The two axes as the toggle above Verlauf and the month pages names them, in one place so
/// both offer the same words for the same thing.
export const DATE_AXES = [
	{ value: 'value', label: 'Wertstellung' },
	{ value: 'accrued', label: 'Leistungszeitraum' }
];

/// How wide one bar of an overview is, spelled the way the schema's enum is.
export type Granularity = 'MONTH' | 'YEAR';

/// Where the grain goes over from months to years when nobody has said.
///
/// Three years is 36 bars, which is about as many as the axis can label before apex starts
/// thinning them; ten years of months is 120 and unreadable. Asking for a long window is
/// itself the request for a coarser grain, so the toggle only has to override this, and
/// mostly does not have to be found at all.
const YEARS_ABOVE_MONTHS = 3;

/// The grain a range asks for on its own, in months.
export const grainFor = (months: number): Granularity =>
	months > YEARS_ABOVE_MONTHS * 12 ? 'YEAR' : 'MONTH';

/// Verlauf: the periods of a range, as totals or broken into categories.
///
/// One state for what were two tabs drawing the same bars off the same query, one stacked by
/// side and one by category. They kept a range each, which had to be set twice to ask the same
/// question twice -- there is one range now because there is one view.
export const historyState = $state({
	range: range('1y'),
	/// Empty until the range has resolved and chosen for it -- see `grainFor`. The toggle
	/// writes over that and it stays until the range changes length again. A string rather
	/// than a nullable one because `ToggleGroup` binds a string.
	granularity: '' as '' | Granularity,
	/// The range, in months, that `granularity` was last decided for. Here rather than in the
	/// view because the tab is unmounted when you leave it, and a grain you picked by hand
	/// should not be re-decided by coming back to it.
	grainDecidedFor: 0,
	/// `total` stacks income against expenses; `categories` stacks one of them by category
	breakdown: 'total',
	/// which side the categories are of, and which the treemap shows. Only read while the
	/// breakdown is by category: the totals chart draws both sides at once.
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
