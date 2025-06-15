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
		Li
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		FilterSolid,
		ChevronRightOutline,
		ChevronLeftOutline
	} from 'flowbite-svelte-icons';
	import type { ExpenseTableVariables } from './$houdini';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');

	export const _ExpenseTableVariables: ExpenseTableVariables = () => {
		let filter = JSON.stringify({});
		if (searchTerm) {
			filter = JSON.stringify({
				$or: [
					{
						title: {
							$regex: searchTerm,
							$options: 'i'
						}
					},
					{
						description: {
							$regex: searchTerm,
							$options: 'i'
						}
					},
					{
						store: {
							$regex: searchTerm,
							$options: 'i'
						}
					},
					{
						positions: {
							$elemMatch: {
								title: {
									$regex: searchTerm,
									$options: 'i'
								}
							}
						}
					}
				]
			});
		}

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
							positions {
								title
								amount
								units
								comments
							}
						}
					}
				}
			}
		}
	`);

	const totalCount = $derived($store.data?.expenses.list.totalCount || 0);
	const totalPages = $derived(Math.ceil(totalCount / itemsPerPage));

	let expenses = $derived($store.data?.expenses.list.edges.map((expense) => expense.node) || []);
</script>

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-3 sm:p-5">
	<TableSearch
		placeholder="Suchen"
		hoverable={true}
		striped={true}
		bind:inputValue={searchTerm}
        tableClass="w-full text-left text-sm table-fixed"
		divClass="bg-white dark:bg-gray-800 relative shadow-md sm:rounded-lg overflow-x-auto"
		innerDivClass="flex flex-col md:flex-row items-center justify-between space-y-3 md:space-y-0 md:space-x-4 p-4"
		searchClass="w-full md:w-1/2 relative"
	>
		{#snippet header()}
			<div
				class="flex w-full flex-shrink-0 flex-col items-stretch justify-end space-y-2 md:w-auto md:flex-row md:items-center md:space-y-0 md:space-x-3"
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
			<TableHeadCell class="px-4 py-3 w-32" scope="col">Datum</TableHeadCell>
			<TableHeadCell class="px-4 py-3 w-32" scope="col">Betrag</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Titel</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col">Geschäft</TableHeadCell>
			<TableHeadCell class="px-4 py-3 w-128" scope="col">Beschreibung</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each expenses as expense (expense.id)}
				<TableBodyRow>
					<TableBodyCell class="px-4 py-3">
						{#if expense.bookingStart.toISOString() == expense.bookingEnd.toISOString()}
							{expense.bookingStart.toLocaleDateString('de-DE', {
								month: '2-digit',
								day: '2-digit',
								year: 'numeric'
							})}
						{:else}
							{expense.bookingStart.toLocaleDateString('de-DE', {
								month: '2-digit',
								day: '2-digit',
								year: 'numeric'
							})} - {expense.bookingEnd.toLocaleDateString('de-DE', {
								month: '2-digit',
								day: '2-digit',
								year: 'numeric'
							})}
						{/if}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3"
						>{(expense.effectiveAmount / 100).toFixed(2)}€</TableBodyCell
					>
					<TableBodyCell class="px-4 py-3">{expense.title}</TableBodyCell>
                    <TableBodyCell class="px-4 py-3">{expense.store}</TableBodyCell>
					<TableBodyCell class="px-4 py-3 truncate"
						>{expense.description}
						{expense.positions
							.map(
								(position) => `${position.units > 1 ? position.units + 'x ' : ''}${position.title}`
							)
							.join(', ')}
					</TableBodyCell>
				</TableBodyRow>
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
