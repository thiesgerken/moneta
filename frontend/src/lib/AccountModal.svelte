<script lang="ts">
	import { Modal, Label, Input, Textarea, Button, Select } from 'flowbite-svelte';
	import ColorPicker from 'svelte-awesome-color-picker';
	import DateField from './DateField.svelte';

	import { graphql } from '$houdini';
	import type { AccountInput } from '$houdini/graphql/inputs';
	import { EditOutline, PlusOutline } from 'flowbite-svelte-icons';

	interface Props {
		open?: boolean;
		/// The depots this reader has in Renditier. Empty when there is no token, no Renditier or
		/// nothing reachable — the field then goes disabled rather than disappearing, since a
		/// missing control reads as a feature that does not exist.
		renditierAccounts?: { readonly id: number; readonly name: string }[];
		onsubmit?: () => Promise<void>; // used to signal that the form has been successfully submitted, need to invalidate cached stuff
		accounts: {
			readonly id: string;
			readonly name: string;
			readonly description: string | null;
			readonly color: string | null;
			readonly owner: { readonly name: string };
			readonly iban: string | null;
			readonly kind:
				'CASH' | 'DEBIT' | 'CREDIT' | 'DEBT' | 'STOCKS' | 'REAL_ESTATE' | 'VIRTUAL' | 'OTHER';
			readonly availability: 'IMMEDIATELY' | 'DAYS' | 'WEEKS' | 'MONTHS' | 'YEARS' | 'DECADES';
			readonly risk: 'NONE' | 'SLIGHT' | 'LOW' | 'MEDIUM' | 'HIGH' | 'HUGE';
			readonly opened: Date;
			readonly closed: Date | null;
			readonly ownershipFraction: number;
			readonly renditierAccountId: number | null;
			readonly synchronization: {
				readonly account: { readonly id: string };
				readonly invert: boolean;
			} | null;
		}[];
		account?: {
			readonly id: string;
			readonly name: string;
			readonly description: string | null;
			readonly color: string | null;
			readonly owner: { readonly name: string };
			readonly iban: string | null;
			readonly kind:
				'CASH' | 'DEBIT' | 'CREDIT' | 'DEBT' | 'STOCKS' | 'REAL_ESTATE' | 'VIRTUAL' | 'OTHER';
			readonly availability: 'IMMEDIATELY' | 'DAYS' | 'WEEKS' | 'MONTHS' | 'YEARS' | 'DECADES';
			readonly risk: 'NONE' | 'SLIGHT' | 'LOW' | 'MEDIUM' | 'HIGH' | 'HUGE';
			readonly opened: Date;
			readonly closed: Date | null;
			readonly ownershipFraction: number;
			readonly renditierAccountId: number | null;
			readonly synchronization: {
				readonly account: { readonly id: string };
				readonly invert: boolean;
			} | null;
		};
	}
	let { open = $bindable(), accounts, renditierAccounts = [], account, onsubmit }: Props = $props();

	let name = $derived(account?.name || '');
	let description = $derived(account?.description || '');
	let color = $derived(account?.color || '');
	let iban = $derived(account?.iban || '');
	let kind = $derived(account?.kind || 'CASH');
	let availability = $derived<'IMMEDIATELY' | 'DAYS' | 'WEEKS' | 'MONTHS' | 'YEARS' | 'DECADES'>(
		account?.availability || 'IMMEDIATELY'
	);
	let risk = $derived(account?.risk || 'NONE');
	let opened = $derived(account?.opened ? new Date(account.opened) : new Date());
	let closed = $derived(account?.closed ? new Date(account.closed) : undefined);
	let ownershipFraction = $derived(
		account?.ownershipFraction ? (account.ownershipFraction * 100).toFixed(0) : '100'
	);
	let renditierAccountId = $derived(
		account?.renditierAccountId === null || account?.renditierAccountId === undefined
			? ''
			: String(account.renditierAccountId)
	);
	let synchronizationAccountId = $derived(account?.synchronization?.account.id.toString() || '');
	let synchronizationInvert = $derived(account?.synchronization?.invert || false);

	const nameError = $derived(!name.trim());
	const descriptionError = $derived(false);
	const colorError = $derived(!color);
	const ibanError = $derived(false);
	const kindError = $derived(false);
	const availabilityError = $derived(false);
	const riskError = $derived(false);
	const openedError = $derived(!opened);
	const closedError = $derived(false);
	const ownershipFractionError = $derived(
		!ownershipFraction ||
			isNaN(Number(ownershipFraction)) ||
			Number(ownershipFraction) < 0 ||
			Number(ownershipFraction) > 100
	);
	const synchronizationError = $derived(false);

	const formHasErrors = $derived(
		!!(
			nameError ||
			descriptionError ||
			colorError ||
			ibanError ||
			kindError ||
			availabilityError ||
			riskError ||
			openedError ||
			closedError ||
			ownershipFractionError ||
			synchronizationError
		)
	);

	const accountKindOptions = [
		{ value: 'CASH', name: 'Bargeld' },
		{ value: 'DEBIT', name: 'Girokonto' },
		{ value: 'CREDIT', name: 'Kreditkarte' },
		{ value: 'DEBT', name: 'Schulden' },
		{ value: 'STOCKS', name: 'Aktien' },
		{ value: 'REAL_ESTATE', name: 'Immobilien' },
		{ value: 'VIRTUAL', name: 'Virtuell' },
		{ value: 'OTHER', name: 'Sonstiges' }
	];

	const availabilityOptions = [
		{ value: 'IMMEDIATELY', name: 'Sofort' },
		{ value: 'DAYS', name: 'Tage' },
		{ value: 'WEEKS', name: 'Wochen' },
		{ value: 'MONTHS', name: 'Monate' },
		{ value: 'YEARS', name: 'Jahre' },
		{ value: 'DECADES', name: 'Jahrzehnte' }
	];

	/// `''` is "not linked", and the ids are strings because that is what a `Select` binds.
	const renditierOptions = $derived([
		{ value: '', name: 'Nicht verknüpft' },
		...renditierAccounts.map((a) => ({ value: String(a.id), name: a.name }))
	]);

	const riskOptions = [
		{ value: 'NONE', name: 'Kein Risiko' },
		{ value: 'SLIGHT', name: 'Geringes Risiko' },
		{ value: 'LOW', name: 'Niedriges Risiko' },
		{ value: 'MEDIUM', name: 'Mittleres Risiko' },
		{ value: 'HIGH', name: 'Hohes Risiko' },
		{ value: 'HUGE', name: 'Sehr hohes Risiko' }
	];

	const UpdateAccount = graphql(`
		mutation UpdateAccount($id: UUID!, $input: AccountInput!) {
			accounts {
				update(id: $id, account: $input)
			}
		}
	`);
	const CreateAccount = graphql(`
		mutation CreateAccount($input: AccountInput!) {
			accounts {
				create(account: $input) {
					id
				}
			}
		}
	`);

	const handleSubmit = async (event: Event) => {
		event.preventDefault();
		if (formHasErrors) return;

		const input: AccountInput = {
			name: name.trim(),
			description: description.trim(),
			color: color.trim() || null,
			iban: iban.trim() || null,
			kind,
			availability,
			risk,
			opened,
			closed,
			ownershipFraction: Number(ownershipFraction) / 100,
			renditierAccountId: renditierAccountId.trim() ? Number(renditierAccountId) : null
		};

		console.log('account', account);
		console.log('update', input);

		if (account) await UpdateAccount.mutate({ id: account.id, input });
		else await CreateAccount.mutate({ input });

		if (onsubmit) await onsubmit();
		open = false;
	};

	// Filter out the current account from synchronization options to prevent circular references
	const availableSynchronizationAccounts = $derived(
		accounts.filter((a) => !account || a.id !== account.id.toString())
	);
</script>

<Modal title={account ? 'Konto bearbeiten' : 'Neues Konto'} bind:open size="lg">
	<form onsubmit={handleSubmit}>
		<div class="mb-4 grid gap-4 sm:grid-cols-2">
			<div class="sm:col-span-2">
				<Label for="name" class="mb-2">Name</Label>
				<Input
					id="name"
					placeholder=""
					bind:value={name}
					color={nameError ? 'red' : undefined}
					required
				/>
			</div>
			<div class="sm:col-span-2">
				<Label for="description" class="mb-2">Beschreibung</Label>
				<Textarea
					id="description"
					clearable
					placeholder=""
					rows={description ? 3 : 1}
					bind:value={description}
					color={descriptionError ? 'red' : undefined}
				/>
			</div>
			<div>
				<Label for="color" class="mb-2">Farbe</Label>
				<div class="flex gap-2">
					<Input
						id="color"
						placeholder=""
						bind:value={color}
						color={colorError ? 'red' : undefined}
						class="flex-1"
					/>
					<ColorPicker bind:hex={color} position="fixed" label="" />
				</div>
			</div>
			<div>
				<Label for="iban" class="mb-2">IBAN</Label>
				<Input id="iban" placeholder="" bind:value={iban} color={ibanError ? 'red' : undefined} />
			</div>
			<div>
				<!-- Which depot in Renditier this account stands for. Only the link is kept here;
				     nothing is ever written back to this account from it. -->
				<Label for="renditierAccountId" class="mb-2">Renditier-Depot</Label>
				<Select
					id="renditierAccountId"
					items={renditierOptions}
					bind:value={renditierAccountId}
					disabled={renditierAccounts.length === 0}
					placeholder=""
				/>
				{#if renditierAccounts.length === 0}
					<!-- Disabled and said out loud, rather than hidden: which of the three reasons it
					     is matters less than not looking like a feature that was never built. -->
					<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
						Keine Renditier-Depots verfügbar – Token hinterlegt?
					</p>
				{/if}
			</div>
			<div>
				<Label for="kind" class="mb-2">Typ</Label>
				<Select
					items={accountKindOptions}
					bind:value={kind}
					selectClass={kindError ? 'text-red-700 border-red-700 bg-red-100' : undefined}
					placeholder=""
					required
				/>
			</div>
			<div>
				<Label for="availability" class="mb-2">Verfügbarkeit</Label>
				<Select
					items={availabilityOptions}
					bind:value={availability}
					selectClass={availabilityError ? 'text-red-700 border-red-700 bg-red-100' : undefined}
					placeholder=""
					required
				/>
			</div>
			<div>
				<Label for="risk" class="mb-2">Risiko</Label>
				<Select
					items={riskOptions}
					bind:value={risk}
					selectClass={riskError ? 'text-red-700 border-red-700 bg-red-100' : undefined}
					placeholder=""
					required
				/>
			</div>
			<div>
				<Label for="ownershipFraction" class="mb-2">Anteil (%)</Label>
				<div class="relative">
					<Input
						id="ownershipFraction"
						placeholder=""
						bind:value={ownershipFraction}
						step="1"
						min="0"
						max="100"
						class="no-spinner pr-6"
						color={ownershipFractionError ? 'red' : undefined}
						required
					/>
					<span class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
						>%</span
					>
				</div>
			</div>
			<div>
				<Label class="mb-2 block">Eröffnet</Label>
				<DateField
					bind:value={opened}
					placeholder=""
					inputClass={openedError ? 'border-red-600 bg-red-100 text-red-400' : ''}
				/>
			</div>
			<div>
				<Label class="mb-2 block">Geschlossen</Label>
				<DateField
					bind:value={closed}
					placeholder=""
					inputClass={closedError ? 'border-red-600 bg-red-100 text-red-400' : ''}
					clearable
				/>
			</div>
			<div class="sm:col-span-2">
				<Label for="synchronization" class="mb-2">Synchronisierung</Label>
				<Select
					items={[
						{ value: '', name: 'Keine' },
						...availableSynchronizationAccounts.map((a) => ({
							value: a.id,
							name: `${a.owner.name} / ${a.name}`
						}))
					]}
					bind:value={synchronizationAccountId}
					selectClass={synchronizationError ? 'text-red-700 border-red-700 bg-red-100' : undefined}
					placeholder=""
					disabled
				/>
			</div>
			{#if synchronizationAccountId}
				<div class="sm:col-span-2">
					<Label class="mb-2">
						<input
							type="checkbox"
							disabled
							bind:checked={synchronizationInvert}
							class="mr-2 h-4 w-4 rounded border-gray-300 bg-gray-100 text-blue-600 focus:ring-2 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:ring-offset-gray-800 dark:focus:ring-blue-600"
						/>
						Synchronisierung umkehren
					</Label>
				</div>
			{/if}

			<div class="justify-center sm:col-span-2">
				<Button type="submit" class="w-full" disabled={formHasErrors}>
					{#if account}
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
