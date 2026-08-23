<script lang="ts">
	import type { ExpenseSortField$options, SortDirection$options } from '$houdini';
	import { graphql } from '$houdini';
	import { invalidate, AFTER_WRITE } from './invalidate';

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
		Datepicker,
		Modal,
		Select,
		Spinner
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		FilterSolid,
		ChevronRightOutline,
		ChevronLeftOutline,
		ChevronDownOutline,
		ArchiveArrowDownOutline,
		FileCopyAltOutline,
		EditOutline,
		TrashBinOutline,
		ExclamationCircleOutline,
		CalendarMonthOutline,
		AlignLeftOutline,
		EuroOutline,
		FlagOutline,
		TextSizeOutline,
		ReceiptOutline,
		StoreOutline,
		TagOutline,
		WalletOutline,
		CheckCircleOutline,
		FileCloneOutline,
		LandmarkOutline,
		QuestionCircleOutline,
		StarOutline
	} from 'flowbite-svelte-icons';
	import {
		accountSorter,
		byUsageThen,
		categorySorter,
		formatAccount,
		formatTransactionAccounts,
		formatAccrualPeriod,
		formatCategories,
		makeAccountColor,
		makeCategoryColor,
		statementLine
	} from './expense_utils';
	import ActionButton from './ActionButton.svelte';
	import ChangeList from './ChangeList.svelte';
	import ExpenseModal from './ExpenseModal.svelte';
	import { tableSearchClasses, tableSearchTableClass, toolbarControl } from './table_styles';
	import { page } from '$app/state';
	import { accrualShare, shortDate } from './analysis/dates';
	import { euro, signedEuro } from './money';
	import QueryError from './QueryError.svelte';
	import SortIcon from './SortIcon.svelte';
	import { DESC, TableSort } from './table_sort.svelte';
	import { debounced } from './debounce.svelte';
	import OptionFilter from './OptionFilter.svelte';
	import FilterChips from './FilterChips.svelte';
	import { toggleOption } from './filters';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');
	const search = debounced(() => searchTerm.trim());
	let expandedRows = $state<string[]>([]);
	/// Unbestätigt and unvollständig first, which is the list somebody opens this page to work
	/// through. A third click on a header comes back here.
	const sort = new TableSort({ field: 'status', direction: DESC });

	/// Which of the three dates the date column, its sort and the period filter mean.
	///
	/// They answer different questions -- what a cost was *for* against when the money left --
	/// and a list showing one while filtering by another does not add up. So it is one choice
	/// rather than a column each: eight columns already do not fit a phone. It is made inside
	/// the period filter, next to the range it decides the meaning of.
	const DATE_AXES = {
		accrual: { label: 'Leistungszeitraum', sort: 'accrualStart' },
		value: { label: 'Wertstellung', sort: 'valueDate' },
		booking: { label: 'Buchungstag', sort: 'bookingDate' }
	} as const;
	type DateAxis = keyof typeof DATE_AXES;

	let dateAxis = $state<DateAxis>('accrual');
	const axis = $derived(DATE_AXES[dateAxis]);

	/// only consulted below `md`, where the filters are collapsed
	let filtersOpen = $state(false);

	let showNewExpenseModal = $state(false);
	let showEditExpenseModal = $state(false);
	let showDeleteExpenseModal = $state(false);
	let expenseIdToEdit: string | undefined = $state(undefined);
	let expenseIdToDelete: string | undefined = $state(undefined);
	let expenseIdToResort: string | undefined = $state(undefined);
	/// The edit modal, filled from an expense but creating a second one. One instance rather
	/// than two: the form is the same form, and only where it submits changes.
	let duplicating = $state(false);

	function openEditModal(id: string, asCopy = false) {
		expenseIdToEdit = id;
		duplicating = asCopy;
		showEditExpenseModal = true;
	}

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

	/// One expense, named by whoever linked here — the template behind an automation rule is
	/// years old and would be nowhere near the first page. Kept as a filter rather than fetched
	/// on the side so the list still says what it is showing, and so it can be cleared.
	const handover = page.url.searchParams.get('expense') ?? '';
	let handedOver = $state(handover);
	// a rule's template is usually flagged as one, and templates are hidden by default, so
	// naming one and then filtering it back out would land on an empty table
	if (handover) showTemplates = true;

	if (page.url.searchParams.has('review')) {
		showTaxRelevant = false;
		showExtraordinary = false;
		showNormal = false;
	}

	const category = page.url.searchParams.get('category');
	if (category) filteredCategories = [category];

	// Which date the period below is about. Without it a handover lands on whichever axis this
	// table happens to default to, and a month of Wertstellung read as a month of
	// Leistungszeitraum is a different set of expenses.
	const handedAxis = page.url.searchParams.get('axis');
	if (handedAxis && handedAxis in DATE_AXES) dateAxis = handedAxis as DateAxis;

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

	/// How much of an expense the filtered period actually holds, where that is less than all
	/// of it.
	///
	/// Only on the accrual axis, and only with a period filter: those two together are the
	/// reader asking what a stretch of time cost, and an expense whose Leistungszeitraum runs
	/// past both ends of it did not cost that much inside it. The other two axes file an
	/// expense on a single day, so there is never a part of one to show.
	///
	/// The sort still ranks by the whole amount -- the backend has no notion of this share --
	/// so a column of parts is not in order. Worth it: the alternative is a figure that answers
	/// a different question from the one the filter asks.
	const accrualWindow = $derived.by(() => {
		const { from, to } = filterDateRange;
		if (dateAxis !== 'accrual' || (!from && !to)) return null;

		// the picker's `to` is the last day the reader means, and the share wants the bound
		// after it
		return { from, before: to ? new Date(to.getTime() + 86400000) : undefined };
	});

	const partOf = (expense: { accrualStart: Date; accrualEnd: Date }) =>
		accrualWindow ? accrualShare(expense, accrualWindow.from, accrualWindow.before) : 1;

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

		// A list narrowed to a single expense looks like an almost-empty database, so this one
		// has to be as visible and as removable as any other filter.
		if (handedOver) {
			out.push({ label: 'Einzelne Ausgabe', clear: () => (handedOver = '') });
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

		// named, because the same range means a different set of expenses per date column
		if (dateRangeLabel) {
			out.push({ label: `${axis.label}: ${dateRangeLabel}`, clear: clearDateRange });
		}

		// one chip for the lot: which of the six is off matters less than the fact that the
		// list is no longer showing everything
		if (!flagsAreDefault) {
			out.push({ label: 'Status', clear: resetFlags });
		}

		return out;
	});

	const withoutOption = (selected: string[], id: string) =>
		selected.filter((other) => other !== id);

	function clearDateRange() {
		filterDateRange = { from: undefined, to: undefined };
	}

	/// Whether the six state checkboxes are where `resetFlags` leaves them. Not "all six on":
	/// templates are hidden by default, so an untouched list would otherwise report itself as
	/// filtered. This is what both the chip and the button's count hang off.
	const flagsAreDefault = $derived(
		!showTemplates &&
			showTaxRelevant &&
			showExtraordinary &&
			showUnchecked &&
			showIncomplete &&
			showNormal
	);

	/// How many states are being shown, but only once that is worth saying.
	const activeFlagCount = $derived(
		flagsAreDefault
			? 0
			: [
					showNormal,
					showUnchecked,
					showIncomplete,
					showExtraordinary,
					showTaxRelevant,
					showTemplates
				].filter(Boolean).length
	);

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
		valueDate: 'VALUE_DATE',
		bookingDate: 'BOOKING_DATE',
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
		search: search.current || undefined,
		ids: handedOver ? [handedOver] : undefined,
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
		// the period filter follows the column: "März" means the month it was for, or the month
		// it was paid in, depending on which of them the reader is looking at
		accruedAfter: dateAxis === 'accrual' ? filterDateRange.from : undefined,
		accruedBefore: dateAxis === 'accrual' ? filterDateRange.to : undefined,
		valuedAfter: dateAxis === 'value' ? filterDateRange.from : undefined,
		valuedBefore: dateAxis === 'value' ? filterDateRange.to : undefined,
		bookedAfter: dateAxis === 'booking' ? filterDateRange.from : undefined,
		bookedBefore: dateAxis === 'booking' ? filterDateRange.to : undefined,
		sortBy: SORT_FIELDS[sort.field],
		sortDir: (sort.direction < 0 ? 'DESC' : 'ASC') as SortDirection$options
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
					usageCount
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
					usageCount
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
			$valuedAfter: DateTime
			$valuedBefore: DateTime
			$bookedAfter: DateTime
			$bookedBefore: DateTime
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
					valuedAfter: $valuedAfter
					valuedBefore: $valuedBefore
					bookedAfter: $bookedAfter
					bookedBefore: $bookedBefore
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
								changes {
									path
									old
									new
								}
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
								bookingDate
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

	// Same order as the pickers in the edit dialog: what you use, then alphabetical. A filter is
	// reached for by name more often than a picker is, which is why only the top few jump the
	// queue rather than the whole list being ranked.
	const filterableCategories = $derived(
		byUsageThen($basicStore.data?.categories.list ?? [], categorySorter).map((category) => ({
			name: category.name,
			value: category.id,
			color: category.color
		}))
	);

	const filterableAccounts = $derived(
		byUsageThen($basicStore.data?.accounts.list ?? [], (a, b) =>
			accountSorter(a, b, $basicStore.data?.users.me.name || '')
		).map((account) => ({
			name:
				account.owner.name == $basicStore.data?.users.me.name
					? account.name
					: `${account.owner.firstName} / ${account.name}`,
			value: account.id,
			color: account.color
		}))
	);

	/// Account and category ids to what they are called, for the changelog: a stored state
	/// carries ids and nothing else, so a changed account would otherwise read as a uuid.
	const changeNames = $derived(
		Object.fromEntries([
			...($basicStore.data?.categories.list ?? []).map((c) => [c.id.toString(), c.name]),
			...($basicStore.data?.accounts.list ?? []).map((a) => [
				a.id.toString(),
				formatAccount(a, $basicStore.data?.users.me.name)
			])
		])
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

	// Opened once the row is actually here: the modal reads its expense out of the loaded page,
	// and opening it against a list that has not arrived shows the empty form for a moment.
	let handoverOpened = false;
	$effect(() => {
		if (handoverOpened || !handedOver) return;
		if (!expenses.some((e) => e.id === handedOver)) return;

		handoverOpened = true;
		openEditModal(handedOver);
	});
	let expenseToDelete = $derived(expenses.find((e) => e.id === expenseIdToDelete) || undefined);
	let expenseToResort = $derived(expenses.find((e) => e.id === expenseIdToResort) || undefined);

	/// The bank's own text for an expense, which is several strings when it is split over more
	/// than one transaction. Newlines collapse: the cell is one line and the full text is in the
	/// expanded row.
	const transactionText = (transactions: { statement: string | null }[]) =>
		transactions
			.map((t) => t.statement?.replace(/\s+/g, ' ').trim())
			.filter(Boolean)
			.join(' · ');

	/// The same, cut to the line of it worth reading in a cell one line high.
	const transactionSummary = (transactions: { statement: string | null }[]) =>
		transactions
			.map((t) => (t.statement ? statementLine(t.statement) : ''))
			.filter(Boolean)
			.join(' · ');

	/// How many columns the header is showing, which is what the detail row has to span.
	///
	/// A fixed layout lays out as many columns as the widest row declares, so `colspan="9"` over
	/// the three columns a phone shows invents six empty ones and shares the width between all
	/// nine -- the title ended up a character wide. Auto layout collapsed them to nothing, which
	/// is why this only became a problem with `table-fixed`.
	let columns = $state(9);
	$effect(() => {
		const wide = window.matchMedia('(min-width: 768px)');
		const widest = window.matchMedia('(min-width: 1536px)');
		const update = () => (columns = wide.matches ? (widest.matches ? 9 : 8) : 3);

		update();
		wide.addEventListener('change', update);
		widest.addEventListener('change', update);
		return () => {
			wide.removeEventListener('change', update);
			widest.removeEventListener('change', update);
		};
	});

	/// The colour an amount is written in. Zero is its own answer rather than the absence of one:
	/// an expense that nets out moved money between two of your own accounts. `gray-200` rather
	/// than the `-100` the other two use, since `-100` is the shade a hovered row is.
	const amountTint = (amount: number) =>
		amount < 0
			? 'bg-red-100 text-red-700'
			: amount > 0
				? 'bg-green-100 text-green-700'
				: 'bg-gray-200 text-gray-600';

	/// A title that names no expense: the automation rules give one this when their template has
	/// no title of its own, and it then says the same thing on every row it produced. Tinting it
	/// is the only thing in the list that says "this one still needs a human".
	const PLACEHOLDER_TITLE = 'Automatisch Generiert';
	const titleIsPlaceholder = (title: string) => !title.trim() || title.trim() === PLACEHOLDER_TITLE;

	function toggleRow(id: string) {
		if (expandedRows.includes(id)) expandedRows = expandedRows.filter((rowId) => rowId !== id);
		else expandedRows = [...expandedRows, id];
	}

	/// What the toolbar's Bearbeiten and Löschen act on. An open row is this list's idea of "the
	/// one you mean", and several open rows are no answer at all rather than a reason to pick the
	/// first -- deleting the wrong expense is not something to be clever about.
	const selectedExpenseId = $derived(expandedRows.length === 1 ? expandedRows[0] : undefined);

	/// Descending, ascending, off. The third state is what there was no way back to: sorting
	/// lives in the component rather than in the url, so a header clicked by accident could only
	/// Switching the column takes the sort with it, but only if the sort was on a date: someone
	/// who ordered by amount asked for that, and it does not become a different question here.
	function setDateAxis(next: DateAxis) {
		const onDate = Object.values(DATE_AXES).some((a) => a.sort === sort.field);
		if (onDate) sort.current = { field: DATE_AXES[next].sort, direction: sort.direction };
		dateAxis = next;
		currentPage = 1;
	}

	/// What the date column shows: the accrual period, or the span the transactions cover.
	///
	/// Value and booking dates sit on the transactions, and an expense can have several -- a
	/// refund is the debit and the credit months apart, a chained prepayment more still. Both
	/// ends, then, and `formatAccrualPeriod` collapses them when they agree, which is the
	/// common case.
	///
	/// A transaction no bank posted has no booking date and falls back to its own
	/// Wertstellung: where a bank did post one the two are a day or two apart anyway, so it is
	/// the closest thing to an answer. Better than a hole in the column, and it is what the
	/// sort and the period filter assume as well.
	function axisPeriod(expense: {
		accrualStart: Date;
		accrualEnd: Date;
		transactions: { date: Date; bookingDate: Date | null }[];
	}): [Date, Date] {
		if (dateAxis === 'accrual') return [expense.accrualStart, expense.accrualEnd];

		const dates = expense.transactions
			.map((t) => (dateAxis === 'value' ? t.date : (t.bookingDate ?? t.date)))
			.map((d) => d.getTime());
		if (dates.length === 0) return [expense.accrualStart, expense.accrualEnd];

		return [new Date(Math.min(...dates)), new Date(Math.max(...dates))];
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
		invalidate();
		await expenseStore.fetch({ variables: expenseVariables(), ...AFTER_WRITE });
		expandedRows = [];
	}

	const ResortExpense = graphql(`
		mutation ResortExpense($id: UUID!) {
			expenses {
				resort(id: $id) {
					id
				}
			}
		}
	`);

	let showResortExpenseModal = $state(false);
	let resortError: string | undefined = $state(undefined);

	async function handleResortExpense() {
		if (!expenseToResort) return;
		resortError = undefined;

		const result = await ResortExpense.mutate({ id: expenseToResort.id });
		// The two things that go wrong here are the reader's to fix -- no rule matches, or the
		// expense never came from a bank -- so the modal says so and stays open rather than
		// closing on a sentence nobody read.
		if (result.errors?.length) {
			resortError = result.errors.map((e) => e.message).join(' ');
			return;
		}

		showResortExpenseModal = false;
		expenseIdToResort = undefined;

		invalidate();
		await expenseStore.fetch({ variables: expenseVariables(), ...AFTER_WRITE });
		expandedRows = [];
	}

	async function handleDeleteExpense() {
		if (!expenseToDelete) return;
		await DeleteExpense.mutate({ id: expenseToDelete.id });
		expenseIdToDelete = undefined;
		showDeleteExpenseModal = false;

		invalidate();
		await expenseStore.fetch({ variables: expenseVariables(), ...AFTER_WRITE });
		expandedRows = [];
	}
</script>

{#snippet newExpenseButton(cls: string)}
	<!-- `primary` spelled out: inside a ButtonGroup a Button with no colour of its own falls back
	     to `alternative` rather than to primary, which left Neu white in the middle of the row. -->
	<Button
		color="primary"
		class="{toolbarControl} {cls}"
		onclick={() => (showNewExpenseModal = true)}
	>
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
	</Button>
{/snippet}

{#snippet fieldLabel(text: string)}
	<dt class="text-xs font-semibold text-gray-500 dark:text-gray-400">{text}</dt>
{/snippet}

{#snippet field(label: string, value: string)}
	<div class="py-2">
		{@render fieldLabel(label)}
		<dd class="mt-0.5 wrap-anywhere">{value}</dd>
	</div>
{/snippet}

{#snippet sectionLabel(text: string)}
	<h4 class="mb-1 text-xs font-semibold text-gray-500 dark:text-gray-400">{text}</h4>
{/snippet}

{#snippet statusBadges(expense: {
	isTaxRelevant: boolean;
	isTemplate: boolean;
	isUnchecked: boolean;
	isIncomplete: boolean;
	isExtraordinary: boolean;
})}
	{#if expense.isTaxRelevant}
		<span class="rounded-md bg-green-100 px-2 py-1 text-green-700">
			<LandmarkOutline class="me-1 inline h-3.5 w-3.5 align-text-bottom" />Steuerrelevant
		</span>
	{/if}
	{#if expense.isTemplate}
		<span class="rounded-md bg-blue-100 px-2 py-1 text-blue-700">
			<FileCloneOutline class="me-1 inline h-3.5 w-3.5 align-text-bottom" />Vorlage
		</span>
	{/if}
	{#if expense.isUnchecked}
		<span class="rounded-md bg-yellow-100 px-2 py-1 text-yellow-700">
			<QuestionCircleOutline class="me-1 inline h-3.5 w-3.5 align-text-bottom" />Unbestätigt
		</span>
	{/if}
	{#if expense.isIncomplete}
		<span class="rounded-md bg-red-100 px-2 py-1 text-red-700">
			<ExclamationCircleOutline class="me-1 inline h-3.5 w-3.5 align-text-bottom" />Unvollständig
		</span>
	{/if}
	{#if expense.isExtraordinary}
		<span class="rounded-md bg-purple-100 px-2 py-1 text-purple-700">
			<StarOutline class="me-1 inline h-3.5 w-3.5 align-text-bottom" />Außergewöhnlich
		</span>
	{/if}
{/snippet}

<Modal bind:open={showResortExpenseModal} size="sm">
	<div class="text-center">
		<ExclamationCircleOutline class="mx-auto mb-4 h-12 w-12 text-gray-400 dark:text-gray-200" />
		<h3 class="mb-3 text-lg font-normal text-gray-500 dark:text-gray-400">
			Soll <b>{expenseToResort?.title}</b> neu einsortiert werden?
		</h3>
		<!-- Said plainly because it cannot be undone from here: what the rule decides replaces
		     what is there, including anything corrected by hand since. -->
		<p class="mb-5 text-sm text-gray-500 dark:text-gray-400">
			Die Automatisierungsregeln laufen erneut über die Ausgabe, als wäre ihre Bankzeile gerade erst
			geliefert worden. Titel, Kategorien, Konten und Merkmale werden von der passenden Regel
			überschrieben; Betrag, Datum und Transaktionstext bleiben.
		</p>
		{#if resortError}
			<p class="mb-5 text-sm text-red-600 dark:text-red-400">{resortError}</p>
		{/if}
		<div class="space-x-2">
			<Button type="submit" value="resort" onclick={handleResortExpense}>Einsortieren</Button>
			<Button
				type="submit"
				value="cancel"
				color="alternative"
				onclick={() => {
					showResortExpenseModal = false;
					expenseIdToResort = undefined;
					resortError = undefined;
				}}
			>
				Abbrechen
			</Button>
		</div>
	</div>
</Modal>

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
		color: a.color,
		closed: a.closed,
		usageCount: a.usageCount
	})) || []}
	categories={$basicStore.data?.categories.list.map((c) => ({
		id: c.id.toString(),
		name: c.name,
		color: c.color,
		usageCount: c.usageCount
	})) || []}
/>

<ExpenseModal
	bind:open={showEditExpenseModal}
	onsubmit={handleExpenseModalSubmit}
	userName={$basicStore.data?.users.me?.name}
	expense={expenseToEdit}
	duplicate={duplicating}
	accounts={$basicStore.data?.accounts.list.map((a) => ({
		id: a.id.toString(),
		name: a.name,
		owner: { name: a.owner.name, firstName: a.owner.firstName },
		color: a.color,
		closed: a.closed,
		usageCount: a.usageCount
	})) || []}
	categories={$basicStore.data?.categories.list.map((c) => ({
		id: c.id.toString(),
		name: c.name,
		color: c.color,
		usageCount: c.usageCount
	})) || []}
/>

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-0 sm:p-5">
	<QueryError
		errors={[...($expenseStore.errors ?? []), ...($basicStore.errors ?? [])]}
		message="Die Ausgaben konnten nicht geladen werden."
		class="mb-4"
	/>
	<FilterChips filters={activeFilters} reset={resetFilters} />

	<TableSearch
		placeholder="Suchen"
		hoverable={true}
		bind:inputValue={searchTerm}
		class="{tableSearchTableClass} table-fixed"
		classes={tableSearchClasses}
	>
		{#snippet header()}
			<div
				class="flex w-full flex-col items-stretch justify-start gap-2 md:w-auto md:flex-row md:items-center md:gap-3"
			>
				<div class="flex items-center gap-2">
					<!-- One group, in moneydb's order. Everything but Neu acts on whichever row is open,
					     so on a phone the group goes and Neu stands on its own: the row you opened fills
					     the screen there and carries the same four buttons itself. -->
					{@render newExpenseButton('grow md:hidden')}
					<ButtonGroup class="hidden md:inline-flex">
						{@render newExpenseButton('')}
						<ActionButton
							color="teal"
							class={toolbarControl}
							disabled={!selectedExpenseId}
							onclick={() => openEditModal(selectedExpenseId!, true)}
						>
							<FileCopyAltOutline class="mr-2 h-3.5 w-3.5" />Duplizieren
						</ActionButton>
						<ActionButton
							color="yellow"
							class="{toolbarControl} text-gray-900"
							disabled={!selectedExpenseId}
							onclick={() => openEditModal(selectedExpenseId!)}
						>
							<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
						</ActionButton>
						<ActionButton
							color="red"
							class={toolbarControl}
							disabled={!selectedExpenseId}
							onclick={() => {
								expenseIdToDelete = selectedExpenseId;
								showDeleteExpenseModal = true;
							}}
						>
							<TrashBinOutline class="mr-2 h-3.5 w-3.5" />Löschen
						</ActionButton>
						<ActionButton
							color="gray"
							class={toolbarControl}
							disabled={!selectedExpenseId}
							onclick={() => {
								expenseIdToResort = selectedExpenseId;
								resortError = undefined;
								showResortExpenseModal = true;
							}}
						>
							<ArchiveArrowDownOutline class="mr-2 h-3.5 w-3.5" />Einsortieren
						</ActionButton>
					</ButtonGroup>
					<!-- Stacked, the filters filled a phone screen before a single expense appeared,
					     so on small screens they fold behind this. -->
					<Button
						color="alternative"
						class="{toolbarControl} md:hidden"
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
					<!-- min-w rather than w: a chosen range is "05.08.2026 – 14.08.2026", which is wider than
					     the other triggers and was running out of the button. Empty it still matches them. -->
					<Button
						color="alternative"
						class="{toolbarControl} justify-between whitespace-nowrap md:min-w-52"
					>
						<span class="flex items-center gap-2">
							<CalendarMonthOutline class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
							{dateRangeLabel ?? axis.label}
						</span>
						<ChevronDownOutline class="ml-2 h-3 w-3 shrink-0" />
					</Button>
					<Dropdown class="flex w-fit flex-col gap-2 divide-y-0 p-2">
						<!-- Which date, above the calendar that picks the range on it. The two belong
						     together: the range means a different set of expenses per date, and this is
						     also what the column shows and sorts by. -->
						<Select
							value={dateAxis}
							onchange={(e) => setDateAxis((e.target as HTMLSelectElement).value as DateAxis)}
							items={Object.entries(DATE_AXES).map(([value, a]) => ({ value, name: a.label }))}
							size="sm"
							selectClass="h-9 py-0"
							placeholder=""
						/>
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
						<ActionButton
							size="xs"
							color="light"
							disabled={!dateRangeLabel}
							onclick={clearDateRange}
						>
							Zeitraum zurücksetzen
						</ActionButton>
					</Dropdown>
					<OptionFilter
						label="Kategorien"
						options={filterableCategories ?? []}
						selected={filteredCategories}
						toggle={(id) => (filteredCategories = toggleOption(filteredCategories, id))}
					>
						{#snippet icon()}
							<TagOutline class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
						{/snippet}
					</OptionFilter>
					<OptionFilter
						label="Konten"
						options={filterableAccounts ?? []}
						selected={filteredAccounts}
						toggle={(id) => (filteredAccounts = toggleOption(filteredAccounts, id))}
					>
						{#snippet icon()}
							<WalletOutline class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
						{/snippet}
					</OptionFilter>
					<!-- The third of three filters, and it has to look like the other two: same trigger,
					     same panel. It said "Filter" over a flowbite List, which drew a rule under a
					     heading repeating the word next to it and spaced six checkboxes like a table
					     of contents. It filters the Status column and is named after it. -->
					<Button color="alternative" class="{toolbarControl} justify-between md:w-40">
						<span class="flex items-center gap-2">
							<FlagOutline class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
							{activeFlagCount ? `Status (${activeFlagCount})` : 'Status'}
						</span>
						<ChevronDownOutline class="ml-2 h-3 w-3 shrink-0" />
					</Button>
					<Dropdown class="max-h-80 w-64 divide-y-0 overflow-y-auto p-3 text-sm">
						<!-- The same icon per state as the badges below and the flags in the edit
						     dialog: six words of similar length in one weight are read one after
						     another, six shapes are recognised. -->
						<div class="space-y-2">
							<Checkbox bind:checked={showNormal}>
								<CheckCircleOutline class="me-1.5 h-4 w-4 shrink-0" />Normal
							</Checkbox>
							<Checkbox bind:checked={showUnchecked}>
								<QuestionCircleOutline class="me-1.5 h-4 w-4 shrink-0" />Unbestätigt
							</Checkbox>
							<Checkbox bind:checked={showIncomplete}>
								<ExclamationCircleOutline class="me-1.5 h-4 w-4 shrink-0" />Unvollständig
							</Checkbox>
							<Checkbox bind:checked={showExtraordinary}>
								<StarOutline class="me-1.5 h-4 w-4 shrink-0" />Außergewöhnlich
							</Checkbox>
							<Checkbox bind:checked={showTaxRelevant}>
								<LandmarkOutline class="me-1.5 h-4 w-4 shrink-0" />Steuerrelevant
							</Checkbox>
							<Checkbox bind:checked={showTemplates}>
								<FileCloneOutline class="me-1.5 h-4 w-4 shrink-0" />Vorlage
							</Checkbox>
						</div>
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
			<!-- The header only says which date it is showing; the period filter picks it, since
			     that is where the range it applies to is chosen too. min-w rather than w, because
			     the names are wider than the dates under them and a fixed width clipped them.
			     On a phone it wraps instead: "Leistungszeitraum" on one line would take half the
			     row from the title, and `wrap-anywhere` is what breaks a word with no space in
			     it. -->
			<TableHeadCell
				class="w-28 py-3 pr-2 pl-7 wrap-anywhere md:w-auto md:min-w-36 md:pr-4 md:pl-10 md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by(axis.sort)}
			>
				<CalendarMonthOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>{axis.label}
				<SortIcon direction={sort.on(axis.sort)} />
			</TableHeadCell>
			<TableHeadCell class="px-2 py-3 md:px-4" scope="col" onclick={() => sort.by('title')}>
				<TextSizeOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Titel
				<SortIcon direction={sort.on('title')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-36"
				scope="col"
				onclick={() => sort.by('accounts')}
			>
				<WalletOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Konten
				<SortIcon direction={sort.on('accounts')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-36"
				scope="col"
				onclick={() => sort.by('categories')}
			>
				<TagOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Kategorien
				<SortIcon direction={sort.on('categories')} />
			</TableHeadCell>
			<TableHeadCell
				class="w-28 px-2 py-3 text-right md:w-32 md:px-4"
				scope="col"
				onclick={() => sort.by('effectiveAmount')}
			>
				<EuroOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Betrag
				<SortIcon direction={sort.on('effectiveAmount')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-32"
				scope="col"
				onclick={() => sort.by('store')}
			>
				<StoreOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Geschäft
				<SortIcon direction={sort.on('store')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-56"
				scope="col"
				onclick={() => sort.by('description')}
			>
				<AlignLeftOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Beschreibung
				<SortIcon direction={sort.on('description')} />
			</TableHeadCell>
			<!-- What the bank wrote, cut to the line of it that says something: see `statementLine`.
			     No sort arrow: the backend names its sortable columns and this is not one of them. From 2xl only, and it shares what the fixed columns leave with
			     the title: at 1280 the nine columns together are wider than the window, and both of
			     them end up too narrow to read. Below that the expanded row still has it in full. -->
			<TableHeadCell class="hidden px-4 py-3 2xl:table-cell" scope="col">
				<ReceiptOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Transaktionstext
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:w-36"
				scope="col"
				onclick={() => sort.by('status')}
			>
				<FlagOutline
					class="me-1.5 inline h-4 w-4 align-[-0.15em] text-gray-400 dark:text-gray-500"
				/>Status
				<SortIcon direction={sort.on('status')} />
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#if slow}
				<tr>
					<td colspan={columns} class="px-4 py-16 text-center">
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
						{@const period = axisPeriod(expense)}
						{formatAccrualPeriod(period[0], period[1], false)}
					</TableBodyCell>
					<!-- `w-full max-w-0` is what makes the inner block truncate: a table cell is as wide
					     as its content unless something says otherwise, and `truncate` on the cell
					     itself is gone now that the cell holds two lines rather than one string. Zero
					     max-width plus full width leaves it exactly the space the other columns did
					     not take. -->
					<TableBodyCell
						class="px-2 py-3 whitespace-normal md:px-4 {titleIsPlaceholder(expense.title)
							? 'bg-red-50 dark:bg-red-950/40'
							: ''}"
					>
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
					{@const part = partOf(expense)}
					<TableBodyCell
						class={'px-2 py-3 text-right whitespace-nowrap md:px-4 ' +
							amountTint(expense.effectiveAmount)}
					>
						{signedEuro(expense.effectiveAmount * part)}
						<!-- under the figure rather than beside it: the column is as wide as an
						     amount, and a second one on the same line was cut off by the one after
						     it. Unsigned, because the sign is already above it. -->
						{#if part < 1}
							<div class="text-xs font-normal text-gray-500 dark:text-gray-400">
								von {euro(Math.abs(expense.effectiveAmount))}
							</div>
						{/if}
					</TableBodyCell>
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
						{transactionSummary(expense.transactions)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{@render statusBadges(expense)}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(expense.id)}
					<tr data-expanded>
						<td colspan={columns} class="border-b px-4 py-4">
							<!-- Two columns from lg. Everything here used to be one narrow stack down the
							     left of a table two thousand pixels wide, so a statement of a dozen lines
							     pushed the buttons under it off the screen while most of the row stayed
							     empty. What is prose goes left, what is a table goes right. -->
							<div class="grid gap-x-8 gap-y-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
								<dl class="divide-y divide-gray-200 dark:divide-gray-700">
									{@render field('Titel', expense.title || 'Ohne Titel')}
									{@render field(
										'Leistungszeitraum',
										formatAccrualPeriod(expense.accrualStart, expense.accrualEnd, true)
									)}
									{#if expense.description}
										{@render field('Beschreibung', expense.description)}
									{/if}
									{#if expense.store}
										{@render field('Geschäft', expense.store)}
									{/if}
									{#if expense.comments}
										{@render field('Kommentare', expense.comments)}
									{/if}
									<div class="py-2">
										{@render fieldLabel('Status')}
										<dd class="mt-1 flex flex-wrap gap-1 text-xs">
											{#if expense.isTaxRelevant || expense.isTemplate || expense.isUnchecked || expense.isIncomplete || expense.isExtraordinary}
												{@render statusBadges(expense)}
											{:else}
												<span class="text-sm text-gray-500 dark:text-gray-400">Normal</span>
											{/if}
										</dd>
									</div>
									{#each expense.transactions.filter((t) => t.statement) as transaction, i (i)}
										<div class="py-2">
											{@render fieldLabel(
												expense.transactions.filter((t) => t.statement).length > 1
													? `Transaktionstext · ${formatAccount(transaction.account, $basicStore.data?.users.me.name)}`
													: 'Transaktionstext'
											)}
											<!-- the bank's own text, kept as it came: the line breaks in it separate
											     one key from the next and are the only structure it has -->
											<dd
												class="mt-1 font-mono text-xs whitespace-pre-wrap text-gray-600 dark:text-gray-300"
											>
												{transaction.statement}
											</dd>
										</div>
									{/each}
								</dl>

								<div class="space-y-6">
									<div>
										{@render sectionLabel('Transaktionen')}
										<table
											class="w-full border border-gray-200 bg-white text-left text-sm dark:border-gray-700 dark:bg-gray-900"
										>
											<thead>
												<tr class="bg-gray-50 dark:bg-gray-800">
													<th class="px-3 py-2" scope="col">Konto</th>
													<th class="px-3 py-2 text-right" scope="col">Betrag</th>
													<th class="px-3 py-2" scope="col">Grund</th>
												</tr>
											</thead>
											<tbody class="divide-y">
												{#each expense.transactions.toSorted((a, b) => Math.abs(b.effectiveAmount) - Math.abs(a.effectiveAmount)) as transaction, i (i)}
													{@const amount = transaction.fraction
														? transaction.fraction * (expense.referenceAmount ?? 0)
														: (transaction.amount ?? 0)}
													<tr>
														<td
															class="px-3 py-2"
															style={makeAccountColor([transaction], expense.referenceAmount)}
														>
															{formatAccount(transaction.account, $basicStore.data?.users.me.name)}
														</td>
														<td
															class={'px-3 py-2 text-right whitespace-nowrap ' + amountTint(amount)}
														>
															{signedEuro(amount)}
														</td>
														<td class="px-3 py-2 text-gray-500 dark:text-gray-400">
															{#if transaction.fraction}
																Anteilig · {(transaction.fraction * 100).toFixed(0)}% von {(
																	(expense.referenceAmount ?? 0) / 100
																).toFixed(2)}€
															{:else}
																Direkt
															{/if}
															<!-- only when it is news: a transaction booked inside its own accrual
															     period tells you nothing you did not already read above -->
															{#if transaction.date.toISOString() !== expense.accrualStart.toISOString() || transaction.date.toISOString() !== expense.accrualEnd.toISOString()}
																<span class="block text-xs">
																	gebucht {transaction.date.toLocaleString('de-DE', {
																		year: 'numeric',
																		month: '2-digit',
																		day: '2-digit',
																		hour: '2-digit',
																		minute: '2-digit'
																	})}
																</span>
															{/if}
														</td>
													</tr>
												{/each}
											</tbody>
										</table>
									</div>

									{#if expense.categories.length > 0}
										<div>
											{@render sectionLabel('Kategorien')}
											<table
												class="w-full border border-gray-200 bg-white text-left text-sm dark:border-gray-700 dark:bg-gray-900"
											>
												<thead>
													<tr class="bg-gray-50 dark:bg-gray-800">
														<th class="px-3 py-2" scope="col">Kategorie</th>
														<th class="px-3 py-2 text-right" scope="col">Gewicht</th>
													</tr>
												</thead>
												<tbody class="divide-y">
													{#each expense.categories as category (category.category.id)}
														<tr>
															<td class="px-3 py-2" style={makeCategoryColor([category])}>
																{category.category.name}
															</td>
															<td class="px-3 py-2 text-right">{category.weight}</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</div>
									{/if}

									{#if expense.positions.length > 0}
										<div>
											{@render sectionLabel('Positionen')}
											<table
												class="w-full border border-gray-200 bg-white text-left text-sm dark:border-gray-700 dark:bg-gray-900"
											>
												<thead>
													<tr class="bg-gray-50 dark:bg-gray-800">
														<th class="px-3 py-2" scope="col">Position</th>
														<th class="px-3 py-2 text-right" scope="col">Menge</th>
														<th class="px-3 py-2 text-right" scope="col">Betrag</th>
													</tr>
												</thead>
												<tbody class="divide-y">
													{#each expense.positions as position, i (i)}
														<tr>
															<td class="px-3 py-2">
																{position.title}
																{#if position.comments}
																	<span class="block text-xs text-gray-500 dark:text-gray-400"
																		>{position.comments}</span
																	>
																{/if}
															</td>
															<td class="px-3 py-2 text-right whitespace-nowrap">
																{position.quantity ?? ''}
																{position.unit ?? ''}
															</td>
															<td class="px-3 py-2 text-right whitespace-nowrap">
																{position.amount != null
																	? (position.amount / 100).toFixed(2) + '€'
																	: ''}
															</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</div>
									{/if}

									{#if expense.receipts.length > 0}
										<div>
											{@render sectionLabel('Belege')}
											<!-- eslint-disable svelte/no-navigation-without-resolve -- receipt uris point outside the app (dms, file store) -->
											<ul class="space-y-1 text-sm">
												{#each expense.receipts as receipt (receipt.uri)}
													<li>
														<a
															class="text-primary-600 dark:text-primary-400 break-all hover:underline"
															href={receipt.uri}
															target="_blank">{receipt.uri}</a
														>
													</li>
												{/each}
											</ul>
											<!-- eslint-enable svelte/no-navigation-without-resolve -->
										</div>
									{/if}

									<div>
										{@render sectionLabel('Changelog')}
										<!-- Newest first, which is the order the resolver returns and the order the
									     activity feed shows: what an expense last had done to it is the thing
									     somebody opening it wants, and its creation is the least of it. -->
										<ol class="ml-1 space-y-2.5 text-sm">
											{#each expense.changelog as change (change.id)}
												<li class="relative border-l border-gray-300 pl-4 dark:border-gray-600">
													<!-- A dot on the rule, which is all a timeline is at this size. flowbite's
							     Timeline draws the same thing around a `text-lg` heading and 40px of
							     margin per entry -- four entries would be taller than the row they sit
							     in, and overriding that away leaves nothing of the component. -->
													<span
														class="absolute top-1.5 -left-[4.5px] h-2 w-2 rounded-full bg-gray-300 dark:bg-gray-600"
													></span>
													<div class="text-xs text-gray-500 dark:text-gray-400">
														{change.user!.name == $basicStore.data?.users.me.name
															? 'Du'
															: change.user!.firstName} · {change.date.toLocaleString('de-DE', {
															year: 'numeric',
															month: '2-digit',
															day: '2-digit',
															hour: '2-digit',
															minute: '2-digit'
														})}{change.tool ? ` · ${change.tool}` : ''}{change.automatic
															? ' · automatisch'
															: ''}
													</div>
													{#if !change.oldState}
														<div class="text-xs text-gray-500 dark:text-gray-400">
															Ausgabe erstellt
														</div>
													{:else if change.changes.length === 0}
														<!-- An entry with nothing to report is one whose two states are the same,
													     which is what an import writes: the export carries the expense as it is
													     now and not as it was, so `import.rs` stores that same state on both
													     sides. Something was changed and there is no way to say what. -->
														<div class="text-xs text-gray-500 dark:text-gray-400">Geändert</div>
													{:else}
														<ChangeList changes={change.changes} names={changeNames} class="mt-1" />
													{/if}
												</li>
											{/each}
										</ol>
									</div>
								</div>
							</div>

							<!-- The toolbar's group again, in the same order and minus Neu, which is the one
							     button up there that is not about a particular expense. A ButtonGroup sizes
							     its children itself, so there is no `size` to set here. -->
							<div class="mt-6">
								<ButtonGroup>
									<Button color="teal" onclick={() => openEditModal(expense.id.toString(), true)}>
										<FileCopyAltOutline class="mr-2 h-3.5 w-3.5" />
										Duplizieren
									</Button>
									<Button
										color="yellow"
										class="text-gray-900"
										onclick={() => openEditModal(expense.id.toString())}
									>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="red"
										onclick={() => {
											expenseIdToDelete = expense.id.toString();
											showDeleteExpenseModal = true;
										}}
									>
										<TrashBinOutline class="mr-2 h-3.5 w-3.5" />
										Löschen
									</Button>
									<Button
										color="gray"
										onclick={() => {
											expenseIdToResort = expense.id.toString();
											resortError = undefined;
											showResortExpenseModal = true;
										}}
									>
										<ArchiveArrowDownOutline class="mr-2 h-3.5 w-3.5" />
										Einsortieren
									</Button>
								</ButtonGroup>
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
					<ActionButton onclick={() => currentPage--} disabled={currentPage <= 1}>
						<ChevronLeftOutline size="xs" class="m-1.5" />
					</ActionButton>
					{#if currentPage > 3}
						<ActionButton
							class="hidden md:inline-flex"
							disabled={currentPage === 1}
							onclick={() => (currentPage = 1)}>1</ActionButton
						>
						<Button class="hidden md:inline-flex" disabled>...</Button>
					{/if}
					{#each [currentPage - 2, currentPage - 1, currentPage, currentPage + 1, currentPage + 2].filter((page) => page > 0 && page <= totalPages) as pageNumber (pageNumber)}
						<ActionButton
							class="hidden md:inline-flex"
							disabled={currentPage === pageNumber}
							onclick={() => (currentPage = pageNumber)}>{pageNumber}</ActionButton
						>
					{/each}
					{#if currentPage < totalPages - 2}
						<Button class="hidden md:inline-flex" disabled>...</Button>
						<ActionButton
							class="hidden md:inline-flex"
							disabled={currentPage === totalPages}
							onclick={() => (currentPage = totalPages)}>{totalPages}</ActionButton
						>
					{/if}
					<ActionButton onclick={() => currentPage++} disabled={currentPage >= totalPages}>
						<ChevronRightOutline size="xs" class="m-1.5" />
					</ActionButton>
				</ButtonGroup>
			</div>
		{/snippet}
	</TableSearch>
</Section>
