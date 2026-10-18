import { describe, expect, it } from 'vitest';
import {
	FREQUENT,
	accountSorter,
	byUsageThen,
	categorySorter,
	describeAccrual,
	statementLine,
	formatAccount,
	formatAccrualPeriod,
	formatCategories,
	formatTransactionAccounts,
	makeAccountColor,
	makeCategoryColor,
	makeStripedBackground,
	referenceAmountMissing,
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

/// An ordinary expense and a prepayment differ by nothing but whether the two ends of the
/// accrual period are the same instant, so this sentence is where that shows up on screen.
describe('formatAccrualPeriod', () => {
	const day = (d: number) => new Date(Date.UTC(2026, 7, d, 12));

	it('writes one date where the period is a single instant', () => {
		expect(formatAccrualPeriod(day(4), day(4))).toBe('04.08.2026');
	});

	it('writes both ends of a span', () => {
		expect(formatAccrualPeriod(day(1), day(31))).toBe('01.08.2026 - 31.08.2026');
	});

	// two instants on the same day are still a span, since the comparison is on the instant
	it('compares instants rather than days', () => {
		const morning = new Date(Date.UTC(2026, 7, 4, 6));
		const evening = new Date(Date.UTC(2026, 7, 4, 18));
		expect(formatAccrualPeriod(morning, evening)).toContain(' - ');
	});
});

describe('formatAccount', () => {
	const account = (name: string, owner: string, firstName: string | null = null) => ({
		name,
		owner: { name: owner, firstName }
	});

	// my own accounts are the common case and naming me on every row of my own list is noise
	it('leaves my own account bare', () => {
		expect(formatAccount(account('Girokonto', 'alice'), 'alice')).toBe('Girokonto');
	});

	it('says whose it is when it is not mine', () => {
		expect(formatAccount(account('Girokonto', 'bob', 'Bob'), 'alice')).toBe('Bob / Girokonto');
	});

	it('falls back to the login name where there is no first name', () => {
		expect(formatAccount(account('Girokonto', 'bob'), 'alice')).toBe('bob / Girokonto');
	});

	// nobody logged in yet: every account is somebody else's, which is the safe way round
	it('qualifies everything when it does not know who is reading', () => {
		expect(formatAccount(account('Girokonto', 'alice', 'Alice'), undefined)).toBe(
			'Alice / Girokonto'
		);
	});
});

describe('formatTransactionAccounts', () => {
	const leg = (name: string, owner: string, effectiveAmount: number) => ({
		effectiveAmount,
		account: { name, owner: { name: owner, firstName: null } }
	});

	it('names the account the most money moved on first', () => {
		expect(
			formatTransactionAccounts(
				[leg('Bargeld', 'alice', -100), leg('Giro', 'alice', -900)],
				'alice'
			)
		).toBe('Giro, Bargeld');
	});

	// two legs on one account are one account, not two entries
	it('adds up the legs that land on the same account', () => {
		expect(
			formatTransactionAccounts(
				[leg('Giro', 'alice', -100), leg('Bargeld', 'alice', -200), leg('Giro', 'alice', -500)],
				'alice'
			)
		).toBe('Giro, Bargeld');
	});

	// a transfer has legs pointing opposite ways, and a big one either way is still the big one
	it('ranks by how much moved, not by which direction', () => {
		expect(
			formatTransactionAccounts([leg('Giro', 'alice', 900), leg('Bargeld', 'alice', -100)], 'alice')
		).toBe('Giro, Bargeld');
	});

	it('has nothing to say about no transactions', () => {
		expect(formatTransactionAccounts([], 'alice')).toBe('');
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

		expect(style).toContain('0% 75%');
		expect(style).toContain('75% 100%');
	});

	/// An empty list used to produce `linear-gradient(to right, )`, which is not a gradient, so
	/// the browser dropped the whole declaration along with the text colour in it.
	it('produces nothing at all rather than an invalid gradient', () => {
		expect(makeStripedBackground([])).toBeUndefined();
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
		expect(style).toContain('0% 75%');
		expect(style).toContain('75% 100%');
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
		expect(style).toContain('0% 75%');
	});

	/// An expense filed partly under a category with no colour keeps the gap: the weights that
	/// do have one are still measured against the whole, so the stripe stops short rather than
	/// claiming the uncoloured share.
	it('measures against the whole weight, colour or not', () => {
		const style = makeCategoryColor([filed('#00ff00', 0.5), filed(null, 0.5)]);
		expect(style).toContain('0% 50%');
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
		expect(sorted([account('Zweitkonto', 'bob'), account('Aaa', 'alice')], 'alice')).toEqual([
			'Aaa',
			'Zweitkonto'
		]);
	});

	it('groups the other people by name', () => {
		expect(
			sorted([account('b', 'bob'), account('a', 'ada'), account('c', 'bob')], 'alice')
		).toEqual(['a', 'b', 'c']);
	});

	// a closed account is history: still reachable, but not in the way
	it('sinks a closed account below the open ones of the same owner', () => {
		expect(
			sorted([account('a', 'alice', new Date('2000-01-01')), account('z', 'alice')], 'alice')
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

/// A rule template's blank booking still has a reference amount: zero is a value, not a gap.
describe('referenceAmountMissing', () => {
	it('treats zero as an answer, not as a gap', () => {
		expect(referenceAmountMissing('0')).toBe(false);
		expect(referenceAmountMissing('0.00')).toBe(false);
		expect(referenceAmountMissing('-1234.56')).toBe(false);
	});

	it('is only unfilled when nothing was typed', () => {
		expect(referenceAmountMissing(undefined)).toBe(true);
		expect(referenceAmountMissing('')).toBe(true);
		expect(referenceAmountMissing('   ')).toBe(true);
	});

	it('rejects what is not a number', () => {
		expect(referenceAmountMissing('keine Ahnung')).toBe(true);
	});
});

describe('statementLine', () => {
	const card = (head: string) =>
		`${head}\n\napplicant_bin: TESTDE00XXX\napplicant_iban: DE00000000000000000000\napplicant_name: BEISPIELMARKT/TESTSTADT\nposting_text: KARTENZAHLUNG`;

	it('names the shop where the bank only said a card was used', () => {
		expect(statementLine(card('VISA Debitkartenumsatz'))).toBe('BEISPIELMARKT/TESTSTADT');
		expect(statementLine(card('VISA Debitkartenumsatz vom 18.03.2030'))).toBe(
			'BEISPIELMARKT/TESTSTADT'
		);
		expect(statementLine(card('Girokartenumsatz'))).toBe('BEISPIELMARKT/TESTSTADT');
	});

	it('names the shop behind the older spelling of the same nothing', () => {
		expect(statementLine(card('2030-01-15T10.20Debitk.16 2099-12 Zahl.System VISA Debit'))).toBe(
			'BEISPIELMARKT/TESTSTADT'
		);
		expect(statementLine(card('2029-11-05T14.35 Debitk.0 2032-12'))).toBe(
			'BEISPIELMARKT/TESTSTADT'
		);
	});

	it('keeps what the older line says beyond the card', () => {
		const withdrawal = '2030-02-06T09.15 Debitk.0 2032-12 Bargeldausz. 80,00 EUR';
		expect(statementLine(card(withdrawal))).toBe(withdrawal);
	});

	it('keeps a free text that says something of its own', () => {
		expect(statementLine(card('000-0000000-0000000 AMZNMktplace'))).toBe(
			'000-0000000-0000000 AMZNMktplace'
		);
		// the merchant on the boilerplate's own line is the free text, not boilerplate
		expect(statementLine(card('VISA Debitkartenumsatz BEISPIELWERKSTATT'))).toBe(
			'VISA Debitkartenumsatz BEISPIELWERKSTATT'
		);
	});

	it('drops the field block a cell one line high cannot show', () => {
		expect(statementLine('BEISPIELMARKT Teststadt\n\napplicant_bin: TESTDE00XXX')).toBe(
			'BEISPIELMARKT Teststadt'
		);
	});

	it('falls back to the boilerplate when there is nothing better', () => {
		expect(statementLine('Girokartenumsatz')).toBe('Girokartenumsatz');
		expect(statementLine('Dauerauftrag Ausgleich Testkonto')).toBe(
			'Dauerauftrag Ausgleich Testkonto'
		);
	});
});
