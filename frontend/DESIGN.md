# Design guidelines

What the UI looks like and why, so that the next screen added matches the ones already there.
Every rule here is one the app already follows; where it does not, that is a bug in the screen
rather than in this file.

The base is Tailwind 4 with flowbite-svelte. Both bring opinions of their own, and a few of
them are overridden below — those overrides live in `src/app.css` and are the only global CSS
in the project.

## Typography

Two typefaces, both self-hosted through `@fontsource`, both variable, no CDN.

|                    |                                                                  |                  |
| ------------------ | ---------------------------------------------------------------- | ---------------- |
| **Inter**          | everything that is read: body text, labels, tables, every number | `--font-sans`    |
| **Source Serif 4** | `h1`, `h2`, `h3` — page titles and card headings                 | `--font-display` |

Shipping them rather than asking the system for a font is the point: the old stack resolved to
SF Pro on macOS and to whatever fontconfig picked on Linux, so two people reading the same
figures were not looking at the same page.

**Numbers are Inter, always** — including inside a heading. Inter has a tabular set, which is
why `body` carries `font-variant-numeric: tabular-nums`: a column of amounts lines up without
resorting to a monospace font, and `1`/`l`/`I` stay apart, which matters next to an IBAN.

**Nothing shouts.** flowbite sets table headers in 12 px ALL CAPS over 500-weight body rows,
which leaves the labels shouting and nothing in a row standing out from anything else. Both
are undone in `app.css`, and the rules sit **outside every `@layer`** — an unlayered rule is
what outranks a utility class without `!important` on forty elements.

Weights: 400 for body and table rows, 500 (`font-medium`) for the one thing in a row that
identifies it, 600 for headings. Anything heavier is for a number that is the answer to the
page's question, like the wealth figure on the dashboard.

## Colour

Colour carries meaning here; it is not decoration.

- **Primary** is the moneta orange (`--color-primary-*`), used for links, primary buttons and
  nothing else. It is also the Stichtag marker, which is why the wealth curve is _not_ orange.
- **Money direction**: green for what came in, red for what went out — `text-green-700
dark:text-green-400` and `text-red-600 dark:text-red-400`. `deltaColor()` in
  `analysis/overviews.ts` picks the badge colour by sign.
- **Wealth** is `WEALTH_COLOR` (`#059669`, emerald) in `analysis/cards.ts`. A curve of what you
  own reads as a warning in orange.
- **Categories and accounts** carry their own colour from the database. `categoryColor()` falls
  back to a fixed palette hashed from the name, so a category without one at least keeps the
  same colour between renders.
- **Availability** has no colour of its own, so it gets a warm-to-cool ramp along the liquidity
  order — see `analysis/availability.ts`.

Every screen has a dark variant. There is no theme toggle: `dark:` follows the system.

## Dates and money

- Dates are `dd.mm.yyyy` through `shortDate()` in `analysis/dates.ts`. `de-DE` on its own gives
  `8.8.2026`, which reads worse in a column because the day, month and year each start
  somewhere different from row to row.
- Month labels are their own thing and are left long: `August 2026`, or `Aug 26` on a chart
  axis (`monthLabel()`).
- Money is formatted with `toLocaleString('de-DE', { style: 'currency', currency: 'EUR' })`.
  Whole euros only where the figure is a headline rather than a number to check.
- Money is integer cents everywhere it travels; only the formatter divides by 100.

## Language

German throughout, informal (`du`, not `Sie`). Labels say what the reader would say out loud:
the feed filters are _Alle · Von mir · Von anderen_ because the entries below them read
"geändert von dir" and "angelegt von Krösus".

Prefer a sentence that says something over a label that names a field: "3 Ausgaben brauchen
Prüfung / älteste: Rewe vom 12.03.2026" beats "Ungeprüft: 3".

Code, comments and commit messages are English. Only what a reader sees is German.

## Layout

- Page width is capped at `max-w-7xl`; the feed on its own is `max-w-3xl`, since a column of
  prose that wide is already too wide.
- Page gutters are `px-4 sm:px-8`, and content is stacked with `space-y-4`.
- Cards use `ANALYSIS_CARD` (`analysis/cards.ts`), which deliberately draws **no frame on a
  phone**: the tab panel is already a box with a gutter, and a card inside it spends forty
  pixels of a narrow screen saying "this is a thing".
- Two columns from `lg` where there is a second thing worth seeing at once. A column whose
  content is unbounded — the feed — is absolutely positioned inside a relative cell, so the row
  is sized by the other column and the long one scrolls.
- A phone drops columns rather than shrinking them. What gets dropped is what answers a
  question nobody asks on a phone: see the overview table, which keeps month, delta, income,
  spending and the closing balance and hides the rest.

## Tables

- One shared look through `table_styles.ts`; the body is `text-sm`.
- Sorting and filtering are **server-side**, through typed arguments. The header cell that
  sorts carries the arrow.
- A filtered list says so: `ExpenseTable` renders a "Gefiltert nach …" line with one removable
  chip per active filter. A list that arrives pre-filtered from somewhere else must never look
  like the whole list, only shorter.
- Amounts are right-aligned and tabular; identifying text truncates rather than wraps.

## Charts

apexcharts, wrapped in `Chart.svelte`, which owns the create-or-update decision — never
instantiate apex directly.

- Series colours come from the data (account, category, availability), not from a chart
  palette. Where there is one series it is the app's own colour for that quantity.
- Bars grow **upwards**. Spending is stored negative; which side of zero a figure is on is what
  the toggle above the chart says, so the bars are free to grow the way bars do.
- Every tooltip formats money through the same `euro()` as the rest of the page. Apex has three
  separate formatters for a donut (tooltip, centre total, centre value on hover) and they all
  need setting.
- Charts are a way in, not a dead end: a bar and its axis label open that month
  (`monthDrilldown()`), a treemap tile opens the expenses behind it. Anything that stands for a
  set of expenses should be able to show them.
- Point counts are chosen for the viewport — around 120 across a screen, 40 on a phone — and
  the backend samples to match.

## Interaction

- Anything clickable looks it: a pointer cursor, and a hover state that changes something.
- Deep links carry state in the URL: `?tab=` on the analysis tabs, `?kategorie=&von=&bis=` and
  `?pruefen` / `?neu` on the expense list. URL parameters are read **once** on mount — after
  that the controls belong to the reader and must not be reset from the address bar.
- State that should survive a tab switch lives in `analysis/state.svelte.ts`, not in the
  component.
- Empty states say which emptiness it is: "Niemand sonst hat etwas geändert" and "In diesem
  Zeitraum ist nichts passiert" are different sentences for a reason.

## Tried and rejected

- **Small caps for table headers.** Source Serif's Google build has no `smcp`, so the browser
  synthesises them by scaling the capitals down and they read thin and light. Real ones would
  mean vendoring Adobe's own build of the same typeface, and the look was not wanted anyway.
  Headers stay 14 px mixed case in Inter.
