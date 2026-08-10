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
	import { formatAccount, tint } from './expense_utils';
	import {
		CircleMinusOutline,
		CirclePlusOutline,
		EditOutline,
		PlusOutline
	} from 'flowbite-svelte-icons';

	interface Props {
		open?: boolean;
		onsubmit?: () => Promise<void>; // used to signal that the form has been successfully submitted, need to invalidate cached stuff
		/// fill the form from `expense` but create a second expense from it rather than updating it
		duplicate?: boolean;
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
			readonly accrualStart: Date;
			readonly accrualEnd: Date;
			readonly comments: string;
			readonly isTaxRelevant: boolean;
			readonly isTemplate: boolean;
			readonly isUnchecked: boolean;
			readonly isIncomplete: boolean;
			readonly isExtraordinary: boolean;
			readonly receipts: {
				readonly uri: string;
				readonly ocr: string | null;
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
				readonly bookingDate: Date | null;
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
		duplicate = false,
		onsubmit
	}: Props = $props();

	/// Duplicating fills the form from an expense and then creates a second one, so `expense`
	/// being set is no longer the same question as "this is an edit".
	const isEdit = $derived(!!expense && !duplicate);

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
	let accrualStart: Date | undefined = $derived(
		expense?.accrualStart ? new Date(expense.accrualStart) : undefined
	);
	let accrualEnd: Date | undefined = $derived(
		expense?.accrualEnd ? new Date(expense.accrualEnd) : undefined
	);

	// Nearly every expense happens on one day, and one entered that way comes back with both
	// ends filled in and equal -- so changing its date meant changing it twice, in two fields
	// that looked like they could disagree. One field until an expense gives a reason for two.
	let isRange = $state(false);

	// The four editable collections are $state rather than $derived, and are filled from the
	// expense by an effect. A $derived array is not deeply reactive: its rows are plain objects,
	// so `tx.isRelative = true` changed nothing on screen and the € / % switch below would have
	// been dead. An assignment into $state is proxied and reaches the template.
	interface TransactionRow {
		date: Date | undefined;
		bookingDate: Date | undefined;
		accountId: string;
		amount: string;
		statement: string;
		isRelative: boolean;
		percentage: string;
	}
	interface PositionRow {
		title: string;
		totalAmount: string;
		quantity: string;
		unit: string;
		comments: string;
	}

	const emptyTransaction = (): TransactionRow => ({
		date: undefined,
		bookingDate: undefined,
		accountId: '',
		amount: '',
		statement: '',
		isRelative: false,
		percentage: ''
	});

	let expenseCategories: { categoryId: string; weight: string }[] = $state([]);
	let transactions: TransactionRow[] = $state([]);
	let positions: PositionRow[] = $state([]);
	// ocr is machine-produced and not editable here, but it has to round-trip: the mutation
	// replaces the whole receipt list
	let receipts: { uri: string; ocr: string | null }[] = $state([]);

	$effect(() => {
		isRange = !!expense && expense.accrualStart.getTime() !== expense.accrualEnd.getTime();
		expenseCategories = expense?.categories.map((cat) => ({
			categoryId: cat.category.id,
			weight: cat.weight.toString()
		})) || [{ categoryId: '', weight: '1' }];
		transactions = expense?.transactions.map((tx) => ({
			date: tx.date ? new Date(tx.date) : undefined,
			// A bank line belongs to the one payment it describes. Carried into a copy it would
			// claim the same one twice, which is wrong in the list, in the changelog and in
			// anything that sorts by what a rule matched.
			bookingDate: duplicate || !tx.bookingDate ? undefined : new Date(tx.bookingDate),
			accountId: tx.account.id.toString(),
			amount: tx.amount ? (tx.amount / 100).toFixed(2) : '',
			statement: duplicate ? '' : tx.statement || '',
			isRelative: tx.fraction !== null,
			percentage: tx.fraction ? (100 * tx.fraction).toString() : ''
		})) || [emptyTransaction()];
		positions =
			expense?.positions.map((pos) => ({
				title: pos.title,
				totalAmount: pos.amount ? (pos.amount / 100).toString() : '',
				quantity: pos.quantity?.toString() || '',
				unit: pos.unit || '',
				comments: pos.comments || ''
			})) || [];
		receipts = expense?.receipts.map((r) => ({ uri: r.uri, ocr: r.ocr })) || [];
	});

	// The two rarely-used lists live in modals of their own, so that an expense without them
	// costs one line in the form rather than a section header and a button.
	let showPositions = $state(false);
	let showReceipts = $state(false);

	const positionsTotal = $derived(
		positions.reduce((sum, pos) => sum + (Number(pos.totalAmount) || 0), 0)
	);

	function addReceipt() {
		receipts = [...receipts, { uri: '', ocr: null }];
	}
	function removeReceipt(i: number) {
		receipts = receipts.slice(0, i).concat(receipts.slice(i + 1));
	}

	function addCategory() {
		expenseCategories = [...expenseCategories, { categoryId: '', weight: '1' }];
	}
	function removeCategory(i: number) {
		if (expenseCategories.length > 1) {
			expenseCategories = expenseCategories.slice(0, i).concat(expenseCategories.slice(i + 1));
		}
	}
	function addTransaction() {
		transactions = [...transactions, emptyTransaction()];
	}
	function removeTransaction(i: number) {
		if (transactions.length > 1) {
			transactions = transactions.slice(0, i).concat(transactions.slice(i + 1));
		}
	}

	/// Switching a transaction between a fixed amount and a share of the reference amount keeps
	/// what it is worth. Without this the number stayed where it was and silently meant something
	/// else: -16.49 € became -16.49 % of the reference amount, which is nobody's intention.
	function toggleRelative(i: number) {
		const tx = transactions[i];

		if (!tx.isRelative) {
			// A split is entered by taking the one transaction that is already there and adding a
			// share beside it, so the amount on that one is what the shares are shares of. Only
			// when it is unambiguous: one other transaction, with an amount, and nothing to
			// overwrite.
			if (!Number(referenceAmount)) {
				const fixed = transactions.filter((t, j) => j !== i && !t.isRelative && Number(t.amount));
				if (fixed.length === 1) referenceAmount = fixed[0].amount;
			}

			const reference = Number(referenceAmount);
			if (reference && Number(tx.amount)) {
				tx.percentage = String(Math.round((Number(tx.amount) / reference) * 10000) / 100);
			}
		} else {
			const reference = Number(referenceAmount);
			if (reference && tx.percentage !== '' && !isNaN(Number(tx.percentage))) {
				tx.amount = ((Number(tx.percentage) / 100) * reference).toFixed(2);
			}
		}

		tx.isRelative = !tx.isRelative;
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
	const accrualStartError = $derived(!accrualStart);
	const accrualEndError = $derived.by(() => {
		if (isRange && accrualStart && accrualEnd && accrualEnd < accrualStart) return true;
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
			accrualStartError ||
			accrualEndError ||
			hasCategoryErrors ||
			hasTransactionErrors ||
			hasPositionErrors
		)
	);

	const UpdateExpense = graphql(`
		mutation UpdateExpense($id: UUID!, $input: ExpenseInput!) {
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

		const endDate = isRange ? accrualEnd || accrualStart : accrualStart;

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
			accrualStart: accrualStart!,
			accrualEnd: endDate!,
			receipts: receipts
				.filter((r) => r.uri.trim())
				.map((r) => ({ uri: r.uri.trim(), ocr: r.ocr })),
			categories: expenseCategories.map((cat) => ({
				categoryId: cat.categoryId,
				weight: cat.weight ? Number(cat.weight) : 1
			})),
			transactions: transactions.map((tx) => ({
				date: tx.date || accrualStart!,
				bookingDate: tx.bookingDate,
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

		if (isEdit) await UpdateExpense.mutate({ id: expense!.id, input });
		else await CreateExpense.mutate({ input });

		if (onsubmit) await onsubmit();
		open = false;
	};
</script>

{#snippet minus(onclick: () => void, disabled = false)}
	<!-- `light` rather than `alternative`: an alternative button is transparent with a hairline
	     border, which is what a disabled button looks like, so the one row you cannot remove and
	     the four you can were the same button twice. -->
	<Button
		color="light"
		size="xs"
		class="p-1.5"
		{disabled}
		onclick={(e: MouseEvent) => {
			e.stopPropagation();
			onclick();
		}}
	>
		<CircleMinusOutline class="h-4 w-4" />
	</Button>
{/snippet}

{#snippet hint(text: string)}
	<span class="block text-xs font-normal text-gray-500 dark:text-gray-400">{text}</span>
{/snippet}

{#snippet textButton(label: string, onclick: () => void)}
	<button
		type="button"
		{onclick}
		class="text-primary-600 dark:text-primary-400 cursor-pointer text-xs hover:underline"
	>
		{label}
	</button>
{/snippet}

{#snippet add(label: string, onclick: () => void)}
	<Button
		color="alternative"
		size="xs"
		class="mt-2"
		onclick={(e: MouseEvent) => {
			e.stopPropagation();
			onclick();
		}}
	>
		<CirclePlusOutline class="mr-1 h-4 w-4" />{label}
	</Button>
{/snippet}

<!-- xl rather than lg: the form is two tables and a dozen fields, and at 896px they were stacked
     down a column with the right half of the dialog empty. -->
<Modal
	title={isEdit ? 'Ausgabe bearbeiten' : duplicate ? 'Ausgabe duplizieren' : 'Neue Ausgabe'}
	bind:open
	size="xl"
>
	<!-- *:min-w-0: a grid item is min-content wide by default, and the transaction and category
	     tables inside this one are wider than a phone. Without it the whole form grew to their
	     width and every field ran off the right of the screen; with it they scroll inside their
	     own `overflow-x-auto` the way they were meant to. -->
	<form
		id="expense-form"
		onsubmit={handleSubmit}
		class="grid min-w-0 gap-4 *:min-w-0 sm:grid-cols-2 lg:grid-cols-6"
	>
		<div class="sm:col-span-2 lg:col-span-3">
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
		<div class="sm:col-span-2 lg:col-span-1">
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
		<div>
			<div class="mb-2 flex items-baseline justify-between gap-2">
				<Label class="block">{isRange ? 'Leistung von' : 'Leistung'}</Label>
				{#if !isRange}
					{@render textButton('Zeitraum', () => (isRange = true))}
				{/if}
			</div>
			<Datepicker
				bind:value={accrualStart}
				placeholder="Datum"
				inputClass={accrualStartError ? 'border-red-600 bg-red-100 text-red-400' : ''}
			/>
		</div>
		{#if isRange}
			<div>
				<div class="mb-2 flex items-baseline justify-between gap-2">
					<Label class="block">Leistung bis</Label>
					{@render textButton('Einzeltag', () => {
						isRange = false;
						accrualEnd = undefined;
					})}
				</div>
				<Datepicker
					bind:value={accrualEnd}
					placeholder="Enddatum"
					inputClass={accrualEndError ? 'border-red-600 bg-red-100 text-red-400' : ''}
				/>
			</div>
		{/if}

		<!-- field-sizing-content instead of guessing a row count: a textarea's default width is
		     `cols=20`, which is where the 20-character Beschreibung box came from, and its default
		     height is as wrong. It now starts one line high and grows with what is in it. -->
		<div class="sm:col-span-2 lg:col-span-4">
			<Label for="description" class="mb-2">Beschreibung</Label>
			<Textarea
				id="description"
				placeholder="Beschreibung"
				rows={1}
				class="field-sizing-content max-h-48 w-full"
				bind:value={description}
				color={descriptionError ? 'red' : undefined}
			/>
		</div>
		<div class="sm:col-span-2 lg:col-span-2">
			<Label for="comments" class="mb-2">Kommentare</Label>
			<Textarea
				id="comments"
				placeholder="Kommentare"
				rows={1}
				class="field-sizing-content max-h-48 w-full"
				bind:value={comments}
			/>
		</div>

		<!-- One row. Split over two grid cells, the three on the right sat under the empty half of
		     the "Flags" label and read as belonging to nothing. -->
		<div class="col-span-full">
			<Label class="mb-2 block">Flags</Label>
			<div class="flex flex-wrap gap-x-6 gap-y-2">
				<Checkbox bind:checked={isTemplate}>Vorlage</Checkbox>
				<Checkbox bind:checked={isUnchecked}>Unbestätigt</Checkbox>
				<Checkbox bind:checked={isIncomplete}>Unvollständig</Checkbox>
				<Checkbox bind:checked={isExtraordinary}>Außergewöhnlich</Checkbox>
				<Checkbox bind:checked={isTaxRelevant}>Steuerrelevant</Checkbox>
			</div>
		</div>

		<div class="col-span-full lg:col-span-4">
			<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
				<Label class="block">Transaktionen</Label>
				<!-- Only a relative transaction has anything to be relative to, which is also the
				     only reason this field exists -- so it lives with them rather than in the form
				     above, where it appeared and disappeared and moved every field after it. -->
				{#if transactions.some((tx) => tx.isRelative)}
					<div class="flex items-center gap-2">
						<Label for="referenceAmount" class="whitespace-nowrap">Referenzbetrag</Label>
						<div class="relative">
							<Input
								id="referenceAmount"
								placeholder="0"
								bind:value={referenceAmount}
								step="0.01"
								class="no-spinner w-28 pr-6"
								color={referenceAmountError ? 'red' : undefined}
							/>
							<span
								class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
								>€</span
							>
						</div>
					</div>
				{/if}
			</div>
			<div class="overflow-x-auto rounded-lg border border-gray-200 dark:border-gray-700">
				<table data-form class="w-full text-left text-sm">
					<thead class="bg-gray-50 dark:bg-gray-700">
						<tr>
							<!-- Two dates that are a day or two apart and neither of them labelled is a
							     column nobody can fill in with confidence, so each says what it is. -->
							<th class="w-32 px-2" scope="col">
								Wertstellung
								{@render hint('als das Geld floss')}
							</th>
							<th class="w-32 px-2" scope="col">
								Buchung
								{@render hint('wie die Bank sie führt')}
							</th>
							<th class="w-36 px-2" scope="col">Konto</th>
							<th class="w-28 px-2" scope="col">Betrag</th>
							<th class="px-2" scope="col">Transaktionstext</th>
							<th class="w-10 px-2" scope="col"><span class="sr-only">Entfernen</span></th>
						</tr>
					</thead>
					<tbody class="divide-y">
						{#each transactions as tx, i (i)}
							<tr>
								<td class="px-2">
									<Datepicker
										bind:value={tx.date}
										placeholder="wie Leistung"
										inputClass={transactionErrors[i]?.date
											? 'border-red-600 bg-red-100 text-red-400'
											: ''}
									/>
								</td>
								<td class="px-2">
									<!-- Only a line that came from a bank has one, and there is no way to
									     clear a Datepicker once it holds a date -- hence the link. -->
									{#if tx.bookingDate}
										<Datepicker bind:value={tx.bookingDate} />
										{@render textButton('entfernen', () => (tx.bookingDate = undefined))}
									{:else}
										{@render textButton('setzen', () => (tx.bookingDate = tx.date ?? accrualStart))}
									{/if}
								</td>
								<td class="px-2">
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
										style={tint(accounts.find((a) => a.id === tx.accountId)?.color)}
										selectClass={transactionErrors[i]?.accountId
											? 'text-red-700 border-red-700 bg-red-100'
											: undefined}
										placeholder="Konto"
									/>
								</td>
								<td class="px-2">
									<!-- The unit is the switch. A column of its own for a checkbox called
									     "Relativ" cost more width than the amount it qualified, and said
									     nothing about what the number in the next cell then meant. -->
									<div class="relative">
										{#if tx.isRelative}
											<Input
												placeholder=""
												bind:value={tx.percentage}
												class="no-spinner pr-8"
												color={transactionErrors[i]?.percentage ? 'red' : undefined}
												min="0"
												max="100"
												step="0.01"
											/>
										{:else}
											<Input
												placeholder=""
												bind:value={tx.amount}
												class="no-spinner pr-8"
												color={transactionErrors[i]?.amount ? 'red' : undefined}
											/>
										{/if}
										<button
											type="button"
											onclick={() => toggleRelative(i)}
											title={tx.isRelative
												? 'Anteil am Referenzbetrag — klicken für festen Betrag'
												: 'Fester Betrag — klicken für Anteil am Referenzbetrag'}
											class="absolute top-1/2 right-1 -translate-y-1/2 cursor-pointer rounded px-1.5 py-0.5 text-gray-500 hover:bg-gray-200 dark:text-gray-400 dark:hover:bg-gray-600"
										>
											{tx.isRelative ? '%' : '€'}
										</button>
									</div>
									{#if tx.isRelative && referenceAmount && tx.percentage && Number(referenceAmount)}
										<div class="mt-1 text-xs text-gray-600 dark:text-gray-400">
											= {((Number(tx.percentage) / 100) * Number(referenceAmount)).toFixed(2)} €
										</div>
									{/if}
								</td>
								<td class="px-2">
									<!-- The bank's own text, and the tallest thing in the dialog when it was
									     given five rows by default. It is machine-written and hardly ever
									     edited, so it starts as one line and grows if you go into it. -->
									<Textarea
										placeholder=""
										bind:value={tx.statement}
										class="field-sizing-content max-h-32 w-full font-mono text-xs"
										rows={1}
									/>
								</td>
								<td class="px-2">
									{@render minus(() => removeTransaction(i), transactions.length === 1)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			{@render add('Transaktion', addTransaction)}
		</div>

		<div class="col-span-full lg:col-span-2">
			<Label class="mb-2 block">Kategorien</Label>
			<div class="overflow-x-auto rounded-lg border border-gray-200 dark:border-gray-700">
				<table data-form class="w-full text-left text-sm">
					<thead class="bg-gray-50 dark:bg-gray-700">
						<tr>
							<th class="px-2" scope="col">Kategorie</th>
							<th class="w-24 px-2" scope="col">Gewicht</th>
							<th class="w-10 px-2" scope="col"><span class="sr-only">Entfernen</span></th>
						</tr>
					</thead>
					<tbody class="divide-y">
						{#each expenseCategories as cat, i (i)}
							<tr>
								<td class="px-2">
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
										style={tint(categories.find((c) => c.id === cat.categoryId)?.color)}
										selectClass={categoryErrors[i]?.categoryId
											? 'text-red-700 border-red-700 bg-red-100'
											: undefined}
									/>
								</td>
								<td class="px-2">
									<Input
										type="number"
										min="1"
										class="no-spinner w-full"
										bind:value={cat.weight}
										placeholder="Gewicht"
										color={categoryErrors[i]?.weight ? 'red' : undefined}
									/>
								</td>
								<td class="px-2">
									{@render minus(() => removeCategory(i), expenseCategories.length === 1)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			{@render add('Kategorie', addCategory)}
		</div>

		<!-- Positionen and Belege are empty on almost every expense and cost a heading and a
		     button each when they were. What is left is what they would have said. -->
		<div class="col-span-full flex flex-wrap gap-2">
			<Button color="alternative" size="sm" onclick={() => (showPositions = true)}>
				<EditOutline class="mr-2 h-4 w-4" />
				Positionen · {positions.length === 0
					? 'keine'
					: `${positions.length}${positionsTotal ? ` (${positionsTotal.toFixed(2)} €)` : ''}`}
			</Button>
			<Button color="alternative" size="sm" onclick={() => (showReceipts = true)}>
				<EditOutline class="mr-2 h-4 w-4" />
				Belege · {receipts.length === 0 ? 'keine' : receipts.length}
			</Button>
			{#if hasPositionErrors}
				<span class="self-center text-sm text-red-600 dark:text-red-400">
					Eine Position ist unvollständig
				</span>
			{/if}
		</div>
	</form>

	{#snippet footer()}
		<!-- The submit button sits outside the form it submits, which is what `form=` is for. It
		     was a full-width bar at the bottom of the scroll before, so on a long expense you
		     scrolled past everything to save, and there was no way out but the X. -->
		<div class="flex w-full justify-end gap-2">
			<Button color="alternative" onclick={() => (open = false)}>Abbrechen</Button>
			<Button type="submit" form="expense-form" disabled={formHasErrors}>
				{#if isEdit}
					<EditOutline class="mr-1 h-4 w-4" />
					Aktualisieren
				{:else}
					<PlusOutline class="mr-1 h-4 w-4" />
					Erstellen
				{/if}
			</Button>
		</div>
	{/snippet}
</Modal>

<!-- Outside the modal above rather than inside it: a dialog nested in the form would have its
     buttons submit that form, and `showModal()` stacks these over the parent either way. -->
<Modal title="Positionen" bind:open={showPositions} size="lg">
	{#if positions.length > 0}
		<div class="overflow-x-auto rounded-lg border border-gray-200 dark:border-gray-700">
			<table data-form class="w-full text-left text-sm">
				<thead class="bg-gray-50 dark:bg-gray-700">
					<tr>
						<th class="px-2" scope="col">Titel</th>
						<th class="w-28 px-2" scope="col">Betrag</th>
						<th class="w-24 px-2" scope="col">Menge</th>
						<th class="w-24 px-2" scope="col">Einheit</th>
						<th class="px-2" scope="col">Kommentare</th>
						<th class="w-10 px-2" scope="col"><span class="sr-only">Entfernen</span></th>
					</tr>
				</thead>
				<tbody class="divide-y">
					{#each positions as pos, i (i)}
						<tr>
							<td class="px-2">
								<Input
									type="text"
									placeholder="Titel"
									bind:value={pos.title}
									class="w-full"
									color={positionErrors[i]?.title ? 'red' : undefined}
								/>
							</td>
							<td class="px-2">
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
							<td class="px-2">
								<Input
									placeholder=""
									bind:value={pos.quantity}
									class="no-spinner w-full"
									step="0.01"
									color={positionErrors[i]?.quantity ? 'red' : undefined}
								/>
							</td>
							<td class="px-2">
								<Input type="text" placeholder="" bind:value={pos.unit} class="w-full" />
							</td>
							<td class="px-2">
								<Textarea
									placeholder="Kommentare"
									bind:value={pos.comments}
									class="field-sizing-content max-h-32 w-full"
									rows={1}
								/>
							</td>
							<td class="px-2">
								{@render minus(() => removePosition(i))}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else}
		<p class="text-sm text-gray-500 dark:text-gray-400">
			Positionen sind die einzelnen Zeilen eines Belegs. Diese Ausgabe hat keine.
		</p>
	{/if}
	{@render add('Position', addPosition)}

	{#snippet footer()}
		<div class="flex w-full justify-end">
			<Button color="alternative" onclick={() => (showPositions = false)}>Fertig</Button>
		</div>
	{/snippet}
</Modal>

<Modal title="Belege" bind:open={showReceipts} size="lg">
	{#each receipts as receipt, i (i)}
		<div class="flex items-center gap-2">
			<Input
				type="text"
				placeholder="URI (Datei, DMS-Dokument, …)"
				bind:value={receipt.uri}
				class="w-full"
			/>
			{#if receipt.ocr}
				<span class="text-xs whitespace-nowrap text-gray-500" title={receipt.ocr}>OCR</span>
			{/if}
			{@render minus(() => removeReceipt(i))}
		</div>
	{:else}
		<p class="text-sm text-gray-500 dark:text-gray-400">
			Ein Beleg ist ein Verweis auf eine Datei oder ein DMS-Dokument. Diese Ausgabe hat keinen.
		</p>
	{/each}
	{@render add('Beleg', addReceipt)}

	{#snippet footer()}
		<div class="flex w-full justify-end">
			<Button color="alternative" onclick={() => (showReceipts = false)}>Fertig</Button>
		</div>
	{/snippet}
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
