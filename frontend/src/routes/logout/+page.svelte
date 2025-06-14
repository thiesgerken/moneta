<script lang="ts">
	import { graphql } from '$houdini';

	async function logout(event: Event) {
		event.preventDefault();

		console.log('logout');

		const logoutMutation = graphql(`
			mutation Logout {
				users {
					logout
				}
			}
		`);

		await logoutMutation.mutate(null);
		window.location.href = '/';
	}
</script>

<form onsubmit={logout}>
	<button type="submit">Logout</button>
</form>
