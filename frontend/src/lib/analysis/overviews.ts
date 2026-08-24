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

/// The page a period has of its own. A year is the same page over a wider window, with the
/// months it is made of listed on it, so it is a route rather than a zoom back into the chart.
export const periodHref = (start: Date, granularity: Granularity) =>
	resolve('/analysis/[year]/[[month]]', {
		// the backend dates a period at midnight UTC; reading it locally can land in the year
		// before
		year: `${start.getUTCFullYear()}`,
		month: granularity === 'YEAR' ? undefined : `${start.getUTCMonth() + 1}`
	});

/// Where a bar goes when it is clicked: the period's own page.
///
/// Those pages are the least discoverable thing in the app: the only way in was the period's
/// name in the overview table, which is a link that does not look like one until you are on
/// top of it. A bar and its label are the obvious thing to click on a chart of periods.
///
/// `labelIndex` is the category index even where `tickAmount` has thinned the labels — apex
/// blanks the text of the ones it drops rather than skipping them, so the count still lines
/// up with the periods.
export function periodDrilldown(periods: { start: Date }[], granularity: Granularity) {
	const open = (index: number) => {
		const start = periods[index]?.start;
		if (!start) return;

		// the rule wants to see a literal resolve() in the call; `periodHref` is one, and
		// having it in two places is how the chart and the table would drift apart
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		void goto(periodHref(start, granularity));
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
