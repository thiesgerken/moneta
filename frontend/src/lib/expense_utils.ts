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

export function formatTransactionAccounts(
	transactions: {
		effectiveAmount: number;
		account: { name: string; owner: { name: string; firstName: string | null } };
	}[],
	user_name: string | undefined
) {
	const accountSums = transactions.reduce(
		(sums, transaction) => {
			const accountName = formatAccount(transaction.account, user_name);
			sums[accountName] = (sums[accountName] || 0) + transaction.effectiveAmount;
			return sums;
		},
		{} as Record<string, number>
	);

	return Object.entries(accountSums)
		.sort(([, a], [, b]) => Math.abs(b) - Math.abs(a))
		.map(([account]) => account)
		.join(', ');
}

export function formatCategories(categories: { weight: number; category: { name: string } }[]) {
	const categorySums = categories.reduce(
		(sums, category) => {
			sums[category.category.name] = (sums[category.category.name] || 0) + category.weight;
			return sums;
		},
		{} as Record<string, number>
	);

	return Object.entries(categorySums)
		.sort(([, a], [, b]) => b - a)
		.map(([category]) => category)
		.join(', ');
}

export function makeAccountColor(
	transactions: {
		amount: number | null;
		fraction: number | null;
		account: { color: string | null };
	}[],
	referenceAmount: number | null
) {
	const colorWeights = transactions.reduce(
		(sums, transaction) => {
			if (transaction.account.color) {
				const amount = transaction.amount || (transaction.fraction || 0) * (referenceAmount || 0);

				if (amount)
					sums[transaction.account.color] =
						(sums[transaction.account.color] || 0) + Math.abs(amount);
			}
			return sums;
		},
		{} as Record<string, number>
	);

	const totalWeight = Object.values(colorWeights).reduce((sum, weight) => sum + weight, 0);
	const segments = Object.entries(colorWeights).map(([color, weight]) => ({
		color,
		percentage: weight / totalWeight
	}));
	return makeStripedBackground(segments);
}

export function makeCategoryColor(
	categories: { weight: number; category: { color: string | null } }[]
) {
	const colorWeights = categories.reduce(
		(sums, category) => {
			if (category.category.color && category.weight) {
				sums[category.category.color] = (sums[category.category.color] || 0) + category.weight;
			}
			return sums;
		},
		{} as Record<string, number>
	);

	const totalWeight = categories.reduce((sum, category) => sum + category.weight, 0);
	const segments = Object.entries(colorWeights).map(([color, weight]) => ({
		color,
		percentage: weight / totalWeight
	}));
	return makeStripedBackground(segments);
}

/// One sRGB channel, undone from the display gamma, as the luminance formula wants it.
function linear(channel: number): number {
	const c = channel / 255;
	return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

/// Text that can be read on `color`.
///
/// Account and category colours come out of a picker, so nothing stops one being near-black --
/// and the cells tinted with them wrote their name in black regardless, which on a dark purple
/// was a black bar with no text in it.
///
/// The threshold is 0.179 rather than a midpoint: white text and black text have equal contrast
/// against a background of exactly that luminance, so it is the point where the better of the
/// two changes over. Half-way would call the mid greens and purples this palette is full of
/// light and leave them barely legible.
export function readableTextOn(color: string): string {
	const hex = color.replace('#', '');
	const full =
		hex.length < 6
			? hex
					.slice(0, 3)
					.split('')
					.map((c) => c + c)
					.join('')
			: hex.slice(0, 6);

	const [r, g, b] = [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16));
	// an unparseable colour is nobody's idea of dark; leave the page's own text colour alone
	if ([r, g, b].some(Number.isNaN)) return 'inherit';

	const luminance = 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
	return luminance > 0.179 ? '#000' : '#fff';
}

/// The whole inline style for something drawn in an account's or a category's own colour.
///
/// Text colour belongs with the tint rather than with the element: every caller needs both, and
/// the ones that hardcoded `text-black` are exactly the ones that went unreadable.
export function tint(color: string | null | undefined): string | undefined {
	return color ? `background-color: ${color}; color: ${readableTextOn(color)};` : undefined;
}

export function makeStripedBackground(segments: { color: string; percentage: number }[]) {
	// An empty list used to produce `linear-gradient(to right, )`, which is not a gradient and
	// which the browser therefore dropped along with the rest of the declaration.
	if (segments.length === 0) return undefined;

	segments.sort((a, b) => b.percentage - a.percentage);
	let s = 'background: linear-gradient(to right, ';

	let sum = 0;
	for (let i = 0; i < segments.length; i++) {
		if (i > 0) {
			s += ', ';
		}

		s += `${segments[i].color} ${sum * 100}% ${(sum + segments[i].percentage) * 100}%`;
		sum += segments[i].percentage;
	}

	// One piece of text over several stripes, so it follows the widest one -- which is the one
	// most of the text sits on, and for the single-colour case is the only one there is.
	return `${s});color: ${readableTextOn(segments[0].color)};`;
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
