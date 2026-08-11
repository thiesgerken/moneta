<script lang="ts">
	import { graphql, cache } from '$houdini';
	import ExpenseModal from './ExpenseModal.svelte';

	interface Props {
		/// the expense to show; setting it opens the modal, closing it clears it again
		expenseId: string | undefined;
		/// called after a successful edit, so the caller can reload what it was showing
		onsubmit?: () => Promise<void>;
	}

	let { expenseId = $bindable(), onsubmit }: Props = $props();

	// The lists an expense is edited against. The expense table asks for the same thing; sharing
	// one query document would tie the two components' fetches together for no gain, since
	// houdini caches the result either way.
	const basicStore = graphql(`
		query ExpenseDetailBasic {
			users {
				me {
					name
				}
			}
			accounts {
				list {
					id
					name
					owner {
						name
						firstName
					}
					color
					closed
					usageCount
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
		}
	`);

	const expenseStore = graphql(`
		query ExpenseDetail($id: UUID!) {
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

	$effect(() => void basicStore.fetch());

	$effect(() => {
		if (expenseId) void expenseStore.fetch({ variables: { id: expenseId } });
	});

	// Only the expense that was actually asked for: while a second one loads the store still
	// holds the first, and handing that to the form would show the wrong expense for a moment.
	const loaded = $derived($expenseStore.data?.expenses.get);
	const expense = $derived(loaded?.id === expenseId ? loaded : undefined);

	// `ExpenseModal` with no expense is the form for creating one, so the modal may not open
	// before the query has answered. Writable, since the modal closes itself.
	let open = $derived(expense !== undefined);
	$effect(() => {
		if (!open) expenseId = undefined;
	});

	async function handleSubmit() {
		cache.markStale('Expense');
		await onsubmit?.();
	}
</script>

{#if expense}
	<ExpenseModal
		bind:open
		{expense}
		onsubmit={handleSubmit}
		userName={$basicStore.data?.users.me?.name}
		accounts={$basicStore.data?.accounts.list.map((a) => ({
			id: a.id.toString(),
			name: a.name,
			owner: { name: a.owner.name, firstName: a.owner.firstName },
			color: a.color,
			closed: a.closed,
			usageCount: a.usageCount
		})) || []}
		categories={$basicStore.data?.categories.list.map((c) => ({
			id: c.id.toString(),
			name: c.name,
			color: c.color,
			usageCount: c.usageCount
		})) || []}
	/>
{/if}
