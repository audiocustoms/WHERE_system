<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import logo from '../../../../logo/logo.svg';
	import { api, type LoginTenant, type ModuleId } from './api';
	import { auth } from './auth.svelte';
	import { isDark, toggleTheme } from './theme.svelte';
	import { openAddressSearch, openArticleSearch, openSearch } from './search/searchOverlay.svelte';
	import { openAddressCreate } from './addresses/addressDialogStore.svelte';
	import { openArticleCreate } from './articles/articleDialogStore.svelte';
	import { openImport } from './import/importStore.svelte';
	import { askConfirm } from './confirm.svelte';
	import { runEditorCommand, type EditorCommand } from './edit/editorCommands';
	import { showClipboard } from './edit/clipboard.svelte';
	import { showStatus } from './statusMessage.svelte';
	import { modules } from './modules.svelte';
	import type { Entry, Menu, ModuleMenuContribution, Tool, Tone } from './menuTypes';
	import { t } from './i18n.svelte';
	import PersonalSettingsDialog from './settings/PersonalSettingsDialog.svelte';
	import { personalSettings } from './settings/personalSettings.svelte';
	import goMenu from '$modules/where-does-my-stuff-go/web/menu';
	import moneyMenu from '$modules/where-is-my-money/web/menu';
	import peopleMenu from '$modules/where-are-my-people/web/menu';

	const TONES: Record<Tone, string> = {
		slate: '#5c7d99',
		green: '#3f9a4e',
		orange: '#e0a13a',
		red: '#8f3230',
		blue: '#5b8dd6',
		teal: '#4f7186'
	};

	const separator: Entry = { kind: 'separator' };

	/**
	 * Menu band as in `img/menu/*.PNG` — labels, order and separators come
	 * one to one from the screenshots. Entries with a target are links,
	 * all others are greyed out (commands not available in the original).
	 * `QM` is missing on purpose: the entry was removed on request.
	 */
	const MENUS: Menu[] = [
		{
			label: 'Datei',
			items: [
				{ kind: 'item', label: 'Kalender', href: '/agenda', need: 'teamagenda.termine-lesen', module: 'stuff' },
				{ kind: 'item', label: 'Zentrale', href: '/', module: 'stuff' },
				{ kind: 'item', label: 'Gesendete eMails' },
				separator,
				{ kind: 'item', label: 'Neu anlegen...', submenu: true },
				{
					kind: 'item',
					label: 'Aktion',
					submenu: true,
					children: [{ kind: 'item', label: 'Import-Manager', action: 'import' }]
				},
				separator,
				{ kind: 'item', label: 'Persönliche Einstellungen', action: 'personal-settings' },
				separator,
				{ kind: 'item', label: 'Auf Updates prüfen' },
				separator,
				{ kind: 'item', label: 'Beenden', shortcut: 'Ctrl+Q', action: 'quit' }
			]
		},
		{
			label: 'Bearbeiten',
			items: [
				{ kind: 'item', label: 'Widerrufen', shortcut: 'Ctrl+Z', action: 'edit-undo' },
				{ kind: 'item', label: 'Wiederholen', shortcut: 'Ctrl+Shift+Z', action: 'edit-redo' },
				separator,
				{ kind: 'item', label: 'Ausschneiden', shortcut: 'Ctrl+X', action: 'edit-cut' },
				{ kind: 'item', label: 'Kopieren', shortcut: 'Ctrl+C', action: 'edit-copy' },
				{ kind: 'item', label: 'Einsetzen', shortcut: 'Ctrl+V', action: 'edit-paste' },
				{ kind: 'item', label: 'Löschen', shortcut: '[del] / [entf]', action: 'edit-delete' },
				{ kind: 'item', label: 'Alles auswählen', shortcut: 'Ctrl+A', action: 'edit-select-all' },
				separator,
				{ kind: 'item', label: 'Zwischenablage anzeigen', action: 'clipboard-show' }
			]
		},
		{
			label: 'Stammdaten',
			items: [
				{ kind: 'item', label: 'Adresse anlegen', action: 'address-create', need: 'adressen.adresse-anlegen', module: 'stuff' },
				{ kind: 'item', label: 'Adressliste', href: '/addresses', need: 'adressen.adresse-lesen', module: 'stuff' },
				{ kind: 'item', label: 'Ansprechpartnerliste', module: 'stuff' },
				separator,
				{ kind: 'item', label: 'Artikel anlegen...', action: 'article-create', need: 'artikel.artikel-anlegen', module: 'stuff' },
				{ kind: 'item', label: 'Artikelliste', href: '/articles', need: 'artikel.artikel-lesen', module: 'stuff' },
				{
					kind: 'item',
					label: 'Artikelbestandsliste',
					href: '/articles',
					need: 'artikel.artikel-lesen',
					module: 'stuff'
				},
				{ kind: 'item', label: 'Artikelpreisanpassung', module: 'go' },
				separator,
				{ kind: 'item', label: 'Service', module: 'stuff' }
			]
		},
		{
			label: 'Lager',
			items: [
				{ kind: 'item', label: 'Inventur', href: '/stocktaking', need: 'artikel.artikel-lesen', module: 'stuff' },
				{ kind: 'item', label: 'Beschaffung', href: '/procurement', need: 'artikel.artikel-lesen', module: 'stuff' },
				{ kind: 'item', label: 'Bestellungen', href: '/purchase-orders', need: 'artikel.artikel-lesen', module: 'stuff' },
				{ kind: 'item', label: 'Bestandswert', href: '/stock-value', need: 'artikel.artikel-lesen', module: 'stuff' },
				separator,
				{ kind: 'item', label: 'Werkstatt Logbuch', module: 'stuff' },
				separator,
				{ kind: 'item', label: 'Inventur DGUV-Prüfplan', module: 'stuff' },
				{ kind: 'item', label: 'Neue DGUV Prüfung erfassen', module: 'stuff' },
				{ kind: 'item', label: 'Import von DGUV Werten', module: 'stuff' },
				separator,
				{ kind: 'item', label: 'Ungeklärte Geräteliste', module: 'stuff' },
				separator,
				{ kind: 'item', label: 'Gerät [Bestand] öffnen', module: 'stuff' },
				{ kind: 'item', label: 'Gerät [Bestand] - Neuer Lagerplatz', module: 'stuff' },
				separator,
				{ kind: 'item', label: 'Verpackungseinheit öffnen', module: 'stuff' },
				{ kind: 'item', label: 'Verpackungseinheitenliste', module: 'stuff' }
			]
		},
		{
			label: 'Statistik',
			items: [
				{ kind: 'item', label: 'Faktura Umsatz', module: 'money' },
				{ kind: 'item', label: 'Kunden-Umsatz', module: 'go' },
				{ kind: 'item', label: 'Artikel-Umsatz', module: 'go' },
				separator,
				{ kind: 'item', label: 'Kostenstellen-Auswertung', module: 'money' },
				separator,
				{ kind: 'item', label: '*** Erweiterte Statistiken ***', module: 'money' },
				{ kind: 'item', label: 'Faktura Jahresumsätze', module: 'money' },
				{ kind: 'item', label: 'Faktura Monatsumsätze', module: 'money' },
				{ kind: 'item', label: 'Artikel Top10', module: 'go' },
				{ kind: 'item', label: 'Artikel Bestandsübersicht', module: 'stuff' },
				{ kind: 'item', label: 'Vertragsmodul Umsatz', module: 'go' },
				{ kind: 'item', label: 'Vertragsmodul Jahresvorschau  Verteilung', module: 'money' }
			]
		},
		{
			label: 'Admin',
			/*
			 * Content of `img/admin_menu.PNG` (window „Einstellungen"): the three groups
			 * system settings, modules and administration. Two entries are renamed because
			 * they carry the product name of the old system: „EPI-Admin" → „Systemverwaltung",
			 * „epirent-Lizenz" → „Lizenz". On request, nine entries of the
			 * original are gone: „Digitale Unterschrift", „Erweiterungsfelder", „PDF",
			 * „Sprachen", „Standarttexte", „Status", „Schnittstelle REST", „Telefonie"
			 * and „Übersetzer". The
			 * product groups are maintained by the article settings, hence no entry of
			 * their own; an admin overview page no longer exists, the entries lead
			 * directly to their page.
			 */
			items: [
				{ kind: 'heading', label: 'Systemeinstellungen' },
				{ kind: 'item', label: 'Adressen', href: '/admin/addresses', module: 'stuff' },
				{ kind: 'item', label: 'Artikel', href: '/admin/articles', module: 'stuff' },
				{ kind: 'item', label: 'Barcode & CheckOut/-In', module: 'stuff' },
				{ kind: 'item', label: 'Druckereinstellungen', module: 'stuff' },
				{ kind: 'item', label: 'eMailversand', module: 'stuff' },
				{ kind: 'item', label: 'Lager & Lagerplätze', href: '/admin/warehouses', module: 'stuff' },
				{ kind: 'item', label: 'Messenger', module: 'stuff' },
				{ kind: 'item', label: 'Nummernkreisläufe', href: '/admin/number-series', module: 'stuff' },
				{ kind: 'item', label: 'TeamAgenda', href: '/agenda', module: 'stuff' },
				{ kind: 'item', label: 'Textverarbeitung', module: 'stuff' },
				{ kind: 'item', label: 'Währungen', href: '/admin/settings#waehrungen', module: 'stuff' },
				{ kind: 'item', label: 'Werkstatt/DGUV/UGL', module: 'stuff' },
				separator,
				{ kind: 'heading', label: 'Module' },
				{ kind: 'item', label: 'Modulverwaltung', href: '/admin/modules' },
				{ kind: 'item', label: 'Kalender Abonnement', module: 'stuff' },
				{ kind: 'item', label: 'Mandanten', href: '/admin/tenants' },
				{ kind: 'item', label: 'Pyrotech', module: 'stuff' },
				separator,
				{ kind: 'heading', label: 'Administration' },
				{ kind: 'item', label: 'Backup', module: 'stuff' },
				{ kind: 'item', label: 'Benutzer-Netzumgebungen', module: 'stuff' },
				{ kind: 'item', label: 'Benutzerverwaltung', href: '/admin/users' },
				{ kind: 'item', label: 'Systemverwaltung', module: 'stuff' },
				{ kind: 'item', label: 'Lizenz', module: 'stuff' },
				{ kind: 'item', label: 'Logbooks', module: 'stuff' },
				{ kind: 'item', label: 'Login & Logout', module: 'stuff' }
			]
		},
		{
			label: 'Hilfe',
			items: [
				{ kind: 'item', label: 'Hilfe und Dokumentation' },
				{ kind: 'item', label: 'Über WHERE' }
			]
		}
	];

	/** Page column on the left — order on request: refresh, home, search, calendar. */
	const TOOLS: Tool[] = [
		{ label: 'Aktualisieren', icon: 'refresh', tone: 'slate', action: 'refresh' },
		{ label: 'Zentrale', icon: 'house', tone: 'teal', href: '/', module: 'stuff' },
		{ label: 'Suchen', icon: 'search', tone: 'slate', action: 'search', module: 'stuff' },
		{
			label: 'Kalender',
			icon: 'calendar',
			tone: 'slate',
			href: '/agenda',
			need: 'teamagenda.termine-lesen',
		module: 'stuff'
		},
		{
			label: 'Adressen',
			icon: 'people',
			tone: 'green',
			action: 'address-search',
			need: 'adressen.adresse-lesen',
		module: 'stuff'
		},
		{
			label: 'Adresse +',
			icon: 'person-add',
			tone: 'green',
			action: 'address-create',
			need: 'adressen.adresse-anlegen',
		module: 'stuff'
		},
		{
			label: 'Artikel',
			icon: 'boxes',
			tone: 'green',
			action: 'article-search',
			need: 'artikel.artikel-lesen',
		module: 'stuff'
		},
		{
			label: 'Artikel +',
			icon: 'boxes-add',
			tone: 'green',
			action: 'article-create',
			need: 'artikel.artikel-anlegen',
		module: 'stuff'
		},
		{ label: 'Geräte', icon: 'serial', tone: 'green', module: 'stuff' },
		{ label: 'Verfügbarkeit', icon: 'availability', tone: 'green', module: 'stuff' }
	];

	let open = $state<string | null>(null);
	/**
	 * Open submenu of the drop-down list („Aktion") with its place. It is
	 * set to `position: fixed` because the drop-down list itself scrolls and an
	 * absolute submenu would otherwise be cut off.
	 */
	let submenu = $state<{ label: string; left: number; top: number } | null>(null);
	let navEl = $state<HTMLElement | null>(null);

	// --------------------------------------------------------- Tenant switch
	/** The choice behind the tenant of the header. */
	let isTenantOpen = $state(false);
	let mandantEl = $state<HTMLElement | null>(null);
	let allTenants = $state<LoginTenant[]>([]);

	/**
	 * Tenants this account may switch into: its assignments
	 * (`"User_Mandanten"`), all of them with the admin right.
	 */
	const switchableTenants = $derived(
		auth.isAdmin
			? allTenants
			: allTenants.filter((mandant) => auth.user?.mandants.includes(mandant.id))
	);

	onMount(async () => {
		if (allTenants.length > 1) return;
		try {
			allTenants = await api.auth.tenants();
		} catch {
			// Without the list the tenant stays a plain label.
		}
	});

	/** Moves the session to the other tenant; the page reloads afterwards. */
	async function changeTenant(id: number) {
		isTenantOpen = false;
		if (id === auth.user?.mandant) return;
		if (!(await auth.changeTenant(id))) return;
		location.reload();
	}

	/** Menu contributions of the modules; only installed products are mixed in. */
	const MODULE_MENUS: { id: ModuleId; contribution: ModuleMenuContribution }[] = [
		{ id: 'go', contribution: goMenu },
		{ id: 'people', contribution: peopleMenu },
		{ id: 'money', contribution: moneyMenu }
	];

	/** Entries a module brings along only appear after its installation. */
	function visible(item: Entry): boolean {
		if (item.kind !== 'item') return true;
		return !item.module || modules.isInstalled(item.module);
	}

	/**
	 * Separators and headings that would be left alone after hiding fall away —
	 * an installation without the core application shows only the entries of its product.
	 */
	function withoutGap(items: Entry[]): Entry[] {
		const stays: Entry[] = [];
		for (let i = 0; i < items.length; i += 1) {
			const item = items[i];
			if (item.kind === 'separator') {
				if (stays.length === 0 || stays[stays.length - 1].kind === 'separator') continue;
				stays.push(item);
				continue;
			}
			if (item.kind === 'heading') {
				// A heading stays only when an entry follows before the next one.
				let content = false;
				for (let j = i + 1; j < items.length; j += 1) {
					if (items[j].kind === 'heading') break;
					if (items[j].kind === 'item') {
						content = true;
						break;
					}
				}
				if (!content) continue;
			}
			stays.push(item);
		}
		while (stays.length > 0 && stays[stays.length - 1].kind === 'separator') stays.pop();
		return stays;
	}

	/**
	 * Core menus plus the contributions of the installed modules. The `Admin` menu
	 * hangs on the right `administrative-einstellungen.zugriff`; without that right it
	 * does not appear in the taskbar at all.
	 */
	const allMenus = $derived.by(() => {
		const menues: Menu[] = MENUS.map((menu) => ({ ...menu, items: [...menu.items] }));

		for (const { id, contribution } of MODULE_MENUS) {
			if (!modules.isInstalled(id)) continue;
			for (const menu of contribution.ownMenus ?? []) {
				const present = menues.find((entry) => entry.label === menu.label);
				if (present) present.items = [...present.items, ...menu.items];
				else menues.push({ label: menu.label, items: [...menu.items] });
			}
			for (const { menu, items, after, before } of contribution.append ?? []) {
				const target = menues.find((entry) => entry.label === menu);
				if (!target) continue;
				const anchor = after ?? before;
				const index = anchor
					? target.items.findIndex(
							(item) => item.kind === 'item' && item.label === anchor
						)
					: -1;
				if (index < 0) {
					target.items.push(...items);
				} else {
					const at = after ? index + 1 : index;
					target.items.splice(at, 0, ...items);
				}
			}
		}

		return menues
			.filter((menu) => menu.label !== 'Admin' || auth.isAdmin)
			.map((menu) => ({ ...menu, items: withoutGap(menu.items.filter(visible)) }))
			.filter((menu) => menu.items.some((item) => item.kind === 'item'));
	});

	const visibleMenus = $derived(allMenus);

	/** All tiles of the core application and of the installed modules. */
	const allTools = $derived.by(() => {
		const own = TOOLS.filter((tool) => !tool.module || modules.isInstalled(tool.module));
		const module = MODULE_MENUS.filter(({ id }) => modules.isInstalled(id)).flatMap(
			({ contribution }) => contribution.tools ?? []
		);
		return [...own, ...module];
	});

	/**
	 * Whether the account lacks a right (or one of a list) — entries and
	 * tiles without that right disappear entirely (`Adresse +`, `Artikel +`).
	 */
	function hasRight(need: string | string[] | undefined): boolean {
		if (!need) return true;
		const keys = Array.isArray(need) ? need : [need];
		return keys.some((key) => auth.hasRight(key));
	}

	/** Tiles whose right the account lacks are missing entirely (`Adresse +`, `Artikel +`). */
	const visibleTools = $derived(allTools.filter((tool) => hasRight(tool.need)));

	const current = $derived(page.url.pathname + page.url.search);

	/** The import manager starts with the target of the page one is on. */
	const pageImportTarget = () =>
		page.url.pathname.startsWith('/articles') ? ('articles' as const) : ('addresses' as const);

	const toggle = (label: string) => (open = open === label ? null : label);
	const close = () => {
		open = null;
		submenu = null;
	};
	const reload = () => location.reload();

	/**
	 * Ein Eintrag mit `action` — eine Stelle für die Menüleiste und ihr Untermenü.
	 * Die Befehle des Menüs `Bearbeiten` wirken auf das zuletzt benutzte Feld; ohne
	 * Feld sagen sie es kurz an, statt stumm nichts zu tun.
	 */
	async function runAction(action: string): Promise<void> {
		switch (action) {
			case 'address-create':
				openAddressCreate();
				return;
			case 'article-create':
				openArticleCreate();
				return;
			case 'import':
				openImport(pageImportTarget());
				return;
			case 'search':
				openSearch();
				return;
			case 'personal-settings':
				personalSettings.show();
				return;
			case 'quit':
				await beenden();
				return;
			case 'clipboard-show':
				showClipboard();
				return;
			case 'edit-undo':
			case 'edit-redo':
			case 'edit-cut':
			case 'edit-copy':
			case 'edit-paste':
			case 'edit-delete':
			case 'edit-select-all': {
				const command = action.slice('edit-'.length) as EditorCommand;
				if (!(await runEditorCommand(command))) showStatus(t('Kein Eingabefeld aktiv.'));
				return;
			}
			default:
				openAddressSearch();
		}
	}

	/** Sign out: end the session, the mask appears instead of the application. */
	async function signOut() {
		close();
		await auth.logout();
		await goto('/');
	}

	/**
	 * `Datei → Beenden` and `Ctrl+Q`: the session ends and the window closes.
	 *
	 * A browser tab that a script did not open cannot be closed; there the login
	 * mask remains, which is what quitting means outside the desktop shell.
	 */
	async function beenden() {
		close();
		const sicher = await askConfirm({
			title: t('Beenden'),
			message: t('Die Anwendung wirklich beenden?'),
			confirmLabel: t('Ja, beenden'),
			cancelLabel: t('Weiterarbeiten')
		});
		if (!sicher) return;

		await auth.logout();
		fensterSchliessen();
	}

	/** Im Desktop-Fenster (Tauri) schließt nur der eigene Befehl verlässlich. */
	function fensterSchliessen() {
		const tauri = (
			window as {
				__TAURI_INTERNALS__?: {
					invoke?: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
				};
			}
		).__TAURI_INTERNALS__;
		if (tauri?.invoke) {
			void tauri
				.invoke('plugin:window|close', { label: 'main' })
				.catch(() => window.close());
			return;
		}
		window.close();
	}

	/** Tile that belongs to the current address: exact hit before prefix. */
	const activeLabels = $derived.by(() => {
		const exact = allTools.filter((t) => t.href === current).map((t) => t.label);
		if (exact.length) return exact;
		return allTools.filter(
			(t) => t.href && t.href !== '/' && !t.href.includes('?') && current.startsWith(`${t.href}/`)
		).map((t) => t.label);
	});

	/**
	 * Opens the submenu of an entry at its right edge. `position: fixed`,
	 * because the opened list itself scrolls and an absolute submenu would
	 * otherwise be cut off at its edge.
	 */
	function untermenueOeffnen(label: string, element: HTMLElement) {
		const box = element.getBoundingClientRect();
		submenu = { label, left: Math.round(box.right), top: Math.round(box.top) };
	}

	/** Closes the open menu as soon as the click lands outside the menu bar. */
	function onWindowClick(event: MouseEvent) {
		if (isTenantOpen) {
			const target = event.target;
			if (!(mandantEl && target instanceof Node && mandantEl.contains(target))) isTenantOpen = false;
		}
		if (!open) return;
		if (navEl && event.target instanceof Node && navEl.contains(event.target)) return;
		open = null;
	}

	function onWindowKey(event: KeyboardEvent) {
		// `Ctrl+Q` beendet die Anwendung — der Kürzel steht so im Menü `Datei`.
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'q') {
			event.preventDefault();
			void beenden();
			return;
		}
		if (event.key !== 'Escape') return;
		isTenantOpen = false;
		close();
	}
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKey} />

{#snippet cube(cx: number, cy: number, s: number)}
	<path d="M{cx} {cy - s}L{cx + s} {cy - s / 2}L{cx} {cy}L{cx - s} {cy - s / 2}Z" fill="currentColor" />
	<path
		d="M{cx - s} {cy - s / 2}L{cx} {cy}L{cx} {cy + s}L{cx - s} {cy + s / 2}Z"
		fill="currentColor"
		opacity=".78"
	/>
	<path
		d="M{cx + s} {cy - s / 2}L{cx} {cy}L{cx} {cy + s}L{cx + s} {cy + s / 2}Z"
		fill="currentColor"
		opacity=".6"
	/>
{/snippet}

{#snippet symbol(name: string, tone: Tone)}
	<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
		{#if name === 'calendar'}
			<rect x="3" y="5" width="18" height="16" rx="2" fill="none" stroke="currentColor" stroke-width="1.7" />
			<path d="M3 9.6h18M7.5 3v3.2M16.5 3v3.2" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
			<text x="12" y="18.1" text-anchor="middle" font-size="8.4" font-weight="700" fill="currentColor">15</text>
		{:else if name === 'search'}
			<circle cx="10.4" cy="10.4" r="6.4" fill="none" stroke="currentColor" stroke-width="2.2" />
			<path d="M15.3 15.3 20.8 20.8" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" />
		{:else if name === 'people'}
			<circle cx="9" cy="8.2" r="3.3" fill="currentColor" />
			<path d="M2.7 19.7c0-3.4 2.8-5.6 6.3-5.6s6.3 2.2 6.3 5.6z" fill="currentColor" />
			<circle cx="17" cy="9.5" r="2.6" fill="currentColor" opacity=".7" />
			<path d="M13.7 19.7c.3-2.7 2-4.4 4.4-4.4 2 0 3.5 1.2 4 3.1z" fill="currentColor" opacity=".7" />
		{:else if name === 'person-add'}
			<circle cx="9.4" cy="7.8" r="3.6" fill="currentColor" />
			<path d="M2.8 19.9c0-3.6 3-5.9 6.6-5.9 1.5 0 2.9.4 4 1.1" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" />
			<path d="M17.6 13.8v6.2M14.5 16.9h6.2" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" />
		{:else if name === 'boxes' || name === 'boxes-add' || name === 'availability'}
			{#if name === 'boxes'}
				{@render cube(12, 7.4, 4.3)}
				{@render cube(7, 15.6, 3.9)}
				{@render cube(17, 15.6, 3.9)}
			{:else}
				{@render cube(9.4, 6.8, 3.8)}
				{@render cube(5.6, 13.8, 3.4)}
				{@render cube(13.4, 13.8, 3.4)}
			{/if}
			{#if name === 'boxes-add'}
				<path d="M18.9 14.4v6.4M15.7 17.6h6.4" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" />
			{/if}
			{#if name === 'availability'}
				<rect x="14.4" y="2" width="8.6" height="7.4" rx="1.7" fill="currentColor" />
				<path d="M18.7 4.5v.2M18.7 6.1v2.2" stroke={TONES[tone]} stroke-width="1.6" stroke-linecap="round" />
			{/if}
		{:else if name === 'serial'}
			<rect x="2.4" y="3.6" width="19.2" height="16.8" rx="2" fill="currentColor" />
			<text x="12" y="10.8" text-anchor="middle" font-size="7" font-weight="700" fill={TONES[tone]}>{t('S/N:')}</text>
			<path
				d="M5 13.4v3.6M6.9 13.4v3.6M8.8 13.4v3.6M10.7 13.4v3.6M12.6 13.4v3.6M14.5 13.4v3.6M16.4 13.4v3.6M18.3 13.4v3.6"
				stroke={TONES[tone]}
				stroke-width="1.2"
			/>
		{:else if name === 'document'}
			<rect x="4.4" y="2.6" width="15.2" height="18.8" rx="1.6" fill="currentColor" />
			<path d="M7.5 7h9.2M7.5 10.4h9.2M7.5 13.8h6" stroke={TONES[tone]} stroke-width="1.5" stroke-linecap="round" />
		{:else if name === 'packing'}
			<path d="M4.6 2.6h14.8v15.8l-2.4-1.6-2.5 1.6-2.5-1.6-2.5 1.6-2.5-1.6-2.4 1.6z" fill="currentColor" />
			<path d="M7.6 6.6h8.8M7.6 9.9h8.8M7.6 13.2h5.6" stroke={TONES[tone]} stroke-width="1.4" stroke-linecap="round" />
		{:else if name === 'conflict'}
			<path d="M9.4 2.6h5.2l-.7 2.6H10.1z" fill="currentColor" />
			<path
				d="M12 5.4l1.7 2.9 3.2-1.2-.9 3.3 3.2 1.1-2.4 2.3 2.4 2.3-3.2 1 .9 3.3-3.2-1.2-1.7 2.9-1.7-2.9-3.2 1.2.9-3.3-3.2-1 2.4-2.3-2.4-2.3 3.2-1.1-.9-3.3 3.2 1.2z"
				fill="currentColor"
				opacity=".92"
			/>
		{:else if name === 'book'}
			<rect x="4.4" y="2.8" width="15.2" height="18.4" rx="1.4" fill="none" stroke="currentColor" stroke-width="1.7" />
			<path d="M12 6.2v6.6M8.7 9.5h6.6M7.6 16.6h8.8" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
		{:else if name === 'book-add'}
			<rect x="3.4" y="2.4" width="13.4" height="16.4" rx="1.4" fill="none" stroke="currentColor" stroke-width="1.6" />
			<path d="M6.4 6.4h7.4M6.4 9.8h7.4M6.4 13.2h4.8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
			<path d="M18.4 13.6v6.2M15.3 16.7h6.2" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
		{:else if name === 'house'}
			<path d="M12 2.4 21.8 11h-2.6v10.2H4.8V11H2.2z" fill="currentColor" />
			<path d="M10 21.2v-5.4h4v5.4z" fill={TONES[tone]} />
		{:else if name === 'refresh'}
			<path d="M20.4 12a8.4 8.4 0 1 1-2.6-6" fill="none" stroke="currentColor" stroke-width="2.1" />
			<path d="M20.9 3.4v5.3h-5.3z" fill="currentColor" />
		{:else if name === 'user'}
			<circle cx="12" cy="8.4" r="3.7" fill="currentColor" />
			<path d="M4.9 20.3c0-3.7 3.2-6.3 7.1-6.3s7.1 2.6 7.1 6.3z" fill="currentColor" />
		{:else if name === 'logout'}
			<path
				d="M10.2 4.4H6.4A1.9 1.9 0 0 0 4.5 6.3v11.4a1.9 1.9 0 0 0 1.9 1.9h3.8"
				fill="none"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
			/>
			<path
				d="M12.4 8.1 16.6 12l-4.2 3.9M16.2 12H8.6"
				fill="none"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
				stroke-linejoin="round"
			/>
		{/if}
	</svg>
{/snippet}

<header class="top">
	<!-- Logo: only the picture mark, on light ground, not clickable. -->
	<span class="brand" aria-hidden="true">
		<svg class="brand-image" viewBox="250 150 760 700">
			<image href={logo} width="1254" height="1254" />
		</svg>
	</span>

	<nav class="menu-band" aria-label={t('Hauptmenü')} bind:this={navEl}>
		{#each visibleMenus as menu (menu.label)}
			<div class="menu">
				<button
					type="button"
					class="menu-button"
					class:open={open === menu.label}
					aria-haspopup="true"
					aria-expanded={open === menu.label}
					onclick={() => toggle(menu.label)}
				>
					{t(menu.label)}
				</button>
				{#if open === menu.label}
					<ul class="list" role="menu">
						{#each menu.items.filter((item) => item.kind !== 'item' || hasRight(item.need)) as item, i (i)}
							{#if item.kind === 'separator'}
								<li class="separator" role="separator"></li>
							{:else if item.kind === 'heading'}
								<li class="group" role="presentation">{t(item.label)}</li>
							{:else}
								<li role="none">
									{#if item.href}
										<a role="menuitem" href={item.href} onclick={close}>
											<span class="text">{t(item.label)}</span>
											{#if item.shortcut}<span class="shortcut">{item.shortcut}</span>{/if}
										</a>
									{:else if item.action}
										<button
											class="entry"
											role="menuitem"
											type="button"
											onclick={() => {
												close();
												void runAction(item.action!);
											}}
										>
											<span class="text">{t(item.label)}</span>
											{#if item.shortcut}<span class="shortcut">{item.shortcut}</span>{/if}
										</button>
									{:else if item.children}
										<!-- Submenu: opens on hover to the right of the entry. -->
										<div
											class="submenu"
											class:open={submenu?.label === item.label}
											role="none"
											onmouseenter={(event) => untermenueOeffnen(item.label, event.currentTarget)}
											onmouseleave={() => (submenu = null)}
										>
											<button
												class="entry"
												class:expanded={submenu?.label === item.label}
												role="menuitem"
												type="button"
												aria-haspopup="true"
												aria-expanded={submenu?.label === item.label}
												onclick={(event) =>
													submenu?.label === item.label
														? (submenu = null)
														: untermenueOeffnen(item.label, event.currentTarget.parentElement ?? event.currentTarget)}
											>
												<span class="text">{t(item.label)}</span>
												<span class="arrow" aria-hidden="true"></span>
											</button>
											{#if submenu?.label === item.label}
												<ul
													class="list nested"
													role="menu"
													style="left: {submenu.left}px; top: {submenu.top}px"
												>
													{#each item.children as unter (unter.kind === 'item' ? unter.label : 't')}
														{#if unter.kind === 'separator'}
															<li class="separator" role="separator"></li>
														{:else if unter.kind === 'heading'}
															<li class="group" role="presentation">{t(unter.label)}</li>
														{:else if unter.action}
															<li role="none">
																<button
																	class="entry"
																	role="menuitem"
																	type="button"
																	onclick={() => {
																		close();
																		submenu = null;
																		void runAction(unter.action!);
																	}}
																>
																	<span class="text">{t(unter.label)}</span>
																</button>
															</li>
														{:else if unter.href}
															<li role="none">
																<a role="menuitem" href={unter.href} onclick={close}>
																	<span class="text">{t(unter.label)}</span>
																</a>
															</li>
														{:else}
															<li role="none">
																<span class="entry disabled" role="menuitem" aria-disabled="true">
																	<span class="text">{t(unter.label)}</span>
																</span>
															</li>
														{/if}
													{/each}
												</ul>
											{/if}
										</div>
									{:else}
										<span
											class="entry disabled"
											role="menuitem"
											aria-disabled="true"
											title={item.locked ? 'nicht lizenziert' : 'noch nicht umgesetzt'}
										>
											<span class="text">{t(item.label)}</span>
											{#if item.shortcut}<span class="shortcut">{item.shortcut}</span>{/if}
											{#if item.locked}
												<span class="lock" aria-label={t('nicht lizenziert')}>
													<svg viewBox="0 0 24 24" width="9" height="9" aria-hidden="true">
														<path
															d="M8 10V7a4 4 0 0 1 8 0v3"
															fill="none"
															stroke="currentColor"
															stroke-width="2.4"
														/>
														<rect x="5" y="10" width="14" height="10" rx="2" fill="currentColor" />
													</svg>
												</span>
											{/if}
											{#if item.submenu}<span class="arrow" aria-hidden="true"></span>{/if}
										</span>
									{/if}
								</li>
							{/if}
						{/each}
					</ul>
				{/if}
			</div>
		{/each}
	</nav>

	<div class="right">
		<button
			type="button"
			class="theme-toggle"
			role="switch"
			aria-checked={isDark()}
			title={isDark() ? 'Helles Design einschalten' : 'Dunkles Design einschalten'}
			onclick={toggleTheme}
		>
			<span class="track" class:on={isDark()}>
				<span class="thumb"></span>
			</span>
			<span class="theme-text">{t('Design')}</span>
		</button>

		<span class="user" title={t('Angemeldeter Benutzer')}>
			<span class="user-icon">
				{#if auth.user?.color}
					<span class="user-dot" style={`background:${auth.user.color}`}></span>
				{:else}
					{@render symbol('user', 'slate')}
				{/if}
			</span>
			<span class="user-name">{auth.label}</span>
			{#if auth.user?.mandant_name}
				<div class="tenant-field" bind:this={mandantEl}>
					{#if switchableTenants.length > 1}
						<button
							type="button"
							class="tenant switchable"
							class:open={isTenantOpen}
							title={t('Mandant wechseln')}
							aria-haspopup="listbox"
							aria-expanded={isTenantOpen}
							onclick={() => (isTenantOpen = !isTenantOpen)}
						>
							{auth.user.mandant_name}
							<span class="arrow" aria-hidden="true">▾</span>
						</button>
						{#if isTenantOpen}
							<ul class="tenant-list" role="listbox" aria-label={t('Mandant wählen')}>
								{#each switchableTenants as mandant (mandant.id)}
									<li>
										<button
											type="button"
											role="option"
											aria-selected={mandant.id === auth.user?.mandant}
											class:active={mandant.id === auth.user?.mandant}
											onclick={() => changeTenant(mandant.id)}
										>
											<span>{mandant.name}</span>
											<span class="short">[{mandant.short_name}]</span>
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					{:else}
						<span class="tenant" title={t('Mandant der Anmeldung')}>{auth.user.mandant_name}</span>
					{/if}
				</div>
			{/if}
		</span>
		<button
			type="button"
			class="sign-out"
			title={t('Abmelden und zur Anmeldemaske zurückkehren')}
			onclick={signOut}
		>
			{@render symbol('logout', 'slate')}
			<span class="sign-out-text">{t('Abmelden')}</span>
		</button>
	</div>
</header>

<nav class="sidebar" aria-label={t('Schnellzugriff')}>
	{#each visibleTools as tool (tool.label)}
		{#if tool.href}
			<a
				class="tile"
				class:active={activeLabels.includes(tool.label)}
				style="--ton: {TONES[tool.tone]}"
				href={tool.href}
				title={t(tool.label)}
			>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</a>
		{:else if tool.action === 'refresh'}
			<button type="button" class="tile refresh" title={t(tool.label)} onclick={reload}>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</button>
		{:else if tool.action === 'search'}
			<button
				type="button"
				class="tile"
				style="--ton: {TONES[tool.tone]}"
				title={t(tool.label)}
				onclick={openSearch}
			>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</button>
		{:else if tool.action === 'address-search'}
			<button
				type="button"
				class="tile"
				style="--ton: {TONES[tool.tone]}"
				title="{t(tool.label)} — {t('Suchmaske öffnen')}"
				onclick={openAddressSearch}
			>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</button>
		{:else if tool.action === 'article-search'}
			<button
				type="button"
				class="tile"
				style="--ton: {TONES[tool.tone]}"
				title="{t(tool.label)} — {t('Suchmaske öffnen')}"
				onclick={openArticleSearch}
			>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</button>
		{:else if tool.action === 'address-create'}
			<button
				type="button"
				class="tile"
				style="--ton: {TONES[tool.tone]}"
				title="{t(tool.label)} — {t('neue Adresse anlegen')}"
				onclick={openAddressCreate}
			>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</button>
		{:else if tool.action === 'article-create'}
			<button
				type="button"
				class="tile"
				style="--ton: {TONES[tool.tone]}"
				title="{t(tool.label)} — {t('neuen Artikel anlegen')}"
				onclick={openArticleCreate}
			>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</button>
		{:else}
			<span
				class="tile disabled"
				style="--ton: {TONES[tool.tone]}"
				title="{t(tool.label)} — {t('noch nicht umgesetzt')}"
				aria-disabled="true"
			>
				<span class="symbol">{@render symbol(tool.icon, tool.tone)}</span>
				<span class="caption">{t(tool.label)}</span>
			</span>
		{/if}
	{/each}
</nav>

{#if personalSettings.open}
	<PersonalSettingsDialog />
{/if}

<style>
	/* Header: logo, menu band, on the right the design switch and the user. */
	.top {
		grid-area: page-head;
		position: sticky;
		top: 0;
		z-index: 50;
		display: flex;
		align-items: stretch;
		gap: 0.45rem;
		height: 2.05rem;
		padding: 0 0.55rem 0 0.4rem;
		background: linear-gradient(var(--menu-band-1), var(--menu-band-2));
		border-bottom: 1px solid var(--menu-band-border);
		box-shadow: 0 1px 6px rgb(15 23 42 / 18%);
	}
	.brand {
		align-self: center;
		display: grid;
		place-items: center;
		flex: none;
		/* Center of the picture mark on the center of the icons in the sidebar:
		   0.3rem (bar) + 0.4rem (tile) + 1.55rem/2 − 1.5rem/2 = 0.325rem */
		margin-left: 0.325rem;
		width: 1.5rem;
		height: 1.5rem;
		border-radius: 0.2rem;
		background: #fff;
		box-shadow: inset 0 0 0 1px var(--menu-band-border);
		overflow: hidden;
	}
	.brand-image {
		width: 1.25rem;
		height: 1.25rem;
	}

	.menu-band {
		display: flex;
		align-items: stretch;
		gap: 0.1rem;
	}
	.right {
		display: flex;
		align-items: center;
		gap: 0.8rem;
		margin-left: auto;
	}
	.menu {
		position: relative;
		display: flex;
	}
	.menu-button {
		border: 0;
		background: transparent;
		padding: 0 0.6rem;
		font: inherit;
		font-size: 0.84rem;
		color: var(--text);
		cursor: pointer;
	}
	.menu-button:hover {
		background: var(--menu-band-hover);
	}
	.menu-button.open {
		background: var(--menu-open);
	}
	/* Drop-down list: opaque like the original (img/menu/*.PNG) — no transparent background. */
	.list {
		position: absolute;
		top: 100%;
		left: 0;
		z-index: 60;
		min-width: 13rem;
		max-height: calc(100vh - 3.2rem);
		overflow-y: auto;
		scrollbar-width: thin;
		overscroll-behavior: contain;
		margin: 0;
		padding: 0.1rem 0;
		list-style: none;
		background-color: var(--menu-surface);
		border: 1px solid var(--menu-border);
		border-radius: 0;
		box-shadow: 0 5px 14px rgb(15 23 42 / 24%);
	}
	.list a,
	.list .entry {
		display: flex;
		align-items: center;
		min-height: 1.5rem;
		padding: 0 0.55rem 0 2.1rem;
		font-size: 0.79rem;
		color: var(--text);
		text-decoration: none;
		white-space: nowrap;
	}
	.list a:hover {
		background: var(--menu-hover);
	}
	/* Menu entries that open a window instead of navigating. */
	.list button.entry {
		width: 100%;
		border: 0;
		background: transparent;
		font: inherit;
		font-size: 0.79rem;
		text-align: left;
		cursor: pointer;
	}
	.list button.entry:hover {
		background: var(--menu-hover);
	}
	.list .group {
		padding: 0.3rem 0.55rem 0.15rem 0.7rem;
		font-size: 0.76rem;
		font-weight: 700;
		color: var(--muted);
		white-space: nowrap;
	}
	.list .text {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.list .shortcut {
		margin-left: auto;
		padding-left: 1.6rem;
		font-size: 0.75rem;
		color: var(--menu-shortcut);
	}
	.list .arrow {
		display: inline-flex;
		align-items: center;
		margin-left: auto;
		padding-left: 1.4rem;
	}
	.list .arrow::before {
		content: '';
		width: 0.34rem;
		height: 0.34rem;
		border-top: 1.5px solid var(--menu-arrow);
		border-right: 1.5px solid var(--menu-arrow);
		transform: rotate(45deg);
	}
	.list .separator {
		height: 1px;
		margin: 0.2rem 0;
		background: var(--menu-separator);
	}
	/* Without a target: text as in the original, but without hover and without a pointer. */
	.list .disabled {
		cursor: default;
	}
	/* Lock of the unlicensed modules, as in the original behind the name. */
	.list .lock {
		display: inline-flex;
		align-items: center;
		margin-left: 0.3rem;
		color: var(--menu-arrow);
	}

	/* Submenu (`Datei → Aktion`): floats to the right of the entry, above the list. */
	.submenu {
		position: relative;
	}

	.list .submenu .entry.expanded {
		background: var(--menu-hover);
	}

	.list.nested {
		position: fixed;
		z-index: 70;
		min-width: 12rem;
	}

	/* Page column: the same surface as the header, only vertical. */
	.sidebar {
		grid-area: sidebar;
		position: sticky;
		top: 2.05rem;
		z-index: 40;
		align-self: start;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		width: 9.7rem;
		/* The head above is 2.05rem plus its one-pixel line; without the line the bar
		   pushes the page one pixel past the window and every page gets a scrollbar. */
		height: calc(100vh - 2.05rem - 1px);
		overflow-y: auto;
		scrollbar-width: thin;
		overscroll-behavior: contain;
		padding: 0.4rem 0.3rem 0.6rem;
		background: linear-gradient(var(--menu-band-1), var(--menu-band-2));
		border-right: 1px solid var(--menu-band-border);
		box-sizing: border-box;
	}
	.tile {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		width: 100%;
		padding: 0.22rem 0.4rem;
		border: 0;
		border-radius: 0.3rem;
		background: transparent;
		font: inherit;
		font-size: 0.74rem;
		color: var(--text);
		text-align: left;
		text-decoration: none;
		box-sizing: border-box;
	}
	.tile .symbol {
		display: grid;
		place-items: center;
		flex: none;
		width: 1.55rem;
		height: 1.55rem;
		border-radius: 0.32rem;
		background: var(--ton);
		box-shadow: inset 0 1px 0 rgb(255 255 255 / 22%);
		color: #fff;
	}
	.tile svg {
		width: 1.1rem;
		height: 1.1rem;
	}
	.tile .caption {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	a.tile:hover,
	button.tile:hover {
		background: var(--menu-band-hover);
		cursor: pointer;
	}
	a.tile:hover .symbol,
	button.tile:hover .symbol {
		filter: brightness(1.06);
	}
	.tile.active {
		background: var(--menu-band-hover);
		box-shadow: inset 2px 0 0 var(--accent);
	}
	.tile.disabled {
		opacity: 0.45;
		cursor: default;
	}
	.tile.refresh .symbol {
		background: var(--surface-3);
		box-shadow: inset 0 0 0 1px var(--border);
		color: var(--text);
	}

	/* Design switch in the header. */
	.theme-toggle {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0 0.15rem;
		border: 0;
		background: transparent;
		font: inherit;
		font-size: 0.76rem;
		color: var(--text);
		cursor: pointer;
	}
	.track {
		position: relative;
		display: block;
		width: 1.85rem;
		height: 1rem;
		border-radius: 1rem;
		background: var(--menu-band-border);
		box-shadow: inset 0 1px 2px rgb(15 23 42 / 22%);
		transition: background 0.15s ease;
	}
	.track.on {
		background: var(--accent);
	}
	.thumb {
		position: absolute;
		top: 0.13rem;
		left: 0.13rem;
		width: 0.74rem;
		height: 0.74rem;
		border-radius: 50%;
		background: #fff;
		box-shadow: 0 1px 2px rgb(0 0 0 / 35%);
		transition: transform 0.15s ease;
	}
	.track.on .thumb {
		transform: translateX(0.85rem);
	}

	.user {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding-left: 0.8rem;
		border-left: 1px solid var(--menu-band-border);
		font-size: 0.78rem;
		color: var(--text);
		white-space: nowrap;
	}
	.user-icon {
		display: grid;
		place-items: center;
		width: 1.35rem;
		height: 1.35rem;
		border-radius: 50%;
		background: var(--surface-3);
		box-shadow: inset 0 0 0 1px var(--border);
	}
	.user-icon svg {
		width: 0.95rem;
		height: 0.95rem;
	}
	.user-dot {
		width: 0.75rem;
		height: 0.75rem;
		border-radius: 50%;
		box-shadow: 0 0 0 1px var(--border);
	}
	/* Tenant of the login: subtle behind the name. */
	.tenant {
		margin-left: 0.15rem;
		padding: 0.05rem 0.4rem;
		border-radius: 0.6rem;
		background: var(--surface-3);
		color: var(--muted);
		font-size: 0.7rem;
	}
	.tenant-field {
		position: relative;
		display: flex;
		align-items: center;
	}
	/* Several tenants: the chip becomes a choice. */
	.tenant.switchable {
		border: 1px solid transparent;
		font: inherit;
		font-size: 0.7rem;
		cursor: pointer;
	}
	.tenant.switchable:hover,
	.tenant.switchable.open {
		border-color: var(--menu-band-border);
		color: var(--text);
	}
	.tenant .arrow {
		margin-left: 0.2rem;
		font-size: 0.6rem;
	}
	.tenant-list {
		position: absolute;
		top: calc(100% + 0.3rem);
		right: 0;
		z-index: 40;
		min-width: 12rem;
		margin: 0;
		padding: 0.2rem;
		list-style: none;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		box-shadow: 0 6px 18px rgb(0 0 0 / 22%);
	}
	.tenant-list button {
		display: flex;
		gap: 0.4rem;
		align-items: baseline;
		justify-content: space-between;
		width: 100%;
		padding: 0.25rem 0.4rem;
		border: 1px solid transparent;
		border-radius: 3px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.75rem;
		text-align: left;
		cursor: pointer;
	}
	.tenant-list button:hover {
		background: var(--surface-2);
	}
	.tenant-list button.active {
		background: var(--highlight);
		border-color: var(--accent);
	}
	.tenant-list .short {
		color: var(--muted-2);
		font-size: 0.7rem;
	}
	.sign-out {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		margin-left: 0.2rem;
		padding: 0.2rem 0.5rem;
		border: 1px solid var(--menu-band-border);
		border-radius: 4px;
		background: var(--menu-band-1);
		color: var(--text);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;
	}
	.sign-out:hover {
		background: var(--menu-band-hover);
	}
	.sign-out svg {
		width: 0.95rem;
		height: 0.95rem;
	}

	@media (max-width: 1120px) {
		.menu-button {
			padding: 0 0.4rem;
			font-size: 0.8rem;
		}
		.theme-text {
			display: none;
		}
	}
	@media (max-width: 900px) {
		.user-name {
			display: none;
		}
		.brand {
			display: none;
		}
		.menu-button {
			padding: 0 0.26rem;
			font-size: 0.72rem;
		}
		.right {
			gap: 0.5rem;
		}
	}
</style>
