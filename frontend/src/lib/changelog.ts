/// Reading a unified diff of two serialised expenses.
///
/// The backend diffs the JSON of the two states, which is exact but unreadable — a changed
/// amount arrives as a dozen lines of context around one number. What a feed wants is the
/// short version: which fields moved.

/// The keys worth naming. Anything not in here is a field a reader would not recognise either,
/// so it is counted but not named.
const FIELD_LABELS: Record<string, string> = {
	title: 'Titel',
	store: 'Geschäft',
	description: 'Beschreibung',
	comments: 'Kommentar',
	reference_amount: 'Betrag',
	amount: 'Betrag',
	fraction: 'Anteil',
	accrual_start: 'Leistungszeitraum',
	accrual_end: 'Leistungszeitraum',
	date: 'Datum',
	account_id: 'Konto',
	category_id: 'Kategorie',
	weight: 'Gewichtung',
	categories: 'Kategorien',
	transactions: 'Transaktionen',
	positions: 'Positionen',
	receipts: 'Belege',
	statement: 'Verwendungszweck',
	is_template: 'Vorlage',
	is_tax_relevant: 'Steuerrelevanz',
	is_unchecked: 'Prüfstatus',
	is_incomplete: 'Vollständigkeit',
	is_extraordinary: 'Außerordentlich',
	quantity: 'Menge',
	name: 'Bezeichnung',
	uri: 'Beleg'
};

/// Bookkeeping a reader did not change: `updated_at` moves on every save, and ids only show up
/// as context around what did change.
const IGNORED = new Set(['updated_at', 'id', 'expense_id']);

/// The fields a diff touches, in the order they first appear, named where we have a name.
export function changedFields(diff: string): string[] {
	const seen = new Set<string>();

	for (const line of diff.split('\n')) {
		if (!/^[+-]/.test(line) || /^(\+\+\+|---)/.test(line)) continue;

		// `  "title": "Rewe",` — array elements and closing braces carry no key, and a
		// changed array element is reported field by field anyway
		const key = /^[+-]\s*"([^"]+)":/.exec(line)?.[1];
		if (!key || IGNORED.has(key)) continue;

		seen.add(FIELD_LABELS[key] ?? key);
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
