import houdini from 'houdini/vite';
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		houdini(),
		tailwindcss(),
		sveltekit({
			preprocess: vitePreprocess(),
			compilerOptions: { runes: true },
			adapter: adapter(),
			version: { pollInterval: 300_000 },
			alias: { $houdini: '.houdini/' }
		})
	],
	// The Nix build sandbox has no git history, so the commit has to be handed in from outside:
	// flake.nix sets GIT_SHA from the flake's own revision. Empty anywhere it was not handed in,
	// and the menu leaves the line out rather than showing a placeholder.
	define: {
		__GIT_SHA__: JSON.stringify(process.env.GIT_SHA ?? '')
	},
	optimizeDeps: {
		exclude: ['@urql/svelte']
	},
	server: {
		proxy: {
			'/graphql': {
				target: 'http://localhost:8181',
				changeOrigin: true
			}
		}
	}
});
