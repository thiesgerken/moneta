<script lang="ts">
	import type { BalanceSortField$options, SortDirection$options } from '$houdini';
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
		ButtonGroup,
		Dropdown,
		Datepicker,
		Modal
	} from 'flowbite-svelte';
	import ActionButton from './ActionButton.svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		ChevronRightOutline,
		ChevronLeftOutline,
		ChevronDownOutline,
		EditOutline,
		TrashBinOutline,
		ExclamationCircleOutline,
		FilterSolid,
		CalendarMonthOutline,
		WalletOutline,
		AnnotationOutline,
		CashOutline
	} from 'flowbite-svelte-icons';
	import { accountSorter, byUsageThen, tint } from './expense_utils';
	import { amountTint, euro } from './money';
	import { shortDate } from './analysis/dates';
	import BalanceModal from './BalanceModal.svelte';
	import {
		tableSearchClasses,
		tableSearchTableClass,
		toolbarControl,
		headIcon
	} from './table_styles';
	import QueryError from './QueryError.svelte';
	import SortIcon from './SortIcon.svelte';
	import { DESC, TableSort } from './table_sort.svelte';
	import { debounced } from './debounce.svelte';
	import OptionFilter from './OptionFilter.svelte';
	import FilterChips from './FilterChips.svelte';
	import { toggleOption, type ActiveFilter } from './filters';

	let currentPage = $state(1);
	let itemsPerPage = $derived(15);
	let searchTerm = $state('');
	const search = debounced(() => searchTerm.trim());
	let expandedRows = $state<string[]>([]);

	/// What the toolbar acts on. An open row is this list's idea of "the one you mean", and
	/// several open rows are no answer at all rather than a reason to pick the first.
	const selectedBalanceId = $derived(expandedRows.length === 1 ? expandedRows[0] : undefined);
	/// Newest first, and a third click on a header comes back here.
	const sort = new TableSort({ field: 'date', direction: DESC });

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
		search: search.current || undefined,
		accountIds: filteredAccounts.length > 0 ? filteredAccounts : undefined,
		after: filterDateRange.from,
		before: filterDateRange.to,
		sortBy: SORT_FIELDS[sort.field],
		sortDir: (sort.direction < 0 ? 'DESC' : 'ASC') as SortDirection$options
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
					usageCount
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

	// what you use first, then the usual order -- the same as the expense list's filters
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

	/// The chosen period, spelled out. This is the control's own label as well as its chip, so
	/// what the filter row says and what the button says cannot drift apart.
	const dateRangeLabel = $derived.by(() => {
		const { from, to } = filterDateRange;
		if (from && to) return `${shortDate(from)} – ${shortDate(to)}`;
		if (from) return `ab ${shortDate(from)}`;
		if (to) return `bis ${shortDate(to)}`;
		return null;
	});

	function clearDateRange() {
		filterDateRange = { from: undefined, to: undefined };
	}

	/// What is currently being filtered on, and how to undo each of them. On a phone the filters
	/// are behind a button and on a desktop they are a row of empty-looking triggers, so a list
	/// arriving pre-filtered looked like the whole list, only shorter.
	const activeFilters = $derived.by(() => {
		const out: ActiveFilter[] = [];

		if (searchTerm.trim()) {
			out.push({ label: `Suche: ${searchTerm.trim()}`, clear: () => (searchTerm = '') });
		}

		for (const id of filteredAccounts) {
			const account = filterableAccounts?.find((a) => a.value === id);
			out.push({
				label: account?.name ?? 'Konto',
				color: account?.color,
				clear: () => (filteredAccounts = toggleOption(filteredAccounts, id))
			});
		}

		if (dateRangeLabel) out.push({ label: dateRangeLabel, clear: clearDateRange });

		return out;
	});

	function resetFilters() {
		searchTerm = '';
		filteredAccounts = [];
		clearDateRange();
	}

	/// Descending, ascending, off -- a header clicked by accident is otherwise only undone by
	/// reloading the page, since the sort lives here rather than in the url.

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
		invalidate();
		await balanceStore.fetch({ variables: balanceVariables(), ...AFTER_WRITE });
		expandedRows = [];
	}

	async function handleDeleteBalance() {
		if (!balanceToDelete) return;
		await DeleteBalance.mutate({ id: balanceToDelete.id });
		balanceIdToDelete = undefined;
		showDeleteBalanceModal = false;

		invalidate();
		await balanceStore.fetch({ variables: balanceVariables(), ...AFTER_WRITE });
		expandedRows = [];
	}
</script>

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

{#snippet newButton(cls: string)}
	<!-- `primary` spelled out: inside a ButtonGroup a Button with no colour of its own falls
		     back to `alternative` rather than to primary. -->
	<Button
		color="primary"
		class="{toolbarControl} {cls}"
		onclick={() => (showNewBalanceModal = true)}
	>
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
	</Button>
{/snippet}

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-0 sm:p-5">
	<QueryError
		errors={[...($balanceStore.errors ?? []), ...($basicStore.errors ?? [])]}
		message="Die Kontostände konnten nicht geladen werden."
		class="mb-4"
	/>
	<FilterChips filters={activeFilters} reset={resetFilters} />
	<TableSearch
		placeholder="Suchen"
		hoverable={true}
		bind:inputValue={searchTerm}
		class={tableSearchTableClass}
		classes={tableSearchClasses}
	>
		{#snippet header()}
			<div
				class="flex w-full flex-shrink-0 flex-col items-stretch justify-start gap-2 md:w-auto md:flex-row md:items-center md:gap-3"
			>
				<div class="flex items-center gap-2">
					<!-- Bearbeiten and Löschen act on the open row, which a phone has no room for beside
					     Neu -- and there the open row carries the same two buttons itself. -->
					{@render newButton('grow md:hidden')}
					<ButtonGroup class="hidden md:inline-flex">
						{@render newButton('')}
						<ActionButton
							color="yellow"
							class="{toolbarControl} text-gray-900"
							disabled={!selectedBalanceId}
							onclick={() => {
								balanceIdToEdit = selectedBalanceId;
								showEditBalanceModal = true;
							}}
						>
							<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
						</ActionButton>
						<ActionButton
							color="red"
							class={toolbarControl}
							disabled={!selectedBalanceId}
							onclick={() => {
								balanceIdToDelete = selectedBalanceId;
								showDeleteBalanceModal = true;
							}}
						>
							<TrashBinOutline class="mr-2 h-3.5 w-3.5" />Löschen
						</ActionButton>
					</ButtonGroup>
					<!-- as in the expense list: stacked, the filters filled the screen before the
					     first balance did -->
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
					<!-- The period is the button's label rather than the contents of a text field:
					     flowbite's own input is sized for one date, so a range arrived clipped, and
					     in range mode it prints a bare " - " instead of its placeholder. -->
					<Button
						color="alternative"
						class="{toolbarControl} justify-between whitespace-nowrap md:min-w-52"
					>
						<span class="flex items-center gap-2">
							<CalendarMonthOutline class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
							{dateRangeLabel ?? 'Datum'}
						</span>
						<ChevronDownOutline class="ml-2 h-3 w-3 shrink-0" />
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
						label="Konten"
						options={filterableAccounts ?? []}
						selected={filteredAccounts}
						toggle={(id) => (filteredAccounts = toggleOption(filteredAccounts, id))}
					>
						{#snippet icon()}
							<WalletOutline class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
						{/snippet}
					</OptionFilter>
				</div>
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell
				class="px-4 py-3 md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('date')}
			>
				<CalendarMonthOutline class={headIcon} />Datum
				<SortIcon direction={sort.on('date')} />
			</TableHeadCell>
			<TableHeadCell
				class="px-4 py-3 md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('account')}
			>
				<WalletOutline class={headIcon} />Konto
				<SortIcon direction={sort.on('account')} />
			</TableHeadCell>
			<TableHeadCell
				class="px-2 py-3 text-right md:px-4 md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('amount')}
			>
				<CashOutline class={headIcon} />Betrag
				<SortIcon direction={sort.on('amount')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('comments')}
			>
				<AnnotationOutline class={headIcon} />Kommentare
				<SortIcon direction={sort.on('comments')} />
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each balances as balance (balance.id)}
				<TableBodyRow onclick={() => toggleRow(balance.id)}>
					<TableBodyCell class="px-2 py-3 text-xs whitespace-nowrap md:px-4 md:text-sm">
						<button type="button" class="mr-1 focus:outline-none md:mr-2">
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
						class="w-full max-w-0 truncate px-2 py-3 md:w-56 md:max-w-56 md:min-w-56 md:px-4"
						style={tint(balance.account.color)}
					>
						{#if balance.account.owner.name != $basicStore.data?.users.me.name}
							{balance.account.owner.firstName} / {balance.account.name}
						{:else}
							{balance.account.name}
						{/if}
					</TableBodyCell>
					<TableBodyCell
						class={'px-2 py-3 text-right whitespace-nowrap md:px-4 ' + amountTint(balance.amount)}
						>{euro(balance.amount)}</TableBodyCell
					>
					<TableBodyCell class="hidden truncate px-4 py-3 md:table-cell md:w-full md:max-w-0"
						>{balance.comments}</TableBodyCell
					>
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
									<span class="rounded-md px-1.5 py-0.5" style={tint(balance.account.color)}>
										{#if balance.account.owner.name != $basicStore.data?.users.me.name}
											{balance.account.owner.firstName} / {balance.account.name}
										{:else}
											{balance.account.name}
										{/if}
									</span>
								</div>
								<div>
									<b>Betrag:</b>
									{euro(balance.amount)}
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
