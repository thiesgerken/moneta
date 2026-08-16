/// Colouring a regular expression, wherever one is shown.
///
/// The point is the same in a form and in a table: what is structure and what is the text you
/// are matching on. `Muster\s+Stadtreinigung` is one word and a gap, not three tokens of
/// punctuation.

export type TokenKind = keyof typeof COLORS;
export type Token = { text: string; kind: TokenKind };

export const COLORS = {
	literal: '',
	escape: 'text-purple-600 dark:text-purple-400',
	class: 'text-amber-600 dark:text-amber-400',
	group: 'text-blue-600 dark:text-blue-400',
	anchor: 'text-rose-600 dark:text-rose-400',
	quantifier: 'text-emerald-600 dark:text-emerald-400'
};

/// Enough of a regex reader to colour one. Deliberately not a parser — it never has to reject
/// anything, since `regexError` already says whether the pattern is legal.
export function tokenize(src: string): Token[] {
	const out: Token[] = [];
	let literal = '';

	const push = (text: string, kind: TokenKind) => {
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

/// The message the engine itself gives, which names the position — far more use than
/// "ungültig". An empty pattern is no pattern rather than a broken one.
export function regexError(pattern: string): string | null {
	if (!pattern.trim()) return null;
	try {
		new RegExp(pattern);
		return null;
	} catch (e) {
		return e instanceof Error ? e.message : 'ungültiger Ausdruck';
	}
}
