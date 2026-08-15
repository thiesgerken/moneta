<script lang="ts">
	import { Label, Input, Spinner } from 'flowbite-svelte';
	import { CloseOutline, SearchOutline } from 'flowbite-svelte-icons';
	import { graphql } from '$houdini';
	import { shortDate } from './analysis/dates';
	import QueryError from '$lib/QueryError.svelte';

	interface Props {
		templateId: string;
		error?: boolean;
	}
	let { templateId = $bindable(), error = false }: Props = $props();

	type Candidate = {
		readonly id: string;
		readonly title: string;
		readonly store: string;
		readonly accrualStart: Date;
		readonly effectiveAmount: number;
	};

	const label = (c: Candidate) =>
		`${c.title}${c.store ? ' · ' + c.store : ''} · ${shortDate(c.accrualStart)} · ${(
			c.effectiveAmount / 100
		).toFixed(2)}€`;

	/// What the input shows: the chosen expense, or what is being searched for. One field for
	/// both, since searching is how you choose.
	let query = $state('');
	let open = $state(false);
	let typed = $state(false);
	let active = $state(0);
	let selected = $state<Candidate | null>(null);
	let inputRef = $state<HTMLInputElement | undefined>(undefined);

	// Rules mostly point at ordinary past expenses rather than flagged templates, so the
	// picker searches everything by title instead of listing templates only.
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

	/// The expense a rule already points at, which a search for something else would not turn
	/// up. Fetched by id so it can be shown as itself rather than as "the current one".
	const selectedStore = graphql(`
		query SelectedTemplate($id: UUID!) {
			expenses {
				get(id: $id) {
					id
					title
					store
					accrualStart
					effectiveAmount
				}
			}
		}
	`);

	// with nothing typed the picker shows the flagged templates; a search widens it to every
	// expense, because rules mostly point at ordinary past ones
	const variables = $derived(
		typed && query.trim()
			? { search: query.trim(), flags: undefined }
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

	$effect(() => {
		// the same instance serves the next rule too, so a template that went away has to take
		// the text in the box with it
		if (!templateId) {
			if (selected) {
				selected = null;
				query = '';
			}
			return;
		}
		if (selected?.id === templateId) return;

		// untracked on purpose: this writes `selected`, which it also reads above
		void selectedStore.fetch({ variables: { id: templateId } }).then((r) => {
			const e = r.data?.expenses.get;
			if (e && e.id === templateId) {
				selected = e;
				query = label(e);
			}
		});
	});

	const candidates = $derived($templateStore.data?.expenses.list.edges.map((e) => e.node) || []);

	function choose(c: Candidate) {
		selected = c;
		templateId = c.id;
		query = label(c);
		typed = false;
		open = false;
	}

	function clear() {
		selected = null;
		templateId = '';
		query = '';
		typed = false;
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			open = false;
			return;
		}
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			open = true;
			const step = event.key === 'ArrowDown' ? 1 : -1;
			active = (active + step + candidates.length) % Math.max(candidates.length, 1);
			return;
		}
		// the picker sits in a form, and picking from a list must not submit it
		if (event.key === 'Enter' && open && candidates[active]) {
			event.preventDefault();
			choose(candidates[active]);
		}
	}
</script>

<QueryError
	errors={$templateStore.errors}
	message="Die Vorlagen konnten nicht geladen werden."
	class="mb-3"
/>

<!-- The list is closed by a click anywhere else rather than by blur: blur fires before the
     click on an option lands, which would close it out from under the pointer. -->
<svelte:window onclick={() => (open = false)} />

<Label class="space-y-2">
	<span>Vorlage</span>
	<div class="relative" onclick={(e) => e.stopPropagation()} role="none">
		<Input
			type="text"
			bind:value={query}
			placeholder="Ausgabe suchen (leer = nur Vorlagen)"
			color={error ? 'red' : undefined}
			autocomplete="off"
			bind:elementRef={inputRef}
			onfocus={() => {
				open = true;
				inputRef?.select();
			}}
			oninput={() => {
				typed = true;
				open = true;
				active = 0;
			}}
			onkeydown={keydown}
		>
			{#snippet left()}
				<SearchOutline class="h-4 w-4 text-gray-400 dark:text-gray-500" />
			{/snippet}
			{#snippet right()}
				{#if $templateStore.fetching}
					<Spinner size="4" />
				{:else if selected}
					<button type="button" aria-label="Vorlage entfernen" onclick={clear}>
						<CloseOutline class="h-4 w-4 text-gray-400 hover:text-gray-600 dark:text-gray-500" />
					</button>
				{/if}
			{/snippet}
		</Input>

		{#if open}
			<ul
				class="absolute z-20 mt-1 max-h-64 w-full overflow-y-auto rounded-lg border border-gray-200 bg-white shadow-lg dark:border-gray-600 dark:bg-gray-700"
			>
				{#each candidates as c, i (c.id)}
					<li>
						<button
							type="button"
							class="block w-full px-3 py-2 text-left text-sm {i === active
								? 'bg-gray-100 dark:bg-gray-600'
								: ''} {c.id === templateId ? 'font-semibold' : ''}"
							onmouseenter={() => (active = i)}
							onclick={() => choose(c)}
						>
							{c.title}
							<span class="block text-xs text-gray-500 dark:text-gray-400">
								{c.store ? c.store + ' · ' : ''}{shortDate(c.accrualStart)} · {(
									c.effectiveAmount / 100
								).toFixed(2)}€
							</span>
						</button>
					</li>
				{:else}
					<li class="px-3 py-2 text-sm text-gray-500 dark:text-gray-400">
						{typed && query.trim() ? 'Nichts gefunden.' : 'Keine Vorlagen vorhanden.'}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</Label>
