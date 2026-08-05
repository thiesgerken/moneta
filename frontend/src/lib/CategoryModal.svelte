<script lang="ts">
	import { Modal, Label, Input, Textarea, Button, Select } from 'flowbite-svelte';
	import ColorPicker from 'svelte-awesome-color-picker';

	import { graphql } from '$houdini';
	import type { CategoryInput } from '$houdini/graphql/inputs';
	import { EditOutline, PlusOutline } from 'flowbite-svelte-icons';

	interface Props {
		open?: boolean;
		onsubmit?: () => Promise<void>; // used to signal that the form has been successfully submitted, need to invalidate cached stuff
		categories: {
			readonly id: string;
			readonly name: string;
			readonly description: string | null;
			readonly color: string | null;
			readonly parent: { readonly id: string } | null;
			readonly monthlyLimit: number | null;
		}[];
		category?: {
			readonly id: string;
			readonly name: string;
			readonly description: string | null;
			readonly color: string | null;
			readonly parent: { readonly id: string } | null;
			readonly monthlyLimit: number | null;
		};
	}
	let { open = $bindable(), categories, category, onsubmit }: Props = $props();

	let name = $derived(category?.name || '');
	let description = $derived(category?.description || '');
	let color = $derived(category?.color || '');
	let parentId = $derived(category?.parent?.id.toString() || '');
	let monthlyLimit: string | undefined = $derived(
		category?.monthlyLimit ? (category.monthlyLimit / 100).toFixed(2) : undefined
	);

	const nameError = $derived(!name.trim());
	const descriptionError = $derived(false);
	const colorError = $derived(false);
	const parentError = $derived(false);
	const monthlyLimitError = $derived(
		monthlyLimit !== undefined && (isNaN(Number(monthlyLimit)) || Number(monthlyLimit) < 0)
	);

	const formHasErrors = $derived(
		!!(nameError || descriptionError || colorError || parentError || monthlyLimitError)
	);

	const UpdateCategory = graphql(`
		mutation UpdateCategory($id: UUID!, $input: CategoryInput!) {
			categories {
				update(id: $id, category: $input)
			}
		}
	`);
	const CreateCategory = graphql(`
		mutation CreateCategory($input: CategoryInput!) {
			categories {
				create(category: $input) {
					id
				}
			}
		}
	`);

	const handleSubmit = async (event: Event) => {
		event.preventDefault();
		if (formHasErrors) return;

		const input: CategoryInput = {
			name: name.trim(),
			description: description.trim(),
			color: color.trim() || null,
			parentId: parentId || null,
			monthlyLimit: monthlyLimit ? Number(monthlyLimit) * 100 : null
		};

		console.log('category', category);
		console.log('update', input);

		if (category) await UpdateCategory.mutate({ id: category.id, input });
		else await CreateCategory.mutate({ input });

		if (onsubmit) await onsubmit();
		open = false;
	};

	// Filter out the current category from parent options to prevent circular references
	const availableParents = $derived(
		categories.filter((c) => !category || c.id !== category.id.toString())
	);
</script>

<Modal title={category ? 'Kategorie bearbeiten' : 'Neue Kategorie'} bind:open size="lg">
	<form onsubmit={handleSubmit}>
		<div class="mb-4 grid gap-4 sm:grid-cols-2">
			<div class="sm:col-span-2">
				<Label for="name" class="mb-2">Name</Label>
				<Input
					id="name"
					placeholder="Name"
					bind:value={name}
					color={nameError ? 'red' : undefined}
					required
				/>
			</div>
			<div class="sm:col-span-2">
				<Label for="description" class="mb-2">Beschreibung</Label>
				<Textarea
					id="description"
					clearable
					placeholder="Beschreibung (optional)"
					rows={description ? 3 : 1}
					bind:value={description}
					color={descriptionError ? 'red' : undefined}
				/>
			</div>
			<div>
				<Label for="color" class="mb-2">Farbe</Label>
				<div class="flex gap-2">
					<Input
						id="color"
						placeholder="#FF0000"
						bind:value={color}
						color={colorError ? 'red' : undefined}
						class="flex-1"
					/>

					<ColorPicker bind:hex={color} position="fixed" label="" />
				</div>
			</div>
			<div>
				<Label for="monthlyLimit" class="mb-2">Monatliches Limit</Label>
				<div class="relative">
					<Input
						id="monthlyLimit"
						placeholder=""
						bind:value={monthlyLimit}
						step="0.01"
						class="no-spinner pr-6"
						color={monthlyLimitError ? 'red' : undefined}
					/>
					<span class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-gray-500"
						>€</span
					>
				</div>
			</div>
			<div class="sm:col-span-2">
				<Label for="parent" class="mb-2">Übergeordnete Kategorie</Label>
				<Select
					items={[
						{ value: '', name: 'Keine' },
						...availableParents.map((c) => ({
							value: c.id,
							name: c.name
						}))
					]}
					bind:value={parentId}
					selectClass={parentError ? 'text-red-700 border-red-700 bg-red-100' : undefined}
					placeholder=""
				/>
			</div>

			<div class="justify-center sm:col-span-2">
				<Button type="submit" class="w-full" disabled={formHasErrors}>
					{#if category}
						<EditOutline class="mr-1 h-4 w-4" />
						Aktualisieren
					{:else}
						<PlusOutline class="mr-1 h-4 w-4" />
						Erstellen
					{/if}
				</Button>
			</div>
		</div>
	</form>
</Modal>

<style>
	:global(.no-spinner::-webkit-outer-spin-button),
	:global(.no-spinner::-webkit-inner-spin-button) {
		-webkit-appearance: none;
		margin: 0;
	}
	:global(.no-spinner) {
		-moz-appearance: textfield;
		appearance: textfield;
	}
</style>
