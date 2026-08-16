import { describe, expect, it } from 'vitest';
import { ASC, DESC, TableSort } from './table_sort.svelte';

type Row = { name: string; amount: number };

const rows: Row[] = [
	{ name: 'Bäcker', amount: -3 },
	{ name: 'Aral', amount: -10 },
	{ name: 'Cafe', amount: -3 }
];

const keys = {
	name: (r: Row) => r.name.toLowerCase(),
	amount: (r: Row) => r.amount
};

const names = (sort: TableSort) => sort.apply(rows, keys, (r) => r.name).map((r) => r.name);

describe('TableSort', () => {
	it('starts on the fallback it was given', () => {
		const sort = new TableSort({ field: 'name', direction: ASC });

		expect(sort.field).toBe('name');
		expect(sort.on('name')).toBe(ASC);
		expect(sort.on('amount')).toBeNull();
	});

	/// Descending, ascending, off -- the third state is the one a table without it could only
	/// get back to by reloading the page.
	it('cycles a column through descending, ascending and back to the fallback', () => {
		const sort = new TableSort({ field: 'name', direction: ASC });

		sort.by('amount');
		expect([sort.field, sort.direction]).toEqual(['amount', DESC]);

		sort.by('amount');
		expect([sort.field, sort.direction]).toEqual(['amount', ASC]);

		sort.by('amount');
		expect([sort.field, sort.direction]).toEqual(['name', ASC]);
	});

	/// Its own column has no "off" to reach -- off is that column -- so it flips rather than
	/// looking like a click that did nothing.
	it('flips the fallback column instead of resetting to it', () => {
		const sort = new TableSort({ field: 'name', direction: ASC });

		sort.by('name');
		expect([sort.field, sort.direction]).toEqual(['name', DESC]);

		sort.by('name');
		expect([sort.field, sort.direction]).toEqual(['name', ASC]);
	});

	it('starts a different column at descending, whatever the last one was on', () => {
		const sort = new TableSort({ field: 'name', direction: ASC });

		sort.by('amount');
		sort.by('amount'); // now ascending
		sort.by('name');

		expect([sort.field, sort.direction]).toEqual(['name', DESC]);
	});

	it('sorts by the field it is on, in the direction it is on', () => {
		const sort = new TableSort({ field: 'name', direction: ASC });
		expect(names(sort)).toEqual(['Aral', 'Bäcker', 'Cafe']);

		sort.by('name'); // descending
		expect(names(sort)).toEqual(['Cafe', 'Bäcker', 'Aral']);
	});

	/// Rows that compare equal must not swap places between renders, which is what the tiebreak
	/// is for -- here two expenses of the same amount.
	it('breaks a tie the same way every time', () => {
		const sort = new TableSort({ field: 'amount', direction: ASC });

		expect(names(sort)).toEqual(['Aral', 'Bäcker', 'Cafe']);
		sort.by('amount'); // descending: the tie holds its order rather than reversing
		expect(names(sort)).toEqual(['Bäcker', 'Cafe', 'Aral']);
	});

	it('falls back to the fallback key when the field has none', () => {
		const sort = new TableSort({ field: 'name', direction: ASC });
		sort.current = { field: 'nonesuch', direction: ASC };

		expect(names(sort)).toEqual(['Aral', 'Bäcker', 'Cafe']);
	});

	it('leaves the input alone', () => {
		const sort = new TableSort({ field: 'amount', direction: DESC });
		const before = [...rows];

		sort.apply(rows, keys, (r) => r.name);

		expect(rows).toEqual(before);
	});
});
