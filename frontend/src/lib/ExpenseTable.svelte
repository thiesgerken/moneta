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
		MultiSelect,
		Badge,
		Datepicker,
		Modal
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
		ExclamationCircleOutline
	} from 'flowbite-svelte-icons';
	import {
		accountSorter,
		categorySorter,
		formatTransactionAccounts,
		formatAccrualPeriod,
		formatCategories,
		makeAccountColor,
		makeCategoryColor
	} from './expense_utils';
	import ExpenseModal from './ExpenseModal.svelte';
	import { tableSearchClasses, tableSearchTableClass } from './table_styles';
	import { page } from '$app/state';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');
	let expandedRows = $state<string[]>([]);
	let sortBy: { field: string; direction: number } = $state({ field: 'status', direction: -1 });

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
	if (page.url.searchParams.has('neu')) showNewExpenseModal = true;
	if (page.url.searchParams.has('pruefen')) {
		showTaxRelevant = false;
		showExtraordinary = false;
		showNormal = false;
	}

	const category = page.url.searchParams.get('kategorie');
	if (category) filteredCategories = [category];

	// A period the caller worked out; the analysis pages snap theirs to whole months, and a
	// list that quietly used different ends would not add up to the chart it came from.
	const from = page.url.searchParams.get('von');
	const to = page.url.searchParams.get('bis');
	if (from) filterDateRange.from = new Date(from);
	if (to) filterDateRange.to = new Date(to);

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

	let expenseToEdit = $derived(expenses.find((e) => e.id === expenseIdToEdit) || undefined);
	let expenseToDelete = $derived(expenses.find((e) => e.id === expenseIdToDelete) || undefined);

	function toggleRow(id: string) {
		if (expandedRows.includes(id)) expandedRows = expandedRows.filter((rowId) => rowId !== id);
		else expandedRows = [...expandedRows, id];
	}

	function sortByField(field: string) {
		sortBy = { field, direction: sortBy.field === field ? -sortBy.direction : -1 };
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
	<TableSearch
		placeholder="Suchen"
		hoverable={true}
		striped={true}
		bind:inputValue={searchTerm}
		class={tableSearchTableClass}
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
					<Datepicker
						range
						bind:rangeFrom={filterDateRange.from}
						bind:rangeTo={filterDateRange.to}
						placeholder="Datum"
					/>
					<MultiSelect
						items={filterableAccounts ?? []}
						bind:value={filteredAccounts}
						size="lg"
						class="ml-2 h-1 min-w-48"
						placeholder="Konten"
					>
						{#snippet children({ item, clear })}
							<Badge
								style={`background-color: ${item.color}`}
								class="text-black"
								dismissable
								params={{ duration: 100 }}
								onclose={clear}
							>
								{item.name}
							</Badge>
						{/snippet}
					</MultiSelect>
					<MultiSelect
						items={filterableCategories ?? []}
						bind:value={filteredCategories}
						size="lg"
						class="ml-2 h-1 min-w-48"
						placeholder="Kategorien"
					>
						{#snippet children({ item, clear })}
							<Badge
								style={`background-color: ${item.color}`}
								class="text-black"
								dismissable
								params={{ duration: 100 }}
								onclose={clear}
							>
								{item.name}
							</Badge>
						{/snippet}
					</MultiSelect>
					<Button size="lg" color="alternative">Filter<FilterSolid class="ml-2 h-3 w-3 " /></Button>
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

		<TableHead>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('accrualStart')}>
				Datum
				{@render sortIcon('accrualStart')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('title')}>
				Titel
				{@render sortIcon('title')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('accounts')}>
				Konten
				{@render sortIcon('accounts')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('categories')}>
				Kategorien
				{@render sortIcon('categories')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('effectiveAmount')}>
				Betrag
				{@render sortIcon('effectiveAmount')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('store')}>
				Geschäft
				{@render sortIcon('store')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('description')}>
				Beschreibung
				{@render sortIcon('description')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('status')}>
				Status
				{@render sortIcon('status')}
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each expenses as expense (expense.id)}
				<TableBodyRow onclick={() => toggleRow(expense.id)}>
					<TableBodyCell class="px-4 py-3">
						<button type="button" class="mr-2 focus:outline-none">
							{#if expandedRows.includes(expense.id)}
								<ChevronDownOutline class="inline h-4 w-4" />
							{:else}
								<ChevronRightOutline class="inline h-4 w-4" />
							{/if}
						</button>
						{formatAccrualPeriod(expense.accrualStart, expense.accrualEnd, false)}
					</TableBodyCell>
					<TableBodyCell class="max-w-96 truncate px-4 py-3">{expense.title}</TableBodyCell>
					<TableBodyCell
						class="max-w-64 truncate px-4 py-3 text-black"
						style={makeAccountColor(expense.transactions, expense.referenceAmount)}
						>{formatTransactionAccounts(
							expense.transactions,
							$basicStore.data?.users.me.name
						)}</TableBodyCell
					>
					<TableBodyCell
						class="max-w-64 truncate px-4 py-3 text-black"
						style={makeCategoryColor(expense.categories)}
						>{formatCategories(expense.categories)}</TableBodyCell
					>
					<TableBodyCell
						class={'px-4 py-3 ' +
							(expense.effectiveAmount < 0
								? 'bg-red-100 text-red-700'
								: expense.effectiveAmount > 0
									? 'bg-green-100 text-green-700'
									: '')}>{(expense.effectiveAmount / 100).toFixed(2)}€</TableBodyCell
					>
					<TableBodyCell class="max-w-64 truncate px-4 py-3">{expense.store}</TableBodyCell>
					<TableBodyCell class="max-w-96 truncate px-4 py-3"
						>{expense.description}
						{expense.positions
							.map(
								(position) =>
									`${position.quantity ? position.quantity + (position.unit ? ' ' + position.unit : '') : ''}${position.title}`
							)
							.join(', ')}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
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
							<span class="rounded-md bg-purple-100 px-2 py-1 text-purple-700">Außergewöhnlich</span
							>
						{/if}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(expense.id)}
					<tr>
						<td colspan="5" class="border-b bg-gray-100 px-4 py-3 dark:bg-gray-800">
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
				<ButtonGroup>
					<Button onclick={() => currentPage--} disabled={currentPage <= 1}>
						<ChevronLeftOutline size="xs" class="m-1.5" />
					</Button>
					{#if currentPage > 3}
						<Button disabled={currentPage === 1} onclick={() => (currentPage = 1)}>1</Button>
						<Button disabled>...</Button>
					{/if}
					{#each [currentPage - 2, currentPage - 1, currentPage, currentPage + 1, currentPage + 2].filter((page) => page > 0 && page <= totalPages) as pageNumber (pageNumber)}
						<Button disabled={currentPage === pageNumber} onclick={() => (currentPage = pageNumber)}
							>{pageNumber}</Button
						>
					{/each}
					{#if currentPage < totalPages - 2}
						<Button disabled>...</Button>
						<Button disabled={currentPage === totalPages} onclick={() => (currentPage = totalPages)}
							>{totalPages}</Button
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
