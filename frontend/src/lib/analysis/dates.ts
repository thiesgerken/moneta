/// Date helpers for the analysis views.
///
/// A plain module rather than `.svelte.ts` on purpose: `svelte/prefer-svelte-reactivity` flags
/// every `new Date` in a rune module, since one held in reactive state would need to be a
/// `SvelteDate` to stay reactive. These are transient values, so they live outside.

/// `yyyy-mm-dd` in local time, which is what `<input type="date">` expects. Going through
/// `toISOString` would shift the day for anyone east of UTC.
export function isoDate(d: Date): string {
	const pad = (n: number) => `${n}`.padStart(2, '0');
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export const todayIso = () => isoDate(new Date());

/// A date the way it is written down: `08.08.2026`.
///
/// `de-DE` on its own gives `8.8.2026`, which is the same date in fewer characters and reads
/// worse in a column -- the day, the month and the year all start in a different place from
/// one row to the next.
export const shortDate = (d: Date): string =>
	d.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });

export const daysAgoIso = (days: number) => isoDate(new Date(Date.now() - days * 86400000));

/// Midnight on the first of the month containing `at`, as epoch milliseconds.
export function startOfMonth(at: number): number {
	const d = new Date(at);
	return new Date(d.getFullYear(), d.getMonth(), 1).getTime();
}

/// `months` before `at`, built rather than mutated: a negative month rolls into the previous
/// year on its own.
export function monthsBefore(at: number, months: number): number {
	const d = new Date(at);
	return new Date(d.getFullYear(), d.getMonth() - months, d.getDate()).getTime();
}

/// The first of the month `n` months from the one containing `at`.
export function shiftMonth(at: number, n: number): number {
	const d = new Date(at);
	return new Date(d.getFullYear(), d.getMonth() + n, 1).getTime();
}

/// How much of an expense's cost falls inside a window, as a fraction of the whole.
///
/// The frontend's copy of what `analysis_movements` does in SQL: an equal share of the cost per
/// day of the Leistungszeitraum, both ends inclusive, so a period of one day is one day rather
/// than none. Kept in step by `dates.test.ts`, which uses the numbers the SQL test asserts.
///
/// It is here rather than asked of the backend because the caller already holds both dates:
/// the expense list carries the period, and a list of ten rows is not worth a second query
/// against a table the month page has already read.
/// Either end may be left out, for a window that is open on that side.
export function accrualShare(
	period: { accrualStart: Date; accrualEnd: Date },
	from?: Date,
	before?: Date
): number {
	const DAY = 86400000;
	// both ends inclusive, and `before` is exclusive, so the last day of the window is the day
	// before it
	const days = (a: number, b: number) => Math.round((b - a) / DAY) + 1;

	const start = period.accrualStart.getTime();
	const end = period.accrualEnd.getTime();

	const total = days(start, end);
	if (total <= 0) return 0;

	const inside = days(
		from ? Math.max(start, from.getTime()) : start,
		before ? Math.min(end, before.getTime() - DAY) : end
	);

	return Math.min(1, Math.max(0, inside / total));
}
