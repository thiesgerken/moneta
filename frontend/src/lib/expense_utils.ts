/// How an automation rule will date what it generates, in words.
///
/// None of it is configured on the rule — it is read off the template's own period against the
/// day the template was paid — so this is the only place the behaviour is visible.
/// Whether the Referenzbetrag field is unfilled, for an expense that has a share in it.
///
/// Zero is a real answer and not a missing one: an automation template keeps a blank booking,
/// and `impose` overwrites its reference amount from the delivery. A falsiness check refuses
/// exactly those templates, so this asks what was typed rather than what it is worth.
export function referenceAmountMissing(entered: string | undefined | null): boolean {
	const text = entered?.trim() ?? '';
	return text === '' || isNaN(Number(text));
}

export function describeAccrual(a: {
	months: number | null;
	days: number | null;
	anchorDay: number;
	phaseDays: number;
}): string {
	if (a.months === null) {
		return `${a.days} ${a.days === 1 ? 'Tag' : 'Tage'} ab Zahlung`;
	}

	const grid =
		{ 1: 'Monatlich', 3: 'Quartalsweise', 6: 'Halbjährlich', 12: 'Jährlich' }[a.months] ??
		`alle ${a.months} Monate`;

	// The phase is what places a payment in the period it pays for rather than the one it lands
	// in, so a rule that is anything other than paid on day one has to say so.
	const phase =
		a.phaseDays === 0
			? ''
			: `, Zahlung ${a.phaseDays > 0 ? `+${a.phaseDays}` : a.phaseDays} ${
					Math.abs(a.phaseDays) === 1 ? 'Tag' : 'Tage'
				}`;

	return `${grid} ab dem ${a.anchorDay}.${phase}`;
}

export function formatAccrualPeriod(begin: Date, end: Date, time: boolean = false) {
	const fmt: Intl.DateTimeFormatOptions = time
		? {
				month: '2-digit',
				day: '2-digit',
				year: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			}
		: { month: '2-digit', day: '2-digit', year: 'numeric' };

	if (begin.toISOString() == end.toISOString()) return begin.toLocaleString('de-DE', fmt);

	return `${begin.toLocaleString('de-DE', fmt)} - ${end.toLocaleString('de-DE', fmt)}`;
}

export function formatAccount(
	account: { name: string; owner: { name: string; firstName: string | null } },
	user_name: string | undefined
) {
	return user_name == account.owner.name
		? account.name
		: `${account.owner.firstName || account.owner.name} / ${account.name}`;
}

/// A name with the colour its account or category carries, for a dot in front of it.
export interface Dot {
	name: string;
	color: string | null;
	/// What part of the whole this one is, between 0 and 1; the parts of one list add up to 1.
	share: number;
}

/// Weighted entries as dots in the order given, each with its part of the total.
///
/// Nothing weighed anything -- an expense whose every leg is zero -- shares evenly rather than
/// dividing by zero.
function shares(entries: { name: string; color: string | null; weight: number }[]): Dot[] {
	const total = entries.reduce((sum, e) => sum + e.weight, 0);
	return entries.map(({ name, color, weight }) => ({
		name,
		color,
		share: total > 0 ? weight / total : 1 / entries.length
	}));
}

/// The accounts an expense moved money on, the reader's own first.
///
/// The share is how much moved on each account, whoever it belongs to: a purchase split between
/// two people is shared two ways even though the reader carries only one side, and each leg of a
/// transfer counts by its size although the two cancel out as a sum. The order is the reader's:
/// the account their own part of the money moved on comes first, so a shared purchase is named by
/// the side of it that is theirs.
export function transactionAccounts(
	transactions: {
		calculatedAmount: number;
		effectiveAmount: number;
		account: {
			name: string;
			color: string | null;
			owner: { name: string; firstName: string | null };
		};
	}[],
	user_name: string | undefined
): Dot[] {
	const moved = new Map<
		string,
		{ name: string; color: string | null; weight: number; mine: number }
	>();
	for (const transaction of transactions) {
		const name = formatAccount(transaction.account, user_name);
		const entry = moved.get(name) ?? {
			name,
			color: transaction.account.color,
			weight: 0,
			mine: 0
		};
		entry.weight += Math.abs(transaction.calculatedAmount);
		entry.mine += Math.abs(transaction.effectiveAmount);
		moved.set(name, entry);
	}

	return shares([...moved.values()].toSorted((a, b) => b.mine - a.mine || b.weight - a.weight));
}

/// The categories an expense is filed under, the heaviest first, a repeated one added up.
export function weightedCategories(
	categories: { weight: number; category: { name: string; color: string | null } }[]
): Dot[] {
	const filed = new Map<string, { name: string; color: string | null; weight: number }>();
	for (const { weight, category } of categories) {
		const entry = filed.get(category.name) ?? {
			name: category.name,
			color: category.color,
			weight: 0
		};
		entry.weight += weight;
		filed.set(category.name, entry);
	}

	return shares([...filed.values()].toSorted((a, b) => b.weight - a.weight));
}

export interface SortableAccount {
	readonly name: string;
	readonly owner: {
		readonly name: string;
	};
	readonly closed: Date | null;
}

export function accountSorter(a: SortableAccount, b: SortableAccount, user_name: string) {
	const aBelongsToMe = a.owner.name == user_name;
	const bBelongsToMe = b.owner.name == user_name;

	if (aBelongsToMe && !bBelongsToMe) return -1;
	if (!aBelongsToMe && bBelongsToMe) return 1;

	if (a.owner.name != b.owner.name) {
		return a.owner.name.localeCompare(b.owner.name);
	}

	if (a.closed && !b.closed) return 1;
	if (!a.closed && b.closed) return -1;

	return a.name.localeCompare(b.name);
}

export interface SortableCategory {
	readonly name: string;
}

export function categorySorter(a: SortableCategory, b: SortableCategory) {
	return a.name.localeCompare(b.name);
}

/// How many entries a picker puts up top before it goes back to alphabetical.
///
/// Five, because five is what the tail looks like: the top accounts here were used 600, 450,
/// 400, 250 and 50 times in a year, and then it falls off a cliff to twelve. Sorting a whole
/// list by frequency instead would put the accounts you touch twice a year in an order nobody
/// can predict, and alphabetical is faster once you already know the name you are looking for.
export const FREQUENT = 5;

/// The few things you actually use first, everything else in whatever order it had.
export function byUsageThen<T extends { usageCount: number }>(
	options: T[],
	rest: (a: T, b: T) => number
): T[] {
	const frequent = options
		.filter((option) => option.usageCount > 0)
		.toSorted((a, b) => b.usageCount - a.usageCount)
		.slice(0, FREQUENT);

	return [...frequent, ...options.filter((option) => !frequent.includes(option)).toSorted(rest)];
}

/// What the bank writes where the free text should be when a card was used: that a card was
/// used, and on which day. It says neither what was bought nor from whom, and it is all a
/// column three hundred pixels wide ever got to show for a card payment.
///
/// Only the empty ones. The older line also carries a cash withdrawal or an exchange rate on
/// occasion, and that is a fact about the payment rather than about the card.
const CARD_ONLY = [
	/^(VISA )?(Debit|Giro)kartenumsatz( vom \d{2}\.\d{2}\.\d{4})?$/i,
	/^\d{4}-\d{2}-\d{2}T[\d.]+\s*(Debitk|Karte)[\d.]*\s+\d{4}-\d{2}( Zahl\.System VISA Debit)?$/
];

/// One line of a statement worth reading.
///
/// A statement is a free text and then the block of `key: value` fields the bank fills in.
/// The free text is the half a human wrote for a human, so that is the line -- except after
/// a card payment, where it is boilerplate and the shop's name is down in `applicant_name`.
/// Everything cut here is a scroll away in the expanded row, and in the cell's own tooltip.
export const statementLine = (statement: string) => {
	const lines = statement.split('\n');
	const field = lines.findIndex((l) => /^[a-z_]+:\s/.test(l));
	const meta = field === -1 ? [] : lines.slice(field);
	const free = (field === -1 ? lines : lines.slice(0, field)).join(' ').replace(/\s+/g, ' ').trim();

	if (free && !CARD_ONLY.some((r) => r.test(free))) return free;
	const shop = meta
		.find((l) => l.startsWith('applicant_name:'))
		?.slice(15)
		.trim();
	return shop || free;
};
