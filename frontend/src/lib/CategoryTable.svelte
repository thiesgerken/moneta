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
		AnnotationOutline,
		CashOutline,
		LayersOutline,
		TextSizeOutline
	} from 'flowbite-svelte-icons';
	import CategoryModal from './CategoryModal.svelte';
	import {
		tableSearchClasses,
		tableSearchTableClass,
		toolbarControl,
		headIcon
	} from './table_styles';
	import QueryError from './QueryError.svelte';

	let searchTerm = $state('');
	let expandedRows = $state<string[]>([]);

	/// What the toolbar acts on. An open row is this list's idea of "the one you mean", and
	/// several open rows are no answer at all rather than a reason to pick the first.
	const selectedCategoryId = $derived(expandedRows.length === 1 ? expandedRows[0] : undefined);
	/// By name, and a third click on a header comes back here.
	const DEFAULT_SORT = [{ field: 'name', direction: 1 }];
	let sortBy: { field: string; direction: number }[] = $state(DEFAULT_SORT.map((s) => ({ ...s })));

	let showNewCategoryModal = $state(false);
	let showEditCategoryModal = $state(false);
	let showDeleteCategoryModal = $state(false);
	let categoryIdToEdit: string | undefined = $state(undefined);
	let categoryIdToDelete: string | undefined = $state(undefined);

	const categoryStore = graphql(`
		query CategoryTableCategories {
			categories {
				list(limit: 1000) {
					id
					name
					description
					color
					parent {
						id
						name
					}
					monthlyLimit
				}
			}
		}
	`);

	$effect(() => void categoryStore.fetch());

	const totalCount = $derived($categoryStore.data?.categories.list.length || 0);

	let allCategories = $derived($categoryStore.data?.categories.list || []);

	// Filter and sort categories locally
	let filteredCategories = $derived.by(() => {
		let filtered = allCategories;

		// Apply search filter
		if (searchTerm) {
			const searchLower = searchTerm.toLowerCase();
			filtered = filtered.filter(
				(category) =>
					category.name.toLowerCase().includes(searchLower) ||
					(category.description && category.description.toLowerCase().includes(searchLower))
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
					case 'parent':
						aVal = a.parent?.name || '';
						bVal = b.parent?.name || '';
						break;
					case 'monthlyLimit':
						aVal = a.monthlyLimit || 0;
						bVal = b.monthlyLimit || 0;
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

	let categoryToEdit = $derived(allCategories.find((c) => c.id === categoryIdToEdit) || undefined);
	let categoryToDelete = $derived(
		allCategories.find((c) => c.id === categoryIdToDelete) || undefined
	);

	function toggleRow(id: string) {
		if (expandedRows.includes(id)) expandedRows = expandedRows.filter((rowId) => rowId !== id);
		else expandedRows = [...expandedRows, id];
	}

	/// Descending, ascending, off -- a header clicked by accident is otherwise only undone by
	/// reloading the page, since the sort lives here rather than in the url.
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

	const DeleteCategory = graphql(`
		mutation DeleteCategory($id: UUID!) {
			categories {
				delete(id: $id)
			}
		}
	`);

	async function handleCategoryModalSubmit() {
		invalidate();
		await categoryStore.fetch({ ...AFTER_WRITE });
		expandedRows = [];
	}

	async function handleDeleteCategory() {
		if (!categoryToDelete) return;
		await DeleteCategory.mutate({ id: categoryToDelete.id });
		categoryIdToDelete = undefined;
		showDeleteCategoryModal = false;

		invalidate();
		await categoryStore.fetch({ ...AFTER_WRITE });
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

<Modal bind:open={showDeleteCategoryModal} size="xs">
	<div class="text-center">
		<ExclamationCircleOutline class="mx-auto mb-4 h-12 w-12 text-gray-400 dark:text-gray-200" />
		<h3 class="mb-5 text-lg font-normal text-gray-500 dark:text-gray-400">
			Soll die Kategorie <b>{categoryToDelete?.name}</b> wirklich gelöscht werden?
		</h3>
		<div class="space-x-2">
			<Button type="submit" value="delete" color="red" onclick={handleDeleteCategory}
				>Löschen</Button
			>
			<Button
				type="submit"
				value="cancel"
				color="alternative"
				onclick={() => {
					showDeleteCategoryModal = false;
					categoryIdToDelete = undefined;
				}}
			>
				Abbrechen
			</Button>
		</div>
	</div>
</Modal>

<CategoryModal
	bind:open={showNewCategoryModal}
	onsubmit={handleCategoryModalSubmit}
	categories={allCategories.map((c) => ({
		id: c.id.toString(),
		name: c.name,
		description: c.description,
		color: c.color,
		parent: c.parent ? { id: c.parent.id.toString() } : null,
		monthlyLimit: c.monthlyLimit
	}))}
/>

<CategoryModal
	bind:open={showEditCategoryModal}
	onsubmit={handleCategoryModalSubmit}
	category={categoryToEdit}
	categories={allCategories.map((c) => ({
		id: c.id.toString(),
		name: c.name,
		description: c.description,
		color: c.color,
		parent: c.parent ? { id: c.parent.id.toString() } : null,
		monthlyLimit: c.monthlyLimit
	}))}
/>

{#snippet newButton(cls: string)}
	<!-- `primary` spelled out: inside a ButtonGroup a Button with no colour of its own falls
		     back to `alternative` rather than to primary. -->
	<Button
		color="primary"
		class="{toolbarControl} {cls}"
		onclick={() => (showNewCategoryModal = true)}
	>
		<PlusOutline class="mr-2 h-3.5 w-3.5" />Neu
	</Button>
{/snippet}

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-0 sm:p-5">
	<QueryError
		errors={$categoryStore.errors}
		message="Die Kategorien konnten nicht geladen werden."
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
						disabled={!selectedCategoryId}
						onclick={() => {
							categoryIdToEdit = selectedCategoryId;
							showEditCategoryModal = true;
						}}
					>
						<EditOutline class="mr-2 h-3.5 w-3.5" />Bearbeiten
					</ActionButton>
					<ActionButton
						color="red"
						class={toolbarControl}
						disabled={!selectedCategoryId}
						onclick={() => {
							categoryIdToDelete = selectedCategoryId;
							showDeleteCategoryModal = true;
						}}
					>
						<TrashBinOutline class="mr-2 h-3.5 w-3.5" />Löschen
					</ActionButton>
				</ButtonGroup>
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
				onclick={() => sortByField('parent')}
			>
				<LayersOutline class={headIcon} />Übergeordnet
				{@render sortIcon('parent')}
			</TableHeadCell>
			<TableHeadCell
				class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap"
				scope="col"
				onclick={() => sortByField('monthlyLimit')}
			>
				<CashOutline class={headIcon} />Monatliches Limit
				{@render sortIcon('monthlyLimit')}
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each filteredCategories as category (category.id)}
				<TableBodyRow onclick={() => toggleRow(category.id)}>
					<TableBodyCell
						class="w-full max-w-0 px-4 py-3 wrap-anywhere whitespace-normal md:w-56 md:max-w-56 md:min-w-56"
					>
						<button type="button" class="mr-2 focus:outline-none">
							{#if expandedRows.includes(category.id)}
								<ChevronDownOutline class="inline h-4 w-4" />
							{:else}
								<ChevronRightOutline class="inline h-4 w-4" />
							{/if}
						</button>
						<span
							class="mr-2 inline-block h-3 w-3 rounded-full"
							style={category.color ? `background-color: ${category.color}` : undefined}
						></span>
						{category.name}
					</TableBodyCell>
					<TableBodyCell class="hidden truncate px-4 py-3 md:table-cell md:w-full md:max-w-0">
						{category.description || '-'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{category.parent?.name || '-'}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{category.monthlyLimit ? (category.monthlyLimit / 100).toFixed(2) + '€' : '-'}
					</TableBodyCell>
				</TableBodyRow>
				{#if expandedRows.includes(category.id)}
					<tr>
						<td colspan="4" class="border-b bg-gray-100 px-4 py-3 dark:bg-gray-800">
							<div class="flex flex-col gap-2">
								<div>
									<b>Name:</b>
									{category.name}
								</div>
								{#if category.description}
									<div>
										<b>Beschreibung:</b>
										{category.description}
									</div>
								{/if}
								<div>
									<b>Farbe:</b>
									{#if category.color}
										<span class="inline-flex items-center gap-2">
											<div
												class="h-4 w-4 rounded border border-gray-300"
												style={`background-color: ${category.color}`}
											></div>
											{category.color}
										</span>
									{:else}
										Keine
									{/if}
								</div>
								<div>
									<b>Übergeordnete Kategorie:</b>
									{category.parent?.name || 'Keine'}
								</div>
								<div>
									<b>Monatliches Limit:</b>
									{category.monthlyLimit
										? (category.monthlyLimit / 100).toFixed(2) + '€'
										: 'Kein Limit'}
								</div>

								<div class="flex gap-2">
									<Button
										color="yellow"
										size="xs"
										onclick={() => {
											categoryIdToEdit = category.id.toString();
											showEditCategoryModal = true;
										}}
									>
										<EditOutline class="mr-2 h-3.5 w-3.5" />
										Bearbeiten
									</Button>
									<Button
										color="red"
										size="xs"
										onclick={() => {
											categoryIdToDelete = category.id.toString();
											showDeleteCategoryModal = true;
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
						>{filteredCategories.length}</span
					>
					von
					<span class="font-semibold text-gray-900 dark:text-white">{totalCount}</span>
				</span>
			</div>
		{/snippet}
	</TableSearch>
</Section>
