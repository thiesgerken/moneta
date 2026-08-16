import { describe, expect, it } from 'vitest';
import { describeAccrual } from './expense_utils';

/// Nothing about a rule's Leistungszeitraum is configured, so this sentence is the only place
/// the derivation is visible -- which makes it worth pinning down.
describe('describeAccrual', () => {
	const gridded = { months: 1, days: null, anchorDay: 1, phaseDays: 0 };

	it('names the cadences that have a name', () => {
		expect(describeAccrual(gridded)).toBe('Monatlich ab dem 1.');
		expect(describeAccrual({ ...gridded, months: 3, anchorDay: 15 })).toBe(
			'Quartalsweise ab dem 15.'
		);
		expect(describeAccrual({ ...gridded, months: 6 })).toBe('Halbjährlich ab dem 1.');
		expect(describeAccrual({ ...gridded, months: 12 })).toBe('Jährlich ab dem 1.');
	});

	it('counts out the ones that do not', () => {
		expect(describeAccrual({ ...gridded, months: 2 })).toBe('alle 2 Monate ab dem 1.');
	});

	/// The phase is what puts a payment in the period it pays for rather than the one it lands
	/// in, so a rule that is anything other than paid on day one has to say so.
	it('says where in its period the payment falls, unless that is the first day', () => {
		expect(describeAccrual({ ...gridded, phaseDays: 30 })).toBe(
			'Monatlich ab dem 1., Zahlung +30 Tage'
		);
		expect(describeAccrual({ ...gridded, phaseDays: 1 })).toBe(
			'Monatlich ab dem 1., Zahlung +1 Tag'
		);
		// paid in advance
		expect(describeAccrual({ ...gridded, phaseDays: -2 })).toBe(
			'Monatlich ab dem 1., Zahlung -2 Tage'
		);
		expect(describeAccrual({ ...gridded, phaseDays: -1 })).toBe(
			'Monatlich ab dem 1., Zahlung -1 Tag'
		);
	});

	/// A span that is not a whole number of months follows the payment around instead of
	/// snapping to a grid, and then there is no anchor day to report.
	it('describes a free-floating span by its length', () => {
		expect(describeAccrual({ months: null, days: 1, anchorDay: 4, phaseDays: 0 })).toBe(
			'1 Tag ab Zahlung'
		);
		expect(describeAccrual({ months: null, days: 9, anchorDay: 4, phaseDays: 0 })).toBe(
			'9 Tage ab Zahlung'
		);
	});
});
