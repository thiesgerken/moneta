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
		TableHeadCell
	} from 'flowbite-svelte';
	import { incompleteExpenses, prepayments } from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	let date = $state(new Date().toISOString().slice(0, 10));

	const rows = $derived(prepayments(new Date(date)));
	const totalPaid = $derived(rows.reduce((sum, p) => sum + p.amount, 0));
	const totalRemaining = $derived(rows.reduce((sum, p) => sum + p.remaining, 0));
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<Label class="space-y-2">
			<span>Stichtag</span>
			<Input type="date" bind:value={date} class="w-48" />
		</Label>
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
		<h3 class="mb-2 text-lg font-semibold">Vorauszahlungen</h3>
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
				{#each rows as p (p.id)}
					<TableBodyRow>
						<TableBodyCell class="px-4 py-3">{p.title}</TableBodyCell>
						<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
							{p.paidOnDate.toLocaleDateString('de-DE')}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
							{#if p.endDate.getTime() <= p.startDate.getTime()}
								<span class="italic">kein Zeitraum</span>
							{:else}
								{p.startDate.toLocaleDateString('de-DE')} – {p.endDate.toLocaleDateString('de-DE')}
							{/if}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3">{euro(p.amount)}</TableBodyCell>
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
	</Card>

	<Card class="max-w-none p-4">
		<h3 class="text-lg font-semibold">Unvollständige Ausgaben</h3>
		<p class="mb-2 text-sm text-gray-500 dark:text-gray-400">
			Hier fehlen noch Transaktionen, deren Höhe unbekannt ist – anders als bei Vorauszahlungen gibt
			es keinen Restbetrag zu berechnen.
		</p>
		<Table hoverable striped class="w-full text-left text-sm">
			<TableHead>
				<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Datum</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Bisher gebucht</TableHeadCell>
				<TableHeadCell class="px-4 py-3">Status</TableHeadCell>
			</TableHead>
			<TableBody class="divide-y">
				{#each incompleteExpenses as e (e.id)}
					<TableBodyRow>
						<TableBodyCell class="px-4 py-3">{e.title}</TableBodyCell>
						<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
							{e.date.toLocaleDateString('de-DE')}
						</TableBodyCell>
						<TableBodyCell class="px-4 py-3">{euro(e.amount)}</TableBodyCell>
						<TableBodyCell class="px-4 py-3"
							><Badge color="yellow">Unvollständig</Badge></TableBodyCell
						>
					</TableBodyRow>
				{/each}
			</TableBody>
		</Table>
	</Card>
</div>
