<script lang="ts">
	import { graphql } from '$houdini';
	import { Alert, Badge, Button, Input, Label, Modal, Select } from 'flowbite-svelte';
	import { shortDate } from '$lib/analysis/dates';

	const keys = graphql(`
		query ApiKeys {
			apiKeys {
				list {
					id
					name
					prefix
					access
					createdAt
					expiresAt
					lastUsedAt
				}
			}
		}
	`);
	const createKey = graphql(`
		mutation CreateApiKey($name: String!, $access: ApiKeyAccess!, $expiresAt: DateTime) {
			apiKeys {
				create(name: $name, access: $access, expiresAt: $expiresAt) {
					token
				}
			}
		}
	`);
	const revokeKey = graphql(`
		mutation RevokeApiKey($id: UUID!) {
			apiKeys {
				revoke(id: $id)
			}
		}
	`);

	$effect(() => {
		void keys.fetch();
	});
	let creating = $state(false);
	let name = $state('');
	let access = $state<'READ' | 'WRITE'>('READ');
	let expiry = $state('');
	let busy = $state(false);
	let error = $state('');
	let token = $state('');
	let copied = $state(false);
	let removing = $state<{ id: string; name: string } | null>(null);
	const expiryInvalid = $derived(
		expiry !== '' &&
			(!Number.isFinite(new Date(expiry).getTime()) || new Date(expiry).getTime() <= Date.now())
	);
	const list = $derived($keys.data?.apiKeys.list ?? []);

	function openCreate() {
		name = '';
		access = 'READ';
		expiry = '';
		error = '';
		creating = true;
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !name.trim() || expiryInvalid) return;
		busy = true;
		error = '';
		try {
			const result = await createKey.mutate({
				name: name.trim(),
				access,
				expiresAt: expiry ? new Date(expiry) : null
			});
			if (result.errors?.length || !result.data?.apiKeys.create.token) {
				error = 'Der Schlüssel konnte nicht erstellt werden. Prüfe Name und Ablaufdatum.';
				return;
			}
			token = result.data.apiKeys.create.token;
			copied = false;
			creating = false;
			await keys.fetch({ policy: 'NetworkOnly' });
		} catch {
			error = 'Der Schlüssel konnte nicht erstellt werden. Prüfe deine Verbindung.';
		} finally {
			busy = false;
		}
	}

	async function copy() {
		try {
			await navigator.clipboard.writeText(token);
			copied = true;
		} catch {
			error = 'Kopieren nicht möglich. Markiere und kopiere den Schlüssel manuell.';
		}
	}

	async function revoke() {
		if (!removing || busy) return;
		busy = true;
		error = '';
		try {
			const result = await revokeKey.mutate({ id: removing.id });
			if (result.errors?.length || !result.data?.apiKeys.revoke) {
				error = 'Der Schlüssel konnte nicht entfernt werden.';
				return;
			}
			removing = null;
			await keys.fetch({ policy: 'NetworkOnly' });
		} catch {
			error = 'Der Schlüssel konnte nicht entfernt werden. Prüfe deine Verbindung.';
		} finally {
			busy = false;
		}
	}
</script>

<main class="mx-auto max-w-7xl space-y-4 px-4 py-4 sm:px-8">
	<div class="flex items-center justify-between gap-4">
		<h1 class="text-2xl font-semibold text-gray-900 dark:text-white">API-Schlüssel</h1>
		<Button onclick={openCreate}>Neuer Schlüssel</Button>
	</div>
	<p class="text-sm text-gray-600 dark:text-gray-400">
		Verbinde Ferret oder andere Programme ohne dein Passwort. Für Ferret reicht „Nur lesen“.
		Schlüssel gelten unabhängig von deinem Passwort, bis sie ablaufen oder du sie entfernst.
	</p>
	{#if error}<Alert color="red">{error}</Alert>{/if}
	{#if $keys.errors?.length}
		<Alert color="red">Die Schlüssel konnten nicht geladen werden.</Alert>
	{:else if $keys.fetching && !$keys.data}
		<p class="text-sm text-gray-500">Schlüssel werden geladen …</p>
	{:else if list.length === 0}
		<p class="text-sm text-gray-500 dark:text-gray-400">Du hast noch keine API-Schlüssel.</p>
	{:else}
		<ul class="divide-y divide-gray-200 dark:divide-gray-700">
			{#each list as key (key.id)}
				<li class="flex flex-wrap items-start justify-between gap-4 py-4">
					<div class="min-w-0 space-y-2">
						<div class="flex flex-wrap items-center gap-2">
							<span class="font-medium text-gray-900 dark:text-white">{key.name}</span>
							<Badge color="gray"
								>{key.access === 'READ' ? 'Nur lesen' : 'Lesen und schreiben'}</Badge
							>
							{#if key.expiresAt && key.expiresAt <= new Date()}
								<Badge color="red">Abgelaufen</Badge>
							{/if}
						</div>
						<p class="font-mono text-sm text-gray-600 dark:text-gray-400">mk_{key.prefix}.…</p>
						<dl class="flex flex-wrap gap-x-6 gap-y-1 text-sm text-gray-600 dark:text-gray-400">
							<div>
								<dt class="inline">Erstellt:</dt>
								<dd class="inline">{shortDate(key.createdAt)}</dd>
							</div>
							<div>
								<dt class="inline">Ablauf:</dt>
								<dd class="inline">{key.expiresAt ? shortDate(key.expiresAt) : 'Kein Ablauf'}</dd>
							</div>
							<div>
								<dt class="inline">Zuletzt verwendet:</dt>
								<dd class="inline">
									{key.lastUsedAt ? key.lastUsedAt.toLocaleString('de-DE') : 'Noch nie'}
								</dd>
							</div>
						</dl>
					</div>
					<Button
						color="alternative"
						onclick={() => {
							removing = { id: key.id, name: key.name };
							error = '';
						}}>Entfernen</Button
					>
				</li>
			{/each}
		</ul>
	{/if}
</main>

<Modal title="Neuer API-Schlüssel" bind:open={creating} size="lg">
	{#if error}<Alert color="red">{error}</Alert>{/if}
	<form id="create-api-key" onsubmit={create} class="space-y-4">
		<div>
			<Label for="key-name" class="mb-2">Name</Label>
			<Input id="key-name" bind:value={name} placeholder="z. B. Ferret" required />
		</div>
		<div>
			<Label for="key-access" class="mb-2">Berechtigung</Label>
			<Select
				id="key-access"
				bind:value={access}
				items={[
					{ value: 'READ', name: 'Nur lesen (für Ferret ausreichend)' },
					{ value: 'WRITE', name: 'Lesen und schreiben' }
				]}
			/>
			<p class="mt-2 text-sm text-gray-500">
				Schreibzugriff erlaubt auch Änderungen und Löschungen in deinen Finanzdaten. Die
				Berechtigung kann später nicht geändert werden.
			</p>
		</div>
		<div>
			<Label for="key-expiry" class="mb-2">Ablaufdatum (optional)</Label>
			<Input
				id="key-expiry"
				type="datetime-local"
				bind:value={expiry}
				color={expiryInvalid ? 'red' : undefined}
			/>
			<p class="mt-2 text-sm text-gray-500">
				Leer lassen für keinen Ablauf. Ein Datum muss in der Zukunft liegen.
			</p>
		</div>
	</form>
	{#snippet footer()}
		<Button color="alternative" onclick={() => (creating = false)} disabled={busy}>Abbrechen</Button
		>
		<Button type="submit" form="create-api-key" disabled={busy || !name.trim() || expiryInvalid}
			>Erstellen</Button
		>
	{/snippet}
</Modal>

<Modal
	title="Schlüssel jetzt sichern"
	open={!!token}
	oncancel={() => {
		token = '';
		copied = false;
	}}
	size="lg"
>
	{#if error}<Alert color="red">{error}</Alert>{/if}
	<p class="mb-4 text-sm text-gray-600 dark:text-gray-400">
		Dieser Schlüssel wird nur einmal angezeigt. Kopiere ihn jetzt und bewahre ihn sicher auf. Später
		kannst du nur einen neuen erstellen.
	</p>
	<Label for="created-token" class="mb-2">Dein API-Schlüssel</Label>
	<Input id="created-token" value={token} readonly class="font-mono text-sm" />
	{#snippet footer()}
		<Button
			color="alternative"
			onclick={() => {
				token = '';
				copied = false;
			}}>Gesichert, schließen</Button
		>
		<Button onclick={copy}>{copied ? 'Kopiert' : 'Kopieren'}</Button>
	{/snippet}
</Modal>
<Modal
	title="API-Schlüssel entfernen?"
	open={!!removing}
	oncancel={() => (removing = null)}
	size="xs"
>
	{#if error}<Alert color="red">{error}</Alert>{/if}
	<p class="text-sm text-gray-600 dark:text-gray-400">
		„{removing?.name}“ wird endgültig gelöscht und funktioniert ab der nächsten Anfrage nicht mehr.
		Das kann nicht rückgängig gemacht werden.
	</p>
	{#snippet footer()}
		<Button color="alternative" onclick={() => (removing = null)} disabled={busy}>Abbrechen</Button>
		<Button color="red" onclick={revoke} disabled={busy}>Endgültig entfernen</Button>
	{/snippet}
</Modal>
