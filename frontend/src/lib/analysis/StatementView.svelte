<script lang="ts">
	import {
		Card,
		Input,
		Label,
		Select,
		Button,
		Badge,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell
	} from 'flowbite-svelte';
	import { DownloadOutline } from 'flowbite-svelte-icons';
	import { accounts, statement } from '$lib/analysis_mock';

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });
	const iso = (d: Date) => d.toISOString().slice(0, 10);

	let accountId = $state(accounts[0].id);
	let from = $state(iso(new Date(Date.now() - 90 * 86400000)));
	let to = $state(iso(new Date()));

	const account = $derived(accounts.find((a) => a.id === accountId) ?? accounts[0]);
	const rows = $derived(statement(accountId, new Date(from), new Date(to)));

	const turnover = $derived(
		rows.reduce(
			(acc, r) => {
				if (r.amount == null) return acc;
				if (r.amount > 0) acc.in += r.amount;
				else acc.out += r.amount;
				return acc;
			},
			{ in: 0, out: 0 }
		)
	);

	function exportCsv() {
		const head = ['Datum', 'Buchungsdatum', 'Betrag', 'Saldo', 'Titel', 'Geschäft'];
		// quote everything: titles and store names contain commas and semicolons
		const cell = (v: string) => `"${v.replace(/"/g, '""')}"`;
		const body = rows.map((r) =>
			[
				iso(r.date),
				r.bookingDate ? iso(r.bookingDate) : '',
				r.amount == null ? '' : (r.amount / 100).toFixed(2),
				(r.balance / 100).toFixed(2),
				r.title,
				r.store
			]
				.map((v) => cell(String(v)))
				.join(',')
		);

		const blob = new Blob([[head.map(cell).join(','), ...body].join('\n')], {
			type: 'text/csv;charset=utf-8'
		});
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `kontoauszug-${account.name.replace(/\s+/g, '-').toLowerCase()}-${from}_${to}.csv`;
		// a detached anchor does not reliably trigger a download in every browser
		document.body.appendChild(a);
		a.click();
		a.remove();
		URL.revokeObjectURL(url);
	}
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-end gap-4">
		<Label class="space-y-2">
			<span>Konto</span>
			<Select
				bind:value={accountId}
				items={accounts.map((a) => ({ value: a.id, name: a.name }))}
				class="w-64"
			/>
		</Label>
		<Label class="space-y-2">
			<span>Von</span>
			<Input type="date" bind:value={from} class="w-44" />
		</Label>
		<Label class="space-y-2">
			<span>Bis</span>
			<Input type="date" bind:value={to} class="w-44" />
		</Label>
		<Button color="alternative" onclick={exportCsv}>
			<DownloadOutline class="mr-2 h-4 w-4" />CSV
		</Button>
	</div>

	<div class="grid gap-4 md:grid-cols-3">
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Eingang</span>
			<span class="text-2xl font-semibold text-green-700 dark:text-green-400"
				>{euro(turnover.in)}</span
			>
		</Card>
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Ausgang</span>
			<span class="text-2xl font-semibold text-red-600 dark:text-red-400">{euro(turnover.out)}</span
			>
		</Card>
		<Card class="max-w-none p-4">
			<span class="text-sm text-gray-500 dark:text-gray-400">Saldo am {to}</span>
			<span class="text-2xl font-semibold">{euro(rows[0]?.balance ?? 0)}</span>
		</Card>
	</div>

	<Card class="max-w-none p-4">
		<h3 class="mb-2 text-lg font-semibold">
			<span
				class="mr-2 inline-block h-3 w-3 rounded-full align-middle"
				style="background-color: {account.color}"
			></span>{account.name}
		</h3>
		<div class="overflow-x-auto">
			<Table hoverable striped class="w-full text-left text-sm">
				<TableHead>
					<TableHeadCell class="px-4 py-3">Datum</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Gebucht</TableHeadCell>
					<TableHeadCell class="px-4 py-3 text-right">Betrag</TableHeadCell>
					<TableHeadCell class="px-4 py-3 text-right">Saldo</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
					<TableHeadCell class="px-4 py-3">Geschäft</TableHeadCell>
				</TableHead>
				<TableBody class="divide-y">
					{#each rows as r (r.id)}
						<TableBodyRow class={r.kind === 'expense' ? '' : 'bg-gray-50/80 dark:bg-gray-900/60'}>
							<TableBodyCell class="px-4 py-3 whitespace-nowrap">
								{r.date.toLocaleDateString('de-DE')}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3 whitespace-nowrap text-gray-500 dark:text-gray-400">
								{r.bookingDate?.toLocaleDateString('de-DE') ?? '–'}
							</TableBodyCell>
							<TableBodyCell
								class="px-4 py-3 text-right whitespace-nowrap {(r.amount ?? 0) < 0
									? 'text-red-600 dark:text-red-400'
									: 'text-green-700 dark:text-green-400'}"
							>
								{r.amount == null ? '' : euro(r.amount)}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3 text-right font-medium whitespace-nowrap">
								{euro(r.balance)}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3">
								{#if r.kind === 'expense'}
									{r.title}
								{:else}
									<span class="text-gray-600 italic dark:text-gray-300">{r.title}</span>
									{#if r.drift}
										<Badge color="red" class="ml-2">
											weicht um {euro(r.drift)} ab
										</Badge>
									{/if}
								{/if}
							</TableBodyCell>
							<TableBodyCell class="px-4 py-3 text-gray-500 dark:text-gray-400">
								{r.store}
							</TableBodyCell>
						</TableBodyRow>
					{/each}
				</TableBody>
			</Table>
		</div>
	</Card>
</div>
