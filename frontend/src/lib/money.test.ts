import { describe, expect, it } from 'vitest';
import { amountColor, balanceColor, euro, restingAmountColor, signedEuro } from './money';

// Intl puts a non-breaking space before the sign, and a test written with an ordinary one
// fails on a difference nobody can see in the diff.
const eur = (s: string) => `${s}\u00A0€`;

describe('euro', () => {
	it('writes cents as money the German way', () => {
		expect(euro(123456)).toBe(eur('1.234,56'));
		expect(euro(-599)).toBe(eur('-5,99'));
		expect(euro(0)).toBe(eur('0,00'));
	});

	// the whole point of storing integer cents is that a third of a cent cannot exist; if one
	// ever arrives, rounding it beats printing 1,3333333 €
	it('never shows more than two decimals', () => {
		expect(euro(133 + 1 / 3)).toBe(eur('1,33'));
	});

	it('keeps the decimals on a round amount', () => {
		expect(euro(500)).toBe(eur('5,00'));
	});
});

describe('signedEuro', () => {
	// a minus is loud and an unmarked positive beside it reads as a number someone forgot to
	// sign, so the plus is spelled out
	it('spells out the plus, and leaves the minus to Intl', () => {
		expect(signedEuro(599)).toBe(eur('+5,99'));
		expect(signedEuro(-599)).toBe(eur('-5,99'));
	});

	it('leaves zero unsigned', () => {
		expect(signedEuro(0)).toBe(eur('0,00'));
	});
});

describe('amountColor', () => {
	it('is red going out and green coming in', () => {
		expect(amountColor(-1)).toContain('red');
		expect(amountColor(1)).toContain('green');
	});

	it('calls zero green, since it is not money going out', () => {
		expect(amountColor(0)).toContain('green');
	});
});

describe('restingAmountColor', () => {
	// where "nothing moved" is a real answer, a green 0,00 € claims otherwise
	it('greys out zero and otherwise agrees with amountColor', () => {
		expect(restingAmountColor(0)).toContain('gray');
		expect(restingAmountColor(-1)).toBe(amountColor(-1));
		expect(restingAmountColor(1)).toBe(amountColor(1));
	});
});

describe('balanceColor', () => {
	// a positive balance is the normal case; only an overdrawn one is news
	it('leaves a positive balance alone and marks only a negative one', () => {
		expect(balanceColor(100)).toBe('');
		expect(balanceColor(-100)).toBe(amountColor(-100));
		expect(balanceColor(0)).toContain('text-gray-500');
	});
});
