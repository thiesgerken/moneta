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
