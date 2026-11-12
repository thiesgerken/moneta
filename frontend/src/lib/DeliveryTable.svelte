<script lang="ts">
	import { graphql } from '$houdini';
	import {
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		TableSearch,
		Checkbox
	} from 'flowbite-svelte';
	import TableSection from './TableSection.svelte';
	import { restingAmountColor, signedEuro } from './money';
	import { chip, tableSearchClasses, tableSearchTableClass, headIcon } from './table_styles';
	import {
		CalendarMonthOutline,
		CashOutline,
		FlagOutline,
		SearchOutline,
		WalletOutline
	} from 'flowbite-svelte-icons';
	import { shortDate } from './analysis/dates';
	import QueryError from './QueryError.svelte';

	let searchTerm = $state('');
	let onlyUnmatched = $state(false);

	const deliveryStore = graphql(`
		query DeliveryList($onlyUnmatched: Boolean) {
			deliveries {
				list(limit: 200, onlyUnmatched: $onlyUnmatched, sortBy: DATE, sortDir: DESC) {
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
	$effect(() => void deliveryStore.fetch({ variables: { onlyUnmatched } }));

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

<TableSection>
	<QueryError
		errors={$deliveryStore.errors}
		message="Die Lieferungen konnten nicht geladen werden."
		class="mb-4"
	/>
	<TableSearch
		placeholder="Verwendungszweck durchsuchen"
		hoverable={true}
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

		<!-- No header below md: the two columns left are a line and its amount, and the line says
		     what the dropped columns said. -->
		<TableHead class="hidden md:table-header-group">
			<TableHeadCell class="px-4 py-3 md:whitespace-nowrap">
				<CalendarMonthOutline class={headIcon} />Datum
			</TableHeadCell>
			<TableHeadCell class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap">
				<WalletOutline class={headIcon} />Konto
			</TableHeadCell>
			<TableHeadCell class="px-2 py-3 text-right md:px-4 md:whitespace-nowrap">
				<CashOutline class={headIcon} />Betrag
			</TableHeadCell>
			<TableHeadCell class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap">
				<SearchOutline class={headIcon} />Verwendungszweck
			</TableHeadCell>
			<TableHeadCell class="hidden px-4 py-3 md:table-cell md:whitespace-nowrap">
				<FlagOutline class={headIcon} />Status
			</TableHeadCell>
		</TableHead>
		<TableBody class="divide-y">
			{#each deliveries as delivery (delivery.id)}
				<TableBodyRow>
					<TableBodyCell class="w-full max-w-0 px-2 py-3 whitespace-normal md:px-4">
						{shortDate(delivery.date)}
						<!-- the account and the statement text, which are columns of their own from md
						     on: a booking without its text says nothing about what it was -->
						<span class="mt-1 block text-xs text-gray-500 md:hidden dark:text-gray-400">
							{accountNames.get(delivery.accountId) ?? '?'} ·
							<span class="wrap-anywhere">{delivery.statement}</span>
							· {delivery.matched ? 'Ausgabe erzeugt' : 'Keine Regel'}
						</span>
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{accountNames.get(delivery.accountId) ?? '?'}
					</TableBodyCell>
					<TableBodyCell
						class="px-2 py-3 text-right align-top whitespace-nowrap md:px-4 md:align-middle {restingAmountColor(
							delivery.amount
						)}"
					>
						{signedEuro(delivery.amount)}
					</TableBodyCell>
					<TableBodyCell class="hidden max-w-md truncate px-4 py-3 text-xs md:table-cell">
						{delivery.statement}
					</TableBodyCell>
					<TableBodyCell class="hidden px-4 py-3 md:table-cell">
						{#if delivery.matched}
							<span class={chip}>Ausgabe erzeugt</span>
						{:else}
							<span class={chip}>Keine Regel</span>
						{/if}
					</TableBodyCell>
				</TableBodyRow>
			{/each}
		</TableBody>
	</TableSearch>
</TableSection>
