/// How the figures of a period are read: what a number means, and what colour says so.
///
/// Apart from `overviews.ts` because none of it needs the query or the router, and that module
/// cannot be loaded without them — which left this arithmetic untestable for want of a database
/// and a browser it never touches.

import type { Granularity } from './state.svelte';

/// How far the integrated end balance drifts from the one the recorded balances give.
///
/// A large gap does not mean the arithmetic drifted — it means an expense is missing, which
/// is the whole point of showing it.
///
/// Null on the accrual axis, where the backend has nothing to integrate: a share of a
/// subscription is a cost the month carried rather than money that left an account.
export const integrationError = (o: { endTotal: number; integratedEndTotal: number | null }) =>
	o.integratedEndTotal === null ? null : Math.abs(o.endTotal - o.integratedEndTotal);

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

/// What one bar of an overview is called. A year needs all four digits: `26` under a bar a
/// year wide reads as a day of the month.
export const periodLabel = (d: Date, granularity: Granularity) =>
	granularity === 'YEAR'
		? d.toLocaleDateString('de-DE', { year: 'numeric', timeZone: 'UTC' })
		: d.toLocaleDateString('de-DE', { month: 'short', year: '2-digit', timeZone: 'UTC' });

/// How many labels the axis can carry before they turn into a smear.
///
/// A category axis draws one label per bar unless told otherwise, which at ten years of
/// months is 120 of them. `tickAmount` thins them to every nth; apex picks which ones, and
/// the first and last always survive.
export const periodTicks = (count: number) => Math.min(count, 12);

/// The value cell apex writes for a sunburst slice: a bare number, then the two percentages.
///
/// It is a bare number because apexcharts 6.10.0 interpolates `e.value` into the markup there
/// -- see `_showTooltip` -- where the treemap's branch calls `_formatValue` and honours
/// `tooltip.y.formatter`. A sum of euros arrives as `1234.5600000000001`.
const RAW_SUNBURST_VALUE = /^(-?\d+(?:\.\d+)?)( \(.*)$/;

/// That cell rewritten as money, or null where there is nothing to rewrite -- which is both
/// "this is some other tooltip" and "this one has been done already", and is what keeps a
/// rewrite from setting off another.
export function asMoney(text: string, money: (cents: number) => string): string | null {
	const found = RAW_SUNBURST_VALUE.exec(text.trim());
	return found ? `${money(Number(found[1]) * 100)}${found[2]}` : null;
}

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
