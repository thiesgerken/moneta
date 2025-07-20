<script lang="ts">
	import { graphql } from '$houdini';
	import { fail } from '@sveltejs/kit';
	import { Section, Register } from 'flowbite-svelte-blocks';
	import { Button, Label, Input } from 'flowbite-svelte';

	async function login(event: Event) {
		event.preventDefault();

		const username = (event.target as HTMLFormElement).username.value;
		const password = (event.target as HTMLFormElement).password.value;

		if (!username || !password) {
			return fail(403);
		}

		const loginMutation = graphql(`
			mutation Login($name: String!, $password: String!) {
				users {
					login(name: $name, password: $password) {
						id
					}
				}
			}
		`);

		await loginMutation.mutate({ name: username, password });
		window.location.href = '/';
	}
</script>

<Section name="login" class="">
	<div class="space-y-4 p-6 sm:p-8 md:space-y-6">
		<form class="flex flex-col space-y-6" onsubmit={login}>
			<h3 class="p-0 text-xl font-medium text-gray-900 dark:text-white">Login</h3>
			<Label class="space-y-2">
				<span>Benutzername</span>
				<Input type="text" name="username" placeholder="Benutzername" required />
			</Label>
			<Label class="space-y-2">
				<span>Passwort</span>
				<Input type="password" name="password" placeholder="Passwort" required />
			</Label>

			<Button type="submit" class="w-full1">Anmelden</Button>
		</form>
	</div>
</Section>
