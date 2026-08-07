<script lang="ts">
	import {
		Card,
		Input,
		Label,
		Badge,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Spinner
	} from 'flowbite-svelte';
	import { graphql } from '$houdini';
	import { prepaymentsState } from './state.svelte';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const prepaymentStore = graphql(`
		query AnalysisPrepayments($at: DateTime!) {
			analysis {
				prepayments(at: $at) {
					consumedFraction
					remaining
					expense {
						id
						title
						store
						accrualStart
						accrualEnd
						effectiveAmount
						transactions {
							date
						}
					}
				}
			}
		}
	`);

	// The open items with an unknown amount are ordinary expenses carrying a flag, so they
	// come from the expense list rather than needing an endpoint of their own. Every other
	// flag is off, which together with `normal: false` leaves exactly the incomplete ones.
	const incompleteStore = graphql(`
		query AnalysisIncompleteExpenses {
			expenses {
				list(
					flags: {
						templates: false
						taxRelevant: false
						extraordinary: false
						unchecked: false
						incomplete: true
						normal: false
					}
					sortBy: ACCRUAL_START
					sortDir: DESC
					limit: 100
				) {
					edges {
						node {
							id
							title
							store
							accrualStart
							effectiveAmount
						}
					}
				}
			}
		}
	`);

	$effect(() => {
		void prepaymentStore.fetch({ variables: { at: new Date(prepaymentsState.date) } });
	});

	$effect(() => {
		void incompleteStore.fetch();
	});

	const rows = $derived($prepaymentStore.data?.analysis.prepayments ?? []);
	const incomplete = $derived($incompleteStore.data?.expenses.list.edges.map((e) => e.node) ?? []);

	const totalPaid = $derived(rows.reduce((sum, p) => sum + p.expense.effectiveAmount, 0));
	const totalRemaining = $derived(rows.reduce((sum, p) => sum + p.remaining, 0));

	// when the money actually left, as opposed to the period its value is spread over
	const paidOn = (p: (typeof rows)[number]) =>
		p.expense.transactions.reduce<Date | null>(
			(earliest, t) => (earliest === null || t.date < earliest ? t.date : earliest),
			null
		);
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<Label class="space-y-2">
			<span>Stichtag</span>
			<Input type="date" bind:value={prepaymentsState.date} class="w-48" />
		</Label>
		{#if $prepaymentStore.fetching}
			<Spinner size="5" />
		{/if}
	</div>

	<div class="grid gap-4 md:grid-cols-3">
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Vorausgezahlt</span>
			<span class="text-2xl font-semibold">{euro(totalPaid)}</span>
		</Card>
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Davon verbraucht</span>
			<span class="text-2xl font-semibold">{euro(totalPaid - totalRemaining)}</span>
		</Card>
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Noch offen</span>
			<span class="text-2xl font-semibold text-orange-600 dark:text-orange-400"
				>{euro(totalRemaining)}</span
			>
		</Card>
	</div>

	<Card class="max-w-none p-4">
		<h3 class="text-lg font-semibold">Vorauszahlungen</h3>
		<p class="mb-2 text-sm text-gray-500 dark:text-gray-400">
			Ausgaben, deren Wert sich über einen Leistungszeitraum verteilt und an diesem Stichtag noch
			nicht aufgebraucht ist. Der Verbrauch läuft linear über den Zeitraum.
		</p>
		{#if $prepaymentStore.errors}
			<p class="text-sm text-red-600 dark:text-red-400">
				{$prepaymentStore.errors.map((e) => e.message).join(', ')}
			</p>
		{:else if rows.length === 0 && !$prepaymentStore.fetching}
			<p class="text-sm text-gray-500 dark:text-gray-400">
				Zu diesem Stichtag ist nichts offen. Eine Ausgabe taucht hier auf, sobald ihr
				Leistungszeitraum länger als ein Zeitpunkt ist.
			</p>
		{:else}
			<Table hoverable striped class="w-full text-left text-sm">
				<TableHead>
					<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Bezahlt am</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Zeitraum</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Betrag</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Verbraucht</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Rest</TableHeadCell>
				</TableHead>
				<TableBody class="divide-y">
					{#each rows as p (p.expense.id)}
						{@const paid = paidOn(p)}
						<TableBodyRow>
							<TableBodyCell class="px-4 py-3">
								{p.expense.title}
								{#if p.expense.store}
									<span class="text-gray-500 dark:text-gray-400"> · {p.expense.store}</span>
								{/if}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
								{paid ? paid.toLocaleDateString('de-DE') : '–'}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
								{p.expense.accrualStart.toLocaleDateString('de-DE')} – {p.expense.accrualEnd.toLocaleDateString(
									'de-DE'
								)}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3">{euro(p.expense.effectiveAmount)}</TableBodyCell>
							<TableBodyCell class="px-4 py-3">
								<div class="flex items-center gap-2">
									<div class="h-2 w-24 rounded bg-gray-200 dark:bg-gray-700">
										<div
											class="bg-primary-600 h-2 rounded"
											style="width: {Math.round(p.consumedFraction * 100)}%"
										></div>
									</div>
									<span class="text-xs text-gray-500 dark:text-gray-400">
										{Math.round(p.consumedFraction * 100)}%
									</span>
								</div>
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3 font-medium text-orange-600 dark:text-orange-400">
								{euro(p.remaining)}
							</TableBodyCell>
						</TableBodyRow>
					{/each}
				</TableBody>
			</Table>
		{/if}
	</Card>

	<Card class="max-w-none p-4">
		<h3 class="text-lg font-semibold">Unvollständige Ausgaben</h3>
		<p class="mb-2 text-sm text-gray-500 dark:text-gray-400">
			Hier fehlen noch Transaktionen, deren Höhe unbekannt ist – anders als bei Vorauszahlungen gibt
			es keinen Restbetrag zu berechnen.
		</p>
		{#if incomplete.length === 0}
			<p class="text-sm text-gray-500 dark:text-gray-400">Keine offenen Posten.</p>
		{:else}
			<Table hoverable striped class="w-full text-left text-sm">
				<TableHead>
					<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Datum</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Bisher gebucht</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Status</TableHeadCell>
				</TableHead>
				<TableBody class="divide-y">
					{#each incomplete as e (e.id)}
						<TableBodyRow>
							<TableBodyCell class="px-4 py-3">
								{e.title}
								{#if e.store}
									<span class="text-gray-500 dark:text-gray-400"> · {e.store}</span>
								{/if}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
								{e.accrualStart.toLocaleDateString('de-DE')}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3">{euro(e.effectiveAmount)}</TableBodyCell>
							<TableBodyCell class="px-4 py-3">
								<Badge color="yellow">Unvollständig</Badge>
							</TableBodyCell>
						</TableBodyRow>
					{/each}
				</TableBody>
			</Table>
		{/if}
	</Card>
</div>
