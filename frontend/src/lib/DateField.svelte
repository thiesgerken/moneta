<!--
@component
A date field, the way this app writes dates: `22.04.2026`, weeks starting on Monday, and a
calendar that hangs out of whatever is scrolling around it.

It exists because the same four props were repeated at every picker and two of them had
been forgotten, so the account and balance modals were writing `5.8.2026` while the expense
modal wrote `05.08.2026`.
-->
<script lang="ts">
	import { Button, Datepicker } from 'flowbite-svelte';
	import { today } from './analysis/dates';
	import { unclipped } from './unclipped';

	interface Props {
		value: Date | undefined;
		placeholder?: string;
		inputClass?: string;
		/// `hidden` where the column is too narrow for the calendar icon to sit beside the year
		btnClass?: string;
		/// whether the calendar offers to empty the field again, for a date that is optional
		clearable?: boolean;
	}

	let {
		value = $bindable(),
		placeholder = 'Datum',
		inputClass = '',
		btnClass,
		clearable = false
	}: Props = $props();

	/// flowbite's own default is `numeric`, which gives `5.8.2026`: the day, the month and the
	/// year each start somewhere different from one row to the next.
	const DATE_FORMAT: Intl.DateTimeFormatOptions = {
		day: '2-digit',
		month: '2-digit',
		year: 'numeric'
	};
</script>

<div use:unclipped>
	<Datepicker
		bind:value
		{placeholder}
		{inputClass}
		{btnClass}
		locale="de-DE"
		firstDayOfWeek={1}
		dateFormat={DATE_FORMAT}
	>
		<!-- In the calendar rather than under the field: a link that appears only once a date is
		     set makes the row taller the moment you pick one, and every other row jumps.
		     flowbite's own action buttons say Today/Clear/Apply in English, so this is the slot
		     it offers for saying it in German instead. -->
		{#snippet actionSlot({ close }: { close: () => void })}
			<div class="flex justify-end gap-2 px-3 pb-3">
				{#if clearable}
					<Button
						size="xs"
						color="light"
						onclick={() => {
							value = undefined;
							close();
						}}
					>
						Datum entfernen
					</Button>
				{/if}
				<Button
					size="xs"
					color="light"
					onclick={() => {
						value = today();
						close();
					}}
				>
					Heute
				</Button>
			</div>
		{/snippet}
	</Datepicker>
</div>
