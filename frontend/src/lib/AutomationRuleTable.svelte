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
		ExclamationCircleOutline
	} from 'flowbite-svelte-icons';
	import AutomationRuleModal from './AutomationRuleModal.svelte';
	import { tableSearchClasses, tableSearchTableClass } from './table_styles';

	let searchTerm = $state('');
	let showNewModal = $state(false);
	let showEditModal = $state(false);
	let showDeleteModal = $state(false);
	let ruleIdToEdit: string | undefined = $state(undefined);
	let ruleIdToDelete: string | undefined = $state(undefined);

	const ruleStore = graphql(`
		query AutomationRuleList {
			automationRules {
				list(sortBy: PRIORITY, sortDir: DESC) {
					id
					title
					priority
					regexTime
					regexStatement
					filterAmount
					filterAccountId
					lastDelivery
					templateId
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

	const rules = $derived(
		searchTerm.trim()
			? allRules.filter(
					(r) =>
						r.title.toLowerCase().includes(searchTerm.trim().toLowerCase()) ||
						(r.regexStatement ?? '').toLowerCase().includes(searchTerm.trim().toLowerCase())
				)
			: allRules
	);

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
		cache.markStale('AutomationRule');
		await ruleStore.fetch();
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

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-3 sm:p-5">
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
				class="flex w-full flex-shrink-0 flex-col items-stretch justify-end space-y-2 md:w-auto md:flex-row md:items-center md:space-y-0 md:space-x-3"
			>
				<Button onclick={() => (showNewModal = true)}>
					<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
				</Button>
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Verwendungszweck</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Konto</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Betrag</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Prio</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Zuletzt</TableHeadCell>
			<TableHeadCell class="px-4 py-3"></TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each rules as rule (rule.id)}
				<TableBodyRow>
					<TableBodyCell class="px-4 py-3">
						{rule.title}
						{#if !rule.templateId}
							<Badge color="red" class="ml-2">Vorlage fehlt</Badge>
						{/if}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3 font-mono text-xs">
						{rule.regexStatement ?? '–'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{rule.filterAccountId ? (accountNames.get(rule.filterAccountId) ?? '?') : '–'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{rule.filterAmount != null ? (rule.filterAmount / 100).toFixed(2) + '€' : '–'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">{rule.priority}</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{rule.lastDelivery?.toLocaleDateString('de-DE') ?? '–'}
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
