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
		Badge,
		Modal
	} from 'flowbite-svelte';
	import ActionButton from './ActionButton.svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		EditOutline,
		TrashBinOutline,
		ExclamationCircleOutline,
		ChevronDownOutline,
		ChevronRightOutline
	} from 'flowbite-svelte-icons';
	import AutomationRuleModal from './AutomationRuleModal.svelte';
	import { tableSearchClasses, tableSearchTableClass, toolbarControl } from './table_styles';
	import { shortDate } from './analysis/dates';
	import { describeAccrual, formatAccrualPeriod } from './expense_utils';
	import QueryError from './QueryError.svelte';
	import RegexText from './RegexText.svelte';
	import { euro } from './money';

	let searchTerm = $state('');
	let showNewModal = $state(false);
	let showEditModal = $state(false);
	let showDeleteModal = $state(false);
	let ruleIdToEdit: string | undefined = $state(undefined);
	let ruleIdToDelete: string | undefined = $state(undefined);
	let expandedRows = $state<string[]>([]);

	function toggleRow(id: string) {
		expandedRows = expandedRows.includes(id)
			? expandedRows.filter((rowId) => rowId !== id)
			: [...expandedRows, id];
	}

	/// What the toolbar acts on. An open row is this list's idea of "the one you mean", and
	/// several open rows are no answer at all rather than a reason to pick the first.
	const selectedRuleId = $derived(expandedRows.length === 1 ? expandedRows[0] : undefined);

	const ruleStore = graphql(`
		query AutomationRuleList {
			automationRules {
				list(limit: 1000) {
					id
					title
					priority
					regexTime
					regexStatement
					filterAmount
					filterAccountId
					lastDelivery
					templateId
					accrual {
						months
						days
						anchorDay
						phaseDays
						exampleAt
						exampleFrom
						exampleTo
					}
				}
			}
			accounts {
				list(limit: 1000) {
					id
					name
				}
			}
		}
	`);

	$effect(() => void ruleStore.fetch());

	const allRules = $derived($ruleStore.data?.automationRules.list || []);
	const accounts = $derived($ruleStore.data?.accounts.list || []);
	const accountNames = $derived(new Map(accounts.map((a) => [a.id, a.name])));

	/// Most recently used first: a rule that fired last week is one you still have, and the
	/// long tail that has not matched anything for years is not what the page is for. A third
	/// click on a header comes back here.
	const DEFAULT_SORT = { field: 'lastDelivery', direction: -1 };
	let sortBy: { field: string; direction: number } = $state({ ...DEFAULT_SORT });

	/// Descending, ascending, off. Sorted here rather than by the server: the whole list is
	/// loaded anyway, and Konto reads as an account name that only this side knows.
	function sortByField(field: string) {
		if (sortBy.field !== field) sortBy = { field, direction: -1 };
		else if (sortBy.direction === -1) sortBy = { field, direction: 1 };
		else sortBy = { ...DEFAULT_SORT };
	}

	const isSortedBy = (field: string) => (sortBy.field === field ? sortBy.direction : null);

	const sortKeys: Record<string, (r: (typeof allRules)[number]) => string | number> = {
		title: (r) => r.title.toLowerCase(),
		regexStatement: (r) => (r.regexStatement ?? '').toLowerCase(),
		account: (r) => (r.filterAccountId ? (accountNames.get(r.filterAccountId) ?? '') : ''),
		amount: (r) => r.filterAmount ?? 0,
		priority: (r) => r.priority,
		lastDelivery: (r) => r.lastDelivery?.getTime() ?? 0
	};

	const rules = $derived.by(() => {
		const term = searchTerm.trim().toLowerCase();
		const filtered = term
			? allRules.filter(
					(r) =>
						r.title.toLowerCase().includes(term) ||
						(r.regexStatement ?? '').toLowerCase().includes(term)
				)
			: allRules;

		const key = sortKeys[sortBy.field] ?? sortKeys.lastDelivery;
		return [...filtered].sort((a, b) => {
			const [x, y] = [key(a), key(b)];
			if (x === y) return a.title.localeCompare(b.title);
			// -1 is descending, as everywhere else here: the expense list maps the same field
			// onto the API's DESC.
			return (x < y ? -1 : 1) * sortBy.direction;
		});
	});

	const ruleToEdit = $derived(allRules.find((r) => r.id === ruleIdToEdit));
	const ruleToDelete = $derived(allRules.find((r) => r.id === ruleIdToDelete));

	const DeleteAutomationRule = graphql(`
		mutation DeleteAutomationRule($id: UUID!) {
			automationRules {
				delete(id: $id)
			}
		}
	`);

	async function reload() {
		invalidate();
		await ruleStore.fetch(AFTER_WRITE);
		expandedRows = [];
	}

	async function handleSubmit() {
		showNewModal = false;
		showEditModal = false;
		await reload();
	}

	async function handleDelete() {
		if (!ruleToDelete) return;
		await DeleteAutomationRule.mutate({ id: ruleToDelete.id });
		ruleIdToDelete = undefined;
		showDeleteModal = false;
		await reload();
	}
</script>

<Modal bind:open={showDeleteModal} size="xs">
	<div class="text-center">
		<ExclamationCircleOutline class="mx-auto mb-4 h-12 w-12 text-gray-400 dark:text-gray-200" />
		<h3 class="mb-5 text-lg font-normal text-gray-500 dark:text-gray-400">
			Soll die Regel <b>{ruleToDelete?.title}</b> wirklich gelöscht werden?
		</h3>
		<div class="space-x-2">
			<Button color="red" onclick={handleDelete}>Löschen</Button>
			<Button
				color="alternative"
				onclick={() => {
					showDeleteModal = false;
					ruleIdToDelete = undefined;
				}}
			>
				Abbrechen
			</Button>
		</div>
	</div>
</Modal>

<AutomationRuleModal bind:open={showNewModal} onsubmit={handleSubmit} {accounts} />
<AutomationRuleModal
	bind:open={showEditModal}
	onsubmit={handleSubmit}
	{accounts}
	rule={ruleToEdit}
/>

{#snippet sortIcon(field: string)}
	{#if isSortedBy(field) === -1}
		▲
	{:else if isSortedBy(field) === 1}
		▼
	{/if}
{/snippet}

{#snippet amount(cents: number | null)}
	{#if cents == null}
		<span class="text-gray-400 dark:text-gray-500">–</span>
	{:else}
		<span
			class={cents < 0 ? 'text-red-600 dark:text-red-400' : 'text-green-700 dark:text-green-400'}
			>{euro(cents)}</span
		>
	{/if}
{/snippet}

<!-- Priority decides which rule wins when several match, so it is worth seeing at a glance
     which ones were deliberately pushed above or below the default of 1. -->
{#snippet prio(value: number)}
	<span
		class={value > 1
			? 'font-semibold text-orange-600 dark:text-orange-400'
			: value < 1
				? 'text-gray-400 dark:text-gray-500'
				: ''}
	>
		{value}
	</span>
{/snippet}

{#snippet newButton(cls: string)}
	<!-- `primary` spelled out: inside a ButtonGroup a Button with no colour of its own falls
		     back to `alternative` rather than to primary. -->
	<Button color="primary" class="{toolbarControl} {cls}" onclick={() => (showNewModal = true)}>
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
	</Button>
{/snippet}

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-0 sm:p-5">
	<QueryError
		errors={$ruleStore.errors}
		message="Die Regeln konnten nicht geladen werden."
		class="mb-4"
	/>
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
						disabled={!selectedRuleId}
						onclick={() => {
							ruleIdToEdit = selectedRuleId;
							showEditModal = true;
						}}
					>
						<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
					</ActionButton>
					<ActionButton
						color="red"
						class={toolbarControl}
						disabled={!selectedRuleId}
						onclick={() => {
							ruleIdToDelete = selectedRuleId;
							showDeleteModal = true;
						}}
					>
						<TrashBinOutline class="mr-2 h-3.5 w-3.5" />Löschen
					</ActionButton>
				</ButtonGroup>
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('title')}>
				Titel{@render sortIcon('title')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell"
				scope="col"
				onclick={() => sortByField('regexStatement')}
			>
				Verwendungszweck{@render sortIcon('regexStatement')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell"
				scope="col"
				onclick={() => sortByField('account')}
			>
				Konto{@render sortIcon('account')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell"
				scope="col"
				onclick={() => sortByField('amount')}
			>
				Betrag{@render sortIcon('amount')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell"
				scope="col"
				onclick={() => sortByField('priority')}
			>
				Prio{@render sortIcon('priority')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell"
				scope="col"
				onclick={() => sortByField('lastDelivery')}
			>
				Zuletzt{@render sortIcon('lastDelivery')}
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each rules as rule (rule.id)}
				<TableBodyRow onclick={() => toggleRow(rule.id)}>
					<TableBodyCell class="w-full max-w-0 px-4 py-3 whitespace-normal">
						<button type="button" class="mr-2 focus:outline-none" aria-label="Details">
							{#if expandedRows.includes(rule.id)}
								<ChevronDownOutline class="inline h-4 w-4" />
							{:else}
								<ChevronRightOutline class="inline h-4 w-4" />
							{/if}
						</button>
						{rule.title}
						{#if !rule.templateId}
							<Badge color="red" class="ml-2">Vorlage fehlt</Badge>
						{/if}
						<!-- Five columns do not fit on a phone, and a rule is nothing without the pattern
						     it matches on, so the hidden ones come back as a line under the title
						     rather than being dropped. -->
						<span class="mt-1 block text-xs text-gray-500 md:hidden dark:text-gray-400">
							<RegexText pattern={rule.regexStatement} />
							· {rule.filterAccountId ? (accountNames.get(rule.filterAccountId) ?? '?') : '–'}
							· {@render amount(rule.filterAmount)}
							· Prio {rule.priority}
							· {rule.lastDelivery ? shortDate(rule.lastDelivery) : '–'}
						</span>
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 text-xs md:table-cell">
						<RegexText pattern={rule.regexStatement} />
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{rule.filterAccountId ? (accountNames.get(rule.filterAccountId) ?? '?') : '–'}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{@render amount(rule.filterAmount)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{@render prio(rule.priority)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{rule.lastDelivery ? shortDate(rule.lastDelivery) : '–'}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(rule.id)}
					<tr>
						<td colspan="6" class="border-b bg-gray-100 px-4 py-3 dark:bg-gray-800">
							<div class="flex flex-col gap-2">
								<div>
									<b>Verwendungszweck:</b>
									<span class="font-mono wrap-anywhere">{rule.regexStatement ?? 'beliebig'}</span>
								</div>
								<div>
									<b>Datum:</b>
									<span class="font-mono wrap-anywhere">{rule.regexTime ?? 'beliebig'}</span>
								</div>
								<div>
									<b>Konto:</b>
									{rule.filterAccountId
										? (accountNames.get(rule.filterAccountId) ?? 'unbekannt')
										: 'beliebig'}
								</div>
								<div>
									<b>Betrag:</b>
									{rule.filterAmount != null
										? (rule.filterAmount / 100).toFixed(2) + '€'
										: 'beliebig'}
								</div>
								<div>
									<b>Priorität:</b>
									{rule.priority}
								</div>
								<div>
									<b>Zuletzt ausgelöst:</b>
									{rule.lastDelivery ? shortDate(rule.lastDelivery) : 'nie'}
								</div>
								{#if rule.accrual}
									<!-- Derived from the template, editable only there, and wrong in a way
									     nobody would notice until a generated expense turns up in the wrong
									     month. -->
									<div>
										<b>Leistungszeitraum:</b>
										{describeAccrual(rule.accrual)}
										<span class="text-gray-500 dark:text-gray-400">
											· {shortDate(rule.accrual.exampleAt)} →
											{formatAccrualPeriod(rule.accrual.exampleFrom, rule.accrual.exampleTo)}
										</span>
									</div>
								{/if}

								<div class="flex gap-2">
									<Button
										color="yellow"
										size="xs"
										onclick={() => {
											ruleIdToEdit = rule.id;
											showEditModal = true;
										}}
									>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="red"
										size="xs"
										onclick={() => {
											ruleIdToDelete = rule.id;
											showDeleteModal = true;
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
	</TableSearch>
</Section>
