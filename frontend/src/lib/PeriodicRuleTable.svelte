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
		Badge,
		Modal
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import {
		PlusOutline,
		EditOutline,
		TrashBinOutline,
		ExclamationCircleOutline,
		PlayOutline
	} from 'flowbite-svelte-icons';
	import PeriodicRuleModal from './PeriodicRuleModal.svelte';
	import { tableSearchClasses, tableSearchTableClass } from './table_styles';
	import { shortDate } from './analysis/dates';
	import QueryError from './QueryError.svelte';

	let searchTerm = $state('');
	let showNewModal = $state(false);
	let showEditModal = $state(false);
	let showDeleteModal = $state(false);
	let ruleIdToEdit: string | undefined = $state(undefined);
	let ruleIdToDelete: string | undefined = $state(undefined);
	let runResult: string | undefined = $state(undefined);

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
	const rules = $derived(
		searchTerm.trim()
			? allRules.filter((r) => r.title.toLowerCase().includes(searchTerm.trim().toLowerCase()))
			: allRules
	);

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
				class="flex w-full flex-shrink-0 flex-col items-stretch justify-end space-y-2 md:w-auto md:flex-row md:items-center md:space-y-0 md:space-x-3"
			>
				{#if runResult}
					<span class="self-center text-sm text-gray-500 dark:text-gray-400">{runResult}</span>
				{/if}
				<Button color="alternative" onclick={handleRun}>
					<PlayOutline class="mr-2 h-3.5 w-3.5" />Jetzt erzeugen
				</Button>
				<Button onclick={() => (showNewModal = true)}>
					<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
				</Button>
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
			<TableHeadCell class="hidden px-4 py-3 md:table-cell">Intervall</TableHeadCell>
			<TableHeadCell class="hidden px-4 py-3 md:table-cell">Betrag</TableHeadCell>
			<TableHeadCell class="hidden px-4 py-3 md:table-cell">Zuletzt erzeugt</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Fällig</TableHeadCell>
			<TableHeadCell class="px-4 py-3"></TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each rules as rule (rule.id)}
				<TableBodyRow>
					<TableBodyCell class="w-full max-w-0 px-4 py-3 whitespace-normal">
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
						{rule.amountReplacement != null
							? (rule.amountReplacement / 100).toFixed(2) + '€'
							: 'wie Vorlage'}
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
					<TableBodyCell class="px-4 py-3 text-right">
						<button
							type="button"
							aria-label="Bearbeiten"
							onclick={() => {
								ruleIdToEdit = rule.id;
								showEditModal = true;
							}}
						>
							<EditOutline class="inline h-4 w-4" />
						</button>
						<button
							type="button"
							aria-label="Löschen"
							class="ml-2"
							onclick={() => {
								ruleIdToDelete = rule.id;
								showDeleteModal = true;
							}}
						>
							<TrashBinOutline class="inline h-4 w-4" />
						</button>
					</TableBodyCell>
				</TableBodyRow>
			{/each}
		</TableBody>
	</TableSearch>
</Section>
