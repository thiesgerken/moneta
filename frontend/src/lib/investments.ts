/// The arithmetic behind the market-value overlay.
///
/// A plain module rather than `.svelte.ts` for the same reason `analysis/dates.ts` is one: a
/// `new Date` inside a rune module trips `svelte/prefer-svelte-reactivity`, and these are
/// transient values that nothing holds in reactive state.

export type Investment = {
	readonly account: { readonly id: string };
	readonly value: number | null;
	readonly asOf: Date | null;
};

/// What the linked depots add to a booked total, in cents.
///
/// Per depot, the market value minus the balance that has actually been booked for it -- the
/// gain nobody has committed yet, which is the figure the year-end decision is about. A depot
/// stockdb could not price contributes nothing rather than dropping out, so the total it is
/// added to stays a total of everything.
export function uncommittedGain(
	investments: readonly Investment[],
	committed: Map<string, number>
): number {
	return investments.reduce(
		(sum, i) => (i.value === null ? sum : sum + (i.value - (committed.get(i.account.id) ?? 0))),
		0
	);
}

/// The oldest price behind the figures, or null when nothing could be priced. The pessimistic
/// end on purpose: stockdb will use a price up to four days old, and a total is only as fresh
/// as its stalest part.
export function asOf(investments: readonly Investment[]): Date | null {
	const dates = investments.filter((i) => i.asOf !== null).map((i) => i.asOf!.getTime());
	return dates.length ? new Date(Math.min(...dates)) : null;
}

/// Whether anything is worth showing: a linked depot that stockdb could actually price.
export function priced(investments: readonly Investment[]): boolean {
	return investments.some((i) => i.value !== null);
}
