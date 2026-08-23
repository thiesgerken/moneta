import { describe, expect, it } from 'vitest';
import {
	ABSENT,
	changedFields,
	describeChanges,
	runs,
	stateAmount,
	summarizeFields,
	type Change
} from './changelog';

/// Intl puts a non-breaking space before the sign. Escaped rather than typed, because the two
/// spaces are indistinguishable in the source and the failure they cause is a diff of two
/// strings that look identical.
const eur = (s: string) => `${s}\u00A0€`;

/// The backend hands the two sides over as JSON, so a test that passes plain strings would be
/// testing something the frontend never sees.
const change = (path: string, oldValue: unknown, newValue: unknown): Change => ({
	path,
	old: oldValue === undefined ? null : JSON.stringify(oldValue),
	new: newValue === undefined ? null : JSON.stringify(newValue)
});

const only = (c: Change, names?: Record<string, string>) => describeChanges([c], names)[0];

describe('describeChanges', () => {
	it('names the field in German and quotes a string', () => {
		expect(only(change('title', 'Kaffee', 'Kaffee und Kuchen'))).toEqual({
			label: 'Titel',
			old: '„Kaffee“',
			new: '„Kaffee und Kuchen“'
		});
	});

	it('reads an amount as money rather than as a number', () => {
		const described = only(change('amount', -1380, -690));
		expect(described.old).toBe(eur('-13,80'));
		expect(described.new).toBe(eur('-6,90'));
	});

	// 0.5 is how nobody says "half"
	it('writes a fraction as a percentage', () => {
		expect(only(change('fraction', 0.25, 0.5)).new).toBe('50 %');
	});

	it('leaves a plain number alone', () => {
		expect(only(change('weight', 1, 2)).new).toBe('2');
	});

	it('writes a date the way it is written down', () => {
		expect(only(change('date', undefined, '2026-08-04T12:00:00Z')).new).toBe('04.08.2026');
	});

	it('says ja and nein for a flag', () => {
		const described = only(change('isTemplate', false, true));
		expect([described.old, described.new]).toEqual(['nein', 'ja']);
	});

	it('resolves an id to what the thing is called', () => {
		const described = only(change('accountId', undefined, 'acc-1'), { 'acc-1': 'Girokonto' });
		expect(described.new).toBe('Girokonto');
	});

	// still better than dropping the line
	it('keeps an id it has no name for', () => {
		expect(only(change('accountId', undefined, 'acc-9')).new).toBe('acc-9');
	});

	// a field that was empty, one that was null and a list element that did not exist yet all
	// say the same thing to a reader
	it('reads an empty string and a missing side the same way', () => {
		expect(only(change('store', '', 'Rewe')).old).toBe(ABSENT);
		expect(only(change('store', undefined, 'Rewe')).old).toBe(ABSENT);
	});

	it('numbers an element of a list and names the field inside it', () => {
		expect(only(change('transactions/1/amount', -100, -200)).label).toBe('2. Transaktion: Betrag');
		expect(only(change('transactions/0', undefined, { amount: -100 })).label).toBe(
			'1. Transaktion'
		);
	});

	it('summarizes a whole list element on one line, leaving the empty fields out', () => {
		const described = only(
			change('transactions/0', undefined, {
				id: 'ignored',
				accountId: 'acc-1',
				amount: -1380,
				comments: '',
				statement: null
			}),
			{ 'acc-1': 'Girokonto' }
		);

		expect(described.new).toBe(`Konto: Girokonto · Betrag: ${eur('-13,80')}`);
	});

	it('says so when a list element carried nothing worth showing', () => {
		expect(only(change('positions/0', undefined, { id: 'x', name: '' })).new).toBe(ABSENT);
	});

	it('drops the bookkeeping a reader never changed', () => {
		expect(
			describeChanges([
				change('id', 'a', 'b'),
				change('createdAt', '2026-01-01Z', '2026-01-02Z'),
				change('title', 'a', 'b')
			])
		).toHaveLength(1);
	});

	// a key nobody named is still better out as itself than as a guess
	it('falls back to the key for a field it has no label for', () => {
		expect(only(change('somethingNew', 1, 2)).label).toBe('somethingNew');
	});
});

describe('changedFields', () => {
	// "Betrag geändert" says more than "Transaktionen geändert"
	it('names the leaf rather than the list it sits in', () => {
		expect(changedFields([change('transactions/0/amount', -1, -2)])).toEqual(['Betrag']);
	});

	it('names the list itself when the whole element moved', () => {
		expect(changedFields([change('transactions', [], [{}])])).toEqual(['Transaktion']);
	});

	it('says a field once however many times it changed', () => {
		expect(
			changedFields([
				change('transactions/0/amount', -1, -2),
				change('transactions/1/amount', -3, -4)
			])
		).toEqual(['Betrag']);
	});

	it('leaves the ignored keys out', () => {
		expect(changedFields([change('updatedAt', 'a', 'b')])).toEqual([]);
	});
});

describe('summarizeFields', () => {
	it('has nothing to say about nothing', () => {
		expect(summarizeFields([])).toBeNull();
	});

	it('joins up to two and counts the rest', () => {
		expect(summarizeFields(['Titel'])).toBe('Titel');
		expect(summarizeFields(['Titel', 'Betrag'])).toBe('Titel und Betrag');
		expect(summarizeFields(['Titel', 'Betrag', 'Konto'])).toBe('Titel, Betrag und 1 weitere');
		expect(summarizeFields(['Titel', 'Betrag', 'Konto', 'Datum'])).toBe(
			'Titel, Betrag und 2 weitere'
		);
	});
});

describe('stateAmount', () => {
	it('adds the fixed legs up', () => {
		expect(stateAmount({ transactions: [{ amount: -1000 }, { amount: -380 }] })).toBe(-1380);
	});

	// a 13,80 € bill paid in full and halved with somebody nets to the 6,90 € it cost
	it('resolves a fractional leg against the reference amount', () => {
		expect(
			stateAmount({ referenceAmount: -1380, transactions: [{ amount: -1380 }, { fraction: -0.5 }] })
		).toBe(-690);
	});

	it('treats a fraction with nothing to resolve against as nothing', () => {
		expect(stateAmount({ transactions: [{ fraction: 0.5 }] })).toBe(0);
	});

	// null rather than 0: an expense with no transactions has no total, and 0,00 € would be a
	// claim that it cost nothing
	it('has no answer for an expense with no transactions', () => {
		expect(stateAmount({ transactions: [] })).toBeNull();
	});
});

describe('runs', () => {
	const entry = (id: string, expenseId: string, name = 'Thies', automatic = false) => ({
		id,
		expenseId,
		automatic,
		user: { name }
	});

	const shape = (list: ReturnType<typeof runs>) => list.map((r) => r.entries.map((e) => e.id));

	it('merges the changes one person made to one expense back to back', () => {
		expect(
			shape(runs([entry('3', 'snickers'), entry('2', 'snickers'), entry('1', 'snickers')]))
		).toEqual([['3', '2', '1']]);
	});

	it('keys the run by its newest entry, so the row does not jump when one is added', () => {
		expect(runs([entry('3', 'snickers'), entry('2', 'snickers')])[0].key).toBe('3');
	});

	it('leaves changes to different expenses alone', () => {
		expect(shape(runs([entry('2', 'snickers'), entry('1', 'wasser')]))).toEqual([['2'], ['1']]);
	});

	// otherwise the row would name one of them and silently swallow the other's edit
	it('does not merge across authors', () => {
		expect(shape(runs([entry('2', 'snickers'), entry('1', 'snickers', 'Krösus')]))).toEqual([
			['2'],
			['1']
		]);
	});

	// the badge says "automatisch" for the whole row, so it has to be true of the whole row
	it('does not merge an automatic change with a hand-made one', () => {
		expect(shape(runs([entry('2', 'snickers'), entry('1', 'snickers', 'Thies', true)]))).toEqual([
			['2'],
			['1']
		]);
	});

	// something else in between means both rows were going to be read anyway, and a merged run
	// would sit at the newer one's place with the older one's siblings out of order around it
	it('only merges what is adjacent', () => {
		expect(
			shape(runs([entry('3', 'snickers'), entry('2', 'wasser'), entry('1', 'snickers')]))
		).toEqual([['3'], ['2'], ['1']]);
	});

	it('has nothing to say about an empty feed', () => {
		expect(runs([])).toEqual([]);
	});
});
