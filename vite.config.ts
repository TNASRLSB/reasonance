import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	clearScreen: false,
	optimizeDeps: {
		include: [
			'codemirror',
			'@codemirror/state',
			'@codemirror/view',
			'@codemirror/language',
			'@codemirror/lang-javascript',
			'@codemirror/lang-html',
			'@codemirror/lang-css',
			'@codemirror/lang-python',
			'@codemirror/lang-rust',
			'@codemirror/lang-json',
			'@codemirror/lang-markdown',
			'@codemirror/theme-one-dark',
			'@xterm/xterm',
			'@xterm/addon-webgl',
			'@xterm/addon-fit',
		],
	},
	server: {
		host: '127.0.0.1',
		port: 1420,
		strictPort: true,
		hmr: {
			overlay: false,
		},
	},
	define: {
		__APP_VERSION__: JSON.stringify(process.env.npm_package_version || '0.0.0'),
	},
});
