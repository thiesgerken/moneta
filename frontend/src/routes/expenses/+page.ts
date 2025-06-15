import type { ExpensesVariables, PageLoadEvent } from './$houdini';

export const _ExpensesVariables: ExpensesVariables = (event: PageLoadEvent) => {
	const page = parseInt(event.url.searchParams.get('page')!) || 1;
	const count = parseInt(event.url.searchParams.get('count')!) || 10;

	return {
		limit: count,
		offset: (page - 1) * count
	};
};
