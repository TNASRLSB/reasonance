import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// @ts-expect-error -- process is available in Node (vite config context)
const appVersion: string = process.env.npm_package_version || '0.0.0';

export default defineConfig({
	plugins: [sveltekit()],
	clearScreen: false,
	build: {
		rollupOptions: {
			output: {
				manualChunks(id) {
					// CodeMirror — loaded on first file open
					if (id.includes('codemirror') || id.includes('@codemirror/')) {
						return 'vendor-codemirror';
					}
					// xterm — loaded on first terminal spawn
					if (id.includes('@xterm/')) {
						return 'vendor-xterm';
					}
					// xyflow/svelte — loaded on hive canvas
					if (id.includes('@xyflow/') || id.includes('xyflow')) {
						return 'vendor-xyflow';
					}
					// Markdown rendering — loaded on first markdown render
					if (id.includes('highlight.js') || id.includes('marked') || id.includes('dompurify')) {
						return 'vendor-markdown';
					}
				},
			},
		},
	},
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
		__APP_VERSION__: JSON.stringify(appVersion),
	},
});
