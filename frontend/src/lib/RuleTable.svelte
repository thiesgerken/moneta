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
		Dropdown,
		DropdownItem,
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
		ChevronRightOutline,
		TextSizeOutline,
		WandMagicSparklesOutline,
		SearchOutline,
		ClockArrowOutline,
		BellOutline
	} from 'flowbite-svelte-icons';
	import AutomationRuleModal from './AutomationRuleModal.svelte';
	import PeriodicRuleModal from './PeriodicRuleModal.svelte';
	import OptionFilter from './OptionFilter.svelte';
	import FilterChips, { type ActiveFilter } from './FilterChips.svelte';
	import RegexText from './RegexText.svelte';
	import QueryError from './QueryError.svelte';
	import {
		tableSearchClasses,
		tableSearchTableClass,
		toolbarControl,
		headIcon
	} from './table_styles';
	import { shortDate } from './analysis/dates';
	import { describeAccrual, formatAccrualPeriod } from './expense_utils';
	import { euro } from './money';

	let searchTerm = $state('');
	let expandedRows = $state<string[]>([]);
	let showNewAutomation = $state(false);
	let showNewPeriodic = $state(false);
	// one flag per kind: the two dialogs are different components, and a single flag would open
	// both of them over each other
	let showEditAutomation = $state(false);
	let showEditPeriodic = $state(false);
	let showDeleteModal = $state(false);
	let ruleIdToEdit: string | undefined = $state(undefined);
	let ruleIdToDelete: string | undefined = $state(undefined);
	let runResult: string | undefined = $state(undefined);
	/// which kinds the list is showing; empty means all of them
	let filteredKinds = $state<string[]>([]);

	const store = graphql(`
		query RuleList {
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
			accounts {
				list(limit: 1000) {
					id
					name
				}
			}
		}
	`);

	$effect(() => void store.fetch());

	const automation = $derived($store.data?.automationRules.list ?? []);
	const periodic = $derived($store.data?.periodicRules.list ?? []);
	const accounts = $derived($store.data?.accounts.list ?? []);
	const accountNames = $derived(new Map(accounts.map((a) => [a.id, a.name])));

	const PERIOD_NAMES: Record<string, string> = {
		DAILY: 'Täglich',
		WEEKLY: 'Wöchentlich',
		MONTHLY: 'Monatlich',
		YEARLY: 'Jährlich'
	};

	const KINDS = [
		{ value: 'automation', name: 'Bankumsatz' },
		{ value: 'periodic', name: 'Kalender' }
	];

	/// One row per rule of either kind.
	///
	/// The two are the same thing -- a template plus a trigger, producing an expense -- and only
	/// what fires them differs, which is a value rather than a section of the page. Everything
	/// kind-specific is folded into `trigger`; the expanded row and the dialog stay per kind.
	///
	/// Betrag deliberately does not become a column: on an automation rule it is a condition
	/// (match −12,34 €) and on a periodic one an instruction (book −800,00 €). Same word,
	/// opposite direction. The first is part of the trigger, the second is not summarised.
	type Row = {
		id: string;
		kind: 'automation' | 'periodic';
		title: string;
		subtitle: string | null;
		/// the text the trigger sorts and searches on
		trigger: string;
		lastUsed: Date | null;
		hasTemplate: boolean;
		/// how far ahead of the others this rule is: priority, or how much it owes
		open: number;
		automation?: (typeof automation)[number];
		periodic?: (typeof periodic)[number];
	};

	const rows = $derived.by<Row[]>(() => [
		...automation.map((r) => ({
			id: r.id,
			kind: 'automation' as const,
			title: r.title,
			subtitle: null,
			trigger: r.regexStatement ?? '',
			lastUsed: r.lastDelivery,
			hasTemplate: !!r.templateId,
			open: r.priority,
			automation: r
		})),
		...periodic.map((r) => ({
			id: r.id,
			kind: 'periodic' as const,
			title: r.title,
			subtitle: r.titleReplacement,
			trigger: PERIOD_NAMES[r.period] ?? r.period,
			lastUsed: r.lastCreation,
			hasTemplate: !!r.templateId,
			open: r.pending.length,
			periodic: r
		}))
	]);

	/// Most recently used first: a rule that fired last week is one you still have, and the long
	/// tail that has not matched anything for years is not what the page is for. A third click
	/// on a header comes back here.
	const DEFAULT_SORT = { field: 'lastUsed', direction: -1 };
	let sortBy: { field: string; direction: number } = $state({ ...DEFAULT_SORT });

	function sortByField(field: string) {
		if (sortBy.field !== field) sortBy = { field, direction: -1 };
		else if (sortBy.direction === -1) sortBy = { field, direction: 1 };
		else sortBy = { ...DEFAULT_SORT };
	}

	const isSortedBy = (field: string) => (sortBy.field === field ? sortBy.direction : null);

	const sortKeys: Record<string, (r: Row) => string | number> = {
		title: (r) => r.title.toLowerCase(),
		kind: (r) => r.kind,
		trigger: (r) => r.trigger.toLowerCase(),
		lastUsed: (r) => r.lastUsed?.getTime() ?? 0,
		open: (r) => r.open
	};

	const toggleOption = (selected: string[], id: string) =>
		selected.includes(id) ? selected.filter((other) => other !== id) : [...selected, id];

	const visible = $derived.by(() => {
		const term = searchTerm.trim().toLowerCase();
		const filtered = rows.filter(
			(r) =>
				(filteredKinds.length === 0 || filteredKinds.includes(r.kind)) &&
				(!term || r.title.toLowerCase().includes(term) || r.trigger.toLowerCase().includes(term))
		);

		const key = sortKeys[sortBy.field] ?? sortKeys.lastUsed;
		return [...filtered].sort((a, b) => {
			const [x, y] = [key(a), key(b)];
			if (x === y) return a.title.localeCompare(b.title);
			// -1 is descending, as everywhere else here
			return (x < y ? -1 : 1) * sortBy.direction;
		});
	});

	const activeFilters = $derived.by(() => {
		const out: ActiveFilter[] = [];

		if (searchTerm.trim()) {
			out.push({ label: `Suche: ${searchTerm.trim()}`, clear: () => (searchTerm = '') });
		}
		for (const kind of filteredKinds) {
			out.push({
				label: KINDS.find((k) => k.value === kind)?.name ?? kind,
				clear: () => (filteredKinds = toggleOption(filteredKinds, kind))
			});
		}

		return out;
	});

	function resetFilters() {
		searchTerm = '';
		filteredKinds = [];
	}

	function toggleRow(id: string) {
		expandedRows = expandedRows.includes(id)
			? expandedRows.filter((rowId) => rowId !== id)
			: [...expandedRows, id];
	}

	/// What the toolbar acts on. An open row is this list's idea of "the one you mean", and
	/// several open rows are no answer at all rather than a reason to pick the first.
	const selected = $derived(
		expandedRows.length === 1 ? rows.find((r) => r.id === expandedRows[0]) : undefined
	);

	const ruleToEdit = $derived(rows.find((r) => r.id === ruleIdToEdit));
	const ruleToDelete = $derived(rows.find((r) => r.id === ruleIdToDelete));

	const DeleteAutomationRule = graphql(`
		mutation DeleteAutomationRuleFromList($id: UUID!) {
			automationRules {
				delete(id: $id)
			}
		}
	`);
	const DeletePeriodicRule = graphql(`
		mutation DeletePeriodicRuleFromList($id: UUID!) {
			periodicRules {
				delete(id: $id)
			}
		}
	`);
	const RunPeriodicRules = graphql(`
		mutation RunPeriodicRulesFromList {
			periodicRules {
				run
			}
		}
	`);

	async function reload() {
		invalidate();
		await store.fetch(AFTER_WRITE);
		expandedRows = [];
	}

	async function handleSubmit() {
		showNewAutomation = false;
		showNewPeriodic = false;
		showEditAutomation = false;
		showEditPeriodic = false;
		await reload();
	}

	async function handleDelete() {
		if (!ruleToDelete) return;

		if (ruleToDelete.kind === 'automation') {
			await DeleteAutomationRule.mutate({ id: ruleToDelete.id });
		} else {
			await DeletePeriodicRule.mutate({ id: ruleToDelete.id });
		}

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

	function edit(id: string) {
		ruleIdToEdit = id;
		if (rows.find((r) => r.id === id)?.kind === 'automation') showEditAutomation = true;
		else showEditPeriodic = true;
	}
</script>

{#snippet sortIcon(field: string)}
	{#if isSortedBy(field) === -1}
		▲
	{:else if isSortedBy(field) === 1}
		▼
	{/if}
{/snippet}

{#snippet kindBadge(kind: Row['kind'])}
	<Badge color={kind === 'automation' ? 'blue' : 'purple'}>
		{kind === 'automation' ? 'Bankumsatz' : 'Kalender'}
	</Badge>
{/snippet}

{#snippet amount(cents: number | null, absent: string)}
	{#if cents == null}
		<span class="text-gray-400 dark:text-gray-500">{absent}</span>
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
		Prio {value}
	</span>
{/snippet}

<!-- What makes a rule fire, whatever fires it. For a bank rule that is the pattern plus
     whatever else it insists on; for a periodic one it is the calendar. -->
{#snippet trigger(row: Row)}
	{#if row.automation}
		<RegexText pattern={row.automation.regexStatement} empty="jeder Umsatz" />
		{#if row.automation.filterAccountId || row.automation.filterAmount != null || row.automation.regexTime}
			<span class="mt-0.5 block text-xs text-gray-500 dark:text-gray-400">
				{#if row.automation.filterAccountId}
					{accountNames.get(row.automation.filterAccountId) ?? 'unbekanntes Konto'}
				{/if}
				{#if row.automation.filterAmount != null}
					· {euro(row.automation.filterAmount)}
				{/if}
				{#if row.automation.regexTime}
					· Datum <RegexText pattern={row.automation.regexTime} />
				{/if}
			</span>
		{/if}
	{:else}
		{row.trigger}
	{/if}
{/snippet}

{#snippet newButton(cls: string)}
	<!-- `primary` spelled out: inside a ButtonGroup a Button with no colour of its own falls back
	     to `alternative` rather than to primary. -->
	<Button data-new-rule color="primary" class="{toolbarControl} {cls}">
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
		<ChevronDownOutline class="ml-2 h-3 w-3" />
	</Button>
{/snippet}

{#snippet runButton(cls: string)}
	<!-- A `title` rather than a flowbite Tooltip: that renders a sibling element, and inside the
	     ButtonGroup it would take the last-child position this button needs for its rounded edge. -->
	<Button
		color="teal"
		class="{toolbarControl} {cls}"
		title="Erzeugt die fälligen Ausgaben aller periodischen Regeln, nicht nur der ausgewählten."
		onclick={handleRun}
	>
		<PlayOutline class="mr-2 h-3.5 w-3.5" />Jetzt erzeugen
	</Button>
{/snippet}

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

<AutomationRuleModal bind:open={showNewAutomation} onsubmit={handleSubmit} {accounts} />
<AutomationRuleModal
	bind:open={showEditAutomation}
	onsubmit={handleSubmit}
	{accounts}
	rule={ruleToEdit?.automation}
/>
<PeriodicRuleModal bind:open={showNewPeriodic} onsubmit={handleSubmit} />
<PeriodicRuleModal
	bind:open={showEditPeriodic}
	onsubmit={handleSubmit}
	rule={ruleToEdit?.periodic}
/>

<!-- Outside the ButtonGroup and hung off the button by attribute: rendered as a sibling it
     would take the last-child position the group's border collapsing depends on, and the
     attribute matches both copies of the button -- the phone's and the desktop's -- where an
     id could only belong to one of them. -->
<Dropdown triggeredBy="[data-new-rule]" class="w-56">
	<DropdownItem onclick={() => (showNewAutomation = true)}>
		<b>Bankumsatz</b>
		<span class="block text-xs text-gray-500 dark:text-gray-400">
			wenn eine passende Buchung ankommt
		</span>
	</DropdownItem>
	<DropdownItem onclick={() => (showNewPeriodic = true)}>
		<b>Kalender</b>
		<span class="block text-xs text-gray-500 dark:text-gray-400">
			in festem Abstand, ohne Anlass
		</span>
	</DropdownItem>
</Dropdown>

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-0 sm:p-5">
	<QueryError
		errors={$store.errors}
		message="Die Regeln konnten nicht geladen werden."
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
				     Neu -- and there the open row carries the same two buttons itself. Jetzt erzeugen
				     acts on every periodic rule at once, so it stays whatever is open. -->
				<div class="flex items-center gap-2 md:hidden">
					{@render newButton('grow')}
					{@render runButton('')}
				</div>
				<ButtonGroup class="hidden md:inline-flex">
					{@render newButton('')}
					<ActionButton
						color="yellow"
						class="{toolbarControl} text-gray-900"
						disabled={!selected}
						onclick={() => edit(selected!.id)}
					>
						<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
					</ActionButton>
					<ActionButton
						color="red"
						class={toolbarControl}
						disabled={!selected}
						onclick={() => {
							ruleIdToDelete = selected!.id;
							showDeleteModal = true;
						}}
					>
						<TrashBinOutline class="mr-2 h-3.5 w-3.5" />Löschen
					</ActionButton>
					{@render runButton('')}
				</ButtonGroup>
				<OptionFilter
					label="Auslöser"
					options={KINDS}
					selected={filteredKinds}
					toggle={(id) => (filteredKinds = toggleOption(filteredKinds, id))}
				>
					{#snippet icon()}
						<WandMagicSparklesOutline class="h-4 w-4 shrink-0 text-gray-400 dark:text-gray-500" />
					{/snippet}
				</OptionFilter>
				{#if runResult}
					<span class="self-center text-sm text-gray-500 dark:text-gray-400">{runResult}</span>
				{/if}
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell
				class="px-4 py-3 md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('title')}
			>
				<TextSizeOutline class={headIcon} />Titel{@render sortIcon('title')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('kind')}
			>
				<WandMagicSparklesOutline class={headIcon} />Auslöser{@render sortIcon('kind')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('trigger')}
			>
				<SearchOutline class={headIcon} />Bedingung{@render sortIcon('trigger')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('lastUsed')}
			>
				<ClockArrowOutline class={headIcon} />Zuletzt{@render sortIcon('lastUsed')}
			</TableHeadCell>
			<TableHeadCell
				class="px-4 py-3 md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('open')}
			>
				<BellOutline class={headIcon} />Offen{@render sortIcon('open')}
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each visible as row (row.id)}
				<TableBodyRow onclick={() => toggleRow(row.id)}>
					<TableBodyCell
						class="w-full max-w-0 px-4 py-3 whitespace-normal md:w-64 md:max-w-64 md:min-w-64"
					>
						<button type="button" class="mr-2 focus:outline-none" aria-label="Details">
							{#if expandedRows.includes(row.id)}
								<ChevronDownOutline class="inline h-4 w-4" />
							{:else}
								<ChevronRightOutline class="inline h-4 w-4" />
							{/if}
						</button>
						{row.title}
						{#if !row.hasTemplate}
							<Badge color="red" class="ml-2">Vorlage fehlt</Badge>
						{/if}
						{#if row.subtitle}
							<span class="block text-xs text-gray-500 dark:text-gray-400">→ {row.subtitle}</span>
						{/if}
						<!-- Four columns do not fit on a phone, and a rule is nothing without what fires
						     it, so they come back as a line under the title rather than being dropped. -->
						<span class="mt-1 block text-xs text-gray-500 md:hidden dark:text-gray-400">
							{@render trigger(row)}
							· {row.lastUsed ? shortDate(row.lastUsed) : 'nie'}
						</span>
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{@render kindBadge(row.kind)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 text-xs md:table-cell md:w-full md:max-w-0">
						{@render trigger(row)}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap">
						{row.lastUsed ? shortDate(row.lastUsed) : '–'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{#if row.automation}
							{@render prio(row.open)}
						{:else if row.open > 0}
							<Badge color="yellow">{row.open} fällig</Badge>
						{:else}
							<span class="text-gray-400 dark:text-gray-500">–</span>
						{/if}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(row.id)}
					<tr>
						<td colspan="5" class="border-b bg-gray-100 px-4 py-3 dark:bg-gray-800">
							<div class="flex flex-col gap-2">
								{#if row.automation}
									{@const rule = row.automation}
									<div>
										<b>Verwendungszweck:</b>
										<RegexText pattern={rule.regexStatement} empty="beliebig" />
									</div>
									<div>
										<b>Datum:</b>
										<RegexText pattern={rule.regexTime} empty="beliebig" />
									</div>
									<div>
										<b>Konto:</b>
										{rule.filterAccountId
											? (accountNames.get(rule.filterAccountId) ?? 'unbekannt')
											: 'beliebig'}
									</div>
									<div>
										<b>Betrag, auf den sie passt:</b>
										{@render amount(rule.filterAmount, 'beliebig')}
									</div>
									<div>
										<b>Priorität:</b>
										{@render prio(rule.priority)}
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
								{:else if row.periodic}
									{@const rule = row.periodic}
									<div>
										<b>Intervall:</b>
										{PERIOD_NAMES[rule.period] ?? rule.period}
									</div>
									<div>
										<b>Titel der erzeugten Ausgabe:</b>
										{rule.titleReplacement || 'wie Vorlage'}
									</div>
									<div>
										<b>Betrag, den sie bucht:</b>
										{@render amount(rule.amountReplacement, 'wie Vorlage')}
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
								{/if}

								<div>
									<b>Zuletzt ausgelöst:</b>
									{row.lastUsed ? shortDate(row.lastUsed) : 'nie'}
								</div>

								<div class="flex gap-2">
									<Button color="yellow" size="xs" onclick={() => edit(row.id)}>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="red"
										size="xs"
										onclick={() => {
											ruleIdToDelete = row.id;
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
