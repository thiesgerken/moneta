<script lang="ts">
	import { Modal, Label, Input, Textarea, Button, Datepicker, Checkbox } from 'flowbite-svelte';

	interface Props {
		open?: boolean;

		user_name: string | undefined;
		accounts: {
			readonly id: String;
			readonly name: String;
			readonly color: string | null;
			readonly owner: { readonly name: String; readonly firstName: string | null };
		}[];
		categories: {
			readonly id: String;
			readonly name: String;
			readonly color: string | null;
		}[];
	}
	let { open = $bindable(), accounts, categories, user_name }: Props = $props();

	let title = $state('');
	let store = $state('');
	let description = $state('');
	let comments = $state('');
	let isTemplate = $state(false);
	let isUnchecked = $state(false);
	let isExtraordinary = $state(false);
	let isTaxRelevant = $state(false);
	let isIncomplete = $state(false);
	let referenceAmount: number | undefined = $state(undefined);
	let bookingStart: Date | undefined = $state(undefined);
	let bookingEnd: Date | undefined = $state(undefined);
	let hasDifferentEndDate = $state(false);

	const titleError = $derived(!title ? 'Titel ist erforderlich.' : '');
	const storeError = $derived('');
	const descriptionError = $derived('');
	const referenceAmountError = $derived('');
	const bookingStartError = $derived(!bookingStart ? 'Buchungsdatum ist erforderlich.' : '');
	const bookingEndError = $derived.by(() => {
		if (!hasDifferentEndDate || !bookingStart) return '';
		if (!bookingEnd) return 'kein Enddatum angegeben.';
		if (bookingEnd < bookingStart) return 'Enddatum darf nicht vor dem Startdatum liegen.';
		return '';
	});
	const formHasErrors = $derived(
		!!(
			titleError ||
			storeError ||
			descriptionError ||
			referenceAmountError ||
			bookingStartError ||
			bookingEndError
		)
	);

	const handleSubmit = (event: Event) => {
		event.preventDefault();

		if (formHasErrors) return;
		const endDate = hasDifferentEndDate ? bookingEnd : bookingStart;
		console.log({
			title,
			store,
			description,
			comments,
			isTemplate,
			isUnchecked,
			isExtraordinary,
			isTaxRelevant,
			isIncomplete,
			referenceAmount,
			bookingStart,
			bookingEnd: endDate
		});
	};

	import { graphql } from '$houdini';

	//     mutation xx {
	//   expenses {
	//     create(
	//       expense: {title: "sss", store: "s", description: "", comments: "", referenceAmount: 0, bookingStart: "2025-01-01T00:00:00Z", bookingEnd: "2025-01-01T00:00:00Z", isTemplate: false, isUnchecked: false, isExtraordinary: false, isTaxRelevant: false, isIncomplete: false, receipts: [], categories: [{categoryId: 4, weight: 1}], transactions: [{date: "00", accountId: 2, comments: "", statement: ""}], positions: [{units: 1, amount: 300, title: "", comments: ""}]}
	//     ) {
	//       id
	//     }
	//   }
	// }

	const CreateExpense = graphql(`
		mutation CreateExpense($input: ExpenseInput!) {
			expenses {
				create(expense: $input) {
					id
				}
			}
		}
	`);
</script>

<Modal title="Neue Ausgabe" bind:open autoclose>
	<form onsubmit={handleSubmit}>
		<div class="mb-4 grid gap-4 sm:grid-cols-2">
			<div>
				<Label for="title" class="mb-2">Titel</Label>
				<Input type="text" id="title" placeholder="Titel" bind:value={title} required />
				{#if titleError}<div class="text-sm text-red-600">{titleError}</div>{/if}
			</div>
			<div>
				<Label for="store" class="mb-2">Geschäft</Label>
				<Input type="text" id="store" placeholder="Geschäft" bind:value={store}  />
				{#if storeError}<div class="text-sm text-red-600">{storeError}</div>{/if}
			</div>
			<div>
				<Label for="referenceAmount" class="mb-2">Referenzbetrag</Label>
				<Input
					type="number"
					id="referenceAmount"
					placeholder="Betrag"
					bind:value={referenceAmount}
					min="0"
					step="0.01"
				/>
				{#if referenceAmountError}<div class="text-sm text-red-600">
						{referenceAmountError}
					</div>{/if}
			</div>
			<div class="sm:col-span-2">
				<Label for="description" class="mb-2">Beschreibung</Label>
				<Textarea
					id="description"
					placeholder="Beschreibung"
					rows={2}
					bind:value={description}

				/>
				{#if descriptionError}<div class="text-sm text-red-600">{descriptionError}</div>{/if}
			</div>
			<div class="sm:col-span-2">
				<Label for="comments" class="mb-2">Kommentare</Label>
				<Textarea id="comments" placeholder="Kommentare" rows={2} bind:value={comments} />
			</div>
			<div class="sm:col-span-2">
				<Label class="mb-2 block">Buchungsbeginn</Label>
				<Datepicker bind:value={bookingStart} placeholder="Startdatum wählen" />
				{#if bookingStartError}<div class="text-sm text-red-600">{bookingStartError}</div>{/if}
			</div>
			<div class="flex items-center gap-2 sm:col-span-2">
				<Checkbox bind:checked={hasDifferentEndDate}>Abweichendes Enddatum</Checkbox>
				<Datepicker
					bind:value={bookingEnd}
					placeholder="Enddatum wählen"
					disabled={!hasDifferentEndDate}
				/>
				{#if bookingEndError}<div class="text-sm text-red-600">{bookingEndError}</div>{/if}
			</div>
			<div class="sm:col-span-2">
				<Label class="mb-2 block">Flags</Label>
				<Checkbox bind:checked={isTemplate}>Vorlage</Checkbox>
				<Checkbox bind:checked={isUnchecked}>Unbestätigt</Checkbox>
				<Checkbox bind:checked={isIncomplete}>Unvollständig</Checkbox>
				<Checkbox bind:checked={isExtraordinary}>Außergewöhnlich</Checkbox>
				<Checkbox bind:checked={isTaxRelevant}>Steuerrelevant</Checkbox>
			</div>
			<div class="sm:col-span-2 justify-center">
				<Button type="submit" class="w-full" disabled={formHasErrors}>
					<svg
						class="-ml-1 mr-1 h-6 w-6"
						fill="currentColor"
						viewBox="0 0 20 20"
						xmlns="http://www.w3.org/2000/svg"
						><path
							fill-rule="evenodd"
							d="M10 5a1 1 0 011 1v3h3a1 1 0 110 2h-3v3a1 1 0 11-2 0v-3H6a1 1 0 110-2h3V6a1 1 0 011-1z"
							clip-rule="evenodd"
						/></svg
					>
					Erstellen
				</Button>
			</div>
		</div>
	</form>
</Modal>
