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

export const calendarExpenses = Array.from({ length: 46 }, (_, i) => ({
	id: `e${i}`,
	day: ((i * 5) % 30) + 1,
	title: ['Lebensmittel', 'Bäcker', 'Apotheke', 'Tanken', 'Miete', 'Strom'][i % 6],
	amount: -[1080, 180, 500, 6500, 95000, 3790][i % 6],
	categoryId: categories[i % categories.length].id,
	color: categories[i % categories.length].color
}));
