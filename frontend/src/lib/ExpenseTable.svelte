<script lang="ts">
	import { fragment, graphql } from '$houdini';

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
		Li
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		FilterSolid,
		ChevronRightOutline,
		ChevronLeftOutline,
		ChevronDownOutline
	} from 'flowbite-svelte-icons';
	import type { ExpenseTableVariables } from './$houdini';
	import {
		formatAccounts,
		formatBookingPeriod,
		formatCategories,
		makeAccountColor,
		makeCategoryColor
	} from './expense_utils';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');
	let expandedRows = $state<String[]>([]);

	export const _ExpenseTableVariables: ExpenseTableVariables = () => {
		let filter = JSON.stringify({});
		// TODO: better filtering via UI, not just searching
		if (searchTerm) {
			const re = {
				$regex: searchTerm,
				$options: 'i'
			};

			filter = JSON.stringify({
				$or: [
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
				]
			});
		}

		// TODO: better sorting via UI — sort by default on stuff that is unchecked/incomplete
		// TODO: a default filter should be to hide templates
		const sort = JSON.stringify({
			bookingStart: -1
		});

		return {
			limit: itemsPerPage || 10,
			offset: ((currentPage || 1) - 1) * (itemsPerPage || 10),
			filter,
			sort
		};
	};

	const store = graphql(`
		query ExpenseTable($limit: Int, $offset: Int, $filter: String, $sort: String) @load {
			users {
				me {
					name
				}
			}
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
								units
								comments
							}
							categories {
								weight
								category {
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
								account {
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

	const totalCount = $derived($store.data?.expenses.list.totalCount || 0);
	const totalPages = $derived(Math.ceil(totalCount / itemsPerPage));

	$effect(() => {
		if (totalPages < currentPage) {
			currentPage = Math.max(totalPages, 1);
		}
	});

	let expenses = $derived($store.data?.expenses.list.edges.map((expense) => expense.node) || []);

	function toggleRow(id: String) {
		if (expandedRows.includes(id)) {
			expandedRows = expandedRows.filter((rowId) => rowId !== id);
		} else {
			expandedRows = [...expandedRows, id];
		}
	}
</script>

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
				<Button>
					<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
				</Button>
				<Button color="alternative">Filtern<FilterSolid class="ml-2 h-3 w-3 " /></Button>
				<Dropdown class="w-48 space-y-2 p-3 text-sm">
					<h6 class="mb-3 text-sm font-medium text-gray-900 dark:text-white">Konto</h6>
					<List tag="dl">
						<Li>
							<Checkbox>Apple (56)</Checkbox>
						</Li>
						<Li>
							<Checkbox>Microsoft (16)</Checkbox>
						</Li>
						<Li>
							<Checkbox>Razor (49)</Checkbox>
						</Li>
						<Li>
							<Checkbox>Nikon (12)</Checkbox>
						</Li>
						<Li>
							<Checkbox>BenQ (74)</Checkbox>
						</Li>
					</List>
				</Dropdown>
			</div>
		{/snippet}
		<TableHead>
			<TableHeadCell class="px-4 py-3" scope="col">Datum</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Titel</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Konten</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Kategorien</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Betrag</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Geschäft</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Beschreibung</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Status</TableHeadCell>
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
						>{formatAccounts(expense.transactions, $store.data?.users.me.name)}</TableBodyCell
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
								(position) => `${position.units > 1 ? position.units + 'x ' : ''}${position.title}`
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
													{#if position.units > 1}
														{position.units}x
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
												<span class="rounded-md px-1.5 py-0.5" style={makeAccountColor([transaction], expense.referenceAmount)}>
													{#if transaction.account.owner.name != $store.data?.users.me.name}
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
												<!-- TODO: shoe the transaction text -->
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
												{change.user!.name == $store.data?.users.me.name
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
					<Button onclick={() => currentPage--} disabled={currentPage <= 1}
						><ChevronLeftOutline size="xs" class="m-1.5" /></Button
					>
					{#each [currentPage - 2, currentPage - 1, currentPage, currentPage + 1, currentPage + 2].filter((page) => page > 0 && page <= totalPages) as pageNumber}
						<Button disabled={currentPage === pageNumber} onclick={() => (currentPage = pageNumber)}
							>{pageNumber}</Button
						>
					{/each}
					<Button onclick={() => currentPage++} disabled={currentPage >= totalPages}
						><ChevronRightOutline size="xs" class="m-1.5" /></Button
					>
				</ButtonGroup>
			</div>
		{/snippet}
	</TableSearch>
</Section>
