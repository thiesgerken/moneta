<script lang="ts">
	import { Modal, Label, Input, Button, Select, Helper } from 'flowbite-svelte';
	import { EditOutline, PlusOutline } from 'flowbite-svelte-icons';

	import { graphql } from '$houdini';
	import type { AutomationRuleInput } from '$houdini/graphql/inputs';
	import TemplatePicker from './TemplatePicker.svelte';
	import RegexInput from './RegexInput.svelte';
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
			readonly lastDelivery: Date | null;
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

	/// What the picked template would make of a payment. Asked for by template rather than read
	/// off the rule, so it follows the picker instead of showing what was saved last time.
	const previewStore = graphql(`
		query AccrualPreview($templateId: UUID!, $accountId: UUID, $at: DateTime) {
			automationRules {
				preview(templateId: $templateId, accountId: $accountId, at: $at) {
					months
					days
					anchorDay
					phaseDays
					exampleAt
					exampleFrom
					exampleTo
				}
			}
		}
	`);

	$effect(() => {
		if (!templateId) return;

		// every variable every time: houdini merges them into the last fetch's, so an account
		// cleared back to "beliebig" would otherwise keep filtering by the old one
		void previewStore.fetch({
			variables: {
				templateId,
				accountId: filterAccountId || null,
				at: rule?.lastDelivery ?? null
			}
		});
	});

	const preview = $derived(
		templateId ? ($previewStore.data?.automationRules.preview ?? null) : null
	);

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

{#snippet heading(text: string)}
	<h3 class="text-xs font-semibold tracking-wide text-gray-500 uppercase dark:text-gray-400">
		{text}
	</h3>
{/snippet}

<Modal title={rule ? 'Regel bearbeiten' : 'Neue Automatisierungsregel'} bind:open size="lg">
	<form class="flex flex-col space-y-6" onsubmit={handleSubmit}>
		<Label class="space-y-2">
			<span>Titel</span>
			<Input type="text" bind:value={title} required color={titleError ? 'red' : undefined} />
		</Label>

		<!-- The template and what follows from it are one thing: everything about the generated
		     expense comes from the template, including a Leistungszeitraum that is derived rather
		     than set. Grouping them is what makes the second half read as a consequence of the
		     first rather than as another setting. -->
		<section class="space-y-2">
			{@render heading('Was erzeugt wird')}
			<TemplatePicker bind:templateId error={templateError} />

			{#if preview}
				<div
					class="ml-1 border-l-2 border-gray-200 py-0.5 pl-3 text-sm text-gray-500 dark:border-gray-600 dark:text-gray-400"
				>
					<span class="text-gray-700 dark:text-gray-300">
						Leistungszeitraum: {describeAccrual(preview)}
					</span>
					<span class="mt-0.5 block text-xs">
						{preview.exampleAt ? shortDate(preview.exampleAt) : ''} →
						{formatAccrualPeriod(preview.exampleFrom, preview.exampleTo)}
						{#if rule?.lastDelivery}(zuletzt ausgelöst){:else}(heute){/if}. Nennt der
						Verwendungszweck selbst einen Zeitraum, gilt dieser.
					</span>
				</div>
			{/if}
		</section>

		<section class="space-y-4">
			{@render heading('Worauf sie passt')}

			<div class="space-y-2">
				<Label for="regex-statement">Verwendungszweck (Regex)</Label>
				<RegexInput
					id="regex-statement"
					bind:value={regexStatement}
					placeholder="z.B. Muster Stadtreinigung"
				/>
			</div>

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

			<div class="space-y-2">
				<Label for="regex-time">Datum (Regex)</Label>
				<RegexInput
					id="regex-time"
					bind:value={regexTime}
					placeholder="beliebig, Format TT.MM.JJJJ HH:MM"
				/>
			</div>

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
		</section>

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
