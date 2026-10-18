<script lang="ts">
	import { graphql, type ChangelogAuthor$options, type ExpenseChangelog$result } from '$houdini';
	import { Badge, Spinner } from 'flowbite-svelte';
	import {
		CirclePlusOutline,
		EditOutline,
		ChevronDownOutline,
		ChevronRightOutline
	} from 'flowbite-svelte-icons';
	import ToggleGroup from './ToggleGroup.svelte';
	import ExpenseModal from './ExpenseModal.svelte';
	import { changedFields, runs, summarizeFields, stateAmount } from './changelog';
	import ChangeList from './ChangeList.svelte';
	import { invalidate } from './invalidate';
	import { formatAccount } from './expense_utils';
	import { categoryColor } from './analysis/format';
	import { feedState } from './analysis/state.svelte';
	import { shortDate } from './analysis/dates';
	import QueryError from './QueryError.svelte';
	import { euro, restingAmountColor } from '#lib/money.ts';

	/// The dashboard shows the same feed in a column beside everything else, where the filters
	/// belong to the tab that owns them and six entries are as many as fit.
	let {
		limit = 100,
		controls = true,
		author,
		since
	}: {
		limit?: number;
		controls?: boolean;
		author?: ChangelogAuthor$options;
		/// only read where the controls are hidden; with them, the reader's own choice wins
		since?: Date | null;
	} = $props();

	const shownAuthor = $derived(author ?? (feedState.author as ChangelogAuthor$options));

	const DAY = 86400000;

	/// How far back the feed reaches. "Alles" is null rather than a very old date, so the query
	/// is the same one it was before this filter existed.
	const SPANS = [
		{ value: '7', label: '7 Tage' },
		{ value: '30', label: '30 Tage' },
		{ value: '365', label: 'Ein Jahr' },
		{ value: '', label: 'Alles' }
	];

	// a day boundary rather than "now minus n days", so the window does not creep while the
	// page is open and the entries of the earliest day are all there or all gone
	const spanSince = $derived.by(() => {
		const days = Number(feedState.span);
		if (!feedState.span || !Number.isFinite(days)) return null;

		const today = new Date();
		const midnight = new Date(today.getFullYear(), today.getMonth(), today.getDate());
		return new Date(midnight.getTime() - (days - 1) * DAY);
	});

	const cutoff = $derived(controls ? spanSince : (since ?? null));

	let expandedDiffs = $state<string[]>([]);

	const changelogStore = graphql(`
		query ExpenseChangelog($author: ChangelogAuthor!, $limit: Int!, $since: DateTime) {
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
					closed
					usageCount
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
					usageCount
				}
			}

			expenses {
				changelog(limit: $limit, skip: 0, author: $author, since: $since) {
					edges {
						node {
							id
							expenseId
							date
							tool
							automatic
							changes {
								path
								old
								new
							}
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
						bookingDate
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
			variables: { author: shownAuthor, limit, since: cutoff }
		});
		if (request === latestRequest) feed = result.data ?? null;
	}

	$effect(() => {
		void shownAuthor;
		void cutoff;
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
		invalidate();
		await load();
	}

	const me = $derived(feed?.users.me?.name);

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
	type Run = { key: string; entries: Entry[] };

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

		return out.map(({ key, label, entries }) => ({ key, label, runs: runs(entries) }));
	});

	/// When a run happened, for the tooltip on its author: one time, or the span of the edits
	/// that were merged into the row.
	function when(run: Run): string {
		const at = (d: Date) => d.toLocaleString('de-DE', { dateStyle: 'full', timeStyle: 'short' });
		const newest = run.entries[0].date;
		const oldest = run.entries.at(-1)!.date;

		if (run.entries.length === 1) return at(newest);
		return `${run.entries.length} Änderungen, ${at(oldest)} bis ${newest.toLocaleTimeString('de-DE', { timeStyle: 'short' })}`;
	}

	/// Dative, since it always follows "von": "von dir", "von Krösus".
	const actor = (entry: Entry) =>
		entry.user?.name === me ? 'dir' : (entry.user?.firstName ?? entry.user?.name ?? 'jemandem');

	/// Every change a run made after the expense existed, oldest first, which is the order they
	/// were made in. A creation is left out: its diff is every field of a new expense against
	/// nothing, which is the row itself written out longhand.
	const changesOf = (run: Run) =>
		run.entries
			.toReversed()
			.filter((e) => e.oldState)
			.flatMap((e) => e.changes);

	/// What the run did, in one phrase. Naming the fields that moved beats "geändert" on every
	/// single row. Capitalized because it opens the line, and the field names it is built from
	/// are nouns anyway.
	///
	/// A run that begins with a creation is one expense arriving, corrections and all: what a
	/// field was set to on the way to what it is now is a detail of that, not a second event.
	function action(run: Run): string {
		if (run.entries.some((e) => !e.oldState)) {
			return run.entries.length > 1 ? 'Angelegt und geändert' : 'Angelegt';
		}

		const fields = summarizeFields(changedFields(changesOf(run)));
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

	/// The same names the chips carry, for the change list: a changed account is an id in the
	/// stored state and a name everywhere a reader looks.
	const changeNames = $derived(Object.fromEntries(named.map((e) => [e.id, e.name])));

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
		const full = (d: Date) => shortDate(d);

		return state.accrualStart.getTime() === state.accrualEnd.getTime()
			? full(state.accrualStart)
			: `${short(state.accrualStart)}–${full(state.accrualEnd)}`;
	}

	const toggleDiff = (id: string) =>
		(expandedDiffs = expandedDiffs.includes(id)
			? expandedDiffs.filter((other) => other !== id)
			: [...expandedDiffs, id]);
</script>

<div class={controls ? 'mx-auto max-w-3xl' : ''}>
	{#if controls}
		<div class="flex flex-wrap items-center gap-x-3 gap-y-2 pb-2">
			<!-- the same words the entries use, which say "von dir" and "von Krösus"; the default
			     leads, so the row reads as everything first and then two ways to narrow it -->
			<ToggleGroup
				bind:value={feedState.author}
				size="sm"
				options={[
					{ value: 'ANYONE', label: 'Alle' },
					{ value: 'ME', label: 'Von mir' },
					{ value: 'OTHERS', label: 'Von anderen' }
				]}
			/>
			<ToggleGroup bind:value={feedState.span} size="sm" options={SPANS} />
			{#if $changelogStore.fetching}
				<Spinner size="4" />
			{/if}
		</div>
	{/if}

	<QueryError
		errors={$expenseStore.errors}
		message="Die Ausgabe konnte nicht geöffnet werden."
		class="my-4"
	/>

	{#if $changelogStore.errors}
		<QueryError
			errors={$changelogStore.errors}
			message="Die Änderungen konnten nicht geladen werden."
			class="my-4"
		/>
	{:else if $changelogStore.fetching && entries.length === 0}
		<div class="flex justify-center py-12"><Spinner size="6" /></div>
	{:else if entries.length === 0}
		<p class="py-12 text-center text-sm text-gray-500 dark:text-gray-400">
			{#if cutoff}
				In diesem Zeitraum ist nichts passiert.
			{:else if shownAuthor === 'OTHERS'}
				Niemand sonst hat etwas geändert.
			{:else}
				Noch keine Aktivität.
			{/if}
		</p>
	{/if}

	{#each days as group (group.key)}
		<section class="mb-6">
			<h2
				class="sticky top-0 z-10 bg-white/85 py-2 text-xs font-semibold tracking-wide text-gray-500 uppercase backdrop-blur dark:bg-gray-900/85 dark:text-gray-400"
			>
				{group.label}
			</h2>

			<!--
				One thread per day. The badges were already a column of dots beside the entries;
				running a line through them is what makes the day read as a sequence rather than as
				a list that happens to have icons. The rule stops at the day header on purpose --
				the header is the break.

				Not flowbite's Timeline: its group variant is this markup in a `p-5` card with no
				line at all, and its item variants pair a `text-lg` heading with 40px of margin.
				A badge over a `border-s` is the whole component at this size.
			-->
			<ol class="relative ms-3.5 border-s border-gray-300 dark:border-gray-600">
				{#each group.runs as run (run.key)}
					<!-- The newest of a run is what the row is: the title, the amount and the chips
					     are what the expense is now, and the older entries in it are how it got
					     there. `created` covers the whole run -- a creation corrected twice is one
					     expense arriving, and a pencil over it would say otherwise. -->
					{@const entry = run.entries[0]}
					{@const created = run.entries.some((e) => !e.oldState)}
					{@const changes = changesOf(run)}
					{@const amount = entry.newState ? stateAmount(entry.newState) : null}
					<li class="relative py-2.5 ps-6">
						<!-- centred on the rule by its own half-width, so the badge can change size
						     without the offset having to be worked out again. Its background is
						     opaque, which is what hides the rule behind it. -->
						<span
							class="absolute top-3.5 left-0 flex h-7 w-7 -translate-x-1/2 items-center justify-center rounded-full {created
								? 'bg-green-100 text-green-600 dark:bg-green-900/40 dark:text-green-400'
								: 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400'}"
						>
							{#if created}
								<CirclePlusOutline class="h-4 w-4" />
							{:else}
								<EditOutline class="h-4 w-4" />
							{/if}
						</span>

						<div class="min-w-0">
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
										class="shrink-0 text-sm font-medium tabular-nums {restingAmountColor(amount)}"
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
									{action(run)} von
									<span
										class="underline decoration-gray-300 decoration-dotted dark:decoration-gray-600"
										title={when(run)}>{actor(entry)}</span
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
								<!-- Not for a creation on its own: its changes are the expense itself. A
								     creation that was then corrected has both, and the corrections are the
								     part that is worth opening. -->
								{#if changes.length > 0}
									<button
										class="ml-auto flex items-center gap-1 text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
										onclick={() => toggleDiff(run.key)}
									>
										{#if expandedDiffs.includes(run.key)}
											<ChevronDownOutline class="h-3 w-3" />
										{:else}
											<ChevronRightOutline class="h-3 w-3" />
										{/if}
										Details
									</button>
								{/if}
							</div>

							{#if changes.length > 0 && expandedDiffs.includes(run.key)}
								<ChangeList
									{changes}
									names={changeNames}
									class="mt-2 rounded-md bg-gray-50 p-2 dark:bg-gray-800/60"
								/>
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
			color: a.color,
			closed: a.closed,
			usageCount: a.usageCount
		}))}
		categories={(feed?.categories.list ?? []).map((c) => ({
			id: c.id.toString(),
			name: c.name,
			color: c.color,
			usageCount: c.usageCount
		}))}
	/>
{/if}
