import houdini from 'houdini/vite';
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [houdini(), tailwindcss(), sveltekit()],
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
