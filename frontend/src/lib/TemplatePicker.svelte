<script lang="ts">
	import { Label, Input, Select } from 'flowbite-svelte';
	import { graphql } from '$houdini';

	interface Props {
		templateId: string;
		error?: boolean;
	}
	let { templateId = $bindable(), error = false }: Props = $props();

	// Rules mostly point at ordinary past expenses rather than flagged templates, so the
	// picker searches everything by title instead of listing templates only.
	let searchTerm = $state('');

	const templateStore = graphql(`
		query TemplateCandidates($search: String, $flags: ExpenseFlagFilter) {
			expenses {
				list(limit: 25, search: $search, flags: $flags, sortBy: ACCRUAL_START, sortDir: DESC) {
					edges {
						node {
							id
							title
							store
							accrualStart
							effectiveAmount
						}
					}
				}
			}
		}
	`);

	// with nothing typed the picker shows the flagged templates; a search widens it to every
	// expense, because rules mostly point at ordinary past ones
	const variables = $derived(
		searchTerm.trim()
			? { search: searchTerm.trim(), flags: undefined }
			: {
					search: undefined,
					flags: {
						templates: true,
						taxRelevant: false,
						extraordinary: false,
						unchecked: false,
						incomplete: false,
						normal: false
					}
				}
	);

	$effect(() => void templateStore.fetch({ variables }));

	const candidates = $derived($templateStore.data?.expenses.list.edges.map((e) => e.node) || []);

	// the rule's current template will usually not be in the search results, so keep it
	// selectable rather than silently dropping it when the form is submitted
	const items = $derived([
		...(templateId && !candidates.some((c) => c.id === templateId)
			? [{ value: templateId, name: '(aktuell gewählte Vorlage)' }]
			: []),
		...candidates.map((c) => ({
			value: c.id,
			name: `${c.title}${c.store ? ' · ' + c.store : ''} · ${c.accrualStart?.toLocaleDateString('de-DE')} · ${(c.effectiveAmount / 100).toFixed(2)}€`
		}))
	]);
</script>

<Label class="space-y-2">
	<span>Vorlage suchen</span>
	<Input
		type="text"
		bind:value={searchTerm}
		placeholder="Titel der Ausgabe (leer = nur Vorlagen)"
	/>
</Label>

<Label class="space-y-2">
	<span>Vorlage</span>
	<Select
		bind:value={templateId}
		{items}
		placeholder="Ausgabe wählen"
		color={error ? 'red' : undefined}
	/>
</Label>
