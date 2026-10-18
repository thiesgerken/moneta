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
		TableHeadCell,
		Spinner
	} from 'flowbite-svelte';
	import { DownloadOutline } from 'flowbite-svelte-icons';
	import { graphql } from '$houdini';
	import { statementState } from './state.svelte';
	import { ANALYSIS_CARD } from './cards';
	import { shortDate } from './dates';
	import QueryError from '#lib/QueryError.svelte';
	import { amountColor, euro } from '#lib/money.ts';

	const iso = (d: Date) => d.toISOString().slice(0, 10);

	const accountStore = graphql(`
		query AnalysisStatementAccounts {
			accounts {
				list(onlyOpen: true, sortBy: NAME) {
					id
					name
					color
				}
			}
		}
	`);

	const statementStore = graphql(`
		query AnalysisStatement($accountId: UUID!, $from: DateTime!, $to: DateTime!) {
			analysis {
				statement(accountId: $accountId, from: $from, to: $to) {
					kind
					date
					title
					store
					expenseId
					amount
					balance
					drift
				}
			}
		}
	`);

	$effect(() => {
		void accountStore.fetch();
	});

	const accounts = $derived($accountStore.data?.accounts.list ?? []);
	const account = $derived(accounts.find((a) => a.id === statementState.accountId));

	// the picker starts empty because the account list is not there yet on first render
	$effect(() => {
		if (!statementState.accountId && accounts.length > 0) statementState.accountId = accounts[0].id;
	});

	$effect(() => {
		const { accountId, from, to } = statementState;
		if (!accountId) return;
		void statementStore.fetch({
			variables: { accountId, from: new Date(from), to: new Date(to) }
		});
	});

	const rows = $derived($statementStore.data?.analysis.statement ?? []);

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

	const label = (kind: string) =>
		kind === 'OPENING' ? 'Kontostand (berechnet)' : kind === 'BALANCE' ? 'Kontostand' : '';

	function exportCsv() {
		const head = ['Datum', 'Art', 'Betrag', 'Saldo', 'Titel', 'Geschäft'];
		// quote everything: titles and store names contain commas and semicolons
		const cell = (v: string) => `"${v.replace(/"/g, '""')}"`;
		const body = rows.map((r) =>
			[
				iso(r.date),
				r.kind,
				r.amount == null ? '' : (r.amount / 100).toFixed(2),
				(r.balance / 100).toFixed(2),
				r.title || label(r.kind),
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
		a.download = `kontoauszug-${(account?.name ?? 'konto').replace(/\s+/g, '-').toLowerCase()}-${statementState.from}_${statementState.to}.csv`;
		// a detached anchor does not reliably trigger a download in every browser
		document.body.appendChild(a);
		a.click();
		a.remove();
		URL.revokeObjectURL(url);
	}
</script>

<div class="space-y-4">
	<QueryError errors={$accountStore.errors} message="Die Konten konnten nicht geladen werden." />
	<div class="flex flex-wrap items-end gap-4">
		<Label class="space-y-2">
			<span>Konto</span>
			<Select
				bind:value={statementState.accountId}
				items={accounts.map((a) => ({ value: a.id, name: a.name }))}
				class="w-64"
			/>
		</Label>
		<Label class="space-y-2">
			<span>Von</span>
			<Input type="date" bind:value={statementState.from} class="w-44" />
		</Label>
		<Label class="space-y-2">
			<span>Bis</span>
			<Input type="date" bind:value={statementState.to} class="w-44" />
		</Label>
		<Button color="alternative" onclick={exportCsv} disabled={rows.length === 0}>
			<DownloadOutline class="mr-2 h-4 w-4" />CSV
		</Button>
		{#if $statementStore.fetching}
			<Spinner size="5" class="self-center" />
		{/if}
	</div>

	<div class="grid gap-4 md:grid-cols-3">
		<Card class={ANALYSIS_CARD}>
			<span class="text-sm text-gray-500 dark:text-gray-400">Eingang</span>
			<span class="text-2xl font-semibold text-green-700 dark:text-green-400"
				>{euro(turnover.in)}</span
			>
		</Card>
		<Card class={ANALYSIS_CARD}>
			<span class="text-sm text-gray-500 dark:text-gray-400">Ausgang</span>
			<span class="text-2xl font-semibold text-red-600 dark:text-red-400">{euro(turnover.out)}</span
			>
		</Card>
		<Card class={ANALYSIS_CARD}>
			<span class="text-sm text-gray-500 dark:text-gray-400">Saldo am {statementState.to}</span>
			<span class="text-2xl font-semibold">{euro(rows[0]?.balance ?? 0)}</span>
		</Card>
	</div>

	<Card class={ANALYSIS_CARD}>
		<h3 class="mb-2 text-lg font-semibold">
			{#if account}
				<span
					class="mr-2 inline-block h-3 w-3 rounded-full align-middle"
					style="background-color: {account.color ?? '#94a3b8'}"
				></span>{account.name}
			{/if}
		</h3>
		{#if $statementStore.errors}
			<QueryError
				errors={$statementStore.errors}
				message="Der Kontoauszug konnte nicht geladen werden."
			/>
		{:else}
			<div class="overflow-x-auto">
				<Table hoverable striped class="w-full text-left text-sm">
					<TableHead>
						<TableHeadCell class="px-4 py-3">Datum</TableHeadCell>
						<TableHeadCell class="px-4 py-3 text-right">Betrag</TableHeadCell>
						<TableHeadCell class="px-4 py-3 text-right">Saldo</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Titel</TableHeadCell>
						<TableHeadCell class="px-4 py-3">Geschäft</TableHeadCell>
					</TableHead>
					<TableBody class="divide-y">
						{#each rows as r, i (`${r.kind}-${r.date.getTime()}-${r.expenseId ?? i}`)}
							<TableBodyRow class={r.kind === 'EXPENSE' ? '' : 'bg-gray-50/80 dark:bg-gray-900/60'}>
								<TableBodyCell class="px-4 py-3 whitespace-nowrap">
									{shortDate(r.date)}
								</TableBodyCell>
								<TableBodyCell
									class="px-4 py-3 text-right whitespace-nowrap {amountColor(r.amount ?? 0)}"
								>
									{r.amount == null ? '' : euro(r.amount)}
								</TableBodyCell>
								<TableBodyCell class="px-4 py-3 text-right font-medium whitespace-nowrap">
									{euro(r.balance)}
								</TableBodyCell>
								<TableBodyCell class="px-4 py-3">
									{#if r.kind === 'EXPENSE'}
										{r.title}
									{:else}
										<span class="text-gray-600 italic dark:text-gray-300">{label(r.kind)}</span>
										{#if r.drift}
											<Badge color="red" class="ml-2">weicht um {euro(r.drift)} ab</Badge>
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
		{/if}
	</Card>
</div>
