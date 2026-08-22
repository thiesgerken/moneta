<script lang="ts">
	import { Label, Input, Spinner } from 'flowbite-svelte';
	import {
		ArrowUpRightFromSquareOutline,
		CloseOutline,
		SearchOutline
	} from 'flowbite-svelte-icons';
	import { graphql } from '$houdini';
	import { resolve } from '$app/paths';
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

	// An automation template's booking is a placeholder, so its amount is 0,00 EUR and saying so
	// is noise. A periodic one has no delivery to take an amount from and the template is the
	// only place the price lives, which is exactly what you are picking between.
	const amount = (c: Candidate) =>
		c.effectiveAmount ? ` · ${(c.effectiveAmount / 100).toFixed(2)}€` : '';

	const label = (c: Candidate) =>
		`${c.title}${c.store ? ' · ' + c.store : ''} · ${shortDate(c.accrualStart)}${amount(c)}`;

	/// What the input shows: the chosen expense, or what is being searched for. One field for
	/// both, since searching is how you choose.
	let query = $state('');
	let open = $state(false);
	let typed = $state(false);
	let active = $state(0);
	let selected = $state<Candidate | null>(null);
	let inputRef = $state<HTMLInputElement | undefined>(undefined);

	// Templates only, searched or not. A rule may point at nothing else: anything unflagged is a
	// real booking, one that is in the balances and in the Auswertung, and a rule pointing at it
	// turns this dialog into a way of rewriting money that has already been spent.
	const templateStore = graphql(`
		query TemplateCandidates($search: String) {
			expenses {
				list(
					limit: 25
					search: $search
					onlyTemplates: true
					sortBy: ACCRUAL_START
					sortDir: DESC
				) {
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

	const variables = $derived({
		search: typed && query.trim() ? query.trim() : undefined
	});

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
		<!-- flowbite lays the left and right slots over the field rather than beside it, and pads
		     for neither, so the text has to be kept out from under them here -->
		<Input
			type="text"
			bind:value={query}
			placeholder="Vorlage suchen"
			color={error ? 'red' : undefined}
			autocomplete="off"
			class="ps-10 pe-16"
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
				<span class="flex items-center gap-2">
					{#if $templateStore.fetching}
						<Spinner size="4" />
					{/if}
					{#if selected}
						<!-- A new tab, because this form holds unsaved edits: going there and coming
						     back must not be how they are lost. The expenses page takes the id as a
						     filter, so it lands on that one expense with its editor open. -->
						<a
							href="{resolve('/expenses')}?expense={selected.id}"
							target="_blank"
							rel="noreferrer"
							aria-label="Vorlage in neuem Tab öffnen"
							title="Vorlage in neuem Tab öffnen"
						>
							<ArrowUpRightFromSquareOutline
								class="h-4 w-4 text-gray-400 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300"
							/>
						</a>
						<button type="button" aria-label="Vorlage entfernen" onclick={clear}>
							<CloseOutline
								class="h-4 w-4 text-gray-400 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300"
							/>
						</button>
					{/if}
				</span>
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
								{c.store ? c.store + ' · ' : ''}{shortDate(c.accrualStart)}{amount(c)}
							</span>
						</button>
					</li>
				{:else}
					<li class="px-3 py-2 text-sm text-gray-500 dark:text-gray-400">
						{typed && query.trim()
							? 'Keine Vorlage mit diesem Titel.'
							: 'Keine Vorlagen. Eine Ausgabe wird zu einer, indem man sie dupliziert und im Editor als Vorlage markiert.'}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</Label>
