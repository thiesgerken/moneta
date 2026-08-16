<script lang="ts">
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
		Modal
	} from 'flowbite-svelte';
	import ActionButton from './ActionButton.svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		ChevronRightOutline,
		ChevronDownOutline,
		EditOutline,
		TrashBinOutline,
		ExclamationCircleOutline,
		FilterSolid,
		AnnotationOutline,
		CalendarMonthOutline,
		ChartPieOutline,
		ClockOutline,
		GridOutline,
		ScaleBalancedOutline,
		TextSizeOutline
	} from 'flowbite-svelte-icons';
	import AccountModal from './AccountModal.svelte';
	import {
		tableSearchClasses,
		tableSearchTableClass,
		toolbarControl,
		headIcon
	} from './table_styles';
	import QueryError from './QueryError.svelte';
	import FilterChips, { type ActiveFilter } from './FilterChips.svelte';
	import OptionFilter from './OptionFilter.svelte';

	let searchTerm = $state('');
	let expandedRows = $state<string[]>([]);

	/// What the toolbar acts on. An open row is this list's idea of "the one you mean", and
	/// several open rows are no answer at all rather than a reason to pick the first.
	const selectedAccountId = $derived(expandedRows.length === 1 ? expandedRows[0] : undefined);
	/// Newest first, and a third click on a header comes back here.
	const DEFAULT_SORT = [
		{ field: 'opened', direction: 1 },
		{ field: 'name', direction: 1 }
	];
	let sortBy: { field: string; direction: number }[] = $state(DEFAULT_SORT.map((s) => ({ ...s })));

	let showNewAccountModal = $state(false);
	let showEditAccountModal = $state(false);
	let showDeleteAccountModal = $state(false);
	let accountIdToEdit: string | undefined = $state(undefined);
	let accountIdToDelete: string | undefined = $state(undefined);

	/// Which states the list shows. A single "also show closed" checkbox could only be described
	/// as a filter for closed accounts, which is the opposite of what it did; two states named
	/// out loud say what is on screen.
	const OPEN_ONLY = ['open'];
	const ACCOUNT_STATES = [
		{ value: 'open', name: 'Aktiv' },
		{ value: 'closed', name: 'Geschlossen' }
	];
	let accountStates = $state<string[]>([...OPEN_ONLY]);

	const toggleOption = (selected: string[], id: string) =>
		selected.includes(id) ? selected.filter((other) => other !== id) : [...selected, id];

	// Get current user and all accounts
	const basicStore = graphql(`
		query AccountTableBasic {
			users {
				me {
					name
				}
			}
		}
	`);

	const accountStore = graphql(`
		query AccountTableAccounts {
			accounts {
				list(limit: 1000) {
					id
					name
					description
					color
					owner {
						name
					}
					iban
					kind
					availability
					risk
					opened
					closed
					ownershipFraction
					synchronization {
						account {
							id
						}
						invert
					}
				}
			}
		}
	`);

	$effect(() => void basicStore.fetch());
	$effect(() => void accountStore.fetch());

	const currentUserName = $derived($basicStore.data?.users.me?.name || '');
	const totalCount = $derived(
		($accountStore.data?.accounts.list || []).filter(
			(account) => account.owner.name === currentUserName
		).length
	);

	let allAccounts = $derived($accountStore.data?.accounts.list || []);

	let filteredAccounts = $derived.by(() => {
		let filtered = allAccounts.filter((account) => {
			return account.owner.name === currentUserName;
		});

		filtered = filtered.filter((account) =>
			accountStates.includes(account.closed ? 'closed' : 'open')
		);

		if (searchTerm) {
			const searchLower = searchTerm.toLowerCase();
			filtered = filtered.filter(
				(account) =>
					account.name.toLowerCase().includes(searchLower) ||
					(account.description && account.description.toLowerCase().includes(searchLower)) ||
					(account.iban && account.iban.toLowerCase().includes(searchLower))
			);
		}

		// Apply sorting
		if (sortBy.length > 0) {
			const { field, direction } = sortBy[0];
			filtered = [...filtered].sort((a, b) => {
				let aVal, bVal;

				switch (field) {
					case 'name':
						aVal = a.name;
						bVal = b.name;
						break;
					case 'description':
						aVal = a.description || '';
						bVal = b.description || '';
						break;
					case 'owner':
						aVal = a.owner.name;
						bVal = b.owner.name;
						break;
					case 'kind':
						aVal = a.kind;
						bVal = b.kind;
						break;
					case 'availability':
						aVal = a.availability;
						bVal = b.availability;
						break;
					case 'risk':
						aVal = a.risk;
						bVal = b.risk;
						break;
					case 'opened':
						aVal = a.opened;
						bVal = b.opened;
						break;
					case 'closed':
						aVal = a.closed || new Date(9999, 11, 31);
						bVal = b.closed || new Date(9999, 11, 31);
						break;
					case 'ownershipFraction':
						aVal = a.ownershipFraction;
						bVal = b.ownershipFraction;
						break;
					default:
						return 0;
				}

				if (aVal < bVal) return direction;
				if (aVal > bVal) return -direction;
				return 0;
			});
		}

		return filtered;
	});

	let accountToEdit = $derived(allAccounts.find((a) => a.id === accountIdToEdit) || undefined);
	let accountToDelete = $derived(allAccounts.find((a) => a.id === accountIdToDelete) || undefined);

	function toggleRow(id: string) {
		if (expandedRows.includes(id)) expandedRows = expandedRows.filter((rowId) => rowId !== id);
		else expandedRows = [...expandedRows, id];
	}

	/// Descending, ascending, off -- a header clicked by accident is otherwise only undone by
	/// reloading the page, since the sort lives here rather than in the url.
	/// What is currently being filtered on, and how to undo each of them. A list arriving
	/// pre-filtered otherwise looks like the whole list, only shorter.
	const activeFilters = $derived.by(() => {
		const out: ActiveFilter[] = [];

		if (searchTerm.trim()) {
			out.push({ label: `Suche: ${searchTerm.trim()}`, clear: () => (searchTerm = '') });
		}
		// only worth a chip when it is not the default, and then it says what is on screen
		// rather than what was ticked
		if (accountStates.join() !== OPEN_ONLY.join()) {
			out.push({
				label: accountStates.length
					? ACCOUNT_STATES.filter((s) => accountStates.includes(s.value))
							.map((s) => s.name)
							.join(' + ')
					: 'Keine Konten',
				clear: () => (accountStates = [...OPEN_ONLY])
			});
		}

		return out;
	});

	function resetFilters() {
		searchTerm = '';
		accountStates = [...OPEN_ONLY];
	}

	function sortByField(field: string) {
		if (sortBy[0]?.field !== field) sortBy = [{ field, direction: -1 }];
		else if (sortBy[0].direction === -1) sortBy = [{ field, direction: 1 }];
		else sortBy = DEFAULT_SORT.map((s) => ({ ...s }));
	}

	function isSortedBy(field: string) {
		const index = sortBy.findIndex(({ field: f }) => f === field);
		if (index === -1) return null;
		return sortBy[index].direction;
	}

	const DeleteAccount = graphql(`
		mutation DeleteAccount($id: UUID!) {
			accounts {
				delete(id: $id)
			}
		}
	`);

	async function handleAccountModalSubmit() {
		invalidate();
		await accountStore.fetch({ ...AFTER_WRITE });
		expandedRows = [];
	}

	async function handleDeleteAccount() {
		if (!accountToDelete) return;
		await DeleteAccount.mutate({ id: accountToDelete.id });
		accountIdToDelete = undefined;
		showDeleteAccountModal = false;

		invalidate();
		await accountStore.fetch({ ...AFTER_WRITE });
		expandedRows = [];
	}

	// Helper functions for display
	function getKindDisplayName(kind: string): string {
		const kindMap: Record<string, string> = {
			CASH: 'Bargeld',
			DEBIT: 'Girokonto',
			CREDIT: 'Kreditkarte',
			DEBT: 'Schulden',
			STOCKS: 'Aktien',
			REAL_ESTATE: 'Immobilien',
			VIRTUAL: 'Virtuell',
			OTHER: 'Sonstiges'
		};
		return kindMap[kind] || kind;
	}

	function getAvailabilityDisplayName(availability: string): string {
		const availabilityMap: Record<string, string> = {
			IMMEDIATELY: 'Sofort',
			DAYS: 'Tage',
			WEEKS: 'Wochen',
			MONTHS: 'Monate',
			YEARS: 'Jahre',
			DECADES: 'Jahrzehnte'
		};
		return availabilityMap[availability] || availability;
	}

	function getRiskDisplayName(risk: string): string {
		const riskMap: Record<string, string> = {
			NONE: 'Kein Risiko',
			SLIGHT: 'Geringes Risiko',
			LOW: 'Niedriges Risiko',
			MEDIUM: 'Mittleres Risiko',
			HIGH: 'Hohes Risiko',
			HUGE: 'Sehr hohes Risiko'
		};
		return riskMap[risk] || risk;
	}

	function getRiskColor(risk: string): string {
		const riskColorMap: Record<string, string> = {
			NONE: 'bg-green-100 text-green-700',
			SLIGHT: 'bg-green-100 text-green-700',
			LOW: 'bg-blue-100 text-blue-700',
			MEDIUM: 'bg-yellow-100 text-yellow-700',
			HIGH: 'bg-orange-100 text-orange-700',
			HUGE: 'bg-red-100 text-red-700'
		};
		return riskColorMap[risk] || 'bg-gray-100 text-gray-700';
	}
</script>

{#snippet sortIcon(field: string)}
	{#if isSortedBy(field) === -1}
		▲
	{:else if isSortedBy(field) === 1}
		▼
	{/if}
{/snippet}

<Modal bind:open={showDeleteAccountModal} size="xs">
	<div class="text-center">
		<ExclamationCircleOutline class="mx-auto mb-4 h-12 w-12 text-gray-400 dark:text-gray-200" />
		<h3 class="mb-5 text-lg font-normal text-gray-500 dark:text-gray-400">
			Soll das Konto <b>{accountToDelete?.name}</b> wirklich gelöscht werden?
		</h3>
		<div class="space-x-2">
			<Button type="submit" value="delete" color="red" onclick={handleDeleteAccount}>Löschen</Button
			>
			<Button
				type="submit"
				value="cancel"
				color="alternative"
				onclick={() => {
					showDeleteAccountModal = false;
					accountIdToDelete = undefined;
				}}
			>
				Abbrechen
			</Button>
		</div>
	</div>
</Modal>

<AccountModal
	bind:open={showNewAccountModal}
	onsubmit={handleAccountModalSubmit}
	accounts={allAccounts.map((a) => ({
		id: a.id.toString(),
		name: a.name,
		description: a.description,
		color: a.color,
		owner: a.owner,
		iban: a.iban,
		kind: a.kind,
		availability: a.availability,
		risk: a.risk,
		opened: a.opened,
		closed: a.closed,
		ownershipFraction: a.ownershipFraction,
		synchronization: a.synchronization
			? {
					account: { id: a.synchronization.account.id.toString() },
					invert: a.synchronization.invert
				}
			: null
	}))}
/>

<AccountModal
	bind:open={showEditAccountModal}
	onsubmit={handleAccountModalSubmit}
	account={accountToEdit}
	accounts={allAccounts.map((a) => ({
		id: a.id.toString(),
		name: a.name,
		description: a.description,
		color: a.color,
		owner: a.owner,
		iban: a.iban,
		kind: a.kind,
		availability: a.availability,
		risk: a.risk,
		opened: a.opened,
		closed: a.closed,
		ownershipFraction: a.ownershipFraction,
		synchronization: a.synchronization
			? {
					account: { id: a.synchronization.account.id.toString() },
					invert: a.synchronization.invert
				}
			: null
	}))}
/>

{#snippet newButton(cls: string)}
	<!-- `primary` spelled out: inside a ButtonGroup a Button with no colour of its own falls
		     back to `alternative` rather than to primary. -->
	<Button
		color="primary"
		class="{toolbarControl} {cls}"
		onclick={() => (showNewAccountModal = true)}
	>
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
	</Button>
{/snippet}

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-0 sm:p-5">
	<QueryError
		errors={[...($accountStore.errors ?? []), ...($basicStore.errors ?? [])]}
		message="Die Konten konnten nicht geladen werden."
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
				<!-- Bearbeiten and Löschen act on the open row, which a phone has no room for beside
				     Neu -- and there the open row carries the same two buttons itself. -->
				{@render newButton('grow md:hidden')}
				<ButtonGroup class="hidden md:inline-flex">
					{@render newButton('')}
					<ActionButton
						color="yellow"
						class="{toolbarControl} text-gray-900"
						disabled={!selectedAccountId}
						onclick={() => {
							accountIdToEdit = selectedAccountId;
							showEditAccountModal = true;
						}}
					>
						<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
					</ActionButton>
					<ActionButton
						color="red"
						class={toolbarControl}
						disabled={!selectedAccountId}
						onclick={() => {
							accountIdToDelete = selectedAccountId;
							showDeleteAccountModal = true;
						}}
					>
						<TrashBinOutline class="mr-2 h-3.5 w-3.5" />Löschen
					</ActionButton>
				</ButtonGroup>
				<OptionFilter
					label="Status"
					options={ACCOUNT_STATES}
					selected={accountStates}
					toggle={(id) => (accountStates = toggleOption(accountStates, id))}
				>
					{#snippet icon()}
						<FilterSolid class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
					{/snippet}
				</OptionFilter>
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell
				class="px-4 py-3 md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('name')}
			>
				<TextSizeOutline class={headIcon} />Name
				{@render sortIcon('name')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('description')}
			>
				<AnnotationOutline class={headIcon} />Beschreibung
				{@render sortIcon('description')}
			</TableHeadCell>
			<TableHeadCell
				class="px-4 py-3 md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('kind')}
			>
				<GridOutline class={headIcon} />Typ
				{@render sortIcon('kind')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('availability')}
			>
				<ClockOutline class={headIcon} />Verfügbarkeit
				{@render sortIcon('availability')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('risk')}
			>
				<ScaleBalancedOutline class={headIcon} />Risiko
				{@render sortIcon('risk')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('ownershipFraction')}
			>
				<ChartPieOutline class={headIcon} />Anteil
				{@render sortIcon('ownershipFraction')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('opened')}
			>
				<CalendarMonthOutline class={headIcon} />Eröffnet
				{@render sortIcon('opened')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('closed')}
			>
				<CalendarMonthOutline class={headIcon} />Geschlossen
				{@render sortIcon('closed')}
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each filteredAccounts as account (account.id)}
				<TableBodyRow onclick={() => toggleRow(account.id)}>
					<TableBodyCell class="px-4 py-3">
						<button type="button" class="mr-2 focus:outline-none">
							{#if expandedRows.includes(account.id)}
								<ChevronDownOutline class="inline h-4 w-4" />
							{:else}
								<ChevronRightOutline class="inline h-4 w-4" />
							{/if}
						</button>
						<span
							class="mr-2 inline-block h-3 w-3 rounded-full"
							style={account.color ? `background-color: ${account.color}` : undefined}
						></span>
						{account.name}
					</TableBodyCell>
					<TableBodyCell class="hidden max-w-96 truncate px-4 py-3 md:table-cell">
						{account.description || '-'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{getKindDisplayName(account.kind)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{getAvailabilityDisplayName(account.availability)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						<span class={`rounded-md px-2 py-1 text-xs ${getRiskColor(account.risk)}`}>
							{getRiskDisplayName(account.risk)}
						</span>
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{(account.ownershipFraction * 100).toFixed(0)}%
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{account.opened.toLocaleDateString('de-DE', {
							year: 'numeric',
							month: '2-digit',
							day: '2-digit'
						})}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{account.closed?.toLocaleDateString('de-DE', {
							year: 'numeric',
							month: '2-digit',
							day: '2-digit'
						})}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(account.id)}
					<tr>
						<td colspan="8" class="border-b bg-gray-100 px-4 py-3 dark:bg-gray-800">
							<div class="flex flex-col gap-2">
								<div>
									<b>Name:</b>
									{account.name}
								</div>
								{#if account.description}
									<div>
										<b>Beschreibung:</b>
										{account.description}
									</div>
								{/if}
								<div>
									<b>Farbe:</b>
									{#if account.color}
										<span class="inline-flex items-center gap-2">
											<div
												class="h-4 w-4 rounded border border-gray-300"
												style={`background-color: ${account.color}`}
											></div>
											{account.color}
										</span>
									{:else}
										Keine
									{/if}
								</div>
								{#if account.iban}
									<div>
										<b>IBAN:</b>
										{account.iban}
									</div>
								{/if}
								<div>
									<b>Typ:</b>
									{getKindDisplayName(account.kind)}
								</div>
								<div>
									<b>Verfügbarkeit:</b>
									{getAvailabilityDisplayName(account.availability)}
								</div>
								<div>
									<b>Risiko:</b>
									{getRiskDisplayName(account.risk)}
								</div>
								<div>
									<b>Eröffnet:</b>
									{account.opened.toLocaleDateString('de-DE', {
										year: 'numeric',
										month: '2-digit',
										day: '2-digit'
									})}
								</div>
								{#if account.closed}
									<div>
										<b>Geschlossen:</b>
										{account.closed.toLocaleDateString('de-DE', {
											year: 'numeric',
											month: '2-digit',
											day: '2-digit'
										})}
									</div>
								{/if}
								<div>
									<b>Anteil:</b>
									{(account.ownershipFraction * 100).toFixed(0)}%
								</div>
								{#if account.synchronization}
									<div>
										<b>Synchronisierung:</b>
										{allAccounts.find((a) => a.id === account.synchronization!.account.id)?.name ||
											'Unbekanntes Konto'}
										{#if account.synchronization.invert}
											<span class="ml-1">(umgekehrt)</span>
										{/if}
									</div>
								{/if}

								<div class="flex gap-2">
									<Button
										color="yellow"
										size="xs"
										onclick={() => {
											accountIdToEdit = account.id.toString();
											showEditAccountModal = true;
										}}
									>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="red"
										size="xs"
										onclick={() => {
											accountIdToDelete = account.id.toString();
											showDeleteAccountModal = true;
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
					<span class="font-semibold text-gray-900 dark:text-white">{filteredAccounts.length}</span>
					von
					<span class="font-semibold text-gray-900 dark:text-white">{totalCount}</span>
				</span>
			</div>
		{/snippet}
	</TableSearch>
</Section>
