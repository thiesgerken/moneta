/// Whether the wealth figures are also shown at today's market prices.
///
/// A balance here is a reading taken on purpose, and for a depot that happens about once a
/// year with a conservative haircut on the gains. That is deliberate: the monthly analysis is
/// about spending, and a stock market moving those figures around would ruin it as an answer.
/// So the market value is only ever offered beside a *stock of wealth right now* -- the start
/// page and the Vermögen tab -- and never anywhere monthly.

import { browser } from '$app/env';

const KEY = 'moneta.market-values';

/// Off by default, remembered per browser. Not in the url: it is a way of looking at the app
/// rather than a thing to link somebody to.
export const marketState = $state({ show: browser && localStorage.getItem(KEY) === 'true' });

export function showMarket(show: boolean) {
	marketState.show = show;
	if (browser) localStorage.setItem(KEY, String(show));
}
