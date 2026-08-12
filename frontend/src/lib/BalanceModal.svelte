<script lang="ts">
	import { Modal, Label, Input, Textarea, Button, Datepicker, Select } from 'flowbite-svelte';

	import { graphql } from '$houdini';
	import type { BalanceInput } from '$houdini/graphql/inputs';
	import { formatAccount, tint } from './expense_utils';
	import { EditOutline, PlusOutline } from 'flowbite-svelte-icons';

	interface Props {
		open?: boolean;
		onsubmit?: () => Promise<void>; // used to signal that the form has been successfully submitted, need to invalidate cached stuff
		userName: string | undefined;
		accounts: {
			readonly id: string;
			readonly name: string;
			readonly color: string | null;
			readonly owner: { readonly name: string; readonly firstName: string | null };
		}[];
		balance?: {
			readonly id: string;
			readonly comments: string;
			readonly amount: number;
			readonly date: Date;
			readonly account: {
				readonly id: string;
			};
		};
	}
	let { open = $bindable(), accounts, userName: user_name, balance, onsubmit }: Props = $props();

	let comments = $derived(balance?.comments || '');
	let amount: string | undefined = $derived(
		balance?.amount ? (balance.amount / 100).toFixed(2) : undefined
	);
	let date: Date | undefined = $derived(balance?.date ? new Date(balance.date) : undefined);
	let accountId = $derived(balance?.account.id.toString() || '');

	const commentsError = $derived(false);
	const amountError = $derived(!amount || isNaN(Number(amount)));
	const dateError = $derived(!date);
	const accountError = $derived(!accountId);

	const formHasErrors = $derived(!!(commentsError || amountError || dateError || accountError));

	const UpdateBalance = graphql(`
		mutation UpdateBalance($id: UUID!, $input: BalanceInput!) {
			balances {
				update(id: $id, balance: $input) {
					id
				}
			}
		}
	`);
	const CreateBalance = graphql(`
		mutation CreateBalance($input: BalanceInput!) {
			balances {
				create(balance: $input) {
					id
				}
			}
		}
	`);

	const handleSubmit = async (event: Event) => {
		event.preventDefault();
		if (formHasErrors) return;

		const input: BalanceInput = {
			comments,
			amount: Math.round(Number(amount) * 100),
			date: date!,
			accountId
		};

		console.log('balance', balance);
		console.log('update', input);

		if (balance) await UpdateBalance.mutate({ id: balance.id, input });
		else await CreateBalance.mutate({ input });

		if (onsubmit) await onsubmit();
		open = false;
	};
</script>

<Modal title={balance ? 'Kontostand bearbeiten' : 'Neuer Kontostand'} bind:open size="lg">
	<form onsubmit={handleSubmit}>
		<div class="mb-4 grid gap-4 sm:grid-cols-2">
			<div class="sm:col-span-2">
				<Label for="comments" class="mb-2">Kommentare</Label>
				<Textarea
					id="comments"
					clearable
					placeholder="Kommentare"
					rows={comments ? 5 : 1}
					bind:value={comments}
					color={commentsError ? 'red' : undefined}
				/>
			</div>
			<div>
				<Label for="amount" class="mb-2">Betrag</Label>
				<div class="relative">
					<Input
						id="amount"
						placeholder="0"
						bind:value={amount}
						step="0.01"
						class="no-spinner pr-6"
						color={amountError ? 'red' : undefined}
						required
					/>
					<span class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
						>€</span
					>
				</div>
			</div>
			<div>
				<Label class="mb-2 block">Datum</Label>
				<Datepicker
					bind:value={date}
					placeholder="Datum"
					inputClass={dateError ? 'border-red-600 bg-red-100 text-red-400' : ''}
				/>
			</div>
			<div class="sm:col-span-2">
				<Label for="account" class="mb-2">Konto</Label>
				<Select
					items={accounts.map((a) => ({
						value: a.id,
						name: formatAccount(
							{
								name: a.name,
								owner: { name: a.owner.name, firstName: a.owner.firstName }
							},
							user_name
						)
					}))}
					bind:value={accountId}
					style={tint(accounts.find((a) => a.id === accountId)?.color)}
					selectClass={accountError ? 'text-red-700 border-red-700 bg-red-100' : undefined}
					placeholder="Konto"
					required
				/>
			</div>

			<div class="justify-center sm:col-span-2">
				<Button type="submit" class="w-full" disabled={formHasErrors}>
					{#if balance}
						<EditOutline class="mr-1 h-4 w-4" />
						Aktualisieren
					{:else}
						<PlusOutline class="mr-1 h-4 w-4" />
						Erstellen
					{/if}
				</Button>
			</div>
		</div>
	</form>
</Modal>

<style>
	:global(.no-spinner::-webkit-outer-spin-button),
	:global(.no-spinner::-webkit-inner-spin-button) {
		-webkit-appearance: none;
		margin: 0;
	}
	:global(.no-spinner) {
		-moz-appearance: textfield;
		appearance: textfield;
	}
</style>
