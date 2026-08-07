<script lang="ts">
	import {
		cache,
		graphql,
		type ChangelogAuthor$options,
		type ExpenseChangelog$result
	} from '$houdini';
	import { Badge, Spinner } from 'flowbite-svelte';
	import {
		CirclePlusOutline,
		EditOutline,
		ChevronDownOutline,
		ChevronRightOutline
	} from 'flowbite-svelte-icons';
	import ToggleGroup from './ToggleGroup.svelte';
	import ExpenseModal from './ExpenseModal.svelte';
	import { changedFields, summarizeFields, stateAmount } from './changelog';
	import { formatAccount } from './expense_utils';
	import { categoryColor } from './analysis/overviews';
	import { feedState } from './analysis/state.svelte';

	let expandedDiffs = $state<string[]>([]);

	const changelogStore = graphql(`
		query ExpenseChangelog($author: ChangelogAuthor!) {
			users {
				me {
					name
				}
			}

			# a changelog state stores ids, not the accounts and categories themselves, so the
			# chips are looked up here -- both lists are short enough to fetch whole
			accounts {
				list {
					id
					name
					color
					# whose account it is: two people can both keep a "Bargeld"
					owner {
						name
						firstName
					}
				}
			}

			categories {
				list {
					id
					name
					color
				}
			}

			expenses {
				changelog(limit: 100, skip: 0, author: $author) {
					edges {
						node {
							id
							expenseId
							date
							tool
							automatic
							diff
							user {
								name
								firstName
							}
							oldState {
								# only to tell a creation from a change
								__typename
							}
							newState {
								title
								store
								accrualStart
								accrualEnd
								referenceAmount
								categories {
									categoryId
								}
								transactions {
									amount
									fraction
									accountId
								}
							}
						}
					}
				}
			}
		}
	`);

	/// An entry names an expense but does not carry it: the states it stores are snapshots, and
	/// what the editor needs is the expense as it is now.
	const expenseStore = graphql(`
		query ChangelogExpense($id: UUID!) {
			expenses {
				get(id: $id) {
					id
					title
					description
					store
					accrualStart
					accrualEnd
					comments
					isTaxRelevant
					isTemplate
					isUnchecked
					isIncomplete
					isExtraordinary
					referenceAmount
					receipts {
						uri
						ocr
					}
					positions {
						title
						amount
						quantity
						unit
						comments
					}
					categories {
						weight
						category {
							id
						}
					}
					transactions {
						fraction
						effectiveAmount
						amount
						date
						statement
						account {
							id
						}
					}
				}
			}
		}
	`);

	// the store is cleared while a fetch is in flight, which empties the list between filters
	let latestRequest = 0;
	let feed = $state<ExpenseChangelog$result | null>(null);

	async function load() {
		const request = ++latestRequest;
		const result = await changelogStore.fetch({
			variables: { author: feedState.author as ChangelogAuthor$options }
		});
		if (request === latestRequest) feed = result.data ?? null;
	}

	$effect(() => {
		void feedState.author;
		void load();
	});

	let editing = $state(false);
	const expense = $derived($expenseStore.data?.expenses.get);

	/// Opened only once the expense is there, so a click never shows an empty form -- and an
	/// entry whose expense has since been deleted opens nothing at all.
	async function edit(id: string) {
		const result = await expenseStore.fetch({ variables: { id } });
		editing = result.data?.expenses.get != null;
	}

	// editing an expense writes another changelog entry, so the feed itself is out of date
	async function saved() {
		cache.markStale('Expense');
		await load();
	}

	const me = $derived(feed?.users.me?.name);

	const euro = (cents: number) =>
		(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

	const day = (d: Date) => `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;

	/// "Heute" and "Gestern" carry more than a date does — everything older is dated in full,
	/// with the weekday, which is how people remember when they bought something.
	function dayLabel(d: Date): string {
		const today = new Date();
		if (day(d) === day(today)) return 'Heute';

		const yesterday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - 1);
		if (day(d) === day(yesterday)) return 'Gestern';

		return d.toLocaleDateString('de-DE', {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			year: d.getFullYear() === today.getFullYear() ? undefined : 'numeric'
		});
	}

	type Entry = (typeof entries)[number];

	const entries = $derived(
		(feed?.expenses.changelog.edges ?? [])
			.map((e) => e.node)
			.sort((a, b) => b.date.getTime() - a.date.getTime())
	);

	// Grouped rather than one flat stream: without the day headers every row has to repeat its
	// date, which is what made the list read as one long paragraph.
	const days = $derived.by(() => {
		const out: { key: string; label: string; entries: Entry[] }[] = [];

		for (const entry of entries) {
			const key = day(entry.date);
			if (out.at(-1)?.key !== key) out.push({ key, label: dayLabel(entry.date), entries: [] });
			out.at(-1)?.entries.push(entry);
		}

		return out;
	});

	/// Dative, since it always follows "von": "von dir", "von Krösus".
	const actor = (entry: Entry) =>
		entry.user?.name === me ? 'dir' : (entry.user?.firstName ?? entry.user?.name ?? 'jemandem');

	/// What the entry did, in one phrase. The diff says which fields moved, which beats
	/// "geändert" on every single row. Capitalized because it opens the line, and the field
	/// names it is built from are nouns anyway.
	function action(entry: Entry): string {
		if (!entry.oldState) return 'Angelegt';

		const fields = summarizeFields(entry.diff ? changedFields(entry.diff) : []);
		return fields ? `${fields} geändert` : 'Geändert';
	}

	const named = $derived([
		...(feed?.categories.list ?? []).map((c) => ({
			id: c.id.toString(),
			name: c.name,
			color: categoryColor(c)
		})),
		// an account of somebody else's is named after them, as everywhere else: the chip is
		// there to say which pot the money came out of, and "Bargeld" alone does not
		...(feed?.accounts.list ?? []).map((a) => ({
			id: a.id.toString(),
			name: formatAccount(a, me),
			color: categoryColor(a)
		}))
	]);

	/// One chip per category and account the expense touches. Deduplicated, since an expense
	/// split over two cards names the same account twice, and the second chip adds nothing.
	function chips(state: NonNullable<Entry['newState']>) {
		const ids = [
			...state.categories.map((c) => c.categoryId.toString()),
			...state.transactions.map((t) => t.accountId.toString())
		];

		return ids
			.filter((id, i) => ids.indexOf(id) === i)
			.map((id) => named.find((e) => e.id === id))
			.filter((e) => e !== undefined);
	}

	/// The accrual period, short: same-day expenses are the common case and need no range.
	function period(state: { accrualStart: Date; accrualEnd: Date }): string {
		// `de-DE` puts a trailing dot on a day-and-month date, which a year must not follow
		const short = (d: Date) =>
			d.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit' }).replace(/\.$/, '');
		const full = (d: Date) => d.toLocaleDateString('de-DE');

		return state.accrualStart.getTime() === state.accrualEnd.getTime()
			? full(state.accrualStart)
			: `${short(state.accrualStart)}–${full(state.accrualEnd)}`;
	}

	const toggleDiff = (id: string) =>
		(expandedDiffs = expandedDiffs.includes(id)
			? expandedDiffs.filter((other) => other !== id)
			: [...expandedDiffs, id]);

	/// A unified diff is unreadable in one colour; the marker column is the whole signal.
	const diffLineClass = (line: string) => {
		if (line.startsWith('+')) return 'text-green-700 dark:text-green-400';
		if (line.startsWith('-')) return 'text-red-700 dark:text-red-400';
		if (line.startsWith('@@')) return 'text-gray-400 dark:text-gray-500';
		return 'text-gray-500 dark:text-gray-400';
	};
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between gap-3 pb-2">
		<ToggleGroup
			bind:value={feedState.author}
			size="sm"
			options={[
				{ value: 'OTHERS', label: 'Nur andere' },
				{ value: 'ME', label: 'Nur meine' },
				{ value: 'ANYONE', label: 'Alle' }
			]}
		/>
		{#if $changelogStore.fetching}
			<Spinner size="4" />
		{/if}
	</div>

	{#if $changelogStore.fetching && entries.length === 0}
		<div class="flex justify-center py-12"><Spinner size="6" /></div>
	{:else if entries.length === 0}
		<p class="py-12 text-center text-sm text-gray-500 dark:text-gray-400">
			{feedState.author === 'OTHERS'
				? 'Niemand sonst hat etwas geändert.'
				: 'Noch keine Aktivität.'}
		</p>
	{/if}

	{#each days as group (group.key)}
		<section class="mb-6">
			<h2
				class="sticky top-0 z-10 bg-white/85 py-2 text-xs font-semibold tracking-wide text-gray-500 uppercase backdrop-blur dark:bg-gray-900/85 dark:text-gray-400"
			>
				{group.label}
			</h2>

			<ol class="divide-y divide-gray-100 dark:divide-gray-800">
				{#each group.entries as entry (entry.id)}
					{@const created = !entry.oldState}
					{@const amount = entry.newState ? stateAmount(entry.newState) : null}
					<li class="flex gap-3 py-3">
						<span
							class="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full {created
								? 'bg-green-100 text-green-600 dark:bg-green-900/40 dark:text-green-400'
								: 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400'}"
						>
							{#if created}
								<CirclePlusOutline class="h-4 w-4" />
							{:else}
								<EditOutline class="h-4 w-4" />
							{/if}
						</span>

						<div class="min-w-0 flex-1">
							<button
								type="button"
								class="group flex w-full items-baseline justify-between gap-3 text-left"
								onclick={() => edit(entry.expenseId.toString())}
							>
								<span
									class="truncate font-medium text-gray-900 group-hover:underline dark:text-white"
								>
									{entry.newState?.title || 'Ohne Titel'}
								</span>
								{#if amount !== null}
									<!-- the same red/green the expense list uses, so a number means the
									     same thing in both places -->
									<span
										class="shrink-0 text-sm font-medium tabular-nums {amount < 0
											? 'text-red-700 dark:text-red-400'
											: amount > 0
												? 'text-green-700 dark:text-green-400'
												: 'text-gray-500 dark:text-gray-400'}"
									>
										{euro(amount)}
									</span>
								{/if}
							</button>

							<div
								class="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-xs text-gray-500 dark:text-gray-400"
							>
								<!-- One flex item: the row's gap is sized to separate the ·-delimited segments,
								     and using it between the words of a phrase spaces them like a heading.
								     Inside, a plain space does what a space should.

								     The clock hangs on the name rather than sitting in the row: the day header
								     already places the entry, so the exact minute is only worth asking for -->
								<span>
									{action(entry)} von
									<span
										class="underline decoration-gray-300 decoration-dotted dark:decoration-gray-600"
										title={entry.date.toLocaleString('de-DE', {
											dateStyle: 'full',
											timeStyle: 'short'
										})}>{actor(entry)}</span
									>
								</span>
								{#if entry.newState}
									<span aria-hidden="true">·</span>
									<span>gebucht {period(entry.newState)}</span>
								{/if}
								{#if entry.newState?.store}
									<span aria-hidden="true">·</span>
									<span class="truncate">{entry.newState.store}</span>
								{/if}
								{#each entry.newState ? chips(entry.newState) : [] as chip (chip.id)}
									<span
										class="inline-flex items-center gap-1 rounded-full bg-gray-100 px-1.5 py-0.5 dark:bg-gray-800"
									>
										<!-- a dot rather than a tinted pill: the colours are picked for a chart, and
										     half of them cannot carry readable text behind them -->
										<span
											class="h-2 w-2 shrink-0 rounded-full"
											style="background-color: {chip.color}"
										></span>
										{chip.name}
									</span>
								{/each}
								{#if entry.tool}
									<!-- which client wrote the change is the least of what the row says, and
									     on a phone it was the part that pushed it onto a fourth line -->
									<span class="hidden sm:inline" aria-hidden="true">·</span>
									<span class="hidden sm:inline">{entry.tool}</span>
								{/if}
								{#if entry.automatic}
									<Badge color="gray" class="px-1.5 py-0">automatisch</Badge>
								{/if}
								{#if entry.diff && !created}
									<button
										class="ml-auto flex items-center gap-1 text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
										onclick={() => toggleDiff(entry.id.toString())}
									>
										{#if expandedDiffs.includes(entry.id.toString())}
											<ChevronDownOutline class="h-3 w-3" />
										{:else}
											<ChevronRightOutline class="h-3 w-3" />
										{/if}
										Details
									</button>
								{/if}
							</div>

							{#if entry.diff && expandedDiffs.includes(entry.id.toString())}
								<pre
									class="mt-2 overflow-x-auto rounded-md bg-gray-50 p-2 font-mono text-xs dark:bg-gray-800/60">{#each entry.diff.split('\n') as line, i (i)}<span
											class="block {diffLineClass(line)}">{line || ' '}</span
										>{/each}</pre>
							{/if}
						</div>
					</li>
				{/each}
			</ol>
		</section>
	{/each}
</div>

{#if expense}
	<ExpenseModal
		bind:open={editing}
		onsubmit={saved}
		userName={me}
		{expense}
		accounts={(feed?.accounts.list ?? []).map((a) => ({
			id: a.id.toString(),
			name: a.name,
			owner: { name: a.owner.name, firstName: a.owner.firstName },
			color: a.color
		}))}
		categories={(feed?.categories.list ?? []).map((c) => ({
			id: c.id.toString(),
			name: c.name,
			color: c.color
		}))}
	/>
{/if}
