<script lang="ts">
	import { graphql } from '$houdini';
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

<!-- the page is the whole window here, the bar being gone, so the form sits in the middle of it
     rather than under a corner it no longer has -->
<div class="mx-auto flex min-h-dvh w-full max-w-sm flex-col justify-center px-6">
	<div class="mb-8 flex items-center justify-center">
		<img src="/logo.svg" class="me-2.5 h-9" alt="" />
		<span class="font-display text-2xl font-semibold whitespace-nowrap dark:text-white">
			Moneta
		</span>
	</div>

	<!-- no heading: the wordmark above the two fields says what this is -->
	<form class="flex flex-col space-y-6" onsubmit={login}>
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
