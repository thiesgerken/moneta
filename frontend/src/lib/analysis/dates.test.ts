import { describe, expect, it } from 'vitest';
import { accrualShare, isoDate, monthsBefore, shiftMonth, shortDate, startOfMonth } from './dates';

/// Local time throughout, deliberately — the ranges these build are what a reader typed into a
/// date field, not instants on a wire.
const local = (y: number, m: number, d: number, h = 12) => new Date(y, m - 1, d, h).getTime();

describe('isoDate', () => {
	it('writes the date as an input element wants it', () => {
		expect(isoDate(new Date(2026, 7, 4))).toBe('2026-08-04');
	});

	it('pads the month and the day', () => {
		expect(isoDate(new Date(2026, 0, 9))).toBe('2026-01-09');
	});

	/// Going through `toISOString` would shift the day for anyone east of UTC: a date picked as
	/// the 4th would be submitted as the 3rd. This is the case that catches it, since an early
	/// local hour is the previous day in UTC for a positive offset.
	it('keeps the local day rather than the UTC one', () => {
		expect(isoDate(new Date(2026, 7, 4, 0, 30))).toBe('2026-08-04');
		expect(isoDate(new Date(2026, 7, 4, 23, 30))).toBe('2026-08-04');
	});
});

describe('shortDate', () => {
	// two-digit, so the day, the month and the year start in the same place from row to row
	it('pads every field, unlike de-DE on its own', () => {
		expect(shortDate(new Date(2026, 7, 8))).toBe('08.08.2026');
	});
});

describe('startOfMonth', () => {
	it('lands on the first at midnight', () => {
		expect(startOfMonth(local(2026, 8, 17))).toBe(local(2026, 8, 1, 0));
	});

	it('leaves a date already on the first alone', () => {
		expect(startOfMonth(local(2026, 8, 1, 0))).toBe(local(2026, 8, 1, 0));
	});
});

describe('shiftMonth', () => {
	it('moves whole months and lands on the first', () => {
		expect(shiftMonth(local(2026, 3, 15), -2)).toBe(local(2026, 1, 1, 0));
	});

	// built rather than mutated, so this rolls the year over on its own
	it('crosses a year boundary in either direction', () => {
		expect(shiftMonth(local(2026, 11, 15), 3)).toBe(local(2027, 2, 1, 0));
		expect(shiftMonth(local(2026, 2, 15), -3)).toBe(local(2025, 11, 1, 0));
	});
});

describe('monthsBefore', () => {
	// the time of day is dropped along the way, which is what the ranges want: a window that
	// starts at noon would cut the first day of it in half
	it('keeps the day of the month and drops the time', () => {
		expect(monthsBefore(local(2026, 4, 15), 3)).toBe(local(2026, 1, 15, 0));
	});

	it('rolls into the previous year', () => {
		expect(monthsBefore(local(2026, 1, 15), 3)).toBe(local(2025, 10, 15, 0));
	});

	/// A day the target month does not have overflows into the next one: the 31st of March less
	/// a month is "the 31st of February", which is the 3rd of March. Pinned rather than fixed —
	/// this only ever builds the left edge of a chart window, where a couple of days either way
	/// is invisible, and clamping instead would be a rule with its own surprises.
	it('overflows a day the shorter month does not have', () => {
		expect(monthsBefore(local(2026, 3, 31), 1)).toBe(local(2026, 3, 3, 0));
		expect(monthsBefore(local(2028, 2, 29), 12)).toBe(local(2027, 3, 1, 0));
	});
});

describe('accrualShare', () => {
	const utc = (iso: string) => new Date(`${iso}T00:00:00Z`);

	// the quarter from tests/sql/analysis_accrual_axis.sql: 92 days over three months, chosen
	// so that a wrong denominator is visible
	const quarter = { accrualStart: utc('2025-06-01'), accrualEnd: utc('2025-08-31') };
	const first = (m: number) =>
		m > 12 ? utc('2026-01-01') : utc(`2025-${`${m}`.padStart(2, '0')}-01`);
	const month = (m: number) => [first(m), first(m + 1)] as const;

	it('splits a period by the days it spends in each month', () => {
		expect(accrualShare(quarter, ...month(6))).toBeCloseTo(30 / 92);
		expect(accrualShare(quarter, ...month(7))).toBeCloseTo(31 / 92);
		expect(accrualShare(quarter, ...month(8))).toBeCloseTo(31 / 92);
	});

	it('counts a single day as a whole period', () => {
		const point = { accrualStart: utc('2025-07-10'), accrualEnd: utc('2025-07-10') };
		expect(accrualShare(point, ...month(7))).toBe(1);
	});

	it('is nothing outside the period', () => {
		expect(accrualShare(quarter, ...month(5))).toBe(0);
		expect(accrualShare(quarter, ...month(9))).toBe(0);
	});
});
