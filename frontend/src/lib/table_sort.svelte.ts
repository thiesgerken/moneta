/// The one way a table in this app is sorted.
///
/// Five tables had grown their own copy of this, two of them keeping the sort as an array whose
/// later entries nothing ever read. It is one field and a direction everywhere now.

export type Sort = { field: string; direction: number };

/// `-1` is descending, matching what the expense and balance lists hand the API as `DESC`.
export const DESC = -1;
export const ASC = 1;

export class TableSort {
	#fallback: Sort;
	current: Sort = $state({ field: '', direction: DESC });

	/// `fallback` is what an untouched table is sorted by, and what a third click returns to.
	constructor(fallback: Sort) {
		this.#fallback = fallback;
		this.current = { ...fallback };
	}

	get field() {
		return this.current.field;
	}

	get direction() {
		return this.current.direction;
	}

	/// Descending, ascending, off. The third state is what there was no way back to: sorting
	/// lives in the component rather than in the url, so a header clicked by accident could
	/// otherwise only be undone by reloading the page.
	///
	/// The column the fallback is already on has no "off" to go to -- off *is* that column --
	/// so it flips instead of appearing not to react.
	by(field: string) {
		if (this.current.field !== field) this.current = { field, direction: DESC };
		else if (this.current.direction === DESC) this.current = { field, direction: ASC };
		else if (field === this.#fallback.field) this.current = { field, direction: DESC };
		else this.current = { ...this.#fallback };
	}

	/// The direction this field is sorted in, or null when it is not the one.
	on(field: string): number | null {
		return this.current.field === field ? this.current.direction : null;
	}

	/// Sorts in place of a hand-written comparator, for the tables that hold the whole list and
	/// sort it here rather than asking the server. `keys` maps a field to what to compare on;
	/// `tiebreak` decides rows that compare equal, so the order does not wobble between renders.
	apply<T>(
		rows: T[],
		keys: Record<string, (row: T) => string | number>,
		tiebreak: (row: T) => string
	) {
		const key = keys[this.current.field] ?? keys[this.#fallback.field];
		if (!key) return [...rows];

		return [...rows].sort((a, b) => {
			const [x, y] = [key(a), key(b)];
			if (x === y) return tiebreak(a).localeCompare(tiebreak(b));
			return (x < y ? -1 : 1) * this.current.direction;
		});
	}
}
