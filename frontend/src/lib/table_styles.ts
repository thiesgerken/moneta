// Shared look for the TableSearch-based tables. flowbite 1.33 deprecated the individual
// *Class props in favour of a single `classes` object; `table` is not read from it, so the
// table's own classes go through `class=` instead.
export const tableSearchClasses = {
	root: 'bg-white dark:bg-gray-800 relative shadow-md sm:rounded-lg overflow-x-auto',
	// p-2 on a phone: the page, the section and this bar each used to add their own gutter, which
	// together took a fifth of the screen before anything was in it
	inner:
		'flex flex-col md:flex-row items-center justify-between space-y-3 md:space-y-0 md:space-x-4 p-2 sm:p-4',
	search: 'w-full md:w-1/2 relative'
};

export const tableSearchTableClass = 'w-full text-left text-sm';
