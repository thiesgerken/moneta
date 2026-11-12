# Design guidelines

What the UI looks like and why, so that the next screen added matches the ones already there.
Every rule here is one the app already follows; where it does not, that is a bug in the screen
rather than in this file.

The base is Tailwind 4 with flowbite-svelte. Both bring opinions of their own, and a few of
them are overridden below — those overrides live in `src/app.css` and are the only global CSS
in the project.

## Brand

`static/brand/` contains the canonical SVG artwork used by the README, login screen and shared
navbar. Open `static/brand/preview.html` for the light and dark treatments and the mark at
16, 24, 32 and 48 pixels. The favicon and installed-app icons are derived from `mark.svg`.

The mark is an open ledger: two facing pages share a spine without becoming the same page.
That is the household model in one shape. It deliberately avoids a coin, currency sign or
growth arrow, which would make a bookkeeping tool look like an investment product.

- `mark.svg`: the symbol on a 64 × 64 canvas, in Moneta orange (`#ef562f`).
- `wordmark.svg`: lowercase lettering, in ink (`#111827`).
- `lockup.svg`: the horizontal combination, with the same baseline and proportions.
- `lockup-dark.svg`: the same orange mark with off-white (`#f9fafb`) lettering for dark surfaces.
- `mark-512.png` and `lockup-1200.png`: transparent raster exports for consumers without SVG.

The lettering is Source Serif 4, weight 600, optical size 32, with optical pair spacing.
It is converted to paths, so an image needs no font download and cannot acquire a fallback
typeface. Source Serif 4 is SIL Open Font License 1.1; these assets are outlined artwork,
not a redistributed font.

Keep the canvas padding and aspect ratios intact. Use the mark alone below a 148-pixel-wide
lockup; its minimum intended canvas is 16 × 16 pixels. The navbar uses a 148 × 32 lockup and
login uses 222 × 48, including on phones. `Brand.svelte` shares the image markup and gives each
logo one accessible name. It selects `lockup-dark.svg` with a `prefers-color-scheme` picture
source and retains the light artwork as the image fallback.

The README uses repository-relative `lockup.svg#gh-light-mode-only` and
`lockup-dark.svg#gh-dark-mode-only` images, so no running application is needed. GitHub and
Gitea use these fragments to show only the logo matching the selected repository theme,
including a manual theme choice that differs from the operating system. Image fragments
also survive repository renderers wrapping images in links, unlike HTML picture sources.

On a dark surface, keep the orange mark and use the off-white lettering. Monochrome is an
optional treatment for contexts that cannot reproduce the brand colour, not the dark-mode
default. For an external image, `filter: brightness(0) invert(1)` makes both parts white; when
inlining the SVG, set both fills to `currentColor`. Do not add a surrounding coin, shadow or
gradient.

The 64-pixel PNG favicon is transparent. Installed-app icons use a solid off-white
(`#f9fafb`) canvas so the orange mark is legible independently of the launcher's background.
The maskable icon centers a 368-pixel mark canvas inside 512 pixels, keeping the complete
artwork within the central 80%-diameter safe circle. Do not scale it to fill the mask.
Existing icon URLs in `app.html` and `manifest.webmanifest` point to these derived assets;
the service worker populates a fresh static-asset cache for each build.

Regenerate derived artwork from the frontend directory (requires librsvg and ImageMagick):

```sh
cp static/brand/mark.svg static/favicon.svg
rsvg-convert --width 64 --height 64 --output static/favicon.png static/brand/mark.svg
rsvg-convert --width 192 --height 192 --background-color '#f9fafb' --output static/icon-192.png static/brand/mark.svg
rsvg-convert --width 512 --height 512 --background-color '#f9fafb' --output static/icon-512.png static/brand/mark.svg
rsvg-convert --width 368 --height 368 static/brand/mark.svg | magick png:- -background '#f9fafb' -gravity center -extent 512x512 -alpha remove -alpha off static/icon-maskable-512.png
rsvg-convert --width 512 --height 512 --output static/brand/mark-512.png static/brand/mark.svg
rsvg-convert --width 1200 --output static/brand/lockup-1200.png static/brand/lockup.svg
```

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

Colour carries meaning here; it is not decoration. A screen has **one accent and neutral
surfaces**: the orange, and greys around it. Everything that is coloured beyond that has to be
saying something a reader would otherwise have to read.

- **Primary** is the moneta orange (`--color-primary-*`), used for the brand, links and the one
  primary button on a page. It is also the Stichtag marker, which is why the wealth curve is _not_
  orange.
- **Money direction is text, not a surface**: green for what came in, red for what went out —
  `text-green-700 dark:text-green-400` and `text-red-600 dark:text-red-400` — and always with its
  sign (`signedEuro()`), so the colour is never the only signal. **Zero is grey**, not blank: an
  expense that nets to nothing moved money between two of your own accounts, which is a third
  thing rather than a missing one. `restingAmountColor()` in `money.ts` gives every amount cell
  that treatment; `deltaColor()` in `analysis/overviews.ts` picks the colour of a delta by sign.
  A filled cell or pill behind an amount is not used: a column of pink and green blocks is read
  as a pattern before any figure in it is.
- **Wealth** is `WEALTH_COLOR` (`#059669`, emerald) in `analysis/cards.ts`. A curve of what you
  own reads as a warning in orange.
- **Categories and accounts** carry their own colour from the database, shown as **a dot in front
  of the name** (`ColorDots.svelte`), never as the fill of a cell. The dot is enough to recognise an
  account at a glance; a filled cell made the account and category columns the loudest thing in a
  row whose point is its amount. Several accounts or categories are several dots, the one most
  money moved on (or the heaviest) first. `categoryColor()` falls back to a fixed palette hashed from
  the name, so a category without one at least keeps the same colour between renders. Charts are
  the exception: a series *is* its colour, so there the account and category colours fill bars and
  slices.
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

- Dashboard width is capped at `max-w-7xl`; the feed on its own is `max-w-3xl`, since a column of
  prose that wide is already too wide.
- Page gutters are `px-4 sm:px-8`, and content is stacked with `space-y-4`.
- List pages use the available viewport width through `TableSection.svelte`, with `px-4` inner
  gutters and `sm:p-5` section padding. Fixed table columns need the remaining space for dates,
  titles and bank text; do not apply the dashboard's width cap or larger desktop gutters here.
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

- **A list of things becomes a list on the phone; a table of numbers stays a table and drops
  columns.** The expense list below `sm` is a list of two-line items (title and amount, then date,
  account, category and state icons), because an expense is a thing to recognise and open, and
  three table columns at 390px left its title a dozen characters wide. The smaller lists (Konten,
  Kategorien, Regeln, Lieferungen) keep one column there with a second line carrying what the
  dropped columns said, and no header naming that one column. A table of figures — the monthly
  overview, Kontostände, Kontoauszug, the analysis tables — is read across, so it stays a table and
  each column says from which breakpoint it shows. **Nothing scrolls sideways on a phone**: a
  table scrolled inside its card hides its most interesting columns behind a gesture nobody
  expects.
- **Header icons from `md` up** (`headIcon` in `table_styles.ts`), and **minimum column widths
  desktop-only** (`md:min-w-*`). On a phone a column is as narrow as its label, and an icon or a
  minimum width there is what forces the sideways scroll the rule above avoids.
- One shared look through `table_styles.ts`; the body is `text-sm` over a 10 px vertical cell
  padding, which is a 40 px row. These are lists to scan, not paragraphs to read — but 8 px was
  past scanning and into cramped.
- **Dark text, faint grid, rows only.** The rows are what the reader came for and the ruling is
  scaffolding, so rows are `gray-900` and every rule is `gray-200`. flowbite has it the other way
  round — it sets the row text to `gray-500`, and `divide-y` in tailwind 4 defaults to
  `currentColor` — and both are fixed in `app.css`, on the `tr`, so that a cell with a colour of
  its own still wins. There are no rules between columns: aligned text and right-aligned figures
  are columns already. The header keeps a rule under it.
- **Hover and open are the same shade**, `gray-100`, set in `app.css`. There is no zebra
  striping: flowbite painted the even rows in `gray-50` and hovered in `gray-50` too, so half
  the table did not react to the pointer.
- **What an open row unfolds into stays on the card's surface** (`tr[data-detail]`). Its buttons
  are a step off that surface, and on the open row's shade they disappeared.
- The bank's own text is **monospace** wherever it is shown in full — reference numbers and
  IBANs are read character by character. It is a column of its own from `lg`.
- Sorting and filtering are **server-side**, through typed arguments. The header cell that
  sorts carries the arrow, and clicking it a third time drops back to the list's default sort —
  sorting lives in the component rather than in the url, so without that there was no way back
  to it but a reload. One column at a time; the API takes one `sortBy`.
- **Column widths are fixed**, not derived from the page being shown. `table-fixed` with a width
  on every header cell but the one or two that take what is left. Auto layout re-measured on
  every page turn, so the same list moved its columns as you paged through it. A fixed table
  also lays out as many columns as the _widest row_ declares, so a detail row's `colspan` has to
  match the number of columns the breakpoint is actually showing — nine over three invents six
  empty ones and shares the width between all nine. A table that hides its header on a phone
  takes `md:table-fixed`: without a header the detail row's `colspan` is the widest row.
  The expense date column reserves a full date range's width on desktop. Use a real width:
  fixed table layout ignores cell min-width. The description column yields space below `2xl`
  so dates and titles fit smaller desktops too.
- **A detail row is two columns from `lg`**: what is prose on the left as a `dl`, what is a table
  on the right. A row of the expense list is two thousand pixels wide and its detail used to be
  one narrow stack down the left of it. The small tables in it (`detailTable`) are ruled like the
  list around them and nothing else — no frame, no filled header, no surface of their own.
- **States, kinds and triggers are chips** (`chip` in `table_styles.ts`): a hairline and muted
  text with the state's icon. Five pastel fills in one column competed with the amounts and won.
- A filtered list says so: `ExpenseTable` renders a "Gefiltert nach …" line with one removable
  chip per active filter. A list that arrives pre-filtered from somewhere else must never look
  like the whole list, only shorter.
- Amounts are right-aligned and tabular; identifying text truncates rather than wraps.

## Forms and modals

- A modal is as wide as its content needs: `xl` for a form with tables in it, `lg` for a list,
  `xs` for a confirmation. `lg` on the expense form stacked a dozen fields down a column with
  the right half of the dialog empty.
- **The buttons live in the modal's footer**, which does not scroll — `Abbrechen` on the left of
  `Speichern`. A submit button in the footer sits outside the `<form>` in the body, which is
  what the `form="…"` attribute is for. A submit bar at the end of the body means scrolling past
  everything to save.
- A textarea's defaults are `cols=20` and `rows=2`, and both are wrong every time: **`w-full`
  and `field-sizing-content`**, with a `max-h-*` so a long statement cannot take the screen.
  Guessing a row count from whether the field is empty is what this replaces.
- **A table of form controls carries `data-form`**, which drops the row padding (the inputs
  bring their own height) and the hover shading. Everything else about it — the grid, the
  header, the widths — is the same as a table you read.
- The unit is the switch where a value can be one of two: the €/% button inside the amount field
  replaced a `Relativ` checkbox column that cost more width than the number it qualified.
- **What is empty on most records goes in a modal of its own**, reached from a line that says
  what is in it (`Positionen · 3 (42,17 €)`). A section heading plus an add-button for something
  almost never present is worth more than the thing itself.
- Only the button that completes the form is `primary`; five orange buttons in a form leave
  nothing to say which one submits it.
- **Neu is the only orange button on a page; every other action is one neutral group.**
  Duplizieren, Bearbeiten, Löschen, Einsortieren and Zusammenführen — in the toolbar and in the open
  row — share `actionGroup` / `actionButton` from `table_styles.ts`: a solid `gray-100` surface
  with `gray-900` text (`gray-700` / `gray-100` in dark, a step off the card), divided by hairlines.
  Löschen is the one with a colour, and only in its text and icon (`dangerText`): it is the one that
  cannot be undone. The thing a page is for is found without reading, and the rest do not compete
  with it or with the amounts. Inside a modal, the button that completes the form is `primary`
  and a destructive confirmation stays solid red: there it is the one decision on screen.
- **Disabled is 40% opacity and no hover**, which on the neutral surface is clearly lighter than
  the enabled button beside it. `alternative` and `light` with their own white or transparent
  faces are kept for the controls that are never disabled — filter and dropdown triggers, and
  Abbrechen next to a solid Speichern. A button whose disabled state changes needs
  `ActionButton.svelte`: flowbite works out a button's classes once and keeps the disabled look
  for the life of the component.
- Icons carry a **column, a card or a state** — never a button that already says what it does in
  words, and never decoration. Table headers and analysis card titles take one each, in
  `gray-400`, at `h-4` and `h-5` respectively, so that they identify the column without competing
  with the sort arrow beside them.
- **Every state has an icon, and it is the same icon everywhere that state is named** — in the
  list's badges, in the edit dialog's flags, and in the toolbar's Status filter. Six words of
  similar length in one weight are read one after another; six shapes are recognised.
  `CheckCircle` Normal · `QuestionCircle` Unbestätigt · `ExclamationCircle` Unvollständig ·
  `Star` Außergewöhnlich · `Landmark` Steuerrelevant · `FileClone` Vorlage.
- **A placeholder must not look like an entry.** flowbite sets a placeholder colour in dark mode
  only, so an empty Beschreibung field said "Beschreibung" in the same ink as a real one; both
  modes are set in `app.css` now.
- A control that does something on click has to say so at rest, not on hover: the € / % switch
  inside the amount field sits on its own grey chip, because a bare glyph at the end of an input
  reads as a unit label and nobody clicks a unit label.
- Enabled buttons get a **pointer cursor** from a rule in `app.css`, and disabled ones get
  `pointer-events: none`. Neither is a default: tailwind's preflight leaves a `<button>` with
  the arrow cursor, and flowbite's `hover:` background applies just as happily to a button that
  cannot be pressed.

## Charts

apexcharts, wrapped in `Chart.svelte`, which owns the create-or-update decision — never
instantiate apex directly.

- Series colours come from the data (account, category, availability), not from a chart
  palette. Where there is one series it is the app's own colour for that quantity.
- The shared wrapper uses Apex's `theme.follow: 'os'` for initial rendering and live system
  changes. CSS tokens supply foreground and grid colours; the chart surface stays transparent
  so it belongs to its card. Series colours are not replaced by the theme palette. Theme
  updates do not resend the series, so hiding and restoring a legend item keeps its value.
- `onUpdated` runs after Apex's native update event as well as the initial render. Imperative
  additions such as the Stichtag marker must survive a system-theme redraw too.
- Tooltips use a neutral, theme-aware surface and foreground, not the series colour as a
  background: an arbitrary account or category colour cannot guarantee readable tooltip text.
- Donut centre labels inherit the page foreground rather than Apex's fixed label colour.
  A thin neutral outline separates slices from each other and from the card, including
  categories whose configured colour is black.
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
- Deep links carry state in the URL: `?tab=` on the analysis tabs, `?category=&from=&to=` and
  `?review` / `?new` on the expense list. URL parameters are read **once** on mount — after
  that the controls belong to the reader and must not be reset from the address bar.
- State that should survive a tab switch lives in `analysis/state.svelte.ts`, not in the
  component.
- Empty states say which emptiness it is: "Niemand sonst hat etwas geändert" and "In diesem
  Zeitraum ist nichts passiert" are different sentences for a reason.
- **A failed query is not an empty result.** Every query store that feeds something a reader
  looks at renders `QueryError.svelte` — a red panel naming what is missing, with the raw
  message under it, since that is the only thing that says whether it was a timeout, an ended
  session or a bug. It takes the whole sentence rather than a noun: German makes the verb agree
  with the subject, so composing one here produced "Die Übersicht konnten nicht geladen werden".
- A spinner keyed on "no data yet" must also check for an error, or it spins for ever under a
  panel that already explained itself. Anything that would show a **derived** figure — a total,
  a count — is hidden outright rather than shown at zero. A wrong number is worse than none.
- The few stores with no error surface say so in a comment at the query, with the reason: the
  session query in the layout is read for its errors, and the nav and the range picker fall back
  rather than leaving a hole.

## Tried and rejected

- **Small caps for table headers.** Source Serif's Google build has no `smcp`, so the browser
  synthesises them by scaling the capitals down and they read thin and light. Real ones would
  mean vendoring Adobe's own build of the same typeface, and the look was not wanted anyway.
  Headers stay 14 px mixed case in Inter.
- **A solid colour per action** — Duplizieren teal, Bearbeiten yellow, Löschen red, Einsortieren
  grey, Zusammenführen blue. It made a disabled button easy to tell from an enabled one, but five
  colours in a row said nothing about the actions beyond "these are buttons", the orange Neu
  stopped standing out, and colour stopped carrying meaning anywhere near them.
- **Account and category colours filling their table cells**, striped by share for a shared
  expense, with the text colour worked out per stripe. Three tinted cells side by side read as
  one band, they drowned the amount the row is about, and keeping text legible on any colour a
  picker allows took a contrast calculation per cell. A dot says which account it is just as well.
- **Amounts on a red or green cell, and states as pastel badges.** Same reason: too loud, and
  once everything is coloured, colour stops meaning anything.
