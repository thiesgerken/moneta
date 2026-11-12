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
	import TableSection from './TableSection.svelte';
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
	import BalanceModal from './BalanceModal.svelte';
	import { shortDate } from './analysis/dates';
	import { amountColor, euro, signedEuro } from './money';
	import {
		tableSearchClasses,
		tableSearchTableClass,
		toolbarControl,
		headIcon,
		actionButton,
		actionGroup,
		dangerText,
		chip
	} from './table_styles';
	import QueryError from './QueryError.svelte';
	import SortIcon from './SortIcon.svelte';
	import { ASC, TableSort } from './table_sort.svelte';
	import FilterChips from './FilterChips.svelte';
	import { toggleOption, type ActiveFilter } from './filters';
	import OptionFilter from './OptionFilter.svelte';

	let searchTerm = $state('');
	let expandedRows = $state<string[]>([]);

	/// What the toolbar acts on. An open row is this list's idea of "the one you mean", and
	/// several open rows are no answer at all rather than a reason to pick the first.
	const selectedAccountId = $derived(expandedRows.length === 1 ? expandedRows[0] : undefined);
	/// A third click on a header comes back here.
	const sort = new TableSort({ field: 'opened', direction: ASC });

	/// The depot whose market value is being offered as a new balance, if any.
	let balancePrefill = $state<{ amount: number; date: Date; accountId: string } | undefined>(
		undefined
	);
	let showBalanceModal = $state(false);

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
						# the balance dialog labels an account with its owner's first name
						firstName
					}
					iban
					kind
					availability
					risk
					opened
					closed
					ownershipFraction
					renditierAccountId
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

	/// What the linked depots are worth today, beside the balance booked for them. Read-only:
	/// the only thing this page does with it is show it, and offer to open the ordinary balance
	/// dialog with the figure filled in, which is still a decision taken by hand.
	const investmentStore = graphql(`
		query AccountTableInvestments($now: DateTime!) {
			investments {
				url
				# what the account dialog offers instead of asking for a number
				accounts {
					id
					name
				}
				list {
					account {
						id
					}
					renditierAccountId
					value
					invested
					irr
					asOf
				}
			}

			analysis {
				balances(date: $now) {
					amount
					account {
						id
					}
				}
			}
		}
	`);

	$effect(() => void basicStore.fetch());
	$effect(() => void accountStore.fetch());
	$effect(() => void investmentStore.fetch({ variables: { now: new Date() } }));

	const investments = $derived($investmentStore.data?.investments.list ?? []);
	const renditierUrl = $derived($investmentStore.data?.investments.url ?? null);
	const renditierAccounts = $derived($investmentStore.data?.investments.accounts ?? []);
	const committedBalances = $derived(
		new Map(($investmentStore.data?.analysis.balances ?? []).map((b) => [b.account.id, b.amount]))
	);
	const investmentFor = (accountId: string) => investments.find((i) => i.account.id === accountId);

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

		return sort.apply(
			filtered,
			{
				name: (a) => a.name.toLowerCase(),
				description: (a) => (a.description || '').toLowerCase(),
				owner: (a) => a.owner.name.toLowerCase(),
				kind: (a) => a.kind,
				availability: (a) => a.availability,
				risk: (a) => a.risk,
				opened: (a) => a.opened.getTime(),
				// an account still open outlasts every closed one
				closed: (a) => a.closed?.getTime() ?? Infinity,
				ownershipFraction: (a) => a.ownershipFraction
			},
			(a) => a.name
		);
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
</script>

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
	{renditierAccounts}
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
		renditierAccountId: a.renditierAccountId,
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
	{renditierAccounts}
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
		renditierAccountId: a.renditierAccountId,
		synchronization: a.synchronization
			? {
					account: { id: a.synchronization.account.id.toString() },
					invert: a.synchronization.invert
				}
			: null
	}))}
/>

<!-- The one way a market value ever reaches the books, and it goes through the same dialog as
     every other reading: filled in, then edited down to whatever is worth committing. -->
<BalanceModal
	bind:open={showBalanceModal}
	userName={currentUserName}
	accounts={allAccounts}
	prefill={balancePrefill}
	onsubmit={async () => {
		balancePrefill = undefined;
		invalidate();
		await investmentStore.fetch({ ...AFTER_WRITE, variables: { now: new Date() } });
	}}
/>

{#snippet newButton(cls: string)}
	<Button
		color="primary"
		class="{toolbarControl} {cls}"
		onclick={() => (showNewAccountModal = true)}
	>
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
	</Button>
{/snippet}

<TableSection>
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
				{@render newButton('grow md:grow-0')}
				<ButtonGroup class="hidden md:inline-flex {actionGroup}">
					<ActionButton
						color="light"
						class="{toolbarControl} {actionButton}"
						disabled={!selectedAccountId}
						onclick={() => {
							accountIdToEdit = selectedAccountId;
							showEditAccountModal = true;
						}}
					>
						<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
					</ActionButton>
					<ActionButton
						color="light"
						class="{toolbarControl} {actionButton} {dangerText}"
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

		<!-- No header on a phone: one column is left there, and a heading over a list of names
		     names nothing. What the dropped columns said is the second line of each row. -->
		<TableHead class="hidden sm:table-header-group">
			<TableHeadCell
				class="px-4 py-3 md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('name')}
			>
				<TextSizeOutline class={headIcon} />Name
				<SortIcon direction={sort.on('name')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('description')}
			>
				<AnnotationOutline class={headIcon} />Beschreibung
				<SortIcon direction={sort.on('description')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 sm:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('kind')}
			>
				<GridOutline class={headIcon} />Typ
				<SortIcon direction={sort.on('kind')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('availability')}
			>
				<ClockOutline class={headIcon} />Verfügbarkeit
				<SortIcon direction={sort.on('availability')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('risk')}
			>
				<ScaleBalancedOutline class={headIcon} />Risiko
				<SortIcon direction={sort.on('risk')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('ownershipFraction')}
			>
				<ChartPieOutline class={headIcon} />Anteil
				<SortIcon direction={sort.on('ownershipFraction')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('opened')}
			>
				<CalendarMonthOutline class={headIcon} />Eröffnet
				<SortIcon direction={sort.on('opened')} />
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sort.by('closed')}
			>
				<CalendarMonthOutline class={headIcon} />Geschlossen
				<SortIcon direction={sort.on('closed')} />
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each filteredAccounts as account (account.id)}
				<TableBodyRow
					onclick={() => toggleRow(account.id)}
					data-expanded={expandedRows.includes(account.id) ? '' : undefined}
				>
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
						<span class="ms-11 block text-xs text-gray-500 sm:hidden dark:text-gray-400">
							{[getKindDisplayName(account.kind), account.description].filter(Boolean).join(' · ')}
						</span>
					</TableBodyCell>
					<TableBodyCell class="hidden max-w-96 truncate px-4 py-3 md:table-cell">
						{account.description || '-'}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 sm:table-cell">
						{getKindDisplayName(account.kind)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{getAvailabilityDisplayName(account.availability)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						<span class={chip}>
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
					<tr data-detail>
						<td colspan="8" class="px-4 py-4">
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

								<!-- What renditier says the depot is worth today. Nothing here is booked, and
								     nothing here books: the button below opens the ordinary balance dialog
								     with the figure filled in, and what gets saved is still typed by hand. -->
								{#if account.renditierAccountId !== null}
									{@const investment = investmentFor(account.id)}
									{@const committed = committedBalances.get(account.id) ?? 0}
									<div class="mt-1 border-t border-gray-200 pt-2 dark:border-gray-700">
										<b>Depot in Renditier:</b>
										{renditierAccounts.find((a) => a.id === account.renditierAccountId)?.name ??
											`Nr. ${account.renditierAccountId}`}
										{#if renditierUrl}
											<!-- renditier has no page per depot, so this lands on its list of them -->
											<!-- eslint-disable svelte/no-navigation-without-resolve -- another app entirely -->
											<a
												href="{renditierUrl}/accounts"
												target="_blank"
												rel="noreferrer"
												class="text-primary-600 dark:text-primary-500 ml-1 hover:underline"
											>
												öffnen
											</a>
											<!-- eslint-enable svelte/no-navigation-without-resolve -->
										{/if}
									</div>
									{#if investment?.value != null}
										<div>
											<b>Marktwert:</b>
											{euro(investment.value)}
											{#if investment.asOf}
												<span class="text-gray-500 dark:text-gray-400">
													(Kurs vom {shortDate(investment.asOf)})
												</span>
											{/if}
										</div>
										<div>
											<b>Nicht gebucht:</b>
											<span class={amountColor(investment.value - committed)}>
												{signedEuro(investment.value - committed)}
											</span>
											<span class="text-gray-500 dark:text-gray-400">
												gegenüber {euro(committed)} gebucht
											</span>
										</div>
										{#if investment.invested != null}
											<div>
												<b>Eingezahlt:</b>
												{euro(investment.invested)}
												{#if investment.irr != null}
													<span class="text-gray-500 dark:text-gray-400">
														({(investment.irr * 100).toFixed(1).replace('.', ',')} % p.a.)
													</span>
												{/if}
											</div>
										{/if}
									{:else}
										<div class="text-gray-500 dark:text-gray-400">Marktwert nicht verfügbar</div>
									{/if}
								{/if}

								<ButtonGroup class="w-fit {actionGroup}">
									{#if investmentFor(account.id)?.value != null}
										<Button
											color="light"
											class={actionButton}
											onclick={() => {
												balancePrefill = {
													amount: investmentFor(account.id)!.value!,
													date: new Date(),
													accountId: account.id.toString()
												};
												showBalanceModal = true;
											}}
										>
											<ScaleBalancedOutline class="mr-2 h-3.5 w-3.5" />
											Als Kontostand buchen
										</Button>
									{/if}
									<Button
										color="light"
										class={actionButton}
										onclick={() => {
											accountIdToEdit = account.id.toString();
											showEditAccountModal = true;
										}}
									>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="light"
										class="{actionButton} {dangerText}"
										onclick={() => {
											accountIdToDelete = account.id.toString();
											showDeleteAccountModal = true;
										}}
									>
										<TrashBinOutline class="mr-2 h-3.5 w-3.5" />
										Löschen
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
					<span class="font-semibold text-gray-900 dark:text-white">{filteredAccounts.length}</span>
					von
					<span class="font-semibold text-gray-900 dark:text-white">{totalCount}</span>
				</span>
			</div>
		{/snippet}
	</TableSearch>
</TableSection>
