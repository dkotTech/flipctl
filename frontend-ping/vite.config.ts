import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5275,
		strictPort: true,
		host: '0.0.0.0',
		// dev mode talks to the running backend for real data
		proxy: {
			'/api': 'http://127.0.0.1:5173'
		}
	},
	preview: {
		port: 5275,
		strictPort: true
	}
});
