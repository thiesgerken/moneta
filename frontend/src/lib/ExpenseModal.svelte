<script lang="ts">
	import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';

	import type { Component } from 'svelte';
	import { graphql } from '$houdini';
	import ActionButton from './ActionButton.svelte';
	import type { ExpenseInput } from '$houdini/graphql/inputs';
	import {
		accountSorter,
		byUsageThen,
		categorySorter,
		formatAccount,
		referenceAmountMissing
	} from './expense_utils';
	import DateField from './DateField.svelte';
	import { today } from './analysis/dates';
	import ColorSelect from './ColorSelect.svelte';
	import { amountColor, signedEuro } from './money';
	import {
		CircleMinusOutline,
		ArrowsRepeatOutline,
		CirclePlusOutline,
		EditOutline,
		ExclamationCircleOutline,
		FileCloneOutline,
		LandmarkOutline,
		PlusOutline,
		QuestionCircleOutline,
		StarOutline,
		UsersGroupOutline
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
			readonly closed: Date | null;
			readonly usageCount: number;
			readonly owner: { readonly name: string; readonly firstName: string | null };
		}[];
		categories: {
			readonly id: string;
			readonly name: string;
			readonly color: string | null;
			readonly usageCount: number;
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
		expense?.referenceAmount != null ? (expense.referenceAmount / 100).toFixed(2) : undefined
	);
	// An expense typed in by hand is nearly always one that just happened, and a transaction
	// with no date of its own takes this one, so filling it fills the row below as well.
	let accrualStart: Date | undefined = $derived(
		expense?.accrualStart ? new Date(expense.accrualStart) : today()
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

	/// Where the other half of a shared expense goes: the debt account you use most, unless this
	/// expense already books against it.
	///
	/// Only my own -- "Krösus / Schulden Thies" is the same debt seen from her side, and putting
	/// this expense there would record it twice. Guessing at all is defensible here because
	/// there is a name for what the guess means; for a Umbuchung there is no such word, so that
	/// one asks.
	const shareAccount = $derived(
		accounts
			.filter(
				(account) =>
					!account.closed &&
					account.owner.name === user_name &&
					/schulden/i.test(account.name) &&
					!transactions.some((t) => t.accountId === account.id)
			)
			.toSorted((a, b) => b.usageCount - a.usageCount)[0]
	);

	/// What colour the computed share under a percentage is drawn in.
	///
	/// Green and red mean money in and money out, which is only true of an account of mine: on
	/// "Krösus / Commerzbank Girokonto" the same figure is her direction, not mine, and colouring it
	/// would state the opposite of what happened to me. Those stay grey, as does a share of
	/// nothing and a row with no account picked yet.
	function shareColor(accountId: string, value: number) {
		const account = accounts.find((a) => a.id === accountId);
		if (!account || account.owner.name !== user_name || value === 0) {
			return 'text-gray-600 dark:text-gray-400';
		}

		return amountColor(value);
	}

	/// The two things that happen to an expense often enough to be worth a button: half of it is
	/// somebody else's, or the money only moved to another account of yours.
	///
	/// Both are the same shape -- the amount that is already there becomes the reference and a
	/// share of it goes somewhere else -- and both are several steps by hand: switch the unit,
	/// type 100, copy the amount into the reference field, add a row, switch its unit, type the
	/// percentage. A negative share of a negative amount comes out positive, which is what makes
	/// this the money coming back.
	function addShare(percentage: string, accountId: string) {
		if (!Number(referenceAmount)) {
			const fixed = transactions.filter((t) => !t.isRelative && Number(t.amount));
			if (fixed.length === 1) {
				referenceAmount = fixed[0].amount;
				fixed[0].percentage = '100';
				fixed[0].isRelative = true;
			}
		}

		transactions = [
			...transactions,
			{ ...emptyTransaction(), isRelative: true, percentage, accountId }
		];
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
			//
			// That one becomes a share as well, of 100%, which is the same money it was already
			// booking. It is what makes the reference amount worth having: left as a fixed
			// -179, it would sit there unchanged while every share beside it moved, and the
			// expense would stop adding up the moment the reference was corrected.
			if (!Number(referenceAmount)) {
				const fixed = transactions.filter((t, j) => j !== i && !t.isRelative && Number(t.amount));
				if (fixed.length === 1) {
					referenceAmount = fixed[0].amount;
					fixed[0].percentage = '100';
					fixed[0].isRelative = true;
				}
			}

			// Failing that, this transaction's own amount is the only figure in the dialog that
			// the share could be a share of, and switching the one transaction there is to % is
			// how a split starts as often as not. It comes out at 100%, which is what it is.
			if (!Number(referenceAmount) && Number(tx.amount)) {
				referenceAmount = tx.amount;
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

	/// What the account picker offers: the closed ones left out, the five you actually use up
	/// top, and `accountSorter` for the rest -- mine before anyone else's, alphabetical inside
	/// that, the same order the toolbar's filters use.
	///
	/// A closed account stays in the list if this expense books against it. Dropping it would
	/// quietly take the account off the expense the moment somebody opened it to fix a typo.
	const accountOptions = $derived(
		byUsageThen(
			accounts.filter(
				(account) => !account.closed || transactions.some((t) => t.accountId === account.id)
			),
			(a, b) => accountSorter(a, b, user_name ?? '')
		).map((account) => ({
			value: account.id,
			name: formatAccount(account, user_name),
			color: account.color
		}))
	);

	const categoryOptions = $derived(
		byUsageThen(categories, categorySorter).map((category) => ({
			value: category.id,
			name: category.name,
			color: category.color
		}))
	);

	/// The ring a field wears when what is in it is not usable.
	///
	/// It goes on a wrapper rather than through flowbite's own `color="red"`, because
	/// flowbite-svelte 1.33.1 works out a control's class exactly once -- see ActionButton for
	/// the same bug on a Button. This dialog is mounted with no expense, so every amount started
	/// empty, every empty amount is an error, and every field stayed red for the rest of its life
	/// however valid what you then typed into it was. Keying the control would fix the class and
	/// lose the caret on every keystroke, so the state lives outside the control instead.
	const ring = (error: boolean | undefined) =>
		error ? 'rounded-lg ring-2 ring-red-500 dark:ring-red-500' : '';

	const titleError = $derived(!title);
	const referenceAmountError = $derived.by(() => {
		const hasRelativeTransactions = transactions.some((tx) => tx.isRelative);
		if (!hasRelativeTransactions) return false;

		// Must be provided when relative transactions exist
		return referenceAmountMissing(referenceAmount);
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
			// a weight is a share of the expense, so any positive number is one -- the same
			// bound the column carries. A floor of 1 also refused to save the imported
			// expenses, whose shares the importer writes below it.
			if (cat.weight && (isNaN(Number(cat.weight)) || Number(cat.weight) <= 0))
				errors.weight = true;
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
				? Math.round(Number(referenceAmount) * 100)
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
				amount: !tx.isRelative ? Math.round(Number(tx.amount) * 100) : undefined,
				fraction: tx.isRelative ? Number(tx.percentage) / 100 : undefined,
				statement: tx.statement,
				comments: ''
			})),
			positions: positions.map((pos) => ({
				title: pos.title,
				amount: pos.totalAmount ? Math.round(Number(pos.totalAmount) * 100) : undefined,
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
	<!-- Solid, because grey is what disabled looks like: a transparent or white button reads as
	     one you cannot press, and next to its own 50%-opacity version there was nothing to tell
	     the two apart. -->
	<ActionButton
		color="red"
		size="xs"
		class="p-1.5"
		{disabled}
		onclick={(e: MouseEvent) => {
			e.stopPropagation();
			onclick();
		}}
	>
		<CircleMinusOutline class="h-4 w-4" />
	</ActionButton>
{/snippet}

{#snippet flagChip(label: string, Icon: Component, on: boolean, toggle: () => void, tone: string)}
	<!-- A chip rather than a checkbox, and when it is on it is the badge the list draws for the
	     same flag: what you switch on here is what you then look for in the row. Off is an
	     outline rather than a grey fill -- grey is what a disabled control is, and there is one
	     of those in this dialog already. -->
	<button
		type="button"
		aria-pressed={on}
		onclick={toggle}
		class="inline-flex cursor-pointer items-center gap-1.5 rounded-md border px-2.5 py-1 text-sm transition-colors {on
			? tone
			: 'border-gray-300 text-gray-500 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-400 dark:hover:bg-gray-700'}"
	>
		<Icon class="h-4 w-4 shrink-0" />{label}
	</button>
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

{#snippet shortcut(label: string, Icon: Component, title: string, onclick: () => void)}
	<Button
		color="alternative"
		size="xs"
		{title}
		onclick={(e: MouseEvent) => {
			e.stopPropagation();
			onclick();
		}}
	>
		<Icon class="mr-1 h-4 w-4" />{label}
	</Button>
{/snippet}

{#snippet add(label: string, onclick: () => void, cls = 'mt-2')}
	<Button
		color="alternative"
		size="xs"
		class={cls}
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
		{#if isTemplate}
			<!-- The flag is a chip among five others further down, which is not where somebody
			     wondering why this expense is missing from a total would look. -->
			<div
				class="col-span-full flex items-center gap-2 rounded-lg bg-blue-50 px-3 py-2 text-sm text-blue-800 dark:bg-blue-900/40 dark:text-blue-200"
			>
				<FileCloneOutline class="h-4 w-4 shrink-0" />
				<span>
					Vorlage: zählt in keinem Kontostand, keiner Auswertung und keiner Summe mit. Sie ist die
					Form, die eine Regel kopiert.
				</span>
			</div>
		{/if}

		<div class="sm:col-span-2 lg:col-span-3">
			<Label for="title" class="mb-2">Titel</Label>
			<div class={ring(titleError)}>
				<Input type="text" clearable id="title" placeholder="Titel" bind:value={title} required />
			</div>
		</div>
		<div class="sm:col-span-2 lg:col-span-1">
			<Label for="store" class="mb-2">Geschäft</Label>
			<Input type="text" clearable id="store" placeholder="Geschäft" bind:value={store} />
		</div>
		<div>
			<div class="mb-2 flex items-baseline justify-between gap-2">
				<Label class="block">{isRange ? 'Leistung von' : 'Leistung'}</Label>
				{#if !isRange}
					{@render textButton('Zeitraum', () => (isRange = true))}
				{/if}
			</div>
			<div class={ring(accrualStartError)}>
				<DateField bind:value={accrualStart} />
			</div>
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
				<div class={ring(accrualEndError)}>
					<DateField bind:value={accrualEnd} placeholder="Enddatum" />
				</div>
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
			<div class="flex flex-wrap gap-2">
				{@render flagChip(
					'Vorlage',
					FileCloneOutline,
					isTemplate,
					() => (isTemplate = !isTemplate),
					'border-transparent bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-200'
				)}
				{@render flagChip(
					'Unbestätigt',
					QuestionCircleOutline,
					isUnchecked,
					() => (isUnchecked = !isUnchecked),
					'border-transparent bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-200'
				)}
				{@render flagChip(
					'Unvollständig',
					ExclamationCircleOutline,
					isIncomplete,
					() => (isIncomplete = !isIncomplete),
					'border-transparent bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-200'
				)}
				{@render flagChip(
					'Außergewöhnlich',
					StarOutline,
					isExtraordinary,
					() => (isExtraordinary = !isExtraordinary),
					'border-transparent bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-200'
				)}
				{@render flagChip(
					'Steuerrelevant',
					LandmarkOutline,
					isTaxRelevant,
					() => (isTaxRelevant = !isTaxRelevant),
					'border-transparent bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-200'
				)}
			</div>
		</div>

		<div class="col-span-full">
			<!-- min-h: the Referenzbetrag appears the moment a transaction is switched to %, and a
			     header that grows by the height of an input moves everything under it down as you
			     click. The row is as tall as its tallest state from the start. -->
			<div class="mb-2 flex min-h-[38px] flex-wrap items-center justify-between gap-2">
				<div class="flex items-center gap-2">
					<Label class="block">Transaktionen</Label>
					{@render add('Transaktion', addTransaction, '')}
					{@render shortcut(
						'Teilen',
						UsersGroupOutline,
						shareAccount
							? `Die Hälfte geht an ${formatAccount(shareAccount, user_name)}`
							: 'Die Hälfte geht an ein anderes Konto',
						() => addShare('-50', shareAccount?.id ?? '')
					)}
					{@render shortcut(
						'Umbuchung',
						ArrowsRepeatOutline,
						'Das Geld ist auf ein anderes Konto gegangen',
						() => addShare('-100', '')
					)}
				</div>
				<!-- Only a relative transaction has anything to be relative to, which is also the
				     only reason this field exists -- so it lives with them rather than in the form
				     above, where it appeared and disappeared and moved every field after it. -->
				{#if transactions.some((tx) => tx.isRelative)}
					<div class="flex items-center gap-2">
						<Label for="referenceAmount" class="whitespace-nowrap">Referenzbetrag</Label>
						<div class="relative {ring(referenceAmountError)}">
							<Input
								id="referenceAmount"
								size="sm"
								placeholder="0"
								bind:value={referenceAmount}
								step="0.01"
								class="no-spinner w-28 pr-6"
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
							<!-- Two dates a day or two apart and neither of them labelled is a column
							     nobody can fill in with confidence. The `title` and nothing else: an
							     explanation that takes a line of the header is bigger than the thing it
							     explains, and a dotted underline to advertise it would be bigger still. -->
							<th class="w-32 px-2" scope="col" title="als das Geld floss">Wertstellung</th>
							<th class="w-32 px-2" scope="col" title="wie die Bank sie gebucht hat"> Buchung </th>
							<th class="w-36 px-2" scope="col">Konto</th>
							<th class="w-36 px-2" scope="col">Betrag</th>
							<th class="px-2" scope="col">Transaktionstext</th>
							<th class="w-10 px-2" scope="col"><span class="sr-only">Entfernen</span></th>
						</tr>
					</thead>
					<tbody class="divide-y">
						{#each transactions as tx, i (i)}
							<tr>
								<td class="px-2">
									<!-- No calendar button in here: it is positioned over the right of the input
										     rather than beside it, so in a 128px column it sat on top of the year
										     and "05.08.2026" read as "05.08.202". The field still opens on click.
										     The pickers with room for it keep theirs. -->
									<DateField
										bind:value={tx.date}
										placeholder="wie Leistung"
										btnClass="hidden"
										inputClass="px-2"
									/>
								</td>
								<td class="px-2">
									<DateField
										bind:value={tx.bookingDate}
										placeholder="—"
										btnClass="hidden"
										inputClass="px-2"
										clearable
									/>
								</td>
								<td class="px-2">
									<div class={ring(transactionErrors[i]?.accountId)}>
										<ColorSelect
											options={accountOptions}
											bind:value={tx.accountId}
											placeholder="Konto"
											empty="Keine offenen Konten"
										/>
									</div>
								</td>
								<td class="px-2">
									<!-- The unit is the switch. A column of its own for a checkbox called
									     "Relativ" cost more width than the amount it qualified, and said
									     nothing about what the number in the next cell then meant. -->
									<div
										class="relative {ring(
											tx.isRelative
												? transactionErrors[i]?.percentage
												: transactionErrors[i]?.amount
										)}"
									>
										{#if tx.isRelative}
											<Input
												placeholder=""
												bind:value={tx.percentage}
												class="no-spinner pr-8"
												min="0"
												max="100"
												step="0.01"
											/>
										{:else}
											<Input placeholder="" bind:value={tx.amount} class="no-spinner pr-8" />
										{/if}
										<button
											type="button"
											onclick={() => toggleRelative(i)}
											title={tx.isRelative
												? 'Anteil am Referenzbetrag — klicken für festen Betrag'
												: 'Fester Betrag — klicken für Anteil am Referenzbetrag'}
											class="absolute top-1/2 right-1 -translate-y-1/2 cursor-pointer rounded bg-gray-200 px-1.5 py-0.5 text-gray-600 hover:bg-gray-300 dark:bg-gray-600 dark:text-gray-200 dark:hover:bg-gray-500"
										>
											{tx.isRelative ? '%' : '€'}
										</button>
									</div>
									{#if tx.isRelative && referenceAmount && tx.percentage && Number(referenceAmount)}
										{@const share = Math.round(Number(tx.percentage) * Number(referenceAmount))}
										<div class="mt-1 text-xs {shareColor(tx.accountId, share)}">
											= {signedEuro(share)}
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
		</div>

		<!-- Under the transactions rather than beside them: the statement text is the widest
		     thing in the dialog and a category is two short fields, so the pair spent the row on
		     the wrong one. Half a row is all a category needs; the rest of it is below. -->
		<div class="col-span-full lg:col-span-4">
			<!-- min-h on both this and Weiteres beside it: one carries an add button and the other
			     does not, so the two tables under them started 14px apart. -->
			<div class="mb-2 flex min-h-[34px] items-center gap-2">
				<Label class="block">Kategorien</Label>
				{@render add('Kategorie', addCategory, '')}
			</div>
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
									<div class={ring(categoryErrors[i]?.categoryId)}>
										<!-- a category another row already uses is not offered again, but the one
										     this row holds stays in its own list -->
										<ColorSelect
											options={categoryOptions.filter(
												(option) =>
													option.value === cat.categoryId ||
													!expenseCategories.some((other) => other.categoryId === option.value)
											)}
											bind:value={cat.categoryId}
											placeholder="Kategorie"
										/>
									</div>
								</td>
								<td class="px-2">
									<div class={ring(categoryErrors[i]?.weight)}>
										<!-- step: the default of 1 would reject a share like 0.5 before the
										     validation ever sees it -->
										<Input
											type="number"
											min="0"
											step="any"
											class="no-spinner w-full"
											bind:value={cat.weight}
											placeholder="Gewicht"
										/>
									</div>
								</td>
								<td class="px-2">
									{@render minus(() => removeCategory(i), expenseCategories.length === 1)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>

		<!-- Positionen and Belege are empty on almost every expense and cost a heading and a
		     button each when they were. What is left is what they would have said -- beside the
		     categories, which is the space the transactions gave up. -->
		<div class="col-span-full lg:col-span-2">
			<div class="mb-2 flex min-h-[34px] items-center gap-2">
				<Label class="block">Weiteres</Label>
			</div>
			<div class="flex flex-wrap gap-2">
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
		</div>
	</form>

	{#snippet footer()}
		<!-- The submit button sits outside the form it submits, which is what `form=` is for. It
		     was a full-width bar at the bottom of the scroll before, so on a long expense you
		     scrolled past everything to save, and there was no way out but the X. -->
		<div class="flex w-full justify-end gap-2">
			<Button color="alternative" onclick={() => (open = false)}>Abbrechen</Button>
			<ActionButton type="submit" form="expense-form" disabled={formHasErrors}>
				{#if isEdit}
					<EditOutline class="mr-1 h-4 w-4" />
					Aktualisieren
				{:else}
					<PlusOutline class="mr-1 h-4 w-4" />
					Erstellen
				{/if}
			</ActionButton>
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
								<div class={ring(positionErrors[i]?.title)}>
									<Input type="text" placeholder="Titel" bind:value={pos.title} class="w-full" />
								</div>
							</td>
							<td class="px-2">
								<div class="relative {ring(positionErrors[i]?.totalAmount)}">
									<Input
										placeholder=""
										bind:value={pos.totalAmount}
										class="no-spinner w-full pr-6"
										step="0.01"
									/>
									<span
										class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
										>€</span
									>
								</div>
							</td>
							<td class="px-2">
								<div class={ring(positionErrors[i]?.quantity)}>
									<Input
										placeholder=""
										bind:value={pos.quantity}
										class="no-spinner w-full"
										step="0.01"
									/>
								</div>
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
