import { describe, expect, it } from 'vitest';
import {
	FREQUENT,
	accountSorter,
	byUsageThen,
	categorySorter,
	describeAccrual,
	formatCategories,
	makeAccountColor,
	makeCategoryColor,
	makeStripedBackground,
	readableTextOn,
	tint,
	type SortableAccount
} from './expense_utils';

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

/// Account and category colours come out of a free picker, so nothing stops one being
/// near-black — and a cell that writes its name in black on that is a bar with no text in it.
describe('readableTextOn', () => {
	it('writes black on a light colour and white on a dark one', () => {
		expect(readableTextOn('#ffffff')).toBe('#000');
		expect(readableTextOn('#000000')).toBe('#fff');
	});

	/// The crossover sits at a luminance of 0.179, where black and white have equal contrast,
	/// rather than at the midpoint — which would call the mid greens and purples this palette
	/// is full of light and leave them barely legible.
	it('puts the crossover where contrast is equal, not at the midpoint', () => {
		// #757575 has a luminance just under 0.179, #767676 just over — and both are darker
		// than the 50% grey a midpoint rule would break at
		expect(readableTextOn('#757575')).toBe('#fff');
		expect(readableTextOn('#767676')).toBe('#000');
	});

	it('understands the short form of a hex colour', () => {
		expect(readableTextOn('#fff')).toBe(readableTextOn('#ffffff'));
		expect(readableTextOn('#000')).toBe(readableTextOn('#000000'));
	});

	/// One piece of text crosses every stripe of a shared expense, so the darkest and the
	/// lightest decide it together — a colour that would take black on its own takes white
	/// once there is a dark stripe beside it.
	it('answers for all the colours at once', () => {
		expect(readableTextOn('#767676')).toBe('#000');
		expect(readableTextOn('#767676', '#000000')).toBe('#fff');
	});

	/// Where a cell mixes a light colour with a dark one neither answer is good and this picks
	/// the less bad one. Black on white beats white on black, so an exact tie goes to black;
	/// the honest fix is not to give two accounts colours that far apart.
	it('gives a tie to black', () => {
		expect(readableTextOn('#ffffff', '#000000')).toBe('#000');
	});

	// an unparseable colour is nobody's idea of dark; leave the page's own text colour alone
	it('leaves the page alone when it cannot read the colour', () => {
		expect(readableTextOn('rebeccapurple')).toBe('inherit');
		expect(readableTextOn()).toBe('inherit');
	});
});

describe('tint', () => {
	it('carries the text colour along with the background', () => {
		expect(tint('#000000')).toBe('background-color: #000000; color: #fff;');
	});

	it('has nothing to say about an uncoloured thing', () => {
		expect(tint(null)).toBeUndefined();
		expect(tint(undefined)).toBeUndefined();
		expect(tint('')).toBeUndefined();
	});
});

describe('makeStripedBackground', () => {
	it('lays the stripes out biggest first, end to end', () => {
		const style = makeStripedBackground([
			{ color: '#ff0000', percentage: 0.25 },
			{ color: '#00ff00', percentage: 0.75 }
		]);

		expect(style).toContain('linear-gradient(to right, #00ff00 0% 75%, #ff0000 75% 100%)');
	});

	/// An empty list used to produce `linear-gradient(to right, )`, which is not a gradient, so
	/// the browser dropped the whole declaration along with the text colour in it.
	it('produces nothing at all rather than an invalid gradient', () => {
		expect(makeStripedBackground([])).toBeUndefined();
	});

	it('picks a text colour that works across every stripe', () => {
		const style = makeStripedBackground([
			{ color: '#767676', percentage: 0.5 },
			{ color: '#000000', percentage: 0.5 }
		]);

		// #767676 alone would take black; the dark stripe beside it is what flips this
		expect(style).toContain('color: #fff');
	});
});

describe('makeAccountColor', () => {
	const leg = (color: string | null, amount: number | null, fraction: number | null = null) => ({
		amount,
		fraction,
		account: { color }
	});

	it('gives each account a share of the stripe by what moved on it', () => {
		const style = makeAccountColor([leg('#00ff00', -750), leg('#ff0000', -250)], null);
		expect(style).toContain('#00ff00 0% 75%');
		expect(style).toContain('#ff0000 75% 100%');
	});

	// the legs of a split point in opposite directions, and a share is a share either way
	it('weighs a leg by what it moved, not by which way', () => {
		const style = makeAccountColor([leg('#00ff00', -500), leg('#ff0000', 500)], null);
		expect(style).toContain('0% 50%');
	});

	it('resolves a fractional leg against the reference amount', () => {
		const style = makeAccountColor([leg('#00ff00', -1000), leg('#ff0000', null, 0.25)], -4000);
		// 1000 against 1000, so the two halves are equal despite the fraction
		expect(style).toContain('0% 50%');
	});

	it('has nothing to draw when no account carries a colour', () => {
		expect(makeAccountColor([leg(null, -500)], null)).toBeUndefined();
	});

	// an account with a colour but nothing on it would otherwise be a zero-width stripe
	it('leaves out a leg that moved nothing', () => {
		expect(makeAccountColor([leg('#00ff00', 0)], null)).toBeUndefined();
	});
});

describe('makeCategoryColor', () => {
	const filed = (color: string | null, weight: number) => ({ weight, category: { color } });

	it('splits the stripe by categorization weight', () => {
		const style = makeCategoryColor([filed('#00ff00', 0.75), filed('#ff0000', 0.25)]);
		expect(style).toContain('#00ff00 0% 75%');
	});

	/// An expense filed partly under a category with no colour keeps the gap: the weights that
	/// do have one are still measured against the whole, so the stripe stops short rather than
	/// claiming the uncoloured share.
	it('measures against the whole weight, colour or not', () => {
		const style = makeCategoryColor([filed('#00ff00', 0.5), filed(null, 0.5)]);
		expect(style).toContain('#00ff00 0% 50%');
	});
});

describe('formatCategories', () => {
	const category = (name: string, weight: number) => ({ weight, category: { name } });

	it('names the heaviest category first', () => {
		expect(formatCategories([category('Haushalt', 0.25), category('Lebensmittel', 0.75)])).toBe(
			'Lebensmittel, Haushalt'
		);
	});

	// the same category twice is one category with the weights added, not two entries
	it('adds up a category that appears more than once', () => {
		expect(
			formatCategories([
				category('Haushalt', 0.6),
				category('Lebensmittel', 0.5),
				category('Haushalt', 0.3)
			])
		).toBe('Haushalt, Lebensmittel');
	});

	it('has nothing to say about an uncategorized expense', () => {
		expect(formatCategories([])).toBe('');
	});
});

describe('accountSorter', () => {
	const account = (name: string, owner: string, closed: Date | null = null): SortableAccount => ({
		name,
		owner: { name: owner },
		closed
	});

	const sorted = (accounts: SortableAccount[], me: string) =>
		accounts.toSorted((a, b) => accountSorter(a, b, me)).map((a) => a.name);

	it('puts my own accounts first, whatever they are called', () => {
		expect(sorted([account('Zweitkonto', 'kroesus'), account('Aaa', 'thies')], 'thies')).toEqual([
			'Aaa',
			'Zweitkonto'
		]);
	});

	it('groups the other people by name', () => {
		expect(
			sorted([account('b', 'kroesus'), account('a', 'anna'), account('c', 'kroesus')], 'thies')
		).toEqual(['a', 'b', 'c']);
	});

	// a closed account is history: still reachable, but not in the way
	it('sinks a closed account below the open ones of the same owner', () => {
		expect(
			sorted([account('a', 'thies', new Date('2020-01-01')), account('z', 'thies')], 'thies')
		).toEqual(['z', 'a']);
	});
});

describe('categorySorter', () => {
	it('sorts by name, respecting the umlauts', () => {
		expect(
			['Öl', 'Auto', 'Zoo'].toSorted((a, b) => categorySorter({ name: a }, { name: b }))
		).toEqual(['Auto', 'Öl', 'Zoo']);
	});
});

/// The five accounts you actually use are five of dozens and alphabetical order buries
/// them — but sorting the whole list by frequency puts the ones you touch twice a year in an
/// order nobody can predict.
describe('byUsageThen', () => {
	const option = (name: string, usageCount: number) => ({ name, usageCount });
	const byName = (a: { name: string }, b: { name: string }) => a.name.localeCompare(b.name);

	it('puts the most used first and leaves the tail alphabetical', () => {
		const options = [option('b', 0), option('a', 0), option('c', 7)];
		expect(byUsageThen(options, byName).map((o) => o.name)).toEqual(['c', 'a', 'b']);
	});

	it('promotes no more than FREQUENT of them', () => {
		const options = Array.from({ length: 8 }, (_, i) => option(`a${i}`, 100 - i));
		const promoted = byUsageThen(options, byName)
			.slice(0, FREQUENT)
			.map((o) => o.name);

		expect(promoted).toEqual(['a0', 'a1', 'a2', 'a3', 'a4']);
	});

	// something never used is not "least frequent", it is just alphabetical
	it('never promotes something with no uses at all', () => {
		const options = [option('b', 0), option('a', 0)];
		expect(byUsageThen(options, byName).map((o) => o.name)).toEqual(['a', 'b']);
	});

	it('keeps every option exactly once', () => {
		const options = [option('b', 3), option('a', 0), option('c', 1)];
		expect(byUsageThen(options, byName)).toHaveLength(3);
	});
});
