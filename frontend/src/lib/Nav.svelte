<script lang="ts">
	import { graphql } from '$houdini';
	import { Navbar, NavBrand, NavHamburger, NavUl, NavLi, Avatar, Tooltip } from 'flowbite-svelte';
	import { NAV } from '$lib/navigation';

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
<Navbar fluid class="px-4 sm:px-8">
	<NavBrand href="/">
		<img src="/logo.svg" class="me-2.5 h-7 sm:h-8" alt="" />
		<span class="self-center text-xl font-semibold whitespace-nowrap dark:text-white">Moneta</span>
	</NavBrand>
	<!-- The open state is ours rather than flowbite's: its hamburger and its list talk to each
	     other through a context the two never share here, so the menu stayed shut on phones. -->
	<NavHamburger onclick={() => (open = !open)} />
	<NavUl class={open ? 'block' : ''} classes={{ ul: 'md:gap-6' }}>
		{#if $basicStore.data?.users.me}
			{#each NAV as { href, label } (href)}
				<!-- a tap navigates without a page load, which would otherwise leave the menu
				     standing open over the page it just took you to -->
				<NavLi {href} onclick={() => (open = false)}>{label}</NavLi>
			{/each}
			<!-- <NavLi onclick={logout} href="/">Logout</NavLi> -->
			<Avatar class="mr-0 ml-5" onclick={logout} href="/"
				>{$basicStore.data?.users.me?.firstName?.[0]}{$basicStore.data?.users.me
					?.lastName?.[0]}</Avatar
			>
			<Tooltip>Logout</Tooltip>
		{:else}
			<NavLi href="/login">Login</NavLi>
		{/if}
	</NavUl>
</Navbar>
