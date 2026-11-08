import { describe, expect, it } from 'vitest';

import { asOf, priced, uncommittedGain, type Investment } from './investments';

const investment = (id: string, value: number | null, asOfDate?: string): Investment => ({
	account: { id },
	value,
	asOf: asOfDate ? new Date(asOfDate) : null
});

describe('uncommittedGain', () => {
	it('holds the market value against what has been booked for the same account', () => {
		const committed = new Map([['a', 100_000]]);

		expect(uncommittedGain([investment('a', 170_000)], committed)).toBe(70_000);
	});

	it('counts a depot worth less than its booked balance downwards', () => {
		const committed = new Map([['a', 200_000]]);

		expect(uncommittedGain([investment('a', 170_000)], committed)).toBe(-30_000);
	});

	/// The figure it is added to is a total of everything, so a depot nobody could price has to
	/// contribute nothing rather than its whole market value or its whole booked one.
	it('skips a depot with no price instead of guessing at one', () => {
		const committed = new Map([
			['a', 100_000],
			['b', 50_000]
		]);

		expect(uncommittedGain([investment('a', 170_000), investment('b', null)], committed)).toBe(
			70_000
		);
	});

	/// A depot that has never had a balance recorded counts its whole market value: there is
	/// nothing booked for it to be measured against.
	it('treats an account with no booked balance as zero', () => {
		expect(uncommittedGain([investment('a', 170_000)], new Map())).toBe(170_000);
	});

	it('is zero when nothing is linked', () => {
		expect(uncommittedGain([], new Map([['a', 100_000]]))).toBe(0);
	});
});

describe('asOf', () => {
	/// The pessimistic end: a total is only as fresh as its stalest part, and renditier will use a
	/// price up to four days old.
	it('reports the oldest price behind the figures', () => {
		const dates = [
			investment('a', 1, '2026-08-16T09:00:00Z'),
			investment('b', 1, '2026-08-14T17:30:00Z')
		];

		expect(asOf(dates)).toEqual(new Date('2026-08-14T17:30:00Z'));
	});

	it('has nothing to report when nothing could be priced', () => {
		expect(asOf([investment('a', null)])).toBeNull();
	});
});

describe('priced', () => {
	it('is true as soon as one depot has a value', () => {
		expect(priced([investment('a', null), investment('b', 1)])).toBe(true);
	});

	it('is false when every value is missing, which is also what a renditier that is down looks like', () => {
		expect(priced([investment('a', null)])).toBe(false);
		expect(priced([])).toBe(false);
	});
});
