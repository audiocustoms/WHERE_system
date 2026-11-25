import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vite';

/** Dateisystempfad aus einer `file:`-URL, ohne Node-Typen zu benötigen. */
function aliasPath(url: URL): string {
	// Windows liefert `/P:/…`; der führende Schrägstrich muss weg.
	return decodeURIComponent(url.pathname).replace(/^\/([A-Za-z]:)/, '$1');
}

/**
 * Vite beobachtet nur das App-Paket. Die Modul-Oberflächen liegen unter
 * `modules/<produkt>/web` außerhalb dieses Wurzelverzeichnisses — ohne diesen
 * Watcher liefert der Dev-Server nach einer Änderung weiter den ersten Stand
 * und erst ein Neustart zeigt sie.
 */
function watchModuleFrontends(): Plugin {
	return {
		name: 'where-modules-watch',
		configureServer(server) {
			server.watcher.add(aliasPath(new URL('../../modules', import.meta.url)));
		}
	};
}

export default defineConfig({
	plugins: [
		watchModuleFrontends(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// Static SPA build: one artefact for a web server in front of the API
			// (nginx or Caddy), the Tauri desktop app and the Android shell.
			adapter: adapter({ fallback: 'index.html', strict: false })
		})
	],
	// Die Modul-Oberflächen liegen außerhalb des App-Pakets (`modules/<produkt>/web`).
	// Vite braucht dafür absolute Aliase; TypeScript bekommt dieselben über `paths`
	// in `tsconfig.json`.
	resolve: {
		alias: {
			// `$core` instead of `#…`: Vite resolves `#` specifiers through the
			// package "imports" map, which cannot point outside the package.
			$core: aliasPath(new URL('./src/lib', import.meta.url)),
			$modules: aliasPath(new URL('../../modules', import.meta.url))
		}
	},
	server: {
		// Das Logo liegt bewusst einmal im Repo-Wurzelverzeichnis (logo/logo.svg) und wird
		// von dort importiert; im Dev-Server muss der Pfad dafür freigegeben sein.
		fs: { allow: ['..', '../..', '../../modules'] },
		proxy: {
			// Keeps the browser origin identical in development, so no CORS setup is
			// needed while the API runs on its own port.
			'/api': 'http://127.0.0.1:8090',
			'/health': 'http://127.0.0.1:8090',
			'/ready': 'http://127.0.0.1:8090'
		}
	}
});
