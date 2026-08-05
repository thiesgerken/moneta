// Shared look for the TableSearch-based tables. flowbite 1.33 deprecated the individual
// *Class props in favour of a single `classes` object; `table` is not read from it, so the
// table's own classes go through `class=` instead.
export const tableSearchClasses = {
	root: 'bg-white dark:bg-gray-800 relative shadow-md sm:rounded-lg overflow-x-auto',
	inner:
		'flex flex-col md:flex-row items-center justify-between space-y-3 md:space-y-0 md:space-x-4 p-4',
	search: 'w-full md:w-1/2 relative'
};

export const tableSearchTableClass = 'w-full text-left text-sm';
