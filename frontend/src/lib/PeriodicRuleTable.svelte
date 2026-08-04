<script lang="ts">
	import { cache, graphql } from '$houdini';
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
		mutation DeletePeriodicRule($id: ObjectIdWrapper!) {
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
		cache.markStale('PeriodicRule');
		await ruleStore.fetch();
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
		cache.markStale('Expense');
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

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-3 sm:p-5">
	<TableSearch
		placeholder="Suchen"
		hoverable={true}
		striped={true}
		bind:inputValue={searchTerm}
		tableClass="w-full text-left text-sm"
		divClass="bg-white dark:bg-gray-800 relative shadow-md sm:rounded-lg overflow-x-auto"
		innerDivClass="flex flex-col md:flex-row items-center justify-between space-y-3 md:space-y-0 md:space-x-4 p-4"
		searchClass="w-full md:w-1/2 relative"
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
			<TableHeadCell class="px-4 py-3">Intervall</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Betrag</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Zuletzt erzeugt</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Fällig</TableHeadCell>
			<TableHeadCell class="px-4 py-3"></TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each rules as rule (rule.id)}
				<TableBodyRow>
					<TableBodyCell class="px-4 py-3">
						{rule.title}
						{#if rule.titleReplacement}
							<span class="block text-xs text-gray-500 dark:text-gray-400"
								>→ {rule.titleReplacement}</span
							>
						{/if}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">{periodNames[rule.period] ?? rule.period}</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{rule.amountReplacement != null
							? (rule.amountReplacement / 100).toFixed(2) + '€'
							: 'wie Vorlage'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3"
						>{rule.lastCreation?.toLocaleDateString('de-DE')}</TableBodyCell
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
