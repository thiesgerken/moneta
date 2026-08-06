import { graphql, type AnalysisOverviews$result } from '$houdini';

/// The Monate and Kategorien tabs are two readings of the same query, so they share it: the
/// bars come from the monthly totals and the treemap from the whole-period breakdown.
export const overviewsStore = graphql(`
	query AnalysisOverviews($from: DateTime!, $to: DateTime!) {
		analysis {
			overviews(from: $from, to: $to) {
				months {
					month
					startTotal
					endTotal
					integratedEndTotal
					income
					expenses
					expenseCount
					byCategory {
						category {
							id
							name
							color
						}
						income
						expenses
					}
				}
				byCategory {
					category {
						id
						name
						color
					}
					income
					expenses
					restIncome
					restExpenses
					biggest {
						expenseId
						title
						amount
					}
				}
			}
		}
	}
`);

export type OverviewsData = AnalysisOverviews$result['analysis']['overviews'];

/// How far the integrated end balance drifts from the one the recorded balances give.
///
/// A large gap does not mean the arithmetic drifted — it means an expense is missing, which
/// is the whole point of showing it. MoneyDB called it the Integrationsfehler.
export const integrationError = (o: { endTotal: number; integratedEndTotal: number }) =>
	Math.abs(o.endTotal - o.integratedEndTotal);

/// The error is a caveat on the row, not its headline: quiet until it is big enough to mean a
/// missing expense, which is the only time it asks for anything.
export const errorClass = (e: number) =>
	e >= 5000
		? 'text-red-600 dark:text-red-400'
		: e >= 2500
			? 'text-yellow-600 dark:text-yellow-400'
			: 'text-gray-400 dark:text-gray-500';

/// What a month or an account came to, which is what a reader is here for.
export const deltaColor = (cents: number) => (cents < 0 ? 'red' : cents > 0 ? 'green' : 'gray');

export const monthLabel = (d: Date) =>
	d.toLocaleDateString('de-DE', { month: 'short', year: '2-digit' });

/// How many labels a month axis can carry before they turn into a smear.
///
/// A category axis draws one label per bar unless told otherwise, which at ten years is 120
/// of them. `tickAmount` thins them to every nth; apex picks which ones, and the first and
/// last always survive.
export const monthTicks = (count: number) => Math.min(count, 12);

/// A colour for a category that has none set, and for the "no category" bucket. Derived from
/// the name so it at least stays the same between renders.
export function categoryColor(category: { name: string; color?: string | null } | null): string {
	if (category?.color) return category.color;
	if (!category) return '#94a3b8';

	const palette = ['#c0392b', '#d68910', '#16a085', '#8e44ad', '#2980b9', '#7f8c8d', '#c0662b'];
	let hash = 0;
	for (const ch of category.name) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
	return palette[Math.abs(hash) % palette.length];
}
