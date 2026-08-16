// Shared look for the TableSearch-based tables. flowbite 1.33 deprecated the individual
// *Class props in favour of a single `classes` object; `table` is not read from it, so the
// table's own classes go through `class=` instead.
export const tableSearchClasses = {
	// A hairline rather than a drop shadow: the table is the page here, not a card floating on
	// it, and the shadow was the widest of the several frames between the window and the data.
	// shadow-none because flowbite merges its own `shadow-md` in rather than being replaced, and
	// a hairline under a drop shadow is two frames doing one frame's job
	root: 'bg-white dark:bg-gray-800 relative border border-gray-200 dark:border-gray-700 shadow-none sm:rounded-lg overflow-x-auto',
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
	// narrows it at the far one, and every table here says it the same way. `min-w-0 flex-1` so
	// a wide filter row takes the width off the search rather than wrapping it onto its own line
	search: 'relative w-full min-w-0 flex-1 md:order-last md:w-56 md:max-w-xs',
	// flowbite sizes the input itself at a fixed `w-80`, which ignores whatever width the
	// wrapper around it was given and hangs 25px out of the card on the expense list.
	input: 'w-full ps-10 h-10 py-0'
};

export const tableSearchTableClass = 'w-full text-left text-sm';

/// The height every control in a table's toolbar is pinned to.
///
/// flowbite sizes controls by padding, so at the same nominal `size` a borderless primary button
/// comes out 40px, a bordered `alternative` one 42, a `<select>` 42 and a `lg` button 50. Lined
/// up in one row that reads as a row that could not make up its mind, and no combination of
/// flowbite sizes makes them agree — so the height is stated instead.
export const toolbarControl = 'h-10 py-0';
