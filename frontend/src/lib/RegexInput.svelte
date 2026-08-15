<script lang="ts">
	import { Helper } from 'flowbite-svelte';

	interface Props {
		value: string;
		placeholder?: string;
		/// what the pattern is tried against, so an unanchored `.` reads as what it does
		id?: string;
	}
	let { value = $bindable(), placeholder = '', id = undefined }: Props = $props();

	/// The message the engine itself gives, which names the position — far more use than
	/// "ungültig". An empty pattern is no pattern rather than a broken one.
	const error = $derived.by(() => {
		if (!value.trim()) return null;
		try {
			new RegExp(value);
			return null;
		} catch (e) {
			return e instanceof Error ? e.message : 'ungültiger Ausdruck';
		}
	});

	type Token = { text: string; kind: keyof typeof COLORS };

	const COLORS = {
		literal: '',
		escape: 'text-purple-600 dark:text-purple-400',
		class: 'text-amber-600 dark:text-amber-400',
		group: 'text-blue-600 dark:text-blue-400',
		anchor: 'text-rose-600 dark:text-rose-400',
		quantifier: 'text-emerald-600 dark:text-emerald-400'
	};

	/// Enough of a regex reader to colour one: what is structure and what is text you are
	/// matching on. Deliberately not a parser — it never has to reject anything, since the
	/// engine above already says whether the pattern is legal.
	function tokenize(src: string): Token[] {
		const out: Token[] = [];
		let literal = '';

		const push = (text: string, kind: Token['kind']) => {
			if (literal) {
				out.push({ text: literal, kind: 'literal' });
				literal = '';
			}
			out.push({ text, kind });
		};

		for (let i = 0; i < src.length; i++) {
			const c = src[i];

			if (c === '\\' && i + 1 < src.length) {
				push(src.slice(i, i + 2), 'escape');
				i++;
			} else if (c === '[') {
				// a `]` straight after the opening bracket is a literal one, and so is an escaped
				// one anywhere inside
				let j = i + 1;
				if (src[j] === '^') j++;
				if (src[j] === ']') j++;
				while (j < src.length && src[j] !== ']') j += src[j] === '\\' ? 2 : 1;
				push(src.slice(i, Math.min(j + 1, src.length)), 'class');
				i = j;
			} else if (c === '(') {
				const m = /^\(\?(?::|=|!|<[=!]|<[A-Za-z_$][\w$]*>)/.exec(src.slice(i));
				push(m ? m[0] : c, 'group');
				if (m) i += m[0].length - 1;
			} else if (c === ')' || c === '|') {
				push(c, 'group');
			} else if (c === '^' || c === '$') {
				push(c, 'anchor');
			} else if (c === '*' || c === '+' || c === '?' || c === '.') {
				push(c, 'quantifier');
			} else if (c === '{') {
				const m = /^\{\d+(?:,\d*)?\}/.exec(src.slice(i));
				if (m) {
					push(m[0], 'quantifier');
					i += m[0].length - 1;
				} else {
					literal += c;
				}
			} else {
				literal += c;
			}
		}

		if (literal) out.push({ text: literal, kind: 'literal' });
		return out;
	}

	const tokens = $derived(tokenize(value));

	// The colours live on a copy of the text behind a transparent input, so the two have to
	// agree on every dimension that moves a glyph: font, size, padding, border width.
	let mirror = $state<HTMLPreElement | undefined>(undefined);
	const sync = (e: Event) => {
		if (mirror) mirror.scrollLeft = (e.currentTarget as HTMLInputElement).scrollLeft;
	};

	const BOX = 'block w-full rounded-lg border p-2.5 font-mono text-sm leading-tight whitespace-pre';
</script>

<div class="relative">
	<pre
		bind:this={mirror}
		aria-hidden="true"
		class="{BOX} pointer-events-none absolute inset-0 m-0 overflow-hidden border-transparent bg-gray-50 dark:bg-gray-700">{#each tokens as t, i (i)}<span
				class={COLORS[t.kind]}>{t.text}</span
			>{/each}</pre>

	<input
		type="text"
		{id}
		bind:value
		{placeholder}
		autocomplete="off"
		spellcheck="false"
		onscroll={sync}
		class="{BOX} relative bg-transparent text-transparent caret-gray-900 placeholder:text-gray-400 dark:caret-white dark:placeholder:text-gray-500 {error
			? 'border-red-500 focus:border-red-500 focus:ring-red-500 dark:border-red-500'
			: 'focus:border-primary-500 focus:ring-primary-500 border-gray-300 dark:border-gray-600'}"
	/>
</div>

{#if error}
	<Helper color="red" class="mt-1">{error}</Helper>
{/if}
