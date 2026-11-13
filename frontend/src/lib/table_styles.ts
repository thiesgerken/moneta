// Shared look for the TableSearch-based tables. flowbite 1.33 deprecated the individual
// *Class props in favour of a single `classes` object; `table` is not read from it, so the
// table's own classes go through `class=` instead.
export const tableSearchClasses = {
	// A hairline rather than a drop shadow: the table is the page here, not a card floating on
	// it, and the shadow was the widest of the several frames between the window and the data.
	// shadow-none because flowbite merges its own `shadow-md` in rather than being replaced, and
	// a hairline under a drop shadow is two frames doing one frame's job
	// From sm only: on a phone the list fills the width, and a frame around it spends that width
	// on a box saying "this is a thing" where the whole page is the same thing.
	root: 'bg-white dark:bg-gray-800 relative border-gray-200 sm:border dark:border-gray-700 shadow-none sm:rounded-lg overflow-x-auto',
	// p-2 on a phone: the page, the section and this bar each used to add their own gutter, which
	// together took a fifth of the screen before anything was in it
	// flex-wrap and a narrower search: the filter row is a fixed 600px of buttons, and at a 1280px
	// window the search claiming half the width pushed the last of them 14px past the card
	// gap rather than space-x: the expense list reorders its search box to the right with
	// `order-last`, and `space-x` puts its margin on whichever child is second in the markup,
	// which after a reorder is the one on the left
	inner:
		'flex flex-col md:flex-row md:flex-wrap items-center justify-between gap-y-2 space-y-2 md:space-y-0 md:gap-x-4 p-2 sm:p-3',
	// order-last: the buttons that act on the table belong at the reading edge, the search that
	// narrows it at the far one, and every table here says it the same way. `flex-1` so a wide
	// filter row takes the width off the search rather than wrapping it onto its own line, down to
	// `md:min-w-40`: below that a search field is a sliver nobody can type into, and it is better
	// off on a line of its own under the filters (the expense list between 768 and ~1000px)
	search: 'relative w-full min-w-0 flex-1 md:order-last md:w-56 md:max-w-xs md:min-w-40',
	// flowbite sizes the input itself at a fixed `w-80`, which ignores whatever width the
	// wrapper around it was given and hangs 25px out of the card on the expense list.
	input: 'w-full ps-10 h-10 py-0'
};

export const tableSearchTableClass = 'w-full text-left text-sm';

/// A small table inside an open row -- transactions, categories, positions. The same ruling as
/// the list around it and nothing else: a frame, a filled header and a surface of its own made
/// it a second, darker table on top of the first.
export const detailTable = 'w-full text-left text-sm';

/// The height every control in a table's toolbar is pinned to.
///
/// flowbite sizes controls by padding, so at the same nominal `size` a borderless primary button
/// comes out 40px, a bordered `alternative` one 42, a `<select>` 42 and a `lg` button 50. Lined
/// up in one row that reads as a row that could not make up its mind, and no combination of
/// flowbite sizes makes them agree — so the height is stated instead.
export const toolbarControl = 'h-10 py-0';

/// The icon in front of a column heading.
///
/// A row of headings in one weight is read word by word; the same row with a shape in front of
/// each is recognised. `align-[-0.15em]` because an inline svg sits on the baseline and the
/// text does not.
///
/// From `md` only: on a phone a column is as narrow as its label, and icon plus label in it
/// either wraps the heading onto two lines or widens the column past the screen.
export const headIcon =
	'me-1.5 hidden h-4 w-4 align-[-0.15em] text-gray-400 md:inline dark:text-gray-500';

/// Every action on an expense, account, category or rule that is not "create", as one neutral
/// group: `ButtonGroup class={actionGroup}` around buttons with `class={actionButton}`.
///
/// Neu is the one orange button on a page, so it is found without reading. Five solid colours
/// in a row said nothing beyond "these are buttons" -- teal, yellow, red, grey and blue were
/// not five kinds of action -- and they drowned the colours that do carry meaning, the amounts.
/// The surface is a step off the card's in both themes, so the group reads as buttons without
/// needing a colour; the dividers keep five labels from running into one band.
///
/// Disabled is 40% rather than flowbite's 50%: at 50% a grey button is barely lighter than the
/// enabled grey beside it. `ActionButton` is still needed for any whose disabled state changes.
///
/// The divider is each button's own end border, in the same `[&:not(:last-child)]` variant
/// flowbite uses to take it away inside a group, so that tailwind-merge swaps theirs for this
/// one. `divide-x` on the group loses to that variant.
export const actionGroup = 'shadow-none';
export const actionButton =
	'border-0 [&:not(:last-child)]:border-e border-gray-300 bg-gray-100 text-gray-900 hover:bg-gray-200 focus-within:ring-gray-300 disabled:opacity-40 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100 dark:hover:bg-gray-600 dark:focus-within:ring-gray-600';

/// Löschen is the one action in the group with a colour, and only in its text: it is the one
/// that cannot be undone, and red text says so without a red block shouting it at every row.
export const dangerText = 'text-red-700 dark:text-red-300';

/// A state, a kind or a trigger: a hairline and muted text rather than a coloured fill.
///
/// Five pastel fills in one column competed with the amounts for the eye and won, although the
/// amount is what a row is about. The icon in front of each state is what tells them apart.
const chipShape =
	'inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs whitespace-nowrap';
export const chip = `${chipShape} border-gray-300 text-gray-600 dark:border-gray-600 dark:text-gray-300`;

/// Amber means a person still has to do something: confirm what an automation filed, finish
/// what is incomplete, name what came in without a title. It is the one colour among the states
/// because it is the one thing in the list that asks for the reader; red already means money out
/// and Löschen. amber-700 on white and amber-400 on gray-800 both clear 4.5:1, and on the open
/// row's shade as well.
export const attentionText = 'text-amber-700 dark:text-amber-400';
export const attentionChip = `${chipShape} border-amber-600 text-amber-700 dark:border-amber-500 dark:text-amber-400`;
