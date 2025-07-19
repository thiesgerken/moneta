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

	import { graphql, type ExpenseInput } from '$houdini';
	import { formatAccount } from './expense_utils';
	import { CircleMinusOutline, CirclePlusOutline } from 'flowbite-svelte-icons';

	interface Props {
		open?: boolean;
		user_name: string | undefined;
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

	// Categories and transactions state
	let expenseCategories = $state([{ categoryId: '', weight: '1' }]);
	let transactions: {
		date: Date | undefined;
		accountId: string;
		amount: string;
		statement: string;
		isRelative: boolean;
		percentage: string;
	}[] = $state([
		{
			date: undefined,
			accountId: '',
			amount: '',
			statement: '',
			isRelative: false,
			percentage: ''
		}
	]);

	// Positions state
	let positions: {
		title: string;
		totalAmount: string;
		quantity: string;
		unit: string;
		comments: string;
	}[] = $state([]);

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
		return !referenceAmount;
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

	const handleSubmit = (event: Event) => {
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
				? Number(referenceAmount)
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
				amount: !tx.isRelative ? Number(tx.amount) : undefined,
				percentage: tx.isRelative ? Number(tx.percentage) : undefined,
				statement: tx.statement,
				comments: ''
			})),
			positions: positions.map((pos) => ({
				title: pos.title,
				amount: pos.totalAmount ? Number(pos.totalAmount) : undefined,
				quantity: pos.quantity ? Number(pos.quantity) : undefined,
				unit: pos.unit || undefined,
				comments: pos.comments
			}))
		};

		console.log(input);
		CreateExpense.mutate({ input });
	};

	// TODO: switch between create and update, also need to be able to consume an existing expense and display it initially

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

<Modal title="Neue Ausgabe" bind:open>
	<form onsubmit={handleSubmit}>
		<div class="mb-4 grid gap-4 sm:grid-cols-2">
			<div>
				<Label for="title" class="mb-2">Titel</Label>
				<Input
					type="text"
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
							type="number"
							clearable
							id="referenceAmount"
							placeholder="0"
							bind:value={referenceAmount}
							min="0"
							step="0.01"
							class="no-spinner pr-6"
							color={referenceAmountError ? 'red' : undefined}
						/>
						<span
							class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-gray-500"
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
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Datum</th
								>
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Konto</th
								>
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Relativ</th
								>
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Betrag</th
								>
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Transaktionstext</th
								>
								<th
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Aktion</th
								>
							</tr>
						</thead>
						<tbody>
							{#each transactions as tx, i}
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
														type="number"
														placeholder=""
														bind:value={tx.amount}
														class="no-spinner pr-6"
														color={transactionErrors[i]?.amount ? 'red' : undefined}
													/>
													<span
														class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-gray-500"
														>€</span
													>
												</div>
											{:else}
												<div class="relative flex-1">
													<Input
														type="number"
														placeholder=""
														bind:value={tx.percentage}
														class="no-spinner pr-6"
														color={transactionErrors[i]?.percentage ? 'red' : undefined}
														min="0"
														max="100"
														step="0.01"
													/>
													<span
														class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-gray-500"
														>%</span
													>
												</div>
											{/if}
										</div>
										{#if tx.isRelative && referenceAmount && tx.percentage}
											<div class="mt-1 text-xs text-gray-600">
												= {((Number(tx.percentage) / 100) * referenceAmount).toFixed(2)} €
											</div>
										{/if}
									</td>
									<td class="border border-gray-300 px-3 py-2">
										<Textarea
											placeholder=""
											clearable
											bind:value={tx.statement}
											class="w-full font-mono"
											rows={tx.statement ? 3 : 1}
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
									class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
									>Aktion</th
								>
							</tr>
						</thead>
						<tbody>
							{#each expenseCategories as cat, i}
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
										class="border border-gray-300 px-3 py-2 text-left text-sm font-medium text-gray-700"
										>Aktion</th
									>
								</tr>
							</thead>
							<tbody>
								{#each positions as pos, i}
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
													type="number"
													placeholder=""
													bind:value={pos.totalAmount}
													class="no-spinner w-full pr-6"
													min="0"
													step="0.01"
													color={positionErrors[i]?.totalAmount ? 'red' : undefined}
												/>
												<span
													class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-gray-500"
													>€</span
												>
											</div>
										</td>
										<td class="border border-gray-300 px-3 py-2">
											<Input
												type="number"
												placeholder=""
												bind:value={pos.quantity}
												class="no-spinner w-full"
												min="0"
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
