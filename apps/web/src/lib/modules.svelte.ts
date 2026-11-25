/**
 * State of the module registry.
 *
 * The core application (`stuff`) is always present. The modules `go` (orders /
 * packing slips), `people` (personnel) and `money` (accounting) are only visible
 * and usable when they are registered in the database. Before the first load
 * "nothing installed" applies; the interface would then deliberately stay slim
 * instead of showing data the backend rejects with `409` anyway.
 */
import { api, type ModuleId, type ModuleSnapshot } from './api';

class ModuleStore {
	snapshot = $state<ModuleSnapshot | null>(null);
	/** Product name the API reports; `null` until the call answered. */
	installationName = $state<string | null>(null);
	loading = $state(false);
	error = $state<string | null>(null);

	/** Without a loaded registry the module areas stay hidden. */
	get ready() {
		return this.snapshot !== null;
	}

	get coreInstalled() {
		return this.snapshot?.core_installed ?? false;
	}

	isInstalled(id: ModuleId): boolean {
		return this.snapshot?.modules.some((module) => module.id === id && module.installed) ?? false;
	}

	/**
	 * Name of the product this installation runs.
	 *
	 * The registry decides: with the core application it is `WHERE - is my stuff?`,
	 * an installation that only carries the personnel module is
	 * `WHERE - are my people?`. Before signing in the registry is still empty — then
	 * the name the API reported or the one the desktop shell sets in
	 * `window.__WHERE_PRODUCT__` applies.
	 */
	get productName(): string {
		if (this.isInstalled('stuff')) return 'WHERE - is my stuff?';
		if (this.isInstalled('people')) return 'WHERE - are my people?';
		return this.installationName ?? injectedProduct() ?? 'WHERE - is my stuff?';
	}

	/**
	 * Reads the product name of the installation.
	 *
	 * The login mask and the window title show it before a session exists, so the
	 * call is not part of the registry load.
	 */
	async loadInstallationName() {
		if (this.installationName !== null) return;
		try {
			this.installationName = (await api.installation()).product_name;
		} catch {
			// The mask keeps the name the desktop shell injects.
		}
	}

	/** Capability, e.g. `article-pricing` (rental/sales data on articles). */
	has(capability: string): boolean {
		return (
			this.snapshot?.modules.some(
				(module) => module.installed && module.capabilities.includes(capability)
			) ?? false
		);
	}

	status(id: ModuleId) {
		return this.snapshot?.modules.find((module) => module.id === id) ?? null;
	}

	async load() {
		// The registry belongs to the session; without signing in it stays empty.
		this.loading = true;
		this.error = null;
		try {
			this.snapshot = await api.modules.list();
		} catch (error) {
			this.error = error instanceof Error ? error.message : String(error);
		} finally {
			this.loading = false;
		}
	}

	async install(id: ModuleId) {
		const outcome = await api.modules.install(id);
		await this.load();
		return outcome;
	}

	async remove(id: ModuleId) {
		this.snapshot = await api.modules.remove(id);
	}
}

export const modules = new ModuleStore();

/**
 * Product name the desktop shell sets when it opens the window.
 *
 * It is the fallback before signing in: the module registry belongs to the session
 * and is not available yet.
 */
function injectedProduct(): string | null {
	if (typeof window === 'undefined') return null;
	const value = (window as { __WHERE_PRODUCT__?: unknown }).__WHERE_PRODUCT__;
	return typeof value === 'string' && value.trim() !== '' ? value : null;
}
