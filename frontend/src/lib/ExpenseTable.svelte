<script lang="ts">
	import type { ExpenseSortField$options, SortDirection$options } from '$houdini';
	import { cache, graphql } from '$houdini';

	import {
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		TableSearch,
		Button,
		Dropdown,
		Checkbox,
		ButtonGroup,
		List,
		Li,
		Datepicker,
		Modal,
		Spinner
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		FilterSolid,
		ChevronRightOutline,
		ChevronLeftOutline,
		ChevronDownOutline,
		EditOutline,
		TrashBinOutline,
		ExclamationCircleOutline,
		CalendarMonthOutline
	} from 'flowbite-svelte-icons';
	import {
		accountSorter,
		categorySorter,
		formatTransactionAccounts,
		formatAccrualPeriod,
		formatCategories,
		makeAccountColor,
		makeCategoryColor,
		tint
	} from './expense_utils';
	import ExpenseModal from './ExpenseModal.svelte';
	import { tableSearchClasses, tableSearchTableClass } from './table_styles';
	import { page } from '$app/state';
	import { shortDate } from './analysis/dates';
	import QueryError from './QueryError.svelte';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');
	let expandedRows = $state<string[]>([]);
	/// Unbestätigt and unvollständig first, which is the list somebody opens this page to work
	/// through. A third click on a header comes back here.
	const DEFAULT_SORT = { field: 'status', direction: -1 };

	let sortBy: { field: string; direction: number } = $state({ ...DEFAULT_SORT });

	/// only consulted below `md`, where the filters are collapsed
	let filtersOpen = $state(false);

	let showNewExpenseModal = $state(false);
	let showEditExpenseModal = $state(false);
	let showDeleteExpenseModal = $state(false);
	let expenseIdToEdit: string | undefined = $state(undefined);
	let expenseIdToDelete: string | undefined = $state(undefined);

	let filteredAccounts = $state<string[]>([]);
	let filteredCategories = $state<string[]>([]);

	let filterDateRange: { from: Date | undefined; to: Date | undefined } = $state({
		from: undefined,
		to: undefined
	});

	let showTemplates = $state(false);
	let showTaxRelevant = $state(true);
	let showExtraordinary = $state(true);
	let showUnchecked = $state(true);
	let showIncomplete = $state(true);
	let showNormal = $state(true);

	// Read once rather than tracked: this is how the rest of the app hands over -- "12 Ausgaben
	// brauchen Prüfung" has to land on those twelve, a treemap tile on the expenses behind it.
	// Once here the filters are the reader's again, so a later edit must not be reset by the url.
	if (page.url.searchParams.has('new')) showNewExpenseModal = true;
	if (page.url.searchParams.has('review')) {
		showTaxRelevant = false;
		showExtraordinary = false;
		showNormal = false;
	}

	const category = page.url.searchParams.get('category');
	if (category) filteredCategories = [category];

	// A period the caller worked out; the analysis pages snap theirs to whole months, and a
	// list that quietly used different ends would not add up to the chart it came from.
	const from = page.url.searchParams.get('from');
	const to = page.url.searchParams.get('to');
	if (from || to) {
		filterDateRange = {
			from: from ? new Date(from) : undefined,
			to: to ? new Date(to) : undefined
		};
	}

	/// The chosen period, spelled out. This is the control's own label as well as its chip, so
	/// what the filter row says and what the button says cannot drift apart.
	const dateRangeLabel = $derived.by(() => {
		const { from, to } = filterDateRange;
		if (from && to) return `${shortDate(from)} – ${shortDate(to)}`;
		if (from) return `ab ${shortDate(from)}`;
		if (to) return `bis ${shortDate(to)}`;
		return null;
	});

	/// What is currently being filtered on, and how to undo each of them.
	///
	/// The filters live behind a button on a phone and in a row of empty-looking selects on a
	/// desktop, so a list arriving pre-filtered from somewhere else looked like the whole list,
	/// only shorter. Saying it out loud is also the only way to notice a filter you forgot.
	const activeFilters = $derived.by(() => {
		const out: { label: string; color?: string | null; clear: () => void }[] = [];

		if (searchTerm.trim()) {
			out.push({ label: `Suche: ${searchTerm.trim()}`, clear: () => (searchTerm = '') });
		}

		for (const id of filteredAccounts) {
			const account = filterableAccounts?.find((a) => a.value === id);
			out.push({
				label: account?.name ?? 'Konto',
				color: account?.color,
				clear: () => (filteredAccounts = withoutOption(filteredAccounts, id))
			});
		}

		for (const id of filteredCategories) {
			const category = filterableCategories?.find((c) => c.value === id);
			out.push({
				label: category?.name ?? 'Kategorie',
				color: category?.color,
				clear: () => (filteredCategories = withoutOption(filteredCategories, id))
			});
		}

		if (dateRangeLabel) {
			out.push({ label: dateRangeLabel, clear: clearDateRange });
		}

		// one chip for the lot: which of the six is off matters less than the fact that the
		// list is no longer showing everything
		if (
			showTemplates ||
			!showTaxRelevant ||
			!showExtraordinary ||
			!showUnchecked ||
			!showIncomplete ||
			!showNormal
		) {
			out.push({ label: 'Zustand', clear: resetFlags });
		}

		return out;
	});

	const withoutOption = (selected: string[], id: string) =>
		selected.filter((other) => other !== id);

	const toggleOption = (selected: string[], id: string) =>
		selected.includes(id) ? withoutOption(selected, id) : [...selected, id];

	function clearDateRange() {
		filterDateRange = { from: undefined, to: undefined };
	}

	function resetFlags() {
		showTemplates = false;
		showTaxRelevant = true;
		showExtraordinary = true;
		showUnchecked = true;
		showIncomplete = true;
		showNormal = true;
	}

	function resetFilters() {
		searchTerm = '';
		filteredAccounts = [];
		filteredCategories = [];
		clearDateRange();
		resetFlags();
	}

	// The column the header clicks name, mapped onto what the API offers. The backend used to
	// take a mongo sort document built right here; it now names its own columns.
	const SORT_FIELDS: Record<string, ExpenseSortField$options> = {
		status: 'STATUS',
		accrualStart: 'ACCRUAL_START',
		title: 'TITLE',
		store: 'STORE',
		description: 'DESCRIPTION',
		comments: 'COMMENTS',
		effectiveAmount: 'EFFECTIVE_AMOUNT',
		accounts: 'ACCOUNTS',
		categories: 'CATEGORIES'
	};

	const expenseVariables = () => ({
		limit: itemsPerPage || 10,
		offset: ((currentPage || 1) - 1) * (itemsPerPage || 10),
		search: searchTerm.trim() || undefined,
		accountIds: filteredAccounts.length > 0 ? filteredAccounts : undefined,
		categoryIds: filteredCategories.length > 0 ? filteredCategories : undefined,
		flags: {
			templates: showTemplates,
			taxRelevant: showTaxRelevant,
			extraordinary: showExtraordinary,
			unchecked: showUnchecked,
			incomplete: showIncomplete,
			normal: showNormal
		},
		accruedAfter: filterDateRange.from,
		accruedBefore: filterDateRange.to,
		sortBy: SORT_FIELDS[sortBy.field],
		sortDir: (sortBy.direction < 0 ? 'DESC' : 'ASC') as SortDirection$options
	});

	// Stuff that we always need independently of the current page and sorting and so on
	const basicStore = graphql(`
		query ExpenseTableBasic {
			users {
				me {
					name
				}
			}
			accounts {
				list {
					id
					name
					owner {
						name
						firstName
					}
					color
					closed
				}
			}
			categories {
				list {
					id
					name
					parent {
						id
					}
					color
				}
			}
		}
	`);

	const expenseStore = graphql(`
		query ExpenseTableExpenses(
			$limit: Int
			$offset: Int
			$search: String
			$accountIds: [UUID!]
			$categoryIds: [UUID!]
			$flags: ExpenseFlagFilter
			$accruedAfter: DateTime
			$accruedBefore: DateTime
			$sortBy: ExpenseSortField
			$sortDir: SortDirection
		) {
			expenses {
				list(
					limit: $limit
					skip: $offset
					search: $search
					accountIds: $accountIds
					categoryIds: $categoryIds
					flags: $flags
					accruedAfter: $accruedAfter
					accruedBefore: $accruedBefore
					sortBy: $sortBy
					sortDir: $sortDir
				) {
					totalCount
					edges {
						node {
							id
							title
							description
							store
							effectiveAmount
							accrualStart
							accrualEnd
							comments
							isTaxRelevant
							isTemplate
							isUnchecked
							isIncomplete
							isExtraordinary
							receipts {
								uri
								ocr
							}
							changelog {
								id
								user {
									name
									firstName
								}
								date
								tool
								automatic
								oldState {
									id
								}
								diff
							}
							positions {
								title
								amount
								quantity
								unit
								comments
							}
							categories {
								weight
								category {
									id
									name
									color
								}
							}
							referenceAmount
							transactions {
								fraction
								effectiveAmount
								amount
								date
								statement
								account {
									id
									name
									owner {
										name
										firstName
									}
									color
								}
							}
						}
					}
				}
			}
		}
	`);

	const filterableCategories = $derived(
		$basicStore.data?.categories.list.sort(categorySorter).map((category) => ({
			name: category.name,
			value: category.id,
			color: category.color
		}))
	);

	const filterableAccounts = $derived(
		$basicStore.data?.accounts.list
			.sort((a, b) => accountSorter(a, b, $basicStore.data?.users.me.name || ''))
			.map((account) => ({
				name:
					account.owner.name == $basicStore.data?.users.me.name
						? account.name
						: `${account.owner.firstName} / ${account.name}`,
				value: account.id,
				color: account.color
			}))
	);

	const totalCount = $derived($expenseStore.data?.expenses.list.totalCount || 0);
	const totalPages = $derived(Math.ceil(totalCount / itemsPerPage));

	$effect(() => {
		if (totalPages < currentPage) {
			currentPage = Math.max(totalPages, 1);
		}
	});

	let expenses = $derived(
		$expenseStore.data?.expenses.list.edges.map((expense) => expense.node) || []
	);

	/// Whether the running fetch has been running long enough to say so.
	///
	/// Houdini keeps the previous result up while it fetches, so a slow page turn showed the old
	/// page's rows under the new page number -- nothing said anything was on its way. The delay is
	/// what keeps a fast answer from flashing a spinner on its way past.
	let slow = $state(false);
	$effect(() => {
		if (!$expenseStore.fetching) {
			slow = false;
			return;
		}

		const timer = setTimeout(() => (slow = true), 300);
		return () => clearTimeout(timer);
	});

	// What the table draws, as opposed to what the modals reach into: an edit begun before a
	// refetch still has its expense, the rows behind it do not stay up pretending to be the answer.
	let rows = $derived(slow ? [] : expenses);

	let expenseToEdit = $derived(expenses.find((e) => e.id === expenseIdToEdit) || undefined);
	let expenseToDelete = $derived(expenses.find((e) => e.id === expenseIdToDelete) || undefined);

	/// The bank's own text for an expense, which is several strings when it is split over more
	/// than one transaction. Newlines collapse: the cell is one line and the full text is in the
	/// expanded row.
	const transactionText = (transactions: { statement: string | null }[]) =>
		transactions
			.map((t) => t.statement?.replace(/\s+/g, ' ').trim())
			.filter(Boolean)
			.join(' · ');

	function toggleRow(id: string) {
		if (expandedRows.includes(id)) expandedRows = expandedRows.filter((rowId) => rowId !== id);
		else expandedRows = [...expandedRows, id];
	}

	/// Descending, ascending, off. The third state is what there was no way back to: sorting
	/// lives in the component rather than in the url, so a header clicked by accident could only
	/// be undone by reloading the page.
	function sortByField(field: string) {
		if (sortBy.field !== field) sortBy = { field, direction: -1 };
		else if (sortBy.direction === -1) sortBy = { field, direction: 1 };
		else sortBy = { ...DEFAULT_SORT };
	}

	function isSortedBy(field: string) {
		return sortBy.field === field ? sortBy.direction : null;
	}

	$effect(() => void basicStore.fetch());
	$effect(() => void expenseStore.fetch({ variables: expenseVariables() }));

	const DeleteExpense = graphql(`
		mutation DeleteExpense($id: UUID!) {
			expenses {
				delete(id: $id)
			}
		}
	`);

	async function handleExpenseModalSubmit() {
		cache.markStale('Expense');
		await expenseStore.fetch({ variables: expenseVariables() });
		expandedRows = [];
	}

	async function handleDeleteExpense() {
		if (!expenseToDelete) return;
		await DeleteExpense.mutate({ id: expenseToDelete.id });
		expenseIdToDelete = undefined;
		showDeleteExpenseModal = false;

		cache.markStale('Expense');
		await expenseStore.fetch({ variables: expenseVariables() });
		expandedRows = [];
	}
</script>

<!-- Accounts and categories carry a colour, and that colour is how a reader recognises them --
     it is what the cells in the table below are tinted with. flowbite's MultiSelect prints its
     options as bare text and offers no hook to change that, so the filters are a checkbox list
     with the swatch in front of each name, like the Zustand filter beside them. -->
{#snippet optionFilter(
	label: string,
	options: { name: string; value: string; color?: string | null }[],
	selected: string[],
	toggle: (id: string) => void
)}
	<Button color="alternative" class="justify-between md:w-48">
		{selected.length ? `${label} (${selected.length})` : label}
		<ChevronDownOutline class="ml-2 h-3 w-3 shrink-0" />
	</Button>
	<Dropdown class="max-h-80 w-64 divide-y-0 overflow-y-auto p-3 text-sm">
		<div class="space-y-2">
			{#each options as option (option.value)}
				<Checkbox checked={selected.includes(option.value)} onchange={() => toggle(option.value)}>
					<span
						class="mr-2 inline-block h-3 w-3 shrink-0 rounded-sm"
						style="background-color: {option.color}"
					></span>
					{option.name}
				</Checkbox>
			{/each}
		</div>
	</Dropdown>
{/snippet}

{#snippet statusBadges(expense: {
	isTaxRelevant: boolean;
	isTemplate: boolean;
	isUnchecked: boolean;
	isIncomplete: boolean;
	isExtraordinary: boolean;
})}
	{#if expense.isTaxRelevant}
		<span class="rounded-md bg-green-100 px-2 py-1 text-green-700">Steuerrelevant</span>
	{/if}
	{#if expense.isTemplate}
		<span class="rounded-md bg-blue-100 px-2 py-1 text-blue-700">Vorlage</span>
	{/if}
	{#if expense.isUnchecked}
		<span class="rounded-md bg-red-100 px-2 py-1 text-yellow-700">Unbestätigt</span>
	{/if}
	{#if expense.isIncomplete}
		<span class="rounded-md bg-red-100 px-2 py-1 text-red-700">Unvollständig</span>
	{/if}
	{#if expense.isExtraordinary}
		<span class="rounded-md bg-purple-100 px-2 py-1 text-purple-700">Außergewöhnlich</span>
	{/if}
{/snippet}

{#snippet sortIcon(field: string)}
	{#if isSortedBy(field) === -1}
		▲
	{:else if isSortedBy(field) === 1}
		▼
	{/if}
{/snippet}

<Modal bind:open={showDeleteExpenseModal} size="xs">
	<div class="text-center">
		<ExclamationCircleOutline class="mx-auto mb-4 h-12 w-12 text-gray-400 dark:text-gray-200" />
		<h3 class="mb-5 text-lg font-normal text-gray-500 dark:text-gray-400">
			Soll die Ausgabe <b>{expenseToDelete?.title}</b> wirklich gelöscht werden?
		</h3>
		<div class="space-x-2">
			<Button type="submit" value="delete" color="red" onclick={handleDeleteExpense}>Löschen</Button
			>
			<Button
				type="submit"
				value="cancel"
				color="alternative"
				onclick={() => {
					showDeleteExpenseModal = false;
					expenseIdToDelete = undefined;
				}}
			>
				Abbrechen
			</Button>
		</div>
	</div>
</Modal>

<ExpenseModal
	bind:open={showNewExpenseModal}
	onsubmit={handleExpenseModalSubmit}
	userName={$basicStore.data?.users.me?.name}
	accounts={$basicStore.data?.accounts.list.map((a) => ({
		id: a.id.toString(),
		name: a.name,
		owner: { name: a.owner.name, firstName: a.owner.firstName },
		color: a.color
	})) || []}
	categories={$basicStore.data?.categories.list.map((c) => ({
		id: c.id.toString(),
		name: c.name,
		color: c.color
	})) || []}
/>

<ExpenseModal
	bind:open={showEditExpenseModal}
	onsubmit={handleExpenseModalSubmit}
	userName={$basicStore.data?.users.me?.name}
	expense={expenseToEdit}
	accounts={$basicStore.data?.accounts.list.map((a) => ({
		id: a.id.toString(),
		name: a.name,
		owner: { name: a.owner.name, firstName: a.owner.firstName },
		color: a.color
	})) || []}
	categories={$basicStore.data?.categories.list.map((c) => ({
		id: c.id.toString(),
		name: c.name,
		color: c.color
	})) || []}
/>

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-0 sm:p-5">
	<QueryError
		errors={[...($expenseStore.errors ?? []), ...($basicStore.errors ?? [])]}
		message="Die Ausgaben konnten nicht geladen werden."
		class="mb-4"
	/>
	{#if activeFilters.length > 0}
		<div class="flex flex-wrap items-center gap-2 px-4 pb-3 text-sm sm:px-0">
			<span class="text-gray-500 dark:text-gray-400">Gefiltert nach</span>
			{#each activeFilters as filter (filter.label)}
				<!-- tinted where what is being filtered on has a colour of its own, so the chip and
				     the cells it left standing look like each other -->
				<span
					class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 {filter.color
						? ''
						: 'bg-gray-200 dark:bg-gray-700'}"
					style={tint(filter.color)}
				>
					{filter.label}
					<!-- dimmed rather than grey: on a tinted chip the text colour is whatever the tint
					     needs, and a fixed grey × was invisible on the dark ones -->
					<button
						type="button"
						aria-label="{filter.label} entfernen"
						class="opacity-60 hover:opacity-100"
						onclick={filter.clear}
					>
						×
					</button>
				</span>
			{/each}
			<button
				type="button"
				class="text-primary-600 dark:text-primary-500 hover:underline"
				onclick={resetFilters}
			>
				alle entfernen
			</button>
		</div>
	{/if}

	<TableSearch
		placeholder="Suchen"
		hoverable={true}
		bind:inputValue={searchTerm}
		class="{tableSearchTableClass} table-fixed"
		classes={tableSearchClasses}
	>
		{#snippet header()}
			<div
				class="flex w-full flex-shrink-0 flex-col items-stretch justify-end gap-2 md:w-auto md:flex-row md:items-center md:gap-3"
			>
				<div class="flex items-center gap-2">
					<Button class="grow md:grow-0" onclick={() => (showNewExpenseModal = true)}>
						<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
					</Button>
					<!-- Stacked, the filters filled a phone screen before a single expense appeared,
					     so on small screens they fold behind this. -->
					<Button
						color="alternative"
						class="md:hidden"
						aria-label="Filter"
						aria-expanded={filtersOpen}
						onclick={() => (filtersOpen = !filtersOpen)}
					>
						<FilterSolid class="h-4 w-4" />
					</Button>
				</div>
				<div
					class="{filtersOpen
						? 'flex'
						: 'hidden'} flex-col gap-2 md:flex md:flex-row md:items-center md:gap-3"
				>
					<!-- The period is the button's label rather than the contents of a text field.
					     flowbite's own input is sized for one date, so a range arrived clipped, and in
					     range mode it prints a bare " - " instead of its placeholder when nothing is
					     chosen -- which left the control that decides what the table shows unreadable
					     in both states, and with nowhere to undo it. -->
					<Button color="alternative" class="justify-between whitespace-nowrap md:w-64">
						{dateRangeLabel ?? 'Zeitraum'}
						<CalendarMonthOutline class="ml-2 h-4 w-4 shrink-0" />
					</Button>
					<Dropdown class="flex w-fit flex-col gap-2 divide-y-0 p-2">
						<!-- the inline calendar draws its own white, shadowed, padded card, which inside
						     the dropdown's card is a second box around the same thing -->
						<Datepicker
							class="bg-transparent p-0 shadow-none dark:bg-transparent"
							inline
							range
							bind:rangeFrom={filterDateRange.from}
							bind:rangeTo={filterDateRange.to}
							locale="de-DE"
							firstDayOfWeek={1}
						/>
						<Button
							size="xs"
							color="alternative"
							disabled={!dateRangeLabel}
							onclick={clearDateRange}
						>
							Zeitraum zurücksetzen
						</Button>
					</Dropdown>
					{@render optionFilter(
						'Konten',
						filterableAccounts ?? [],
						filteredAccounts,
						(id) => (filteredAccounts = toggleOption(filteredAccounts, id))
					)}
					{@render optionFilter(
						'Kategorien',
						filterableCategories ?? [],
						filteredCategories,
						(id) => (filteredCategories = toggleOption(filteredCategories, id))
					)}
					<Button color="alternative">Filter<FilterSolid class="ml-2 h-3 w-3 " /></Button>
					<Dropdown class="w-48 space-y-2 p-3 text-sm">
						<h6 class="mb-3 text-sm font-medium text-gray-900 dark:text-white">Zustand</h6>
						<List tag="dl">
							<Li>
								<Checkbox bind:checked={showTaxRelevant}>Steuerrelevant</Checkbox>
							</Li>
							<Li>
								<Checkbox bind:checked={showTemplates}>Vorlage</Checkbox>
							</Li>
							<Li>
								<Checkbox bind:checked={showExtraordinary}>Außergewöhnlich</Checkbox>
							</Li>
							<Li>
								<Checkbox bind:checked={showUnchecked}>Unbestätigt</Checkbox>
							</Li>
							<Li>
								<Checkbox bind:checked={showIncomplete}>Unvollständig</Checkbox>
							</Li>
							<Li>
								<Checkbox bind:checked={showNormal}>Normal</Checkbox>
							</Li>
						</List>
					</Dropdown>
				</div>
			</div>
		{/snippet}

		<!-- A phone keeps date, title and amount and hides the other five. They are not lost: the
		     row expands into all of them, and the title cell carries the status flags on its own
		     line, since "braucht Prüfung" is what somebody opens this list on a phone for. Eight
		     columns on a 390px screen meant scrolling sideways past the date to reach the amount. -->
		<TableHead>
			<!-- pl to clear the chevron: it hangs in the column's left margin, and without this the
			     label started 24px to the left of every date under it. -->
			<TableHeadCell
				class="w-28 py-3 pr-2 pl-7 whitespace-nowrap md:w-36 md:pr-4 md:pl-10"
				scope="col"
				onclick={() => sortByField('accrualStart')}
			>
				Datum
				{@render sortIcon('accrualStart')}
			</TableHeadCell>
			<TableHeadCell class="px-2 py-3 md:px-4" scope="col" onclick={() => sortByField('title')}>
				Titel
				{@render sortIcon('title')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-36"
				scope="col"
				onclick={() => sortByField('accounts')}
			>
				Konten
				{@render sortIcon('accounts')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-36"
				scope="col"
				onclick={() => sortByField('categories')}
			>
				Kategorien
				{@render sortIcon('categories')}
			</TableHeadCell>
			<TableHeadCell
				class="w-28 px-2 py-3 text-right md:w-32 md:px-4"
				scope="col"
				onclick={() => sortByField('effectiveAmount')}
			>
				Betrag
				{@render sortIcon('effectiveAmount')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-32"
				scope="col"
				onclick={() => sortByField('store')}
			>
				Geschäft
				{@render sortIcon('store')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-56"
				scope="col"
				onclick={() => sortByField('description')}
			>
				Beschreibung
				{@render sortIcon('description')}
			</TableHeadCell>
			<!-- What the bank wrote. No sort arrow: the backend names its sortable columns and this
			     is not one of them. From 2xl only, and it shares what the fixed columns leave with
			     the title: at 1280 the nine columns together are wider than the window, and both of
			     them end up too narrow to read. Below that the expanded row still has it in full. -->
			<TableHeadCell class="hidden px-4 py-3 2xl:table-cell" scope="col">
				Transaktionstext
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-36"
				scope="col"
				onclick={() => sortByField('status')}
			>
				Status
				{@render sortIcon('status')}
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#if slow}
				<tr>
					<td colspan="9" class="px-4 py-16 text-center">
						<Spinner size="8" />
					</td>
				</tr>
			{/if}
			{#each rows as expense (expense.id)}
				<TableBodyRow
					onclick={() => toggleRow(expense.id)}
					data-expanded={expandedRows.includes(expense.id) ? '' : undefined}
				>
					<!-- text-xs on a phone: a date is what the row is filed under rather than what it
					     says, and the ten characters it takes are ten the title does not get -->
					<TableBodyCell class="px-2 py-3 text-xs whitespace-nowrap md:px-4 md:text-sm">
						<button type="button" class="mr-1 focus:outline-none md:mr-2">
							{#if expandedRows.includes(expense.id)}
								<ChevronDownOutline class="inline h-4 w-4" />
							{:else}
								<ChevronRightOutline class="inline h-4 w-4" />
							{/if}
						</button>
						{formatAccrualPeriod(expense.accrualStart, expense.accrualEnd, false)}
					</TableBodyCell>
					<!-- `w-full max-w-0` is what makes the inner block truncate: a table cell is as wide
					     as its content unless something says otherwise, and `truncate` on the cell
					     itself is gone now that the cell holds two lines rather than one string. Zero
					     max-width plus full width leaves it exactly the space the other columns did
					     not take. -->
					<TableBodyCell class="px-2 py-3 whitespace-normal md:px-4">
						<!-- wraps rather than truncates on a phone: with five columns gone the title is
						     most of what identifies the row, and there is no second place to read it -->
						<!-- wrap-anywhere, not break-words: only `overflow-wrap: anywhere` counts towards
					     the cell's min-content width, and a table column is sized by that. With
					     break-word the column stayed as wide as "Auslandskrankenversicherung" and the
					     title ran across the amount beside it. -->
						<span class="block wrap-anywhere md:truncate">{expense.title}</span>
						<span class="mt-1 flex flex-wrap gap-1 text-xs empty:mt-0 md:hidden">
							{@render statusBadges(expense)}
						</span>
					</TableBodyCell>
					<TableBodyCell
						class="hidden max-w-64 truncate px-4 py-3 md:table-cell"
						style={makeAccountColor(expense.transactions, expense.referenceAmount)}
						>{formatTransactionAccounts(
							expense.transactions,
							$basicStore.data?.users.me.name
						)}</TableBodyCell
					>
					<TableBodyCell
						class="hidden max-w-64 truncate px-4 py-3 md:table-cell"
						style={makeCategoryColor(expense.categories)}
						>{formatCategories(expense.categories)}</TableBodyCell
					>
					<TableBodyCell
						class={'px-2 py-3 text-right whitespace-nowrap md:px-4 ' +
							(expense.effectiveAmount < 0
								? 'bg-red-100 text-red-700'
								: expense.effectiveAmount > 0
									? 'bg-green-100 text-green-700'
									: // zero is not "no colour": it is an expense that moved money between two of
										// your own accounts and came out even, which is a third thing rather than a
										// missing one. Grey, since nothing left or arrived -- and gray-200 rather
										// than the -100 the other two use, which is the shade a hovered row is.
										'bg-gray-200 text-gray-600')}
						>{(expense.effectiveAmount / 100).toFixed(2)}€</TableBodyCell
					>
					<TableBodyCell class="hidden max-w-64 truncate px-4 py-3 md:table-cell"
						>{expense.store}</TableBodyCell
					>
					<TableBodyCell class="hidden max-w-64 truncate px-4 py-3 md:table-cell"
						>{expense.description}
						{expense.positions
							.map(
								(position) =>
									`${position.quantity ? position.quantity + (position.unit ? ' ' + position.unit : '') : ''}${position.title}`
							)
							.join(', ')}
					</TableBodyCell>
					<!-- monospace because it is machine output: the reference numbers and IBANs in it
					     are read character by character, and a proportional font makes that work -->
					<TableBodyCell
						class="hidden truncate px-4 py-3 font-mono text-xs 2xl:table-cell"
						title={transactionText(expense.transactions)}
					>
						{transactionText(expense.transactions)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{@render statusBadges(expense)}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(expense.id)}
					<tr data-expanded>
						<td colspan="9" class="border-b px-4 py-3">
							<div class="flex flex-col gap-2">
								<div>
									<b>Leistungszeitraum:</b>
									{formatAccrualPeriod(expense.accrualStart, expense.accrualEnd, true)}
								</div>
								{#if expense.description}
									<div>
										<b>Beschreibung:</b>
										{expense.description}
									</div>
								{/if}
								{#if expense.store}
									<div>
										<b>Geschäft:</b>
										{expense.store}
									</div>
								{/if}
								{#if expense.comments}
									<div>
										<b>Kommentare:</b>
										{expense.comments}
									</div>
								{/if}
								{#if expense.isTaxRelevant || expense.isTemplate || expense.isUnchecked || expense.isIncomplete || expense.isExtraordinary}
									<div>
										<b>Status:</b>
										{[
											expense.isTaxRelevant && 'Steuerrelevant',
											expense.isTemplate && 'Vorlage',
											expense.isUnchecked && 'Unbestätigt',
											expense.isIncomplete && 'Unvollständig',
											expense.isExtraordinary && 'Außergewöhnlich'
										]
											.filter(Boolean)
											.join(', ') || 'Normal'}
									</div>
								{/if}

								{#if expense.categories.length > 1}
									<div>
										<b>Kategorien:</b>
										<ul class="ml-6 list-disc">
											{#each expense.categories as category (category.category.id)}
												<li>
													<span class="rounded-md px-1" style={makeCategoryColor([category])}>
														{category.category.name}
													</span>
													(Gewicht: {category.weight})
												</li>
											{/each}
										</ul>
									</div>
								{/if}

								{#if expense.positions.length > 0}
									<div>
										<b>Positionen:</b>
										<ul class="ml-6 list-disc">
											{#each expense.positions as position, i (i)}
												<li>
													{#if position.quantity && position.quantity > 1}
														{position.quantity}
														{#if position.unit}
															{position.unit}
														{/if}
													{/if}
													{position.title}
													{#if position.amount}
														für {(position.amount / 100).toFixed(2)}€
													{/if}
													{#if position.comments}
														<span class="text-xs italic">{position.comments}</span>
													{/if}
												</li>
											{/each}
										</ul>
									</div>
								{/if}

								<div>
									<b>Transaktionen:</b>
									<ul class="ml-6 list-disc">
										{#each expense.transactions.toSorted((a, b) => Math.abs(b.effectiveAmount) - Math.abs(a.effectiveAmount)) as transaction, i (i)}
											<li>
												<span
													class="rounded-md px-1.5 py-0.5"
													style={makeAccountColor([transaction], expense.referenceAmount)}
												>
													{#if transaction.account.owner.name != $basicStore.data?.users.me.name}
														{transaction.account.owner.firstName} / {transaction.account.name}
													{:else}
														{transaction.account.name}
													{/if}
												</span>
												{#if transaction.fraction}
													{((transaction.fraction! * expense.referenceAmount!) / 100).toFixed(2)}€ ({transaction.fraction! *
														100}% von {(expense.referenceAmount! / 100).toFixed(2)}€)
												{:else}
													{(transaction.amount! / 100).toFixed(2)}€
												{/if}
												{#if transaction.date.toISOString() != expense.accrualStart.toISOString() || transaction.date.toISOString() != expense.accrualEnd.toISOString()}
													am {transaction.date.toLocaleDateString('de-DE', {
														year: 'numeric',
														month: '2-digit',
														day: '2-digit',
														hour: '2-digit',
														minute: '2-digit'
													})}
												{/if}
												{#if transaction.statement}
													<pre class="text-xs">{transaction.statement}</pre>
												{/if}
											</li>
										{/each}
									</ul>
								</div>

								{#if expense.receipts.length > 0}
									<div>
										<b>Belege:</b>
										<ul class="ml-6 list-disc">
											{#each expense.receipts as receipt (receipt.uri)}
												<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- receipt uris point outside the app (dms, file store) -->
												<li><a href={receipt.uri} target="_blank">{receipt.uri}</a></li>
											{/each}
										</ul>
									</div>
								{/if}

								<div>
									<b>Changelog:</b>
									<ul class="ml-6 list-disc">
										{#each expense.changelog.toReversed() as change (change.id)}
											<li>
												{change.user!.name == $basicStore.data?.users.me.name
													? 'Du'
													: change.user!.firstName} am {change.date.toLocaleDateString('de-DE', {
													year: 'numeric',
													month: '2-digit',
													day: '2-digit',
													hour: '2-digit',
													minute: '2-digit'
												})}
												{#if change.tool}
													via {change.tool}
												{/if}
												{#if change.automatic}
													(automatische Änderung)
												{/if}
												{#if !change.oldState}
													<p>Ausgabe erstellt</p>
												{:else if change.diff}
													<pre class="text-xs">{change.diff}</pre>
												{/if}
											</li>
										{/each}
									</ul>
								</div>

								<div class="flex gap-2">
									<Button
										color="yellow"
										size="xs"
										onclick={() => {
											expenseIdToEdit = expense.id.toString();
											showEditExpenseModal = true;
										}}
									>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="red"
										size="xs"
										onclick={() => {
											expenseIdToDelete = expense.id.toString();
											showDeleteExpenseModal = true;
										}}
									>
										<TrashBinOutline class="mr-2 h-3.5 w-3.5" />
										Löschen
									</Button>
								</div>
							</div>
						</td>
					</tr>
				{/if}
			{/each}
		</TableBody>
		{#snippet footer()}
			<div
				class="flex flex-col items-start justify-between space-y-3 p-4 md:flex-row md:items-center md:space-y-0"
				aria-label="Table navigation"
			>
				<span class="text-sm font-normal text-gray-500 dark:text-gray-400">
					Einträge
					<span class="font-semibold text-gray-900 dark:text-white"
						>{(currentPage - 1) * itemsPerPage + 1}-{Math.min(
							currentPage * itemsPerPage,
							totalCount
						)}</span
					>
					von
					<span class="font-semibold text-gray-900 dark:text-white">{totalCount}</span>
				</span>
				<!-- Up to nine buttons do not fit next to each other on a phone, and the row wrapped
				     into three. There the two arrows are the whole control: which page of how many
				     is what the "Einträge 31-45 von 700" beside them already says. -->
				<ButtonGroup>
					<Button onclick={() => currentPage--} disabled={currentPage <= 1}>
						<ChevronLeftOutline size="xs" class="m-1.5" />
					</Button>
					{#if currentPage > 3}
						<Button
							class="hidden md:inline-flex"
							disabled={currentPage === 1}
							onclick={() => (currentPage = 1)}>1</Button
						>
						<Button class="hidden md:inline-flex" disabled>...</Button>
					{/if}
					{#each [currentPage - 2, currentPage - 1, currentPage, currentPage + 1, currentPage + 2].filter((page) => page > 0 && page <= totalPages) as pageNumber (pageNumber)}
						<Button
							class="hidden md:inline-flex"
							disabled={currentPage === pageNumber}
							onclick={() => (currentPage = pageNumber)}>{pageNumber}</Button
						>
					{/each}
					{#if currentPage < totalPages - 2}
						<Button class="hidden md:inline-flex" disabled>...</Button>
						<Button
							class="hidden md:inline-flex"
							disabled={currentPage === totalPages}
							onclick={() => (currentPage = totalPages)}>{totalPages}</Button
						>
					{/if}
					<Button onclick={() => currentPage++} disabled={currentPage >= totalPages}>
						<ChevronRightOutline size="xs" class="m-1.5" />
					</Button>
				</ButtonGroup>
			</div>
		{/snippet}
	</TableSearch>
</Section>
