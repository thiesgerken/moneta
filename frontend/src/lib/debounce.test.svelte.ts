import { flushSync } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import { debounced } from './debounce.svelte';

describe('debounced', () => {
	it('reports only the value that stopped changing', () => {
		vi.useFakeTimers();

		let term = $state('');
		const cleanup = $effect.root(() => {
			const search = debounced(() => term, 300);

			expect(search.current).toBe('');

			term = 'a';
			flushSync();
			term = 'ab';
			flushSync();
			vi.advanceTimersByTime(299);
			expect(search.current).toBe('');

			vi.advanceTimersByTime(1);
			flushSync();
			expect(search.current).toBe('ab');
		});

		cleanup();
		vi.useRealTimers();
	});
});
