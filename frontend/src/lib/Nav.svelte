<script lang="ts">
	import { graphql } from '$houdini';
	import { Navbar, NavBrand, NavHamburger, NavUl, NavLi, Avatar, Tooltip } from 'flowbite-svelte';

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
		<span class="self-center text-xl font-semibold whitespace-nowrap dark:text-white">Moneta</span>
	</NavBrand>
	<NavHamburger />
	<NavUl>
		{#if $basicStore.data?.users.me}
			<NavLi href="/">Dashboard</NavLi>
			<NavLi href="/expenses">Ausgaben</NavLi>
			<NavLi href="/balances">Kontostände</NavLi>
			<NavLi href="/categories">Kategorien</NavLi>
			<NavLi href="/accounts">Konten</NavLi>
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
