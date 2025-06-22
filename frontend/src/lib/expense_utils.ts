export function formatBookingPeriod(begin: Date, end: Date, time: boolean = false) {
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

export function formatAccounts(
	transactions: {
		effectiveAmount: number;
		account: { name: string; owner: { name: string; firstName: string | null } };
	}[],
	user_name: string | undefined
) {
	const accountSums = transactions.reduce(
		(sums, transaction) => {
			const accountName =
				user_name == transaction.account.owner.name
					? transaction.account.name
					: `${transaction.account.owner.firstName || transaction.account.owner.name} / ${transaction.account.name}`;
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

export function makeStripedBackground(segments: { color: string; percentage: number }[]) {
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

	return s + ');';
}
