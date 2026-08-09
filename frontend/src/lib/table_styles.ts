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
	inner:
		'flex flex-col md:flex-row md:flex-wrap items-center justify-between gap-y-2 space-y-2 md:space-y-0 md:space-x-4 p-2 sm:p-3',
	search: 'w-full md:w-2/5 relative'
};

export const tableSearchTableClass = 'w-full text-left text-sm';
