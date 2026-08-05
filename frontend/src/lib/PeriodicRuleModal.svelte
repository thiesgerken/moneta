<script lang="ts">
	import { Modal, Label, Input, Button, Select, Helper } from 'flowbite-svelte';
	import { EditOutline, PlusOutline } from 'flowbite-svelte-icons';

	import { graphql } from '$houdini';
	import type { PeriodicRuleInput } from '$houdini/graphql/inputs';
	import TemplatePicker from './TemplatePicker.svelte';

	interface Props {
		open?: boolean;
		onsubmit?: () => Promise<void>;
		rule?: {
			readonly id: string;
			readonly title: string;
			readonly period: string;
			readonly titleReplacement: string | null;
			readonly amountReplacement: number | null;
			readonly lastCreation: Date;
			readonly templateId: string | null;
		};
	}
	let { open = $bindable(), rule, onsubmit }: Props = $props();

	let title = $derived(rule?.title || '');
	let period = $derived(rule?.period || 'MONTHLY');
	let titleReplacement = $derived(rule?.titleReplacement || '');
	let amountReplacement = $derived(
		rule?.amountReplacement != null ? (rule.amountReplacement / 100).toFixed(2) : ''
	);
	let lastCreation = $derived((rule?.lastCreation ?? new Date()).toISOString().slice(0, 10));
	let templateId = $derived(rule?.templateId || '');

	const titleError = $derived(!title.trim());
	const templateError = $derived(!templateId);
	const amountError = $derived(!!amountReplacement && isNaN(Number(amountReplacement)));
	const formHasErrors = $derived(titleError || templateError || amountError);

	const CreatePeriodicRule = graphql(`
		mutation CreatePeriodicRule($input: PeriodicRuleInput!) {
			periodicRules {
				create(rule: $input) {
					id
				}
			}
		}
	`);
	const UpdatePeriodicRule = graphql(`
		mutation UpdatePeriodicRule($id: UUID!, $input: PeriodicRuleInput!) {
			periodicRules {
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

		const input: PeriodicRuleInput = {
			title: title.trim(),
			templateId,
			period: period as PeriodicRuleInput['period'],
			titleReplacement: titleReplacement.trim() || null,
			amountReplacement: amountReplacement ? Math.round(Number(amountReplacement) * 100) : null,
			lastCreation: new Date(lastCreation)
		};

		const result = rule
			? await UpdatePeriodicRule.mutate({ id: rule.id, input })
			: await CreatePeriodicRule.mutate({ input });

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

<Modal title={rule ? 'Regel bearbeiten' : 'Neue periodische Regel'} bind:open size="lg">
	<form class="flex flex-col space-y-4" onsubmit={handleSubmit}>
		<Label class="space-y-2">
			<span>Titel</span>
			<Input type="text" bind:value={title} required color={titleError ? 'red' : undefined} />
		</Label>

		<TemplatePicker bind:templateId error={templateError} />

		<Label class="space-y-2">
			<span>Intervall</span>
			<Select
				bind:value={period}
				items={[
					{ value: 'DAILY', name: 'Täglich' },
					{ value: 'WEEKLY', name: 'Wöchentlich' },
					{ value: 'MONTHLY', name: 'Monatlich' },
					{ value: 'YEARLY', name: 'Jährlich' }
				]}
			/>
		</Label>

		<Label class="space-y-2">
			<span>Titel-Ersetzung</span>
			<Input type="text" bind:value={titleReplacement} placeholder="z.B. Miete %m/%y" />
			<Helper>Leer lassen, um den Titel der Vorlage zu behalten. %d, %m, %y werden ersetzt.</Helper>
		</Label>

		<Label class="space-y-2">
			<span>Betrags-Ersetzung (€)</span>
			<Input
				type="text"
				bind:value={amountReplacement}
				placeholder="z.B. -18.36"
				color={amountError ? 'red' : undefined}
			/>
			<Helper>Ausgaben sind negativ. Leer lassen, um den Betrag der Vorlage zu behalten.</Helper>
		</Label>

		<Label class="space-y-2">
			<span>Zuletzt erzeugt</span>
			<Input type="date" bind:value={lastCreation} />
			<Helper>Ausgaben nach diesem Datum werden beim nächsten Lauf erzeugt.</Helper>
		</Label>

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
