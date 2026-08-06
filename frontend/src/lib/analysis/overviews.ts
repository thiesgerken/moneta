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

export const errorColor = (e: number) => (e >= 5000 ? 'red' : e >= 2500 ? 'yellow' : 'green');

export const monthLabel = (d: Date) =>
	d.toLocaleDateString('de-DE', { month: 'short', year: '2-digit' });

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
