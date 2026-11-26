<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type SettingsArea,
		type SettingsField,
		type SettingsList,
		type SettingsRow
	} from '../../../lib/api';
	import SettingsDialog from '../../../lib/settings/SettingsDialog.svelte';
	import { modules } from '../../../lib/modules.svelte';

	let areas = $state<SettingsArea[]>([]);
	let lists = $state<Record<string, SettingsList>>({});
	let load = $state(true);
	let error = $state<string | null>(null);
	let group = $state<string>('Alle');
	let dialog = $state<{ area: SettingsArea; row: SettingsRow | null } | null>(null);

	const GROUPS = ['Alle', 'Adressen', 'Artikel', 'Lager', 'System', 'Buchhaltung', 'Projekt', 'Administration'];

	/** Where the list takes effect in daily business — the bridge to the existing part of the app. */
	const REFERENCE: Record<string, { href: string; label: string }> = {
		salutations: { href: '/addresses', label: t('Adressliste') },
		adressgruppen: { href: '/addresses', label: t('Adresssuche') },
		qualifications: { href: '/addresses', label: t('Adressen') },
		manufacturer: { href: '/articles', label: t('Artikelliste') },
		eigentuemer: { href: '/articles', label: t('Artikelliste') },
		articleState: { href: '/articles', label: t('Artikelmaske') },
		preisklassen: { href: '/articles', label: t('Miete/Verkauf') },
		lager: { href: '/articles', label: t('Lagerbestand') },
		lagerplaetze: { href: '/articles', label: t('Lagerbestand') },
		languages: { href: '/articles', label: t('Artikeltexte') },
		status: { href: '/agenda', label: t('Kalender') },
		projects: { href: '/agenda', label: t('Kalender') },
		tenants: { href: '/', label: t('Zentrale') }
	};
	const KEYS = ['text', 'integer', 'number', 'bool', 'date'] as const;

	/** The accounting lists belong to `WHERE - is my money?`. */
	const groups = $derived(
		GROUPS.filter(
			(name) =>
				name !== 'Projekt' &&
				(name !== 'Buchhaltung' || modules.isInstalled('money'))
		)
	);
	const visibleAreas = $derived(
		areas.filter(
			(area) =>
				area.group !== 'Projekt' &&
				(area.key !== 'versandarten' || modules.isInstalled('go')) &&
				(area.group !== 'Buchhaltung' || modules.isInstalled('money')) &&
				(area.key !== 'preisklassen' || modules.isInstalled('go'))
		)
	);

	const visible = $derived(
		(group === 'Alle'
			? visibleAreas
			: visibleAreas.filter((area) => area.group === group)
		).filter((area) => !OWN_PAGE.includes(area.key))
	);

	/** Lists that have their own page (`/admin/addresses`, `/admin/articles`) — not duplicated here. */
	const OWN_PAGE = [
		'anreden',
		'anreden-kurz',
		'adressgruppen',
		'qualifikationen',
		'status-location',
		'kategorie-location',
		'stromanschluesse',
		'merkmale-location',
		'hersteller',
		'eigentuemer',
		'artikelzustand',
		'provisionsklassen',
		'artikel-suchindex',
		'banken',
		// Warehouses and storage bins have their own mask (`/admin/warehouses`).
		'lager',
		'lagerplaetze'
	];

	async function loadAll() {
		load = true;
		error = null;
		try {
			areas = await api.settings.areas();
			const loaded = await Promise.all(
				areas.map(async (area) => [area.key, await api.settings.list(area.key)] as const)
			);
			lists = Object.fromEntries(loaded);
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	onMount(async () => {
		await loadAll();
		springeZumAnker();
	});

	/** A menu entry of the admin menu links to `#<key>`; the card only arrives with
	 *  the data, so the jump happens after loading and it is briefly highlighted. */
	let target = $state<string | null>(null);

	function springeZumAnker() {
		const key = location.hash.replace('#', '');
		if (!key) return;
		requestAnimationFrame(() => {
			const card = document.getElementById(key);
			if (!card) return;
			card.scrollIntoView({ block: 'start' });
			target = key;
			setTimeout(() => (target = null), 2000);
		});
	}

	function zaehle(key: string): number {
		return lists[key]?.rows.length ?? 0;
	}

	function value(row: SettingsRow, field: SettingsField): string {
		const value = row.values[field.column];
		if (value === null || value === undefined || value === '') return '—';
		if (field.kind === 'bool') return value === 'true' ? t('Ja') : t('Nein');
		if (field.kind === 'date') {
			const [year, month, day] = value.split('-');
			return year ? `${day}.${month}.${year}` : value;
		}
		if (field.kind === 'number') return value.replace('.', ',');
		return value;
	}

	function changed() {
		dialog = null;
		void loadAll();
	}
</script>

<svelte:head><title>{t('Einstellungen — WHERE')}</title></svelte:head>

<h1>{t('Einstellungen')}</h1>

<div class="page">
	<nav class="group-nav" aria-label={t('Bereiche')}>
		{#each groups as name (name)}
			<button
				type="button"
				class:active={group === name}
				aria-pressed={group === name}
				onclick={() => (group = name)}>{t(name)}</button
			>
		{/each}
	</nav>

	<section class="lists">
		{#each visible as area (area.key)}
			<article class="list" id={area.key} class:target={target === area.key}>
				<header>
					<div>
						<h2>{t(area.title)}</h2>
						<p class="description">{t(area.description)}</p>
					</div>
					<div class="head-right">
						{#if REFERENCE[area.key]}
							<a class="link" href={REFERENCE[area.key].href}>
								→ {t(REFERENCE[area.key].label)}
							</a>
						{/if}
						<span class="table-name" title={t('Tabelle der Altanwendung')}>{area.table}</span>
						<button type="button" class="new" onclick={() => (dialog = { area: area, row: null })}>
							{t('+ Neuer Eintrag')}
						</button>
					</div>
				</header>

				<table>
					<thead>
						<tr>
							{#each area.fields as field (field.column)}
								<th class:numeric={field.kind === 'integer' || field.kind === 'number'}>
									{t(field.label)}
								</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each lists[area.key]?.rows ?? [] as row (row.id)}
							<tr
								tabindex="0"
								role="button"
								onclick={() => (dialog = { area: area, row })}
								onkeydown={(e) => {
									if (e.key === 'Enter' || e.key === ' ') dialog = { area: area, row };
								}}
							>
								{#each area.fields as field (field.column)}
									<td class:numeric={field.kind === 'integer' || field.kind === 'number'}>
										{value(row, field)}
									</td>
								{/each}
							</tr>
						{:else}
							<tr class="empty">
								<td colspan={area.fields.length}>
									{load
										? t('lade …')
										: t('Noch kein Eintrag — „+ Neuer Eintrag“ legt den ersten an.')}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
				<p class="foot">
					{zaehle(area.key)}
					{zaehle(area.key) === 1 ? t('Eintrag') : t('Einträge')}
				</p>
			</article>
		{:else}
			<p class="empty">{t('Keine Liste in diesem Bereich.')}</p>
		{/each}
	</section>
</div>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if dialog}
	<SettingsDialog
		area={dialog.area}
		row={dialog.row}
		onsaved={changed}
		oncancel={() => (dialog = null)}
	/>
{/if}

<style>
	.error {
		margin: 0 0 1rem;
		padding: 0.5rem 0.7rem;
		border: 1px solid var(--bad);
		border-radius: 4px;
		color: var(--bad);
		background: color-mix(in srgb, var(--bad) 8%, var(--surface));
	}
	.page {
		display: grid;
		grid-template-columns: 10.5rem minmax(0, 1fr);
		gap: 1rem;
		align-items: start;
	}
	@media (max-width: 900px) {
		.page {
			grid-template-columns: minmax(0, 1fr);
		}
		.group-nav {
			flex-direction: row;
			flex-wrap: wrap;
		}
	}
	.group-nav {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		position: sticky;
		top: 2.6rem;
	}
	.group-nav button {
		padding: 0.25rem 0.5rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.85rem;
		text-align: left;
		cursor: pointer;
	}
	.group-nav button:hover {
		background: var(--surface-2);
	}
	.group-nav button.active {
		background: var(--highlight);
		border-color: var(--accent);
	}
	.lists {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-width: 0;
	}
	.list {
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
		overflow: hidden;
		scroll-margin-top: 3rem;
	}
	.list.target {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 35%, transparent);
	}
	.list header {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
		justify-content: space-between;
		padding: 0.7rem 0.85rem 0.5rem;
		border-bottom: 1px solid var(--border);
	}
	h2 {
		font-size: 1rem;
		margin: 0;
	}
	.description {
		margin: 0.15rem 0 0;
		font-size: 0.8rem;
		color: var(--muted);
		max-width: 44rem;
	}
	.head-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		white-space: nowrap;
	}
	.table-name {
		font-family: var(--mono, ui-monospace, monospace);
		font-size: 0.72rem;
		color: var(--muted-2);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0.05rem 0.3rem;
	}
	.new {
		padding: 0.25rem 0.5rem;
		border: 1px solid var(--accent);
		border-radius: 4px;
		background: none;
		color: var(--accent);
		font: inherit;
		font-size: 0.8rem;
		cursor: pointer;
	}
	.new:hover {
		background: var(--highlight);
	}
	.link {
		font-size: 0.76rem;
		color: var(--accent);
		text-decoration: none;
	}
	.link:hover {
		text-decoration: underline;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.83rem;
	}
	th,
	td {
		padding: 0.25rem 0.55rem;
		text-align: left;
		border-bottom: 1px solid var(--surface-3);
	}
	th {
		font-weight: 600;
		color: var(--muted);
		font-size: 0.76rem;
		background: var(--surface-2);
		white-space: nowrap;
	}
	th.numeric,
	td.numeric {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	tbody tr {
		cursor: pointer;
	}
	tbody tr:hover {
		background: var(--surface-2);
	}
	tbody tr:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}
	tr.empty {
		cursor: default;
	}
	tr.empty td {
		color: var(--muted-2);
		font-size: 0.8rem;
		padding: 0.6rem 0.55rem;
	}
	.foot {
		margin: 0;
		padding: 0.3rem 0.85rem 0.45rem;
		font-size: 0.75rem;
		color: var(--muted-2);
	}
	.empty {
		color: var(--muted);
	}
</style>
