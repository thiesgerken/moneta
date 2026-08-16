import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

// A config of its own rather than vite.config.ts: houdini's plugin keeps a watcher alive that
// left every run hanging for ten seconds after the last assertion, and none of the app's other
// plugins have anything to say about a plain module. The svelte one is here because a
// `.svelte.ts` file is compiled, not merely typed -- `$state` is a rune, not a function.
export default defineConfig({
	plugins: [svelte()],
	test: {
		include: ['src/**/*.test.ts']
	}
});
