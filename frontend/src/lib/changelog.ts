/// Reading the field-by-field changes a changelog entry carries.
///
/// The backend walks the two stored states and reports the leaves that differ: a path like
/// `transactions/0/amount` and the JSON on either side. Naming the field and formatting the
/// value is this file's job — only the browser knows that cents are money, that an id is an
/// account somebody keeps, and how a date is written here.

import { euro } from './money';
import { shortDate } from './analysis/dates';

export type Change = { path: string; old: string | null; new: string | null };

/// One line for a reader: what moved, and what it was and became. A side is null where the
/// field was not there at all, which is what an added or removed list element looks like.
export type Described = { label: string; old: string | null; new: string | null };

/// The keys worth naming. Anything not in here is a field a reader would not recognise under
/// its own name either, so it goes out as the key itself rather than as a guess.
const LABELS: Record<string, string> = {
	title: 'Titel',
	store: 'Geschäft',
	description: 'Beschreibung',
	comments: 'Kommentar',
	referenceAmount: 'Referenzbetrag',
	amount: 'Betrag',
	fraction: 'Anteil',
	accrualStart: 'Leistung ab',
	accrualEnd: 'Leistung bis',
	date: 'Datum',
	bookingDate: 'Buchungsdatum',
	accountId: 'Konto',
	categoryId: 'Kategorie',
	weight: 'Gewichtung',
	statement: 'Verwendungszweck',
	isTemplate: 'Vorlage',
	isTaxRelevant: 'Steuerrelevant',
	isUnchecked: 'Ungeprüft',
	isIncomplete: 'Unvollständig',
	isExtraordinary: 'Außerordentlich',
	quantity: 'Menge',
	unit: 'Einheit',
	name: 'Bezeichnung',
	uri: 'Beleg',
	ocr: 'Texterkennung'
};

/// The lists, singular: an element of them is numbered rather than named.
const ELEMENTS: Record<string, string> = {
	transactions: 'Transaktion',
	categories: 'Kategorie',
	positions: 'Position',
	receipts: 'Beleg'
};

/// Bookkeeping a reader did not change. `updatedAt` is already normalised away by the backend;
/// the rest never moves and would only be noise if it did.
const IGNORED = new Set(['id', 'createdAt', 'updatedAt']);

/// Cents on the way in, money on the way out.
const MONEY = new Set(['amount', 'referenceAmount']);
const DATES = new Set(['date', 'bookingDate', 'accrualStart', 'accrualEnd']);
const IDS = new Set(['accountId', 'categoryId']);

const label = (key: string) => LABELS[key] ?? key;

/// "Betrag", "2. Transaktion", "2. Transaktion: Anteil".
function pathLabel(path: string): string {
	const [head, index, field] = path.split('/');
	if (!(head in ELEMENTS) || index === undefined) return label(head);

	const element = `${Number(index) + 1}. ${ELEMENTS[head]}`;
	return field ? `${element}: ${label(field)}` : element;
}

/// Which field a value belongs to, which is what decides how it is written. For a whole list
/// element it is the list itself, so the object is summarised rather than printed.
const pathKey = (path: string) => path.split('/').at(-1)!;

function value(key: string, raw: unknown, names: (id: string) => string): string {
	if (raw === null || raw === undefined) return '—';

	if (typeof raw === 'object') return summarize(raw as Record<string, unknown>, names);
	if (typeof raw === 'boolean') return raw ? 'ja' : 'nein';

	if (typeof raw === 'number') {
		if (MONEY.has(key)) return euro(raw);
		// a fraction is a share of the reference amount, and 0.5 is how nobody says "half"
		if (key === 'fraction') return `${(raw * 100).toLocaleString('de-DE')} %`;
		return raw.toLocaleString('de-DE');
	}

	if (typeof raw === 'string') {
		if (IDS.has(key)) return names(raw);
		if (DATES.has(key)) return shortDate(new Date(raw));
		return raw === '' ? '—' : `„${raw}“`;
	}

	return String(raw);
}

/// A whole list element, in one line: "Konto: Schulden Krösus · Anteil: -50 %".
///
/// Field by field rather than a shape per list, since the only thing that makes a transaction
/// read differently from a position is which fields it happens to carry. Empty ones are left
/// out — half of what an expense stores is the empty string.
function summarize(object: Record<string, unknown>, names: (id: string) => string): string {
	return (
		Object.entries(object)
			.filter(([key, raw]) => !IGNORED.has(key) && raw !== null && raw !== '')
			.map(([key, raw]) => `${label(key)}: ${value(key, raw, names)}`)
			.join(' · ') || '—'
	);
}

/// `names` turns an account or category id into what that thing is called; anything it does
/// not know keeps its id, which is still better than dropping the line.
export function describeChanges(
	changes: readonly Change[],
	names: Record<string, string> = {}
): Described[] {
	const name = (id: string) => names[id] ?? id;

	return changes
		.filter((change) => !IGNORED.has(pathKey(change.path)))
		.map((change) => {
			const key = pathKey(change.path);
			const side = (raw: string | null) =>
				raw === null ? null : value(key, JSON.parse(raw), name);

			return { label: pathLabel(change.path), old: side(change.old), new: side(change.new) };
		});
}

/// The fields an entry touched, named and deduplicated, for the one line a feed row has.
/// The leaf rather than the list it sits in: "Betrag geändert" says more than
/// "Transaktionen geändert", and which transaction it was is what opening the entry is for.
export function changedFields(changes: readonly Change[]): string[] {
	const seen = new Set<string>();

	for (const change of changes) {
		const key = pathKey(change.path);
		if (IGNORED.has(key)) continue;
		seen.add(key in ELEMENTS ? ELEMENTS[key] : label(key));
	}

	return [...seen];
}

/// "Titel", "Titel und Betrag", "Titel, Betrag und 2 weitere"
export function summarizeFields(fields: string[]): string | null {
	if (fields.length === 0) return null;
	if (fields.length === 1) return fields[0];
	if (fields.length === 2) return `${fields[0]} und ${fields[1]}`;
	return `${fields[0]}, ${fields[1]} und ${fields.length - 2} weitere`;
}

/// What the expense came to, from the raw state a changelog entry stores.
///
/// A transaction carries either an amount or a fraction of the reference amount, and the two
/// sides of a split cancel: a 13,80 € bill paid in full and halved with someone arrives as
/// -1380 and +690, which nets to the 6,90 € it actually cost. Close enough to the
/// `effectiveAmount` the expense list shows, which is not in the stored state — that one also
/// weighs account ownership and mirroring, so a joint or mirrored account can differ here.
export function stateAmount(state: {
	referenceAmount?: number | null;
	transactions: readonly { amount?: number | null; fraction?: number | null }[];
}): number | null {
	if (state.transactions.length === 0) return null;

	return state.transactions.reduce(
		(sum, t) => sum + (t.amount ?? (t.fraction ?? 0) * (state.referenceAmount ?? 0)),
		0
	);
}
