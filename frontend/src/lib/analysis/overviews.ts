import { graphql, type AnalysisOverviews$result } from '$houdini';
import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import type { Granularity } from './state.svelte';

/// Everything Verlauf draws, in one query: the bars and the table come from the monthly
/// totals, and the treemap from the whole-period breakdown.
export const overviewsStore = graphql(`
	query AnalysisOverviews(
		$from: DateTime!
		$to: DateTime!
		$accrued: Boolean
		$granularity: Granularity
	) {
		analysis {
			overviews(from: $from, to: $to, accrued: $accrued, granularity: $granularity) {
				periods {
					start
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

/// Chart events that open what a bar is made of: a year into its months, a month into its own
/// page.
///
/// The month pages are the least discoverable thing in the app: the only way in was the month
/// name in the overview table, which is a link that does not look like one until you are on
/// top of it. A bar and its label are the obvious thing to click on a chart of periods, and a
/// year answers the same gesture one level up rather than jumping past its months.
///
/// `labelIndex` is the category index even where `tickAmount` has thinned the labels — apex
/// blanks the text of the ones it drops rather than skipping them, so the count still lines
/// up with the periods.
export function periodDrilldown(
	periods: { start: Date }[],
	granularity: Granularity,
	intoYear: (start: Date) => void
) {
	const open = (index: number) => {
		const start = periods[index]?.start;
		if (!start) return;

		if (granularity === 'YEAR') {
			intoYear(start);
			return;
		}

		void goto(
			resolve('/analysis/[year]/[month]', {
				// the backend dates a period at midnight UTC; reading it locally can land in the
				// month before
				year: `${start.getUTCFullYear()}`,
				month: `${start.getUTCMonth() + 1}`
			})
		);
	};

	// `unknown[]` rather than the real signatures: apex's own types declare the third argument
	// as one opaque options object for every event, and neither index is on it
	const index = (args: unknown[], key: 'dataPointIndex' | 'labelIndex') =>
		(args[2] as Record<string, number> | undefined)?.[key] ?? -1;

	return {
		dataPointSelection: (...args: unknown[]) => open(index(args, 'dataPointIndex')),
		xAxisLabelClick: (...args: unknown[]) => open(index(args, 'labelIndex'))
	};
}
