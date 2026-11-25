// Copies the release build of the API into the Tauri sidecar directory.
//
// Tauri wants sidecars named with the target triple: `where-server-<triple>.exe`
// becomes `where-server.exe` in the installation.
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..', '..');

const triples = {
	'win32-x64': 'x86_64-pc-windows-msvc',
	'win32-arm64': 'aarch64-pc-windows-msvc',
	'linux-x64': 'x86_64-unknown-linux-gnu',
	'linux-arm64': 'aarch64-unknown-linux-gnu',
	'darwin-x64': 'x86_64-apple-darwin',
	'darwin-arm64': 'aarch64-apple-darwin'
};

const key = `${process.platform}-${process.arch}`;
const triple = triples[key];
if (!triple) {
	console.error(`unsupported platform: ${key}`);
	process.exit(1);
}

const suffix = process.platform === 'win32' ? '.exe' : '';
const source = join(root, 'target', 'release', `where-server${suffix}`);
if (!existsSync(source)) {
	console.error(`missing ${source} — run: cargo build -p where-server --release`);
	process.exit(1);
}

const targetDirectory = join(here, '..', 'src-tauri', 'binaries');
mkdirSync(targetDirectory, { recursive: true });
const target = join(targetDirectory, `where-server-${triple}${suffix}`);
copyFileSync(source, target);
console.log(`sidecar: ${target}`);
