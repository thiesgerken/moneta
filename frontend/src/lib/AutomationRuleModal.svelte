<script lang="ts">
	import { Modal, Label, Input, Button, Select, Helper } from 'flowbite-svelte';
	import { EditOutline, PlusOutline } from 'flowbite-svelte-icons';

	import { graphql } from '$houdini';
	import type { AutomationRuleInput } from '$houdini/graphql/inputs';
	import TemplatePicker from './TemplatePicker.svelte';
	import { describeAccrual, formatAccrualPeriod } from './expense_utils';
	import { shortDate } from './analysis/dates';

	interface Props {
		open?: boolean;
		onsubmit?: () => Promise<void>;
		accounts: { readonly id: string; readonly name: string }[];
		rule?: {
			readonly id: string;
			readonly title: string;
			readonly priority: number;
			readonly regexTime: string | null;
			readonly regexStatement: string | null;
			readonly filterAmount: number | null;
			readonly filterAccountId: string | null;
			readonly templateId: string | null;
			readonly accrual: {
				readonly months: number | null;
				readonly days: number | null;
				readonly anchorDay: number;
				readonly phaseDays: number;
				readonly exampleAt: Date;
				readonly exampleFrom: Date;
				readonly exampleTo: Date;
			} | null;
		};
	}
	let { open = $bindable(), accounts, rule, onsubmit }: Props = $props();

	let title = $derived(rule?.title || '');
	let priority = $derived((rule?.priority ?? 1).toString());
	let regexTime = $derived(rule?.regexTime || '');
	let regexStatement = $derived(rule?.regexStatement || '');
	let filterAmount = $derived(
		rule?.filterAmount != null ? (rule.filterAmount / 100).toFixed(2) : ''
	);
	let filterAccountId = $derived(rule?.filterAccountId || '');
	let templateId = $derived(rule?.templateId || '');

	const titleError = $derived(!title.trim());
	const templateError = $derived(!templateId);
	const priorityError = $derived(isNaN(Number(priority)));
	const amountError = $derived(!!filterAmount && isNaN(Number(filterAmount)));

	// a rule with no filters at all matches every delivered transaction, which is almost
	// certainly a mistake rather than an intent
	const noFilters = $derived(
		!regexTime.trim() && !regexStatement.trim() && !filterAmount && !filterAccountId
	);

	const formHasErrors = $derived(titleError || templateError || priorityError || amountError);

	const CreateAutomationRule = graphql(`
		mutation CreateAutomationRule($input: AutomationRuleInput!) {
			automationRules {
				create(rule: $input) {
					id
				}
			}
		}
	`);
	const UpdateAutomationRule = graphql(`
		mutation UpdateAutomationRule($id: UUID!, $input: AutomationRuleInput!) {
			automationRules {
				update(id: $id, rule: $input) {
					id
				}
			}
		}
	`);

	let error: string | undefined = $state(undefined);

	const handleSubmit = async (event: Event) => {
		event.preventDefault();
		if (formHasErrors) return;

		const input: AutomationRuleInput = {
			title: title.trim(),
			templateId,
			priority: Number(priority),
			regexTime: regexTime.trim() || null,
			regexStatement: regexStatement.trim() || null,
			filterAmount: filterAmount ? Math.round(Number(filterAmount) * 100) : null,
			filterAccountId: filterAccountId || null
		};

		const result = rule
			? await UpdateAutomationRule.mutate({ id: rule.id, input })
			: await CreateAutomationRule.mutate({ input });

		if (result.errors?.length) {
			error = result.errors.map((e) => e.message).join(', ');
			return;
		}

		error = undefined;
		// the parent closes us: these modals live inside a <TabItem>, and from there a
		// write-back through bind:open does not reach it reliably
		await onsubmit?.();
	};
</script>

<Modal title={rule ? 'Regel bearbeiten' : 'Neue Automatisierungsregel'} bind:open size="lg">
	<form class="flex flex-col space-y-4" onsubmit={handleSubmit}>
		<Label class="space-y-2">
			<span>Titel</span>
			<Input type="text" bind:value={title} required color={titleError ? 'red' : undefined} />
		</Label>

		<TemplatePicker bind:templateId error={templateError} />

		{#if rule?.accrual && templateId === rule.templateId}
			<!-- Read-only because there is nothing here to set: the period comes from what the bank
			     names on the line, else from the template's own Leistungszeitraum against the day
			     the template was paid. A template that is a day out from a whole month silently
			     stops snapping, and this line is where that shows. -->
			<div class="text-sm text-gray-500 dark:text-gray-400">
				<span class="font-medium text-gray-700 dark:text-gray-300">Leistungszeitraum</span>
				<span class="ml-2">{describeAccrual(rule.accrual)}</span>
				<Helper class="mt-1">
					Zuletzt ausgelöst {shortDate(rule.accrual.exampleAt)} →
					{formatAccrualPeriod(rule.accrual.exampleFrom, rule.accrual.exampleTo)}. Nennt der
					Verwendungszweck selbst einen Zeitraum, gilt dieser.
				</Helper>
			</div>
		{/if}

		<Label class="space-y-2">
			<span>Verwendungszweck (Regex)</span>
			<Input type="text" bind:value={regexStatement} placeholder="z.B. Muster Stadtreinigung" />
		</Label>

		<Label class="space-y-2">
			<span>Konto</span>
			<Select
				bind:value={filterAccountId}
				items={[
					{ value: '', name: 'Beliebig' },
					...accounts.map((a) => ({ value: a.id, name: a.name }))
				]}
			/>
		</Label>

		<Label class="space-y-2">
			<span>Betrag (€, exakt)</span>
			<Input
				type="text"
				bind:value={filterAmount}
				placeholder="beliebig"
				color={amountError ? 'red' : undefined}
			/>
			<Helper>Ausgaben sind negativ.</Helper>
		</Label>

		<Label class="space-y-2">
			<span>Datum (Regex)</span>
			<Input type="text" bind:value={regexTime} placeholder="beliebig, Format TT.MM.JJJJ HH:MM" />
		</Label>

		<Label class="space-y-2">
			<span>Priorität</span>
			<Input type="number" bind:value={priority} color={priorityError ? 'red' : undefined} />
			<Helper>Bei mehreren Treffern gewinnt die höchste Priorität.</Helper>
		</Label>

		{#if noFilters}
			<p class="text-sm text-orange-600 dark:text-orange-400">
				Ohne Filter passt diese Regel auf jede gelieferte Transaktion.
			</p>
		{/if}

		{#if error}
			<p class="text-sm text-red-600 dark:text-red-400">{error}</p>
		{/if}

		<Button type="submit" disabled={formHasErrors}>
			{#if rule}
				<EditOutline class="mr-2 h-4 w-4" />Speichern
			{:else}
				<PlusOutline class="mr-2 h-4 w-4" />Anlegen
			{/if}
		</Button>
	</form>
</Modal>
