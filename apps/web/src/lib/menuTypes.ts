/**
 * Shared types for the menu band and the tool bar.
 *
 * The core application defines its menus in `TopMenu.svelte`; every module
 * brings its own menus and additions in `modules/<product>/web/menu.ts`. The
 * `module` reference hides an entry while the product is not installed.
 */
import type { ModuleId } from './api';

export type Tone = 'slate' | 'green' | 'orange' | 'red' | 'blue' | 'teal';

export type Entry =
	| { kind: 'separator' }
	| { kind: 'heading'; label: string }
	| {
			kind: 'item';
			label: string;
			href?: string;
			shortcut?: string;
			submenu?: boolean;
			/** Module whose licence does not grant it — drawn with a lock in the template. */
			locked?: boolean;
			/** Entry that opens a window instead of navigating. */
			action?:
				| 'address-create'
				| 'search'
				| 'address-search'
				| 'article-create'
				| 'import'
				| 'personal-settings'
				| 'quit'
				/** Die Befehle des Menüs `Bearbeiten`; sie wirken auf das aktive Feld. */
				| 'edit-undo'
				| 'edit-redo'
				| 'edit-cut'
				| 'edit-copy'
				| 'edit-paste'
				| 'edit-delete'
				| 'edit-select-all'
				| 'clipboard-show';
			/** Right without which the entry does not appear; a list is enough with one of them. */
			need?: string | string[];
			/** Module without which the entry does not appear. */
			module?: ModuleId;
			/** Submenu that unfolds on hover. */
			children?: Entry[];
	  };

export type Menu = { label: string; items: Entry[] };

/** Tile of the side column; `href` missing and no `action` = not implemented yet. */
export type Tool = {
	label: string;
	icon: string;
	tone: Tone;
	href?: string;
	action?:
		| 'refresh'
		| 'search'
		| 'address-search'
		| 'address-create'
		| 'article-search'
		| 'article-create';
	/** Right without which the tile does not appear (e.g. creating). */
	need?: string;
	/** Module without which the tile does not appear. */
	module?: ModuleId;
};

/** What a module contributes to the menu band. */
export interface ModuleMenuContribution {
	/** Own drop-down menus, e.g. `Auftrag` or `Faktura`. */
	ownMenus?: Menu[];
	/** Extra entries in existing menus (`Stammdaten`, `Admin`, …). */
	append?: {
		menu: string;
		items: Entry[];
		/** Insert directly after this entry; without it, at the end of the menu. */
		after?: string;
		/** Insert directly before this entry. */
		before?: string;
	}[];
	/** Further tiles of the side column. */
	tools?: Tool[];
}
