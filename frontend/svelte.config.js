import adapter from '@sveltejs/adapter-node';

import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),

	compilerOptions: {
		runes: true
	},

	kit: {
		adapter: adapter(),

		// Nothing notices a deploy otherwise: `updated` only moves when something asks, and the
		// layout leaves for the new build on the next navigation once it has.
		version: {
			pollInterval: 300_000
		},

		alias: {
			$houdini: '.houdini/'
		}
	}
};

export default config;
