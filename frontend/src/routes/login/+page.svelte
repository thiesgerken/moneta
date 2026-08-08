<script lang="ts">
	import { graphql } from '$houdini';
	import { Section } from 'flowbite-svelte-blocks';
	import { Button, Label, Input, Helper } from 'flowbite-svelte';

	let error = $state<string | null>(null);
	let busy = $state(false);

	const loginMutation = graphql(`
		mutation Login($name: String!, $password: String!) {
			users {
				login(name: $name, password: $password) {
					id
				}
			}
		}
	`);

	async function login(event: Event) {
		event.preventDefault();

		const form = event.target as HTMLFormElement;
		const username = form.username.value;
		const password = form.password.value;
		if (!username || !password) return;

		error = null;
		busy = true;

		try {
			// A rejected login used to leave here for the dashboard anyway, where the session guard
			// turned it straight back around -- so a mistyped password looked like a broken app
			// rather than like a mistyped password.
			const result = await loginMutation.mutate({ name: username, password });
			if (result.errors?.length || !result.data?.users.login) {
				error = 'Benutzername oder Passwort stimmt nicht.';
				return;
			}
		} catch {
			error = 'Der Login hat nicht geklappt. Bist du online?';
			return;
		} finally {
			busy = false;
		}

		window.location.href = '/';
	}
</script>

<Section name="login" class="">
	<div class="space-y-4 p-6 sm:p-8 md:space-y-6">
		<form class="flex flex-col space-y-6" onsubmit={login}>
			<h3 class="p-0 text-xl font-medium text-gray-900 dark:text-white">Login</h3>
			<Label class="space-y-2">
				<span>Benutzername</span>
				<!-- A phone keyboard capitalises the first letter of a plain text field and offers a
				     correction for the rest of it, both of which produce a name nobody has. -->
				<Input
					type="text"
					name="username"
					placeholder="Benutzername"
					autocomplete="username"
					autocapitalize="none"
					autocorrect="off"
					spellcheck="false"
					required
				/>
			</Label>
			<Label class="space-y-2">
				<span>Passwort</span>
				<Input
					type="password"
					name="password"
					placeholder="Passwort"
					autocomplete="current-password"
					required
				/>
			</Label>

			{#if error}
				<Helper color="red">{error}</Helper>
			{/if}

			<Button type="submit" disabled={busy}>Anmelden</Button>
		</form>
	</div>
</Section>
