import houdini from 'houdini/vite';
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [houdini(), tailwindcss(), sveltekit()],
	// The docker context is ./frontend, which has no .git in it, so the commit has to be handed
	// in from outside: CI passes it as a build arg and the Dockerfile turns it into an ARG,
	// which docker exposes to the build as an environment variable. Empty anywhere it was not
	// handed in, and the menu leaves the line out rather than showing a placeholder.
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
