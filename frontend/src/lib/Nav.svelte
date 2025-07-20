<script lang="ts">
	import { graphql } from '$houdini';
	import { Navbar, NavBrand, NavHamburger, NavUl, NavLi, Avatar, Tooltip } from 'flowbite-svelte';

	const basicStore = graphql(`
		query UserInformation @load {
			users {
				me {
					name
					firstName
					lastName
				}
			}
		}
	`);

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

<Navbar>
	<NavBrand href="/">
		<!-- <img src="/images/flowbite-svelte-icon-logo.svg" class="me-3 h-6 sm:h-9" alt="Flowbite Logo" /> -->
		<span class="self-center whitespace-nowrap text-xl font-semibold dark:text-white">Moneta</span>
	</NavBrand>
	<NavHamburger />
	<NavUl>
		<NavLi href="/">Home</NavLi>
		<NavLi href="/expenses">Expenses</NavLi>
		{#if $basicStore.data?.users.me}
			<!-- <NavLi onclick={logout} href="/">Logout</NavLi> -->
			<Avatar class="ml-5 mr-0" onclick={logout} href="/"
				>{$basicStore.data?.users.me?.firstName?.[0]}{$basicStore.data?.users.me
					?.lastName?.[0]}</Avatar
			>
			<Tooltip>Logout</Tooltip>
		{:else}
			<NavLi href="/login">Login</NavLi>
		{/if}
	</NavUl>
</Navbar>
