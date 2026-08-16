import { describe, expect, it } from 'vitest';
import { NAV, pageTitle } from './navigation';

describe('pageTitle', () => {
	it('names the page after its navbar entry', () => {
		expect(pageTitle('/expenses')).toBe('Moneta – Ausgaben');
	});

	// a month page is still Auswertung, which is what the prefix match is for
	it('keeps a nested route under the entry it belongs to', () => {
		expect(pageTitle('/analysis/2026/07')).toBe('Moneta – Auswertung');
	});

	// not in the navbar, but still a page with a name
	it('names the login page too', () => {
		expect(pageTitle('/login')).toBe('Moneta – Login');
	});

	it('falls back to the bare name where there is nothing to add', () => {
		expect(pageTitle('/')).toBe('Moneta');
		expect(pageTitle('/something-else')).toBe('Moneta');
	});

	/// The navbar and the titles are the same list on purpose, so an entry added to one cannot
	/// leave the other behind.
	it('has a title for every navbar entry', () => {
		for (const entry of NAV) {
			expect(pageTitle(entry.href)).toBe(`Moneta – ${entry.label}`);
		}
	});
});
