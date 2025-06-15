<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import type { PageData } from './$houdini';

	interface Props {
		data: PageData;
	}

	const { data } = $props() as Props;
	const { Expenses } = $derived(data);

	const totalCount = $derived($Expenses.data?.expenses.list.totalCount || 0);
	const itemsPerPage = $derived(parseInt(page.url.searchParams.get('count')!) || 10);
	const currentPage = $derived(parseInt(page.url.searchParams.get('page')!) || 1);
	const totalPages = $derived(Math.ceil(totalCount / itemsPerPage));

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

	let searchTerm = $state('');

	const goToPage = (pageNumber: number) => {
		const newParams = new URLSearchParams(page.url.searchParams);
		newParams.set('page', pageNumber.toString());
		goto(page.url.pathname + '?' + newParams.toString());
	};

	const loadNextPage = () => {
		goToPage(currentPage + 1);
	};

	const loadPreviousPage = () => {
		goToPage(currentPage - 1);
	};

	let currentPageItems = $derived(
		$Expenses.data?.expenses.list.edges.map((expense) => expense.node) || []
	);
</script>

{#if $Expenses.fetching}
	<div class="flex h-screen items-center justify-center">
		<div
			class="h-32 w-32 animate-spin rounded-full border-t-2 border-b-2 border-gray-900 dark:border-white"
		></div>
	</div>
{:else}
	<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-3 sm:p-5">
		<TableSearch
			placeholder="Search"
			hoverable={true}
			bind:inputValue={searchTerm}
			divClass="bg-white dark:bg-gray-800 relative shadow-md sm:rounded-lg overflow-hidden"
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
				<TableHeadCell class="px-4 py-3" scope="col">Datum</TableHeadCell>
				<TableHeadCell class="px-4 py-3" scope="col">Betrag</TableHeadCell>
				<TableHeadCell class="px-4 py-3" scope="col">Titel</TableHeadCell>
				<TableHeadCell class="px-4 py-3" scope="col">Beschreibung</TableHeadCell>
				<TableHeadCell class="px-4 py-3" scope="col">Geschäft</TableHeadCell>
			</TableHead>
			<TableBody class="divide-y">
				{#each currentPageItems as item (item.id)}
					<TableBodyRow>
						<TableBodyCell class="px-4 py-3">
							{#if item.bookingStart.toISOString() == item.bookingEnd.toISOString()}
								{item.bookingStart.toLocaleDateString('de-DE', {
									month: '2-digit',
									day: '2-digit',
									year: 'numeric'
								})}
							{:else}
								{item.bookingStart.toLocaleDateString('de-DE', {
									month: '2-digit',
									day: '2-digit',
									year: 'numeric'
								})} - {item.bookingEnd.toLocaleDateString('de-DE', {
									month: '2-digit',
									day: '2-digit',
									year: 'numeric'
								})}
							{/if}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3"
							>{(item.effectiveAmount / 100).toFixed(2)}€</TableBodyCell
						>
						<TableBodyCell class="px-4 py-3">{item.title}</TableBodyCell>
						<TableBodyCell class="px-4 py-3">{item.description}</TableBodyCell>
						<TableBodyCell class="px-4 py-3">{item.store}</TableBodyCell>
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
						<Button onclick={loadPreviousPage} disabled={currentPage <= 1}
							><ChevronLeftOutline size="xs" class="m-1.5" /></Button
						>
						{#each [currentPage - 2, currentPage - 1, currentPage, currentPage + 1, currentPage + 2].filter((page) => page > 0 && page <= totalPages) as pageNumber}
							<Button disabled={currentPage === pageNumber} onclick={() => goToPage(pageNumber)}
								>{pageNumber}</Button
							>
						{/each}
						<Button onclick={loadNextPage} disabled={currentPage >= totalPages}
							><ChevronRightOutline size="xs" class="m-1.5" /></Button
						>
					</ButtonGroup>
				</div>
			{/snippet}
		</TableSearch>
	</Section>
{/if}
