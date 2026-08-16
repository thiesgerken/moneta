import { describe, expect, it } from 'vitest';
import { regexError, tokenize } from './regex_highlight';

/// The tokenizer never rejects anything, so what is worth asserting is that it *segments* the
/// same string the engine would: an escape is two characters, a character class runs to its
/// closing bracket wherever that is, and everything else is text you are matching on.
const kinds = (src: string) => tokenize(src).map((t) => `${t.kind}:${t.text}`);

describe('tokenize', () => {
	it('leaves a pattern with no syntax in it as one piece of text', () => {
		expect(kinds('Muster Stadtreinigung')).toEqual(['literal:Muster Stadtreinigung']);
	});

	it('reads an escape as the pair it is, not as a backslash and a letter', () => {
		expect(kinds('a\\.b')).toEqual(['literal:a', 'escape:\\.', 'literal:b']);
		expect(kinds('\\d{2}')).toEqual(['escape:\\d', 'quantifier:{2}']);
	});

	it('runs a character class to its closing bracket', () => {
		expect(kinds('[a-z]+')).toEqual(['class:[a-z]', 'quantifier:+']);
		// a `]` straight after the bracket is a literal one, and so is an escaped one inside
		expect(kinds('[]]')).toEqual(['class:[]]']);
		expect(kinds('[^]abc]')).toEqual(['class:[^]abc]']);
		expect(kinds('[a\\]b]')).toEqual(['class:[a\\]b]']);
	});

	it('keeps a group opener together with what makes it non-capturing', () => {
		expect(kinds('(?:ab)')).toEqual(['group:(?:', 'literal:ab', 'group:)']);
		expect(kinds('(?<jahr>\\d)')).toEqual(['group:(?<jahr>', 'escape:\\d', 'group:)']);
		expect(kinds('(a|b)')).toEqual(['group:(', 'literal:a', 'group:|', 'literal:b', 'group:)']);
	});

	it('only reads a brace as a quantifier when it is one', () => {
		expect(kinds('a{2,4}')).toEqual(['literal:a', 'quantifier:{2,4}']);
		expect(kinds('a{2,}')).toEqual(['literal:a', 'quantifier:{2,}']);
		// a lone brace is part of the text, not a count
		expect(kinds('a{b}')).toEqual(['literal:a{b}']);
	});

	it('marks the anchors', () => {
		expect(kinds('^REWE$')).toEqual(['anchor:^', 'literal:REWE', 'anchor:$']);
	});

	it('accounts for every character of the input, in order', () => {
		for (const pattern of [
			'((Muster.*Beispiel)|(MUSTER.*BEISPIEL))',
			'00XX\\-0000000',
			'^\\d{2}\\.\\d{2}\\.\\d{4} [0-2]\\d:[0-5]\\d$',
			'a\\',
			'[unterminated'
		]) {
			expect(
				tokenize(pattern)
					.map((t) => t.text)
					.join('')
			).toBe(pattern);
		}
	});

	it('has nothing to say about an empty pattern', () => {
		expect(tokenize('')).toEqual([]);
	});
});

describe('regexError', () => {
	it('treats no pattern as no problem', () => {
		expect(regexError('')).toBeNull();
		expect(regexError('   ')).toBeNull();
	});

	it('passes anything the engine accepts', () => {
		expect(regexError('(Muster|MUSTER).*')).toBeNull();
	});

	/// `matches()` on the server treats an unparseable pattern as matching nothing, so a rule
	/// with one silently never fires. This is what turns the field red instead.
	it('reports what the engine says about one it does not', () => {
		expect(regexError('(unclosed')).toBeTruthy();
		expect(regexError('*')).toBeTruthy();
	});
});
