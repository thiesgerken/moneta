<script lang="ts">
	import type { BalanceSortField$options, SortDirection$options } from '$houdini';
	import { cache, graphql } from '$houdini';

	import {
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		TableSearch,
		Button,
		ButtonGroup,
		MultiSelect,
		Badge,
		Datepicker,
		Modal
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		ChevronRightOutline,
		ChevronLeftOutline,
		ChevronDownOutline,
		EditOutline,
		TrashBinOutline,
		ExclamationCircleOutline,
		FilterSolid
	} from 'flowbite-svelte-icons';
	import { accountSorter } from './expense_utils';
	import BalanceModal from './BalanceModal.svelte';
	import { tableSearchClasses, tableSearchTableClass } from './table_styles';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');
	let expandedRows = $state<string[]>([]);
	let sortBy: { field: string; direction: number } = $state({ field: 'date', direction: -1 });

	/// only consulted below `md`, where the filters are collapsed
	let filtersOpen = $state(false);

	let showNewBalanceModal = $state(false);
	let showEditBalanceModal = $state(false);
	let showDeleteBalanceModal = $state(false);
	let balanceIdToEdit: string | undefined = $state(undefined);
	let balanceIdToDelete: string | undefined = $state(undefined);

	let filteredAccounts = $state<string[]>([]);

	let filterDateRange: { from: Date | undefined; to: Date | undefined } = $state({
		from: undefined,
		to: undefined
	});

	const SORT_FIELDS: Record<string, BalanceSortField$options> = {
		date: 'DATE',
		amount: 'AMOUNT',
		comments: 'COMMENTS',
		account: 'ACCOUNT'
	};

	const balanceVariables = () => ({
		limit: itemsPerPage || 10,
		offset: ((currentPage || 1) - 1) * (itemsPerPage || 10),
		search: searchTerm.trim() || undefined,
		accountIds: filteredAccounts.length > 0 ? filteredAccounts : undefined,
		after: filterDateRange.from,
		before: filterDateRange.to,
		sortBy: SORT_FIELDS[sortBy.field],
		sortDir: (sortBy.direction < 0 ? 'DESC' : 'ASC') as SortDirection$options
	});

	// Stuff that we always need independently of the current page and sorting and so on
	const basicStore = graphql(`
		query BalanceTableBasic {
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
		}
	`);

	const balanceStore = graphql(`
		query BalanceTableBalances(
			$limit: Int
			$offset: Int
			$search: String
			$accountIds: [UUID!]
			$after: DateTime
			$before: DateTime
			$sortBy: BalanceSortField
			$sortDir: SortDirection
		) {
			balances {
				list(
					limit: $limit
					skip: $offset
					search: $search
					accountIds: $accountIds
					after: $after
					before: $before
					sortBy: $sortBy
					sortDir: $sortDir
				) {
					totalCount
					edges {
						node {
							id
							comments
							amount
							date
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
	`);

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

	const totalCount = $derived($balanceStore.data?.balances.list.totalCount || 0);
	const totalPages = $derived(Math.ceil(totalCount / itemsPerPage));

	$effect(() => {
		if (totalPages < currentPage) {
			currentPage = Math.max(totalPages, 1);
		}
	});

	let balances = $derived(
		$balanceStore.data?.balances.list.edges.map((balance) => balance.node) || []
	);

	let balanceToEdit = $derived(balances.find((b) => b.id === balanceIdToEdit) || undefined);
	let balanceToDelete = $derived(balances.find((b) => b.id === balanceIdToDelete) || undefined);

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
	$effect(() => void balanceStore.fetch({ variables: balanceVariables() }));

	const DeleteBalance = graphql(`
		mutation DeleteBalance($id: UUID!) {
			balances {
				delete(id: $id)
			}
		}
	`);

	async function handleBalanceModalSubmit() {
		cache.markStale('Balance');
		await balanceStore.fetch({ variables: balanceVariables() });
		expandedRows = [];
	}

	async function handleDeleteBalance() {
		if (!balanceToDelete) return;
		await DeleteBalance.mutate({ id: balanceToDelete.id });
		balanceIdToDelete = undefined;
		showDeleteBalanceModal = false;

		cache.markStale('Balance');
		await balanceStore.fetch({ variables: balanceVariables() });
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

<Modal bind:open={showDeleteBalanceModal} size="xs">
	<div class="text-center">
		<ExclamationCircleOutline class="mx-auto mb-4 h-12 w-12 text-gray-400 dark:text-gray-200" />
		<h3 class="mb-5 text-lg font-normal text-gray-500 dark:text-gray-400">
			Soll der Kontostand <b>{balanceToDelete?.comments}</b> wirklich gelöscht werden?
		</h3>
		<div class="space-x-2">
			<Button type="submit" value="delete" color="red" onclick={handleDeleteBalance}>Löschen</Button
			>
			<Button
				type="submit"
				value="cancel"
				color="alternative"
				onclick={() => {
					showDeleteBalanceModal = false;
					balanceIdToDelete = undefined;
				}}
			>
				Abbrechen
			</Button>
		</div>
	</div>
</Modal>

<BalanceModal
	bind:open={showNewBalanceModal}
	onsubmit={handleBalanceModalSubmit}
	userName={$basicStore.data?.users.me?.name}
	accounts={$basicStore.data?.accounts.list.map((a) => ({
		id: a.id.toString(),
		name: a.name,
		owner: { name: a.owner.name, firstName: a.owner.firstName },
		color: a.color
	})) || []}
/>

<BalanceModal
	bind:open={showEditBalanceModal}
	onsubmit={handleBalanceModalSubmit}
	userName={$basicStore.data?.users.me?.name}
	balance={balanceToEdit}
	accounts={$basicStore.data?.accounts.list.map((a) => ({
		id: a.id.toString(),
		name: a.name,
		owner: { name: a.owner.name, firstName: a.owner.firstName },
		color: a.color
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
					<Button class="grow md:grow-0" onclick={() => (showNewBalanceModal = true)}>
						<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
					</Button>
					<!-- as in the expense list: stacked, the filters filled the screen before the
					     first balance did -->
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
				</div>
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('date')}>
				Datum
				{@render sortIcon('date')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('account')}>
				Konto
				{@render sortIcon('account')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('amount')}>
				Betrag
				{@render sortIcon('amount')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('comments')}>
				Kommentare
				{@render sortIcon('comments')}
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each balances as balance (balance.id)}
				<TableBodyRow onclick={() => toggleRow(balance.id)}>
					<TableBodyCell class="px-4 py-3">
						<button type="button" class="mr-2 focus:outline-none">
							{#if expandedRows.includes(balance.id)}
								<ChevronDownOutline class="inline h-4 w-4" />
							{:else}
								<ChevronRightOutline class="inline h-4 w-4" />
							{/if}
						</button>
						{balance.date.toLocaleDateString('de-DE', {
							year: 'numeric',
							month: '2-digit',
							day: '2-digit'
						})}
					</TableBodyCell>
					<TableBodyCell
						class="max-w-64 truncate px-4 py-3 text-black"
						style={balance.account.color ? `background-color: ${balance.account.color}` : undefined}
					>
						{#if balance.account.owner.name != $basicStore.data?.users.me.name}
							{balance.account.owner.firstName} / {balance.account.name}
						{:else}
							{balance.account.name}
						{/if}
					</TableBodyCell>
					<TableBodyCell
						class={'px-4 py-3 ' +
							(balance.amount < 0
								? 'bg-red-100 text-red-700'
								: balance.amount > 0
									? 'bg-green-100 text-green-700'
									: '')}>{(balance.amount / 100).toFixed(2)}€</TableBodyCell
					>
					<TableBodyCell class="max-w-96 truncate px-4 py-3">{balance.comments}</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(balance.id)}
					<tr>
						<td colspan="4" class="border-b bg-gray-100 px-4 py-3 dark:bg-gray-800">
							<div class="flex flex-col gap-2">
								<div>
									<b>Datum:</b>
									{balance.date.toLocaleDateString('de-DE', {
										year: 'numeric',
										month: '2-digit',
										day: '2-digit',
										hour: '2-digit',
										minute: '2-digit'
									})}
								</div>
								{#if balance.comments}
									<div>
										<b>Kommentare:</b>
										{balance.comments}
									</div>
								{/if}
								<div>
									<b>Konto:</b>
									<span
										class="rounded-md px-1.5 py-0.5"
										style={balance.account.color
											? `background-color: ${balance.account.color}`
											: undefined}
									>
										{#if balance.account.owner.name != $basicStore.data?.users.me.name}
											{balance.account.owner.firstName} / {balance.account.name}
										{:else}
											{balance.account.name}
										{/if}
									</span>
								</div>
								<div>
									<b>Betrag:</b>
									{(balance.amount / 100).toFixed(2)}€
								</div>

								<div class="flex gap-2">
									<Button
										color="yellow"
										size="xs"
										onclick={() => {
											balanceIdToEdit = balance.id.toString();
											showEditBalanceModal = true;
										}}
									>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="red"
										size="xs"
										onclick={() => {
											balanceIdToDelete = balance.id.toString();
											showDeleteBalanceModal = true;
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
