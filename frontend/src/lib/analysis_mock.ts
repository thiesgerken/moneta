// Mock data for the analysis dashboard, so the layout can be reviewed before the
// backend endpoints exist. Delete this file once /analysis reads real queries.

export const accounts = [
	{ id: 'a1', name: 'Commerzbank Giro', color: '#6cbf84', availability: 'IMMEDIATELY' },
	{ id: 'a2', name: 'Bargeld', color: '#e8ce6a', availability: 'IMMEDIATELY' },
	{ id: 'a3', name: 'Commerzbank Visa', color: '#65aec2', availability: 'IMMEDIATELY' },
	{ id: 'a4', name: 'Beispielbank Tagesgeld', color: '#3cb521', availability: 'DAYS' },
	{ id: 'a5', name: 'Depot', color: '#8e6cbf', availability: 'WEEKS' },
	{ id: 'a6', name: 'Riester', color: '#bf6c8e', availability: 'DECADES' }
];

export const categories = [
	{ id: 'c1', name: 'Essen', color: '#c0392b' },
	{ id: 'c2', name: 'Nebenkosten', color: '#d68910' },
	{ id: 'c3', name: 'Gesundheit', color: '#16a085' },
	{ id: 'c4', name: 'Freizeit', color: '#8e44ad' },
	{ id: 'c5', name: 'Haushalt', color: '#2980b9' },
	{ id: 'c6', name: 'Spenden', color: '#c0392b' }
];

const MONTHS = 24;

// deterministic pseudo-noise: mock data must not reshuffle on every render
const wobble = (i: number, seed: number) =>
	Math.sin(i * 1.7 + seed) * 0.5 + Math.sin(i * 0.6 + seed);

export const historyDates = Array.from({ length: MONTHS }, (_, i) => {
	const d = new Date(2024, 7 + i, 1);
	return d.getTime();
});

export const historyByAccount = accounts.map((a, ai) => ({
	...a,
	amounts: historyDates.map((_, i) => {
		const base = [14000, 300, -400, 9000, 21000, 12000][ai];
		const growth = [60, 0, 0, 120, 260, 90][ai];
		return Math.round(base + growth * i + wobble(i, ai) * base * 0.03);
	})
}));

export const historyTotal = historyDates.map((_, i) =>
	historyByAccount.reduce((sum, a) => sum + a.amounts[i], 0)
);

export const AVAILABILITY_LABELS: Record<string, string> = {
	IMMEDIATELY: 'Sofort',
	DAYS: 'Tage',
	WEEKS: 'Wochen',
	MONTHS: 'Monate',
	YEARS: 'Jahre',
	DECADES: 'Jahrzehnte'
};

// the availability order the backend enum declares; a total for "verfügbar in Tagen"
// includes everything more liquid than that, so the order has to be preserved
const AVAILABILITY_ORDER = ['IMMEDIATELY', 'DAYS', 'WEEKS', 'MONTHS', 'YEARS', 'DECADES'];

export const historyByAvailability = AVAILABILITY_ORDER.filter((av) =>
	accounts.some((a) => a.availability === av)
).map((av) => ({
	availability: av,
	label: AVAILABILITY_LABELS[av],
	amounts: historyDates.map((_, i) =>
		historyByAccount.filter((a) => a.availability === av).reduce((sum, a) => sum + a.amounts[i], 0)
	)
}));

export const availabilityTotals = [
	{ availability: 'IMMEDIATELY', label: 'Sofort', amount: 1889012 },
	{ availability: 'DAYS', label: 'Tage', amount: 1102400 },
	{ availability: 'WEEKS', label: 'Wochen', amount: 2698000 },
	{ availability: 'DECADES', label: 'Jahrzehnte', amount: 1421000 }
];

export const computedBalances = accounts.map((a, i) => ({
	...a,
	amount: historyByAccount[i].amounts[MONTHS - 1] * 100,
	referenceDate: new Date(2026, 6, 12 - i * 3),
	referenceAmount: historyByAccount[i].amounts[MONTHS - 1] * 100 - 4200 * (i + 1),
	expenseCount: [37, 4, 12, 1, 0, 0][i]
}));

export const overviews = Array.from({ length: 12 }, (_, i) => {
	const month = new Date(2025, 7 + i, 1);
	const income = 480000 + Math.round(wobble(i, 3) * 40000);
	const expenses = -(350000 + Math.round(wobble(i, 5) * 60000));
	const start = 5600000 + i * 130000;
	const end = start + income + expenses;
	return {
		month,
		startTotal: start,
		endTotal: end,
		// the measured end balance drifts from the integrated one when an expense is
		// missing; MoneyDB called this the Integrationsfehler and it is the point of the view
		endTotalApprox: end + Math.round(wobble(i, 9) * 3000),
		income,
		expenses,
		expenseCount: 60 + Math.round(Math.abs(wobble(i, 7)) * 30)
	};
});

export const monthlyByCategory = categories.map((c, ci) => ({
	...c,
	amounts: overviews.map((_, i) => Math.round(40000 + wobble(i, ci * 2) * 25000 + ci * 6000))
}));

export const categoryTree = categories.map((c, ci) => ({
	...c,
	amount: monthlyByCategory[ci].amounts.reduce((a, b) => a + b, 0),
	children: [
		{ title: 'Wocheneinkauf', amount: 180000 - ci * 12000 },
		{ title: 'Restaurant', amount: 90000 - ci * 7000 },
		{ title: 'Bäcker', amount: 41000 - ci * 3000 },
		{ title: `Rest (+${11 + ci})`, amount: 62000 - ci * 4000 }
	]
}));

// A month page shows whatever month the URL names, including ones outside the mocked
// range, so this is a function of (year, month) rather than a lookup into `overviews`.
export function monthDetail(year: number, month: number) {
	const seed = year * 12 + month;
	const income = 480000 + Math.round(wobble(seed, 3) * 40000);
	const expenses = -(350000 + Math.round(wobble(seed, 5) * 60000));
	const startTotal = 5600000 + seed * 11000;
	const endTotal = startTotal + income + expenses;

	return {
		date: new Date(year, month - 1, 1),
		income,
		expenses,
		startTotal,
		endTotal,
		endTotalApprox: endTotal + Math.round(wobble(seed, 9) * 3000),
		expenseCount: 60 + Math.round(Math.abs(wobble(seed, 7)) * 30),
		byAccount: accounts.map((a, ai) => {
			const start = [1400000, 30000, -40000, 900000, 2100000, 1200000][ai] + seed * 900;
			const delta = Math.round(wobble(seed, ai) * 90000);
			return {
				...a,
				start,
				end: start + delta,
				endApprox: start + delta + Math.round(wobble(seed, ai + 20) * 1200),
				delta,
				expenseCount: [37, 4, 12, 1, 0, 0][ai]
			};
		}),
		byCategory: categories.map((c, ci) => ({
			...c,
			amount: Math.round(40000 + wobble(seed, ci * 2) * 25000 + ci * 6000),
			children: [
				{ title: 'Wocheneinkauf', amount: 18000 - ci * 1200 },
				{ title: 'Restaurant', amount: 9000 - ci * 700 },
				{ title: 'Bäcker', amount: 4100 - ci * 300 },
				{ title: `Rest (+${4 + ci})`, amount: 6200 - ci * 400 }
			]
		})),
		biggest: [
			{ id: 'b1', title: 'Miete', amount: -95000, categoryId: 'c2' },
			{ id: 'b2', title: 'Jahresbeitrag Versicherung', amount: -48000, categoryId: 'c2' },
			{ id: 'b3', title: 'Wocheneinkauf', amount: -12450, categoryId: 'c1' },
			{ id: 'b4', title: 'Tanken', amount: -6500, categoryId: 'c4' },
			{ id: 'b5', title: 'Strom', amount: -3790, categoryId: 'c2' }
		]
	};
}

export const calendarExpenses = Array.from({ length: 46 }, (_, i) => ({
	id: `e${i}`,
	day: ((i * 5) % 30) + 1,
	title: ['Lebensmittel', 'Bäcker', 'Apotheke', 'Tanken', 'Miete', 'Strom'][i % 6],
	amount: -[1080, 180, 500, 6500, 95000, 3790][i % 6],
	categoryId: categories[i % categories.length].id,
	color: categories[i % categories.length].color
}));
