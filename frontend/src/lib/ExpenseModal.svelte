<script lang="ts">
	import {
		Modal,
		Label,
		Input,
		Textarea,
		Button,
		Datepicker,
		Checkbox,
		Select
	} from 'flowbite-svelte';

	import { graphql } from '$houdini';
	import type { ExpenseInput } from '$houdini/graphql/inputs';
	import { formatAccount } from './expense_utils';
	import {
		CircleMinusOutline,
		CirclePlusOutline,
		EditOutline,
		PlusOutline
	} from 'flowbite-svelte-icons';

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
		categories: {
			readonly id: string;
			readonly name: string;
			readonly color: string | null;
		}[];
		expense?: {
			readonly id: string;
			readonly title: string;
			readonly description: string;
			readonly store: string;
			readonly bookingStart: Date;
			readonly bookingEnd: Date;
			readonly comments: string;
			readonly isTaxRelevant: boolean;
			readonly isTemplate: boolean;
			readonly isUnchecked: boolean;
			readonly isIncomplete: boolean;
			readonly isExtraordinary: boolean;
			readonly receipts: {
				readonly uri: string;
			}[];

			readonly positions: {
				readonly title: string;
				readonly amount: number | null;
				readonly quantity: number | null;
				readonly unit: string | null;
				readonly comments: string;
			}[];
			readonly categories: {
				readonly weight: number;
				readonly category: {
					readonly id: string;
				};
			}[];
			readonly referenceAmount: number | null;
			readonly transactions: {
				readonly fraction: number | null;
				readonly effectiveAmount: number;
				readonly amount: number | null;
				readonly date: Date;
				readonly statement: string;
				readonly account: {
					readonly id: string;
				};
			}[];
		};
	}
	let {
		open = $bindable(),
		accounts,
		categories,
		userName: user_name,
		expense,
		onsubmit
	}: Props = $props();

	let title = $derived(expense?.title || '');
	let store = $derived(expense?.store || '');
	let description = $derived(expense?.description || '');
	let comments = $derived(expense?.comments || '');
	let isTemplate = $derived(expense?.isTemplate || false);
	let isUnchecked = $derived(expense?.isUnchecked || false);
	let isExtraordinary = $derived(expense?.isExtraordinary || false);
	let isTaxRelevant = $derived(expense?.isTaxRelevant || false);
	let isIncomplete = $derived(expense?.isIncomplete || false);
	let referenceAmount: string | undefined = $derived(
		expense?.referenceAmount ? (expense.referenceAmount / 100).toFixed(2) : undefined
	);
	let bookingStart: Date | undefined = $derived(
		expense?.bookingStart ? new Date(expense.bookingStart) : undefined
	);
	let bookingEnd: Date | undefined = $derived(
		expense?.bookingEnd ? new Date(expense.bookingEnd) : undefined
	);

	// Categories and transactions state
	let expenseCategories = $derived(
		expense?.categories.map((cat) => ({
			categoryId: cat.category.id,
			weight: cat.weight.toString()
		})) || [{ categoryId: '', weight: '1' }]
	);
	let transactions: {
		date: Date | undefined;
		accountId: string;
		amount: string;
		statement: string;
		isRelative: boolean;
		percentage: string;
	}[] = $derived(
		expense?.transactions.map((tx) => ({
			date: tx.date ? new Date(tx.date) : undefined,
			accountId: tx.account.id.toString(),
			amount: tx.amount ? (tx.amount / 100).toFixed(2) : '',
			statement: tx.statement || '',
			isRelative: tx.fraction !== null,
			percentage: tx.fraction ? (100 * tx.fraction).toString() : ''
		})) || [
			{
				date: undefined,
				accountId: '',
				amount: '',
				statement: '',
				isRelative: false,
				percentage: ''
			}
		]
	);

	let positions: {
		title: string;
		totalAmount: string;
		quantity: string;
		unit: string;
		comments: string;
	}[] = $derived(
		expense?.positions.map((pos) => ({
			title: pos.title,
			totalAmount: pos.amount ? (pos.amount / 100).toString() : '',
			quantity: pos.quantity?.toString() || '',
			unit: pos.unit || '',
			comments: pos.comments || ''
		})) || []
	);

	function addCategory() {
		expenseCategories = [...expenseCategories, { categoryId: '', weight: '1' }];
	}
	function removeCategory(i: number) {
		if (expenseCategories.length > 1) {
			expenseCategories = expenseCategories.slice(0, i).concat(expenseCategories.slice(i + 1));
		}
	}
	function addTransaction() {
		transactions = [
			...transactions,
			{
				date: undefined,
				accountId: '',
				amount: '',
				statement: '',
				isRelative: false,
				percentage: ''
			}
		];
	}
	function removeTransaction(i: number) {
		if (transactions.length > 1) {
			transactions = transactions.slice(0, i).concat(transactions.slice(i + 1));
		}
	}
	function addPosition() {
		positions = [
			...positions,
			{
				title: '',
				totalAmount: '',
				quantity: '',
				unit: '',
				comments: ''
			}
		];
	}
	function removePosition(i: number) {
		if (positions.length > 0) {
			positions = positions.slice(0, i).concat(positions.slice(i + 1));
		}
	}

	const titleError = $derived(!title);
	const storeError = $derived(false);
	const descriptionError = $derived(false);
	const referenceAmountError = $derived.by(() => {
		const hasRelativeTransactions = transactions.some((tx) => tx.isRelative);
		if (!hasRelativeTransactions) return false;

		// Must be provided when relative transactions exist
		return !referenceAmount || !Number(referenceAmount);
	});
	const bookingStartError = $derived(!bookingStart);
	const bookingEndError = $derived.by(() => {
		if (bookingStart && bookingEnd && bookingEnd < bookingStart) return true;
		return false;
	});

	// Validation for categories and transactions
	const categoryErrors = $derived.by(() => {
		return expenseCategories.map((cat) => {
			const errors: { categoryId?: boolean; weight?: boolean } = {};
			if (!cat.categoryId) errors.categoryId = true;
			if (cat.weight && (isNaN(Number(cat.weight)) || Number(cat.weight) < 1)) errors.weight = true;
			return errors;
		});
	});
	const transactionErrors = $derived.by(() => {
		return transactions.map((tx) => {
			const errors: {
				accountId?: boolean;
				amount?: boolean;
				date?: boolean;
				percentage?: boolean;
			} = {};
			if (!tx.accountId) errors.accountId = true;
			if (tx.isRelative) {
				if (tx.percentage === '' || isNaN(Number(tx.percentage))) errors.percentage = true;
			} else {
				if (tx.amount === '' || isNaN(Number(tx.amount))) errors.amount = true;
			}
			return errors;
		});
	});
	const hasCategoryErrors = $derived(categoryErrors.some((e) => e.categoryId || e.weight));
	const hasTransactionErrors = $derived(
		transactionErrors.some((e) => e.accountId || e.amount || e.percentage)
	);
	const positionErrors = $derived.by(() => {
		return positions.map((pos) => {
			const errors: {
				title?: boolean;
				totalAmount?: boolean;
				quantity?: boolean;
			} = {};
			if (!pos.title) errors.title = true;
			if (pos.totalAmount && isNaN(Number(pos.totalAmount))) errors.totalAmount = true;
			if (pos.quantity && isNaN(Number(pos.quantity))) errors.quantity = true;
			return errors;
		});
	});
	const hasPositionErrors = $derived(
		positionErrors.some((e) => e.title || e.totalAmount || e.quantity)
	);
	const formHasErrors = $derived(
		!!(
			titleError ||
			storeError ||
			descriptionError ||
			referenceAmountError ||
			bookingStartError ||
			bookingEndError ||
			hasCategoryErrors ||
			hasTransactionErrors ||
			hasPositionErrors
		)
	);

	const UpdateExpense = graphql(`
		mutation UpdateExpense($id: ObjectIdWrapper!, $input: ExpenseInput!) {
			expenses {
				update(id: $id, expense: $input) {
					id
				}
			}
		}
	`);
	const CreateExpense = graphql(`
		mutation CreateExpense($input: ExpenseInput!) {
			expenses {
				create(expense: $input) {
					id
				}
			}
		}
	`);

	const handleSubmit = async (event: Event) => {
		event.preventDefault();
		if (formHasErrors) return;

		const endDate = bookingEnd || bookingStart;

		const input: ExpenseInput = {
			title,
			store,
			description,
			comments,
			isTemplate,
			isUnchecked,
			isExtraordinary,
			isTaxRelevant,
			isIncomplete,
			referenceAmount: transactions.some((tx) => tx.isRelative)
				? Number(referenceAmount) * 100
				: undefined,
			bookingStart: bookingStart!,
			bookingEnd: endDate!,
			receipts: [],
			categories: expenseCategories.map((cat) => ({
				categoryId: cat.categoryId,
				weight: cat.weight ? Number(cat.weight) : 1
			})),
			transactions: transactions.map((tx) => ({
				date: tx.date || bookingStart!,
				accountId: tx.accountId,
				amount: !tx.isRelative ? Number(tx.amount) * 100 : undefined,
				fraction: tx.isRelative ? Number(tx.percentage) / 100 : undefined,
				statement: tx.statement,
				comments: ''
			})),
			positions: positions.map((pos) => ({
				title: pos.title,
				amount: pos.totalAmount ? Number(pos.totalAmount) * 100 : undefined,
				quantity: pos.quantity ? Number(pos.quantity) : undefined,
				unit: pos.unit || undefined,
				comments: pos.comments
			}))
		};

		console.log('expense', expense);
		console.log('update', input);

		if (expense) await UpdateExpense.mutate({ id: expense.id, input });
		else await CreateExpense.mutate({ input });

		if (onsubmit) await onsubmit();
		open = false;
	};
</script>

<Modal title={expense ? 'Ausgabe bearbeiten' : 'Neue Ausgabe'} bind:open size="lg">
	<form onsubmit={handleSubmit}>
		<div class="mb-4 grid gap-4 sm:grid-cols-2">
			<div>
				<Label for="title" class="mb-2">Titel</Label>
				<Input
					type="text"
					clearable
					id="title"
					placeholder="Titel"
					bind:value={title}
					required
					color={titleError ? 'red' : undefined}
				/>
			</div>
			<div>
				<Label for="store" class="mb-2">Geschäft</Label>
				<Input
					type="text"
					clearable
					id="store"
					placeholder="Geschäft"
					bind:value={store}
					color={storeError ? 'red' : undefined}
				/>
			</div>

			<div class="sm:col-span-2">
				<Label for="description" class="mb-2">Beschreibung</Label>
				<Textarea
					id="description"
					clearable
					placeholder="Beschreibung"
					rows={description ? 5 : 1}
					class="mb-2"
					bind:value={description}
					color={descriptionError ? 'red' : undefined}
				/>
			</div>
			<div class="sm:col-span-2">
				<Label for="comments" class="mb-2">Kommentare</Label>
				<Textarea
					id="comments"
					clearable
					placeholder="Kommentare"
					rows={comments ? 5 : 1}
					bind:value={comments}
				/>
			</div>
			<div>
				<Label class="mb-2 block">Buchungsbeginn</Label>
				<Datepicker
					bind:value={bookingStart}
					placeholder="Startdatum"
					inputClass={bookingStartError ? 'border-red-600 bg-red-100 text-red-400' : ''}
				/>
			</div>
			<div>
				<Label class="mb-2 block">Buchungsende</Label>
				<Datepicker
					bind:value={bookingEnd}
					placeholder="optionales Enddatum"
					inputClass={bookingEndError ? 'border-red-600 bg-red-100 text-red-400' : ''}
				/>
			</div>
			<div>
				<Label class="mb-2 block">Flags</Label>
				<Checkbox bind:checked={isTemplate}>Vorlage</Checkbox>
				<Checkbox bind:checked={isUnchecked}>Unbestätigt</Checkbox>
			</div>
			<div>
				<Checkbox bind:checked={isIncomplete}>Unvollständig</Checkbox>
				<Checkbox bind:checked={isExtraordinary}>Außergewöhnlich</Checkbox>
				<Checkbox bind:checked={isTaxRelevant}>Steuerrelevant</Checkbox>
			</div>
			<!-- TODO: support receipts -->

			{#if transactions.filter((tx) => tx.isRelative).length > 0}
				<div>
					<Label for="referenceAmount" class="mb-2">Referenzbetrag</Label>
					<div class="relative">
						<Input
							id="referenceAmount"
							placeholder="0"
							bind:value={referenceAmount}
							step="0.01"
							class="no-spinner pr-6"
							color={referenceAmountError ? 'red' : undefined}
						/>
						<span
							class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
							>€</span
						>
					</div>
				</div>
			{/if}

			<div class="mb-4 sm:col-span-2">
				<Label class="mb-2 block">Transaktionen</Label>
				<div class="overflow-x-auto">
					<table class="w-full border-collapse border border-gray-300">
						<thead>
							<tr class="bg-gray-50">
								<th
									class="w-1/6 border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Datum</th
								>
								<th
									class="w-1/6 border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Konto</th
								>
								<th
									class="w-1/12 border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Relativ</th
								>
								<th
									class="w-1/6 border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Betrag</th
								>
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Transaktionstext</th
								>
								<th
									class="w-1/12 border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>&nbsp;</th
								>
							</tr>
						</thead>
						<tbody>
							{#each transactions as tx, i (i)}
								<tr>
									<td class="border border-gray-300 px-3 py-2">
										<Datepicker
											bind:value={tx.date}
											placeholder="Datum"
											inputClass={transactionErrors[i]?.date
												? 'border-red-600 bg-red-100 text-red-400'
												: ''}
										/>
									</td>
									<td class="border border-gray-300 px-3 py-2">
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
											bind:value={tx.accountId}
											style={tx.accountId
												? `background-color: ${accounts.find((a) => a.id === tx.accountId)?.color}`
												: undefined}
											selectClass={transactionErrors[i]?.accountId
												? 'text-red-700 border-red-700 bg-red-100'
												: undefined}
											placeholder="Konto"
										/>
									</td>
									<td class="border border-gray-300 px-3 py-2">
										<Checkbox bind:checked={tx.isRelative} class="ml-1"></Checkbox>
									</td>
									<td class="border border-gray-300 px-3 py-2">
										<div class="flex items-center gap-2">
											{#if !tx.isRelative}
												<div class="relative flex-1">
													<Input
														placeholder=""
														bind:value={tx.amount}
														class="no-spinner pr-6"
														color={transactionErrors[i]?.amount ? 'red' : undefined}
													/>
													<span
														class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
														>€</span
													>
												</div>
											{:else}
												<div class="relative flex-1">
													<Input
														placeholder=""
														bind:value={tx.percentage}
														class="no-spinner pr-6"
														color={transactionErrors[i]?.percentage ? 'red' : undefined}
														min="0"
														max="100"
														step="0.01"
													/>
													<span
														class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
														>%</span
													>
												</div>
											{/if}
										</div>
										{#if tx.isRelative && referenceAmount && tx.percentage && Number(referenceAmount)}
											<div class="mt-1 text-xs text-gray-600">
												= {((Number(tx.percentage) / 100) * Number(referenceAmount)).toFixed(2)} €
											</div>
										{/if}
									</td>
									<td class="border border-gray-300 px-3 py-2">
										<Textarea
											placeholder=""
											clearable
											bind:value={tx.statement}
											class="w-full font-mono"
											rows={tx.statement ? 5 : 1}
										/>
									</td>
									<td class="border border-gray-300 px-3 py-2">
										<Button
											onclick={(e: MouseEvent) => {
												e.stopPropagation();
												removeTransaction(i);
											}}
											color={transactions.length === 1 ? 'gray' : 'red'}
											disabled={transactions.length === 1}
											size="xs"
										>
											<CircleMinusOutline class="h-4 w-4" />
										</Button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				<div class="mt-2 text-center">
					<Button
						onclick={(e: MouseEvent) => {
							e.stopPropagation();
							addTransaction();
						}}
						size="xs"
					>
						<CirclePlusOutline class="mr-1 h-4 w-4" />
						Transaktion hinzufügen
					</Button>
				</div>
			</div>

			<div class="mb-4 sm:col-span-2">
				<Label class="mb-2 block">Kategorien</Label>
				<div class="overflow-x-auto">
					<table class="w-full border-collapse border border-gray-300">
						<thead>
							<tr class="bg-gray-50">
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Kategorie</th
								>
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Gewicht</th
								>
								<th
									class="w-1/12 border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>&nbsp;</th
								>
							</tr>
						</thead>
						<tbody>
							{#each expenseCategories as cat, i (i)}
								<tr>
									<td class="border border-gray-300 px-3 py-2">
										<Select
											items={categories
												.map((c) => ({ value: c.id, name: c.name }))
												.filter(
													(c) =>
														!expenseCategories.some(
															(ec, idx) => idx !== i && ec.categoryId === c.value
														)
												)}
											bind:value={cat.categoryId}
											placeholder="Kategorie"
											style={cat.categoryId
												? `background-color: ${categories.find((c) => c.id === cat.categoryId)?.color}`
												: undefined}
											selectClass={categoryErrors[i]?.categoryId
												? 'text-red-700 border-red-700 bg-red-100'
												: undefined}
										/>
									</td>
									<td class="border border-gray-300 px-3 py-2">
										<Input
											type="number"
											min="1"
											class="no-spinner w-full"
											bind:value={cat.weight}
											placeholder="Gewicht"
											color={categoryErrors[i]?.weight ? 'red' : undefined}
										/>
									</td>
									<td class="border border-gray-300 px-3 py-2">
										<Button
											onclick={(e: MouseEvent) => {
												e.stopPropagation();
												removeCategory(i);
											}}
											color={expenseCategories.length === 1 ? 'gray' : 'red'}
											disabled={expenseCategories.length === 1}
											size="xs"
										>
											<CircleMinusOutline class="h-4 w-4" />
										</Button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				<div class="mt-2 text-center">
					<Button
						onclick={(e: MouseEvent) => {
							e.stopPropagation();
							addCategory();
						}}
						size="xs"
					>
						<CirclePlusOutline class="mr-1 h-4 w-4" />
						Kategorie hinzufügen
					</Button>
				</div>
			</div>

			<div class="mb-4 sm:col-span-2">
				<Label class="mb-2 block">Positionen</Label>
				{#if positions.length > 0}
					<div class="overflow-x-auto">
						<table class="w-full border-collapse border border-gray-300">
							<thead>
								<tr class="bg-gray-50">
									<th
										class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
										>Titel</th
									>
									<th
										class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
										>Gesamtbetrag (€)</th
									>
									<th
										class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
										>Menge</th
									>
									<th
										class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
										>Einheit</th
									>
									<th
										class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
										>Kommentare</th
									>
									<th
										class="w-1/12 border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
										>&nbsp;</th
									>
								</tr>
							</thead>
							<tbody>
								{#each positions as pos, i (i)}
									<tr>
										<td class="border border-gray-300 px-3 py-2">
											<Input
												type="text"
												placeholder="Titel"
												bind:value={pos.title}
												class="w-full"
												color={positionErrors[i]?.title ? 'red' : undefined}
											/>
										</td>
										<td class="border border-gray-300 px-3 py-2">
											<div class="relative">
												<Input
													placeholder=""
													bind:value={pos.totalAmount}
													class="no-spinner w-full pr-6"
													step="0.01"
													color={positionErrors[i]?.totalAmount ? 'red' : undefined}
												/>
												<span
													class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
													>€</span
												>
											</div>
										</td>
										<td class="border border-gray-300 px-3 py-2">
											<Input
												placeholder=""
												bind:value={pos.quantity}
												class="no-spinner w-full"
												step="0.01"
												color={positionErrors[i]?.quantity ? 'red' : undefined}
											/>
										</td>
										<td class="border border-gray-300 px-3 py-2">
											<Input type="text" placeholder="" bind:value={pos.unit} class="w-full" />
										</td>
										<td class="border border-gray-300 px-3 py-2">
											<Textarea
												placeholder="Kommentare"
												bind:value={pos.comments}
												class="w-full"
												rows={pos.comments ? 3 : 1}
											/>
										</td>
										<td class="border border-gray-300 px-3 py-2">
											<Button
												onclick={(e: MouseEvent) => {
													e.stopPropagation();
													removePosition(i);
												}}
												color={positions.length === 1 ? 'gray' : 'red'}
												size="xs"
											>
												<CircleMinusOutline class="h-4 w-4" />
											</Button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
				<div class="mt-2 text-center">
					<Button
						onclick={(e: MouseEvent) => {
							e.stopPropagation();
							addPosition();
						}}
						size="xs"
					>
						<CirclePlusOutline class="mr-1 h-4 w-4" />
						Position hinzufügen
					</Button>
				</div>
			</div>

			<div class="justify-center sm:col-span-2">
				<Button type="submit" class="w-full" disabled={formHasErrors}>
					{#if expense}
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
