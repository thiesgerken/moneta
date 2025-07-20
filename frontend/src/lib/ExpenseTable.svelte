<script lang="ts">
	import { graphql } from '$houdini';

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
		Datepicker
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		FilterSolid,
		ChevronRightOutline,
		ChevronLeftOutline,
		ChevronDownOutline,
		EditOutline
	} from 'flowbite-svelte-icons';
	import type { ExpenseTableVariables } from './$houdini';
	import {
		accountSorter,
		categorySorter,
		formatTransactionAccounts,
		formatBookingPeriod,
		formatCategories,
		makeAccountColor,
		makeCategoryColor
	} from './expense_utils';
	import ExpenseModal from './ExpenseModal.svelte';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');
	let expandedRows = $state<String[]>([]);
	let sortBy: { field: string; direction: number }[] = $state([
		{ field: 'status', direction: -1 },
		{ field: 'bookingStart', direction: -1 }
	]);

	let showNewExpenseModal = $state(false);
	let showEditExpenseModal = $state(false);

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

	export const _ExpenseTableVariables: ExpenseTableVariables = () => {
		let filters = [];
		if (searchTerm) {
			const re = {
				$regex: searchTerm,
				$options: 'i'
			};

			const searchFilters: any[] = [
				{
					title: re
				},
				{
					description: re
				},
				{
					store: re
				},
				{
					positions: {
						$elemMatch: {
							title: re
						}
					}
				}
			];

			const searchFloat = parseFloat(searchTerm.trim());
			if (!isNaN(searchFloat) && searchFloat) {
				const lower = Math.floor(searchFloat * 100);
				const upper = Math.ceil(searchFloat * 100);

				searchFilters.push(
					{
						transactions: {
							$elemMatch: {
								$or: [
									{
										effectiveAmount: { $gte: lower, $lte: upper }
									},
									{
										amount: { $gte: lower, $lte: upper }
									},
									{
										effectiveAmount: { $gte: -upper, $lte: -lower }
									},
									{
										amount: { $gte: -upper, $lte: -lower }
									}
								]
							}
						}
					},
					{
						effectiveAmount: { $gte: lower, $lte: upper }
					},
					{
						effectiveAmount: { $gte: -upper, $lte: -lower }
					}
				);
			}

			filters.push({
				$or: searchFilters
			});
		}

		if (filteredAccounts.length > 0) {
			filters.push({
				'transactions.accountId': {
					$in: filteredAccounts.map((account) => ({ $oid: account }))
				}
			});
		}

		if (filteredCategories.length > 0) {
			filters.push({
				'categories.categoryId': {
					$in: filteredCategories.map((category) => ({ $oid: category }))
				}
			});
		}

		if (!showTemplates) {
			filters.push({
				isTemplate: false
			});
		}

		if (!showTaxRelevant) {
			filters.push({
				isTaxRelevant: false
			});
		}

		if (!showExtraordinary) {
			filters.push({
				isExtraordinary: false
			});
		}

		if (!showUnchecked) {
			filters.push({
				isUnchecked: false
			});
		}

		if (!showIncomplete) {
			filters.push({
				isIncomplete: false
			});
		}

		if (!showNormal) {
			filters.push({
				$or: [
					{ isTemplate: true },
					{ isTaxRelevant: true },
					{ isExtraordinary: true },
					{ isUnchecked: true },
					{ isIncomplete: true }
				]
			});
		}

		if (filterDateRange.from)
			filters.push({ bookingStart: { $gte: { $date: filterDateRange.from } } });

		if (filterDateRange.to) filters.push({ bookingEnd: { $lte: { $date: filterDateRange.to } } });

		let currentSort: any = {};
		for (const { field, direction } of sortBy) {
			switch (field) {
				case 'status':
					currentSort.isTemplate = -direction;
					currentSort.isIncomplete = direction;
					currentSort.isUnchecked = direction;
					break;
				case 'accounts':
					currentSort['transactions.accountId'] = direction;
					break;
				case 'categories':
					currentSort['categories.categoryId'] = direction;
					break;
				case 'bookingStart':
				case 'title':
				case 'store':
				case 'description':
				case 'comments':
				case 'effectiveAmount':
				case 'categories':
					currentSort[field] = direction;
					break;
				default:
					throw new Error(`Unknown sort field: ${field}`);
			}
		}

		return {
			limit: itemsPerPage || 10,
			offset: ((currentPage || 1) - 1) * (itemsPerPage || 10),
			filter: filters.length > 0 ? JSON.stringify({ $and: filters }) : undefined,
			sort: JSON.stringify(currentSort)
		};
	};

	// Stuff that we always need independently of the current page and sorting and so on
	const basicStore = graphql(`
		query BasicInformation @load {
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
		query ExpenseTable($limit: Int, $offset: Int, $filter: String, $sort: String) @load {
			expenses {
				list(limit: $limit, skip: $offset, filter: $filter, sort: $sort) {
					totalCount
					edges {
						node {
							id
							title
							description
							store
							effectiveAmount
							bookingStart
							bookingEnd
							comments
							isTaxRelevant
							isTemplate
							isUnchecked
							isIncomplete
							isExtraordinary
							receipts {
								uri
							}
							changelog {
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

	function toggleRow(id: String) {
		if (expandedRows.includes(id)) {
			expandedRows = expandedRows.filter((rowId) => rowId !== id);
		} else {
			expandedRows = [...expandedRows, id];
		}
	}

	function sortByField(field: string) {
		const index = sortBy.findIndex(({ field: f }) => f === field);
		if (index === -1) {
			sortBy = [{ field, direction: -1 }];
		} else {
			sortBy = [{ field, direction: -sortBy[index].direction }];
		}
	}

	function isSortedBy(field: string) {
		const index = sortBy.findIndex(({ field: f }) => f === field);
		if (index === -1) return null;
		return sortBy[index].direction;
	}
</script>

{#snippet sortIcon(field: string)}
	{#if isSortedBy(field) === -1}
		{'▲'}
	{:else if isSortedBy(field) === 1}
		{'▼'}
	{/if}
{/snippet}

<ExpenseModal
	bind:open={showNewExpenseModal}
	user_name={$basicStore.data?.users.me?.name}
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
	user_name={$basicStore.data?.users.me?.name}
	expense={expenses.find((e) => e.id === expandedRows[0])}
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

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-3 sm:p-5">
	<TableSearch
		placeholder="Suchen"
		hoverable={true}
		striped={true}
		bind:inputValue={searchTerm}
		tableClass="w-full text-left text-sm"
		divClass="bg-white dark:bg-gray-800 relative shadow-md sm:rounded-lg overflow-x-auto"
		innerDivClass="flex flex-col md:flex-row items-center justify-between space-y-3 md:space-y-0 md:space-x-4 p-4"
		searchClass="w-full md:w-1/2 relative"
	>
		{#snippet header()}
			<div
				class="flex w-full flex-shrink-0 flex-col items-stretch justify-end space-y-2 md:w-auto md:flex-row md:items-center md:space-x-3 md:space-y-0"
			>
				<Button disabled={expandedRows.length != 1} color="secondary" onclick={() => (showEditExpenseModal = true)}>
					<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
				</Button>
				<Button onclick={() => (showNewExpenseModal = true)}>
					<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
				</Button>
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
		{/snippet}

		<TableHead>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('bookingStart')}>
				Datum
				{@render sortIcon('bookingStart')}
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
						{formatBookingPeriod(expense.bookingStart, expense.bookingEnd, false)}
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
									<b>Buchungszeit:</b>
									{formatBookingPeriod(expense.bookingStart, expense.bookingEnd, true)}
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
											{#each expense.categories as category}
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
											{#each expense.positions as position}
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
										{#each expense.transactions.toSorted((a, b) => Math.abs(b.effectiveAmount) - Math.abs(a.effectiveAmount)) as transaction}
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
												{#if transaction.date.toISOString() != expense.bookingStart.toISOString() || transaction.date.toISOString() != expense.bookingEnd.toISOString()}
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
											{#each expense.receipts as receipt}
												<li><a href={receipt.uri} target="_blank">{receipt.uri}</a></li>
											{/each}
										</ul>
									</div>
								{/if}

								<div>
									<b>Changelog:</b>
									<ul class="ml-6 list-disc">
										{#each expense.changelog.toReversed() as change}
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
					{#each [currentPage - 2, currentPage - 1, currentPage, currentPage + 1, currentPage + 2].filter((page) => page > 0 && page <= totalPages) as pageNumber}
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
