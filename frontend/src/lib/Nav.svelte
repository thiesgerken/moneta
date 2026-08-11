<script lang="ts">
	import { graphql } from '$houdini';
	import { Navbar, NavBrand, NavHamburger, NavUl, NavLi, Avatar, Tooltip } from 'flowbite-svelte';
	import { NAV } from '$lib/navigation';

	// No error surface, deliberately: a bar across the top of every page is the wrong place
	// to report one, and the only thing lost is the menu. An ended session is the layout's job.
	const basicStore = graphql(`
		query UserInformation {
			users {
				me {
					name
					firstName
					lastName
				}
			}
		}
	`);

	$effect(() => void basicStore.fetch());

	let open = $state(false);

	async function logout() {
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

<!-- fluid, and the same gutter the pages use: the bar centred itself in a fixed container while
     every page under it runs full width, which left the brand adrift from the tab strip below -->
<!-- py-1.5 and a smaller wordmark: 92px of bar stood between the window and the first row of
     every list in the app, and none of it was information. -->
<Navbar fluid class="px-4 py-1.5 sm:px-8">
	<NavBrand href="/">
		<img src="/logo.svg" class="me-2.5 h-7" alt="" />
		<!-- the wordmark is the one place the display face is the point rather than a heading -->
		<span class="font-display self-center text-xl font-semibold whitespace-nowrap dark:text-white">
			Moneta
		</span>
	</NavBrand>
	<!-- The open state is ours rather than flowbite's: its hamburger and its list talk to each
	     other through a context the two never share here, so the menu stayed shut on phones. -->
	<NavHamburger onclick={() => (open = !open)} />
	<!-- md:p-0 is what the bar's height came from: flowbite's own `p-4` is there for the stacked
	     phone menu and it kept padding the row on a desktop too, where the list is one line -->
	<NavUl class={open ? 'block' : ''} classes={{ ul: 'md:gap-6 md:p-0' }}>
		{#if $basicStore.data?.users.me}
			{#each NAV as { href, label } (href)}
				<!-- a tap navigates without a page load, which would otherwise leave the menu
				     standing open over the page it just took you to -->
				<NavLi {href} onclick={() => (open = false)}>{label}</NavLi>
			{/each}
			<!-- <NavLi onclick={logout} href="/">Logout</NavLi> -->
			<Avatar size="sm" class="mr-0 ml-5" onclick={logout} href="/"
				>{$basicStore.data?.users.me?.firstName?.[0]}{$basicStore.data?.users.me
					?.lastName?.[0]}</Avatar
			>
			<Tooltip>Logout</Tooltip>
		{:else}
			<NavLi href="/login">Login</NavLi>
		{/if}
	</NavUl>
</Navbar>
