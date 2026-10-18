<script lang="ts">
	import { graphql } from '$houdini';
	import { version } from '$app/environment';
	import { Navbar, NavBrand, NavHamburger, NavUl, NavLi, Avatar, Dropdown } from 'flowbite-svelte';
	import { ArrowRightToBracketOutline } from 'flowbite-svelte-icons';
	import { NAV } from '$lib/navigation';
	import Brand from '$lib/Brand.svelte';

	// No error surface, deliberately: a bar across the top of every page is the wrong place
	// to report one, and the only thing lost is the menu. An ended session is the layout's job.
	const basicStore = graphql(`
		query UserInformation {
			users {
				me {
					name
					fullName
					firstName
					lastName
				}
			}
		}
	`);

	$effect(() => void basicStore.fetch());

	let open = $state(false);
	let menuOpen = $state(false);

	const me = $derived($basicStore.data?.users.me);
	const initials = $derived(`${me?.firstName?.[0] ?? ''}${me?.lastName?.[0] ?? ''}`);

	// ponytail: no version string of our own. sveltekit already stamps every build -- it is what
	// the layout watches to notice a deploy -- and its default is the build's `Date.now()`. That
	// answers the only question this line is here for, "am I looking at the deploy I just made",
	// without a build arg to thread through the Dockerfile and CI. Non-numeric if anyone ever
	// sets `kit.version.name` by hand, hence the fallback.
	const build = $derived.by(() => {
		const stamp = Number(version);
		if (!Number.isFinite(stamp)) return version;

		// short, so that the stamp and the commit stay on one line of the menu
		return new Date(stamp).toLocaleString('de-DE', { dateStyle: 'short', timeStyle: 'short' });
	});

	// empty outside a CI build, where nothing handed the commit in -- see vite.config.ts
	const commit = __GIT_SHA__.slice(0, 7);

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
		<Brand class="h-8 w-auto" />
	</NavBrand>
	<!-- The open state is ours rather than flowbite's: its hamburger and its list talk to each
	     other through a context the two never share here, so the menu stayed shut on phones. -->
	<NavHamburger onclick={() => (open = !open)} />
	<!-- md:p-0 is what the bar's height came from: flowbite's own `p-4` is there for the stacked
	     phone menu and it kept padding the row on a desktop too, where the list is one line -->
	<!-- md:items-center: the list stretches its children, and the avatar is the one child with a
	     height of its own, so it hung from the top of a row the labels had made taller. -->
	<NavUl class={open ? 'block' : ''} classes={{ ul: 'md:items-center md:gap-6 md:p-0' }}>
		{#if me}
			{#each NAV as { href, label } (href)}
				<!-- a tap navigates without a page load, which would otherwise leave the menu
				     standing open over the page it just took you to -->
				<NavLi {href} onclick={() => (open = false)}>{label}</NavLi>
			{/each}
			<button type="button" data-user-menu class="flex cursor-pointer" aria-label="Konto">
				<Avatar size="sm">{initials}</Avatar>
			</button>
		{:else}
			<NavLi href="/login">Login</NavLi>
		{/if}
	</NavUl>
</Navbar>

{#if me}
	<!-- outside the bar rather than next to the avatar: the trigger sits in the navbar's `ul`,
	     and the dropdown's own panel is not a list item. -->
	<Dropdown
		bind:isOpen={menuOpen}
		triggeredBy="[data-user-menu]"
		placement="bottom-end"
		class="w-60 overflow-hidden p-0"
	>
		<div class="px-4 py-3">
			<div class="truncate text-sm font-medium text-gray-900 dark:text-white">{me.fullName}</div>
			<div class="truncate text-xs text-gray-500 dark:text-gray-400">{me.name}</div>
			<!-- flex rather than one run of text: svelte eats the whitespace either side of a
			     block, so the separator would sit against the stamp -->
			<div class="mt-2 flex gap-1 text-xs text-gray-400 dark:text-gray-500">
				<span>Build {build}</span>
				{#if commit}
					<span>·</span>
					<a
						href="https://gitea.gerken.haus/thies/moneta/commit/{__GIT_SHA__}"
						target="_blank"
						rel="noreferrer"
						class="hover:underline">#{commit}</a
					>
				{/if}
			</div>
		</div>
		<button
			type="button"
			onclick={() => {
				menuOpen = false;
				logout();
			}}
			class="flex w-full cursor-pointer items-center gap-2 border-t border-gray-100 px-4 py-2.5 text-sm text-gray-700 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-600"
		>
			<ArrowRightToBracketOutline class="h-4 w-4" />
			Abmelden
		</button>
	</Dropdown>
{/if}
