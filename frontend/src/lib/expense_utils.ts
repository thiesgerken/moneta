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
