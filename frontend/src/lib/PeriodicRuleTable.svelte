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
		PlayOutline,
		ChevronDownOutline,
		ChevronRightOutline
	} from 'flowbite-svelte-icons';
	import PeriodicRuleModal from './PeriodicRuleModal.svelte';
	import { tableSearchClasses, tableSearchTableClass, toolbarControl } from './table_styles';
	import { shortDate } from './analysis/dates';
	import QueryError from './QueryError.svelte';
	import { euro } from './money';

	let searchTerm = $state('');
	let showNewModal = $state(false);
	let showEditModal = $state(false);
	let showDeleteModal = $state(false);
	let ruleIdToEdit: string | undefined = $state(undefined);
	let ruleIdToDelete: string | undefined = $state(undefined);
	let runResult: string | undefined = $state(undefined);
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
		query PeriodicRuleList {
			periodicRules {
				list {
					id
					title
					period
					titleReplacement
					amountReplacement
					lastCreation
					templateId
					pending
				}
			}
		}
	`);

	$effect(() => void ruleStore.fetch());

	const allRules = $derived($ruleStore.data?.periodicRules.list || []);

	/// What is due first, which is what this page is opened to check. A third click on a header
	/// comes back here.
	const DEFAULT_SORT = { field: 'pending', direction: -1 };
	let sortBy: { field: string; direction: number } = $state({ ...DEFAULT_SORT });

	/// Descending, ascending, off.
	function sortByField(field: string) {
		if (sortBy.field !== field) sortBy = { field, direction: -1 };
		else if (sortBy.direction === -1) sortBy = { field, direction: 1 };
		else sortBy = { ...DEFAULT_SORT };
	}

	const isSortedBy = (field: string) => (sortBy.field === field ? sortBy.direction : null);

	const sortKeys: Record<string, (r: (typeof allRules)[number]) => string | number> = {
		title: (r) => r.title.toLowerCase(),
		period: (r) => ['DAILY', 'WEEKLY', 'MONTHLY', 'YEARLY'].indexOf(r.period),
		amount: (r) => r.amountReplacement ?? 0,
		lastCreation: (r) => r.lastCreation.getTime(),
		pending: (r) => r.pending.length
	};

	const rules = $derived.by(() => {
		const term = searchTerm.trim().toLowerCase();
		const filtered = term ? allRules.filter((r) => r.title.toLowerCase().includes(term)) : allRules;

		const key = sortKeys[sortBy.field] ?? sortKeys.pending;
		return [...filtered].sort((a, b) => {
			const [x, y] = [key(a), key(b)];
			if (x === y) return a.title.localeCompare(b.title);
			return (x < y ? 1 : -1) * sortBy.direction;
		});
	});

	const ruleToEdit = $derived(allRules.find((r) => r.id === ruleIdToEdit));
	const ruleToDelete = $derived(allRules.find((r) => r.id === ruleIdToDelete));

	const periodNames: Record<string, string> = {
		DAILY: 'Täglich',
		WEEKLY: 'Wöchentlich',
		MONTHLY: 'Monatlich',
		YEARLY: 'Jährlich'
	};

	const DeletePeriodicRule = graphql(`
		mutation DeletePeriodicRule($id: UUID!) {
			periodicRules {
				delete(id: $id)
			}
		}
	`);

	const RunPeriodicRules = graphql(`
		mutation RunPeriodicRules {
			periodicRules {
				run
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
		await DeletePeriodicRule.mutate({ id: ruleToDelete.id });
		ruleIdToDelete = undefined;
		showDeleteModal = false;
		await reload();
	}

	async function handleRun() {
		const result = await RunPeriodicRules.mutate(null);
		const count = result.data?.periodicRules.run ?? 0;
		runResult = count === 1 ? '1 Ausgabe erzeugt' : `${count} Ausgaben erzeugt`;
		// the generated expenses change what is still pending
		invalidate();
		await reload();
	}
</script>

<Modal bind:open={showDeleteModal} size="xs">
	<div class="text-center">
		<ExclamationCircleOutline class="mx-auto mb-4 h-12 w-12 text-gray-400 dark:text-gray-200" />
		<h3 class="mb-5 text-lg font-normal text-gray-500 dark:text-gray-400">
			Soll die Regel <b>{ruleToDelete?.title}</b> wirklich gelöscht werden?
		</h3>
		<p class="mb-5 text-sm text-gray-500 dark:text-gray-400">
			Bereits erzeugte Ausgaben bleiben erhalten.
		</p>
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

<PeriodicRuleModal bind:open={showNewModal} onsubmit={handleSubmit} />
<PeriodicRuleModal bind:open={showEditModal} onsubmit={handleSubmit} rule={ruleToEdit} />

{#snippet sortIcon(field: string)}
	{#if isSortedBy(field) === -1}
		▲
	{:else if isSortedBy(field) === 1}
		▼
	{/if}
{/snippet}

{#snippet newButton(cls: string)}
	<!-- `primary` spelled out: inside a ButtonGroup a Button with no colour of its own falls
		     back to `alternative` rather than to primary. -->
	<Button color="primary" class="{toolbarControl} {cls}" onclick={() => (showNewModal = true)}>
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
	</Button>
{/snippet}

{#snippet runButton(cls: string)}
	<Button color="teal" class="{toolbarControl} {cls}" onclick={handleRun}>
		<PlayOutline class="mr-2 h-3.5 w-3.5" />Jetzt erzeugen
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
				     Neu -- and there the open row carries the same two buttons itself. Jetzt erzeugen
				     acts on every rule at once, so it stays whatever is open. -->
				<div class="flex items-center gap-2 md:hidden">
					{@render newButton('grow')}
					{@render runButton('')}
				</div>
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
					{@render runButton('')}
				</ButtonGroup>
				{#if runResult}
					<span class="self-center text-sm text-gray-500 dark:text-gray-400">{runResult}</span>
				{/if}
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('title')}>
				Titel{@render sortIcon('title')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell"
				scope="col"
				onclick={() => sortByField('period')}
			>
				Intervall{@render sortIcon('period')}
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
				onclick={() => sortByField('lastCreation')}
			>
				Zuletzt erzeugt{@render sortIcon('lastCreation')}
			</TableHeadCell>
			<TableHeadCell class="px-4 py-3" scope="col" onclick={() => sortByField('pending')}>
				Fällig{@render sortIcon('pending')}
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
						{#if rule.titleReplacement}
							<span class="block text-xs text-gray-500 dark:text-gray-400"
								>→ {rule.titleReplacement}</span
							>
						{/if}
						<!-- what the three hidden columns said, on one line: a rule without its interval
						     and amount is just a name -->
						<span class="mt-1 block text-xs text-gray-500 md:hidden dark:text-gray-400">
							{periodNames[rule.period] ?? rule.period}
							· {rule.amountReplacement != null
								? (rule.amountReplacement / 100).toFixed(2) + '€'
								: 'wie Vorlage'}
							· zuletzt {shortDate(rule.lastCreation)}
						</span>
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell"
						>{periodNames[rule.period] ?? rule.period}</TableBodyCell
					>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{#if rule.amountReplacement == null}
							<span class="text-gray-400 dark:text-gray-500">wie Vorlage</span>
						{:else}
							<span
								class={rule.amountReplacement < 0
									? 'text-red-600 dark:text-red-400'
									: 'text-green-700 dark:text-green-400'}
							>
								{euro(rule.amountReplacement)}
							</span>
						{/if}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell"
						>{shortDate(rule.lastCreation)}</TableBodyCell
					>
					<TableBodyCell class="px-4 py-3">
						{#if !rule.templateId}
							<Badge color="red">Vorlage fehlt</Badge>
						{:else if rule.pending.length > 0}
							<Badge color="yellow">{rule.pending.length}</Badge>
						{:else}
							<span class="text-gray-400">–</span>
						{/if}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(rule.id)}
					<tr>
						<td colspan="5" class="border-b bg-gray-100 px-4 py-3 dark:bg-gray-800">
							<div class="flex flex-col gap-2">
								<div>
									<b>Intervall:</b>
									{periodNames[rule.period] ?? rule.period}
								</div>
								<div>
									<b>Titel der erzeugten Ausgabe:</b>
									{rule.titleReplacement || 'wie Vorlage'}
								</div>
								<div>
									<b>Betrag:</b>
									{rule.amountReplacement != null
										? (rule.amountReplacement / 100).toFixed(2) + '€'
										: 'wie Vorlage'}
								</div>
								<div>
									<b>Zuletzt erzeugt:</b>
									{shortDate(rule.lastCreation)}
								</div>
								<div>
									<b>Fällig:</b>
									{#if !rule.templateId}
										Die Vorlage gibt es nicht mehr, die Regel kann nichts erzeugen.
									{:else if rule.pending.length > 0}
										{rule.pending.map(shortDate).join(', ')}
									{:else}
										nichts offen
									{/if}
								</div>

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
