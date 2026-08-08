<script lang="ts">
	/// What a failed query looks like, everywhere.
	///
	/// A query that errors leaves its store's `data` null, and a table or a chart bound to that
	/// renders as though the answer were "nothing" -- which is a different statement from "we
	/// could not ask". Since the pool grew a wait timeout the difference matters: a request that
	/// used to hang forever now comes back as an error, and without this it would come back as
	/// an empty screen.
	interface Props {
		errors?: readonly { message: string }[] | null;
		/// The whole sentence, not a noun to build one from: German makes the verb agree with the
		/// subject, and "Die Übersicht konnten nicht geladen werden" is what composing it here got
		/// us. Saying which part of the page is missing is the point, so it is worth the repetition.
		message?: string;
		class?: string;
	}

	const {
		errors,
		message = 'Die Daten konnten nicht geladen werden.',
		class: className = ''
	}: Props = $props();
</script>

{#if errors?.length}
	<div
		role="alert"
		class="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/50 dark:text-red-300 {className}"
	>
		<p class="font-medium">{message}</p>
		<!-- the raw message rather than a friendly rewrite: it is the only thing that says whether
		     this was a timeout, a lost session or a bug, and it is what gets pasted into a report -->
		<p class="mt-1 text-red-600 dark:text-red-400">
			{errors.map((e) => e.message).join(', ')}
		</p>
	</div>
{/if}
