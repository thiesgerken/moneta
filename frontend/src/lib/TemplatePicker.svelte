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
		query TemplateCandidates($filter: String, $sort: String) {
			expenses {
				list(limit: 25, filter: $filter, sort: $sort) {
					edges {
						node {
							id
							title
							store
							bookingStart
							effectiveAmount
						}
					}
				}
			}
		}
	`);

	const filter = $derived(
		searchTerm.trim()
			? JSON.stringify({ title: { $regex: searchTerm.trim(), $options: 'i' } })
			: JSON.stringify({ isTemplate: true })
	);

	$effect(
		() =>
			void templateStore.fetch({
				variables: { filter, sort: JSON.stringify({ bookingStart: -1 }) }
			})
	);

	const candidates = $derived($templateStore.data?.expenses.list.edges.map((e) => e.node) || []);

	// the rule's current template will usually not be in the search results, so keep it
	// selectable rather than silently dropping it when the form is submitted
	const items = $derived([
		...(templateId && !candidates.some((c) => c.id === templateId)
			? [{ value: templateId, name: '(aktuell gewählte Vorlage)' }]
			: []),
		...candidates.map((c) => ({
			value: c.id,
			name: `${c.title}${c.store ? ' · ' + c.store : ''} · ${c.bookingStart?.toLocaleDateString('de-DE')} · ${(c.effectiveAmount / 100).toFixed(2)}€`
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
