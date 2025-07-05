<script lang="ts">
	import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';

	interface Props {
		open?: boolean;

		user_name: string | undefined;
		accounts: {
			readonly id: String;
			readonly name: String;
			readonly color: string | null;
			readonly owner: { readonly name: String; readonly firstName: string | null };
		}[];
		categories: {
			readonly id: String;
			readonly name: String;
			readonly color: string | null;
		}[];
	}
	let { open = $bindable(), accounts, categories, user_name }: Props = $props();

	const handleSubmit = () => {
		console.log('submit');
	};

	import { graphql } from '$houdini';

	//     mutation xx {
	//   expenses {
	//     create(
	//       expense: {title: "sss", store: "s", description: "", comments: "", referenceAmount: 0, bookingStart: "2025-01-01T00:00:00Z", bookingEnd: "2025-01-01T00:00:00Z", isTemplate: false, isUnchecked: false, isExtraordinary: false, isTaxRelevant: false, isIncomplete: false, receipts: [], categories: [{categoryId: 4, weight: 1}], transactions: [{date: "00", accountId: 2, comments: "", statement: ""}], positions: [{units: 1, amount: 300, title: "", comments: ""}]}
	//     ) {
	//       id
	//     }
	//   }
	// }

	const CreateExpense = graphql(`
		mutation CreateExpense($input: ExpenseInput!) {
			expenses {
				create(expense: $input) {
					id
				}
			}
		}
	`);
</script>

<Modal title="Neue Ausgabe" bind:open autoclose>
	<form onsubmit={handleSubmit}>
		<div class="mb-4 grid gap-4 sm:grid-cols-2">
			<div>
				<Label for="name" class="mb-2">Name</Label>
				<Input type="text" id="name" placeholder="Type product name" required />
			</div>
			<div>
				<Label for="brand" class="mb-2">Brand</Label>
				<Input type="text" id="brand" placeholder="Product brand" required />
			</div>
			<div>
				<Label for="price" class="mb-2">Price</Label>
				<Input type="text" id="price" placeholder="$29999" required />
			</div>
			<!-- <div>
				<Label
					>Category
					<Select class="mt-2" items={countries} bind:value={selected} required />
				</Label>
			</div> -->
			<div class="sm:col-span-2">
				<Label for="description" class="mb-2">Description</Label>
				<Textarea
					id="description"
					placeholder="Your description here"
					rows={4}
					name="description"
					required
				/>
			</div>
			<Button type="submit" class="w-52">
				<svg
					class="-ml-1 mr-1 h-6 w-6"
					fill="currentColor"
					viewBox="0 0 20 20"
					xmlns="http://www.w3.org/2000/svg"
					><path
						fill-rule="evenodd"
						d="M10 5a1 1 0 011 1v3h3a1 1 0 110 2h-3v3a1 1 0 11-2 0v-3H6a1 1 0 110-2h3V6a1 1 0 011-1z"
						clip-rule="evenodd"
					/></svg
				>
				Add new product
			</Button>
		</div>
	</form>
</Modal>
