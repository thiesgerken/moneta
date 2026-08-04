<script lang="ts">
	import ExpenseChangelog from '$lib/ExpenseChangelog.svelte';
	import { graphql } from '$houdini';
	import { Tabs, TabItem } from 'flowbite-svelte';

	const About = graphql(`
		query About {
			apiVersion
			users {
				me {
					name
					firstName
					lastName
				}
			}
		}
	`);

	$effect(() => {
		// decide on the resolved result rather than on store state: `users.me` errors
		// out when logged out, so the pre-fetch and logged-out states look identical
		About.fetch().then((result) => {
			if (!result.data?.users.me) window.location.href = '/login';
		});
	});
</script>

<div class="pr-8 pl-8">
	<Tabs tabStyle="underline">
		<TabItem open title="Aktivität">
			<ExpenseChangelog />
		</TabItem>
	</Tabs>
</div>
