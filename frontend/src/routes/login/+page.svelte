<script lang="ts">
	import { graphql } from '$houdini';
	import { fail } from '@sveltejs/kit';

	async function login(event: Event) {
		event.preventDefault();

		console.log('login');

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
		window.location.href = '/about';
	}
</script>

<form onsubmit={login}>
	<input type="text" name="username" placeholder="Username" />
	<input type="password" name="password" placeholder="Password" />
	<button type="submit">Login</button>
</form>
