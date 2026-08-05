<script lang="ts">
	import { graphql } from '$houdini';
	import {
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		TableSearch,
		Badge,
		Checkbox
	} from 'flowbite-svelte';
	import { Section } from 'flowbite-svelte-blocks';
	import { tableSearchClasses, tableSearchTableClass } from './table_styles';

	let searchTerm = $state('');
	let onlyUnmatched = $state(false);

	const deliveryStore = graphql(`
		query DeliveryList($filter: String, $sort: String) {
			deliveries {
				list(limit: 200, filter: $filter, sort: $sort) {
					id
					accountId
					date
					amount
					statement
					matched
					expenseId
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

	// unmatched deliveries are the interesting ones: each is a transaction that arrived and
	// no rule claimed, so it is the queue of rules still to write
	const filter = $derived(onlyUnmatched ? JSON.stringify({ expenseId: null }) : undefined);

	$effect(
		() => void deliveryStore.fetch({ variables: { filter, sort: JSON.stringify({ date: -1 }) } })
	);

	const allDeliveries = $derived($deliveryStore.data?.deliveries.list || []);
	const accountNames = $derived(
		new Map(($deliveryStore.data?.accounts.list || []).map((a) => [a.id, a.name]))
	);

	const deliveries = $derived(
		searchTerm.trim()
			? allDeliveries.filter((d) =>
					d.statement.toLowerCase().includes(searchTerm.trim().toLowerCase())
				)
			: allDeliveries
	);
</script>

<Section name="advancedTable" sectionClass="bg-gray-50 dark:bg-gray-900 p-3 sm:p-5">
	<TableSearch
		placeholder="Verwendungszweck durchsuchen"
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
				<Checkbox bind:checked={onlyUnmatched}>Nur ohne Regel</Checkbox>
			</div>
		{/snippet}

		<TableHead>
			<TableHeadCell class="px-4 py-3">Datum</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Konto</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Betrag</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Verwendungszweck</TableHeadCell>
			<TableHeadCell class="px-4 py-3">Status</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each deliveries as delivery (delivery.id)}
				<TableBodyRow>
					<TableBodyCell class="px-4 py-3">
						{delivery.date?.toLocaleDateString('de-DE')}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{accountNames.get(delivery.accountId) ?? '?'}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{(delivery.amount / 100).toFixed(2)}€
					</TableBodyCell>
					<TableBodyCell class="max-w-md truncate px-4 py-3 text-xs">
						{delivery.statement}
					</TableBodyCell>
					<TableBodyCell class="px-4 py-3">
						{#if delivery.matched}
							<Badge color="green">Ausgabe erzeugt</Badge>
						{:else}
							<Badge color="yellow">Keine Regel</Badge>
						{/if}
					</TableBodyCell>
				</TableBodyRow>
			{/each}
		</TableBody>
	</TableSearch>
</Section>
