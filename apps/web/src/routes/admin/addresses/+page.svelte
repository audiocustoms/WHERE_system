<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type SettingsArea } from '../../../lib/api';
	import { askCancel } from '../../../lib/confirm.svelte';
	import ListWindow from '../../../lib/settings/ListWindow.svelte';
	import '../../../lib/search/search-window.css';

	/** What a button of the mask opens. */
	type Target = { kind: 'liste'; area: string } | { kind: 'neukunde' } | { kind: 'maps' };

	interface Entry {
		label: string;
		target: Target;
	}

	interface Group {
		title: string;
		entries: Entry[];
	}

	/** Layout as in `img/Admin Menü/Adressen/Adresseneinstellungen.PNG`. */
	const GROUPS: Group[] = [
		{
			title: t('Adresse'),
			entries: [
				{ label: t('Anreden & Anredeformen'), target: { kind: 'liste', area: 'anreden' } },
				{ label: t('Adressgruppen'), target: { kind: 'liste', area: 'adressgruppen' } },
				{
					label: t('Einstell. für neue Adr./Knd./Lief.'),
					target: { kind: 'neukunde' }
				},
				{ label: t('Google Maps & Routenplanung'), target: { kind: 'maps' } },
				{
					label: t('Kundenseitige Preisklassenkategorien'),
					target: { kind: 'liste', area: 'preisklassen' },
				},
			],
		},
		{
			title: t('Adresse : Mitarbeiter'),
			entries: [
				{
					label: t('Mitarbeiterqualifikationen ändern…'),
					target: { kind: 'liste', area: 'qualifikationen' },
				},
			],
		},
		{
			title: t('Adresse : Location'),
			entries: [
				{
					label: t('Status Location ändern…'),
					target: { kind: 'liste', area: 'status-location' },
				},
				{
					label: t('Kategorie Location ändern…'),
					target: { kind: 'liste', area: 'kategorie-location' },
				},
				{
					label: t('Stromanschlüsse Location ändern…'),
					target: { kind: 'liste', area: 'stromanschluesse' },
				},
				{ label: t('Merkmale ändern…'), target: { kind: 'liste', area: 'merkmale-location' } },
			],
		},
	];

	/** Country for route planning; the list stays small and fixed. */
	const COUNTRIES = ['Deutschland', 'Österreich', 'Schweiz', 'Niederlande', 'Frankreich'];

	let areas = $state<SettingsArea[]>([]);
	let prefs = $state<Record<string, string>>({});
	let saved = $state<Record<string, string>>({});
	let error = $state<string | null>(null);
	let load = $state(true);
	let isOpen = $state<Target | null>(null);

	onMount(loadAll);

	async function loadAll() {
		load = true;
		error = null;
		try {
			areas = await api.settings.areas();
			const loaded = await api.prefs.load('address.');
			prefs = { ...loaded.values };
			saved = { ...loaded.values };
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	function bool(key: string): boolean {
		return prefs[key] === 'true';
	}

	function setzeBool(key: string, value: boolean) {
		prefs = { ...prefs, [key]: value ? 'true' : 'false' };
	}

	function setze(key: string, value: string) {
		prefs = { ...prefs, [key]: value };
	}

	const changed = $derived(
		Object.keys({ ...prefs, ...saved }).some(
			(key) => (prefs[key] ?? '') !== (saved[key] ?? '')
		)
	);

	async function commit() {
		error = null;
		try {
			for (const key of Object.keys({ ...prefs, ...saved })) {
				const previous = saved[key] ?? '';
				const now = prefs[key] ?? '';
				if (previous !== now) await api.prefs.save(key, now === '' ? null : now);
			}
			await loadAll();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function cancel() {
		if (changed && !(await askCancel(t('Das Bearbeiten der Adresseinstellungen')))) return;
		await loadAll();
	}

	function area(key: string): SettingsArea | null {
		return areas.find((area) => area.key === key) ?? null;
	}

	/** A button opens a window when the list behind it exists. */
	function oeffne(target: Target) {
		if (target.kind === 'liste' && !area(target.area)) {
			error = t('Die Liste „{area}“ fehlt in dieser Installation.', { area: target.area });
			return;
		}
		isOpen = target;
	}
</script>

<svelte:head><title>{t('Adresseinstellungen — WHERE')}</title></svelte:head>

<h1>{t('Adresseinstellungen')}</h1>

<section class="selection" aria-label={t('Adresseinstellungen')}>
	{#each GROUPS as group (group.title)}
		<h2>{group.title}</h2>
		{#each group.entries as entry (entry.label)}
			<button type="button" class="entry" onclick={() => oeffne(entry.target)}>
				{entry.label}
			</button>
		{/each}
	{/each}

	<h2>{t('Adresse : Briefanschrift')}</h2>
	<label class="check">
		<input
			type="checkbox"
			checked={bool('address.brief.anrede')}
			onchange={(e) => setzeBool('address.brief.anrede', e.currentTarget.checked)}
		/>
		{t('Bei Einzelpersonen Anrede ausgeben')}
	</label>
	<label class="check">
		<input
			type="checkbox"
			checked={bool('address.brief.leerzeile')}
			onchange={(e) => setzeBool('address.brief.leerzeile', e.currentTarget.checked)}
		/>
		{t('Leerzeile zwischen Straße und Ort einfügen')}
	</label>

	<h2>{t('Adresse : Listendarstellung Einzelperson')}</h2>
	<label class="check">
		<input
			type="radio"
			name="darstellung"
			checked={(prefs['address.darstellung.name'] ?? 'nachname') === 'nachname'}
			onchange={() => setze('address.darstellung.name', 'nachname')}
		/>
		{t('Nachname, Vorname „Spitzname“')}
	</label>
	<label class="check">
		<input
			type="radio"
			name="darstellung"
			checked={prefs['address.darstellung.name'] === 'vorname'}
			onchange={() => setze('address.darstellung.name', 'vorname')}
		/>
		{t('Vorname „Spitzname“ Nachname')}
	</label>
</section>

<footer class="foot">
	<label class="check">
		<input
			type="checkbox"
			checked={bool('address.indizes.nurHier')}
			onchange={(e) => setzeBool('address.indizes.nurHier', e.currentTarget.checked)}
		/>
		{t('Indizes nur hier änderbar')}
	</label>
	<span class="spacer"></span>
	<button type="button" class="btn" onclick={cancel} disabled={load}>{t('Abbrechen')}</button>
	<button type="button" class="btn primary" onclick={commit} disabled={!changed}>
		{t('Sichern')}
	</button>
</footer>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if isOpen?.kind === 'liste' && area(isOpen.area)}
	<ListWindow
		area={area(isOpen.area)!}
		onclose={() => (isOpen = null)}
		onchange={loadAll}
	/>
{:else if isOpen?.kind === 'neukunde'}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={t('Neukunde')}
			style="width: min(34rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{t('Einstellungen für neue Adressen, Kunden und Lieferanten')}</span>
				<button class="sw-close" type="button" title={t('Schließen')} onclick={() => (isOpen = null)}>×</button>
			</div>
			<div class="sw-content">
				<h3>{t('Adressen')} <span class="small">{t('(mandantenunabhängig)')}</span></h3>
				<label class="check">
					<input
						type="checkbox"
						checked={bool('address.neukunde.indexpflicht')}
						onchange={(e) => setzeBool('address.neukunde.indexpflicht', e.currentTarget.checked)}
					/>
					{t('Adresse muss zum Sichern einen Index besitzen')}
				</label>
				<label class="check">
					<input
						type="checkbox"
						checked={bool('address.neukunde.serienbrief')}
						onchange={(e) => setzeBool('address.neukunde.serienbrief', e.currentTarget.checked)}
					/>
					{t('„Teilnahme an Serienbriefversand“ aktivieren')}
				</label>
				<label class="check">
					<input
						type="checkbox"
						checked={bool('address.neukunde.serienmail')}
						onchange={(e) => setzeBool('address.neukunde.serienmail', e.currentTarget.checked)}
					/>
					{t('„Teilnahme an Serien-eMail“ aktivieren')}
				</label>
				<label class="check">
					<input
						type="checkbox"
						checked={bool('address.neukunde.suchindex')}
						onchange={(e) => setzeBool('address.neukunde.suchindex', e.currentTarget.checked)}
					/>
					{t('Suchindex automatisch hinterlegen')}
				</label>
				<div class="row">
					<input
						class="text"
						placeholder={t('Index')}
						value={prefs['address.neukunde.index'] ?? ''}
						oninput={(e) => setze('address.neukunde.index', e.currentTarget.value)}
					/>
					<span class="small">{t('Der Index wird beim Anlegen vorbelegt.')}</span>
				</div>

				<h3>{t('Kunden')} <span class="small">{t('(mandantenabhängig)')}</span></h3>
				<div class="row">
					<label class="check">
						<input
							type="checkbox"
							checked={bool('address.kunden.versicherung')}
							onchange={(e) => setzeBool('address.kunden.versicherung', e.currentTarget.checked)}
						/>
						{t('Artikel versichern mit')}
					</label>
					<input
						class="numeric"
						type="number"
						step="0.1"
						value={prefs['address.kunden.versicherungssatz'] ?? ''}
						oninput={(e) => setze('address.kunden.versicherungssatz', e.currentTarget.value)}
					/>
					<span class="small">%</span>
				</div>
			</div>
			<div class="sw-footer">
				<span class="spacer"></span>
				<button type="button" class="btn" onclick={() => (isOpen = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="btn primary"
					onclick={async () => {
						await commit();
						isOpen = null;
					}}>{t('Ok')}</button
				>
			</div>
		</div>
	</div>
{:else if isOpen?.kind === 'maps'}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={t('Google Maps')}
			style="width: min(34rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{t('Startadresse für Routenplanung')}</span>
				<button class="sw-close" type="button" title={t('Schließen')} onclick={() => (isOpen = null)}>×</button>
			</div>
			<div class="sw-content">
				<div class="row">
					<span class="caption">{t('Benutze Google mit Ländereinstellung')}</span>
					<select
						value={prefs['address.maps.land'] ?? 'Deutschland'}
						onchange={(e) => setze('address.maps.land', e.currentTarget.value)}
					>
						{#each COUNTRIES as land (land)}<option value={land}>{land}</option>{/each}
					</select>
				</div>
				<div class="row">
					<span class="caption">{t('Startadresse')}</span>
					<input
						class="text"
						placeholder={t('z. B. Pariser Platz, 10117 Berlin, Deutschland')}
						value={prefs['address.maps.start'] ?? ''}
						oninput={(e) => setze('address.maps.start', e.currentTarget.value)}
					/>
				</div>
				<p class="small">
					{t('Die Startadresse ist die Vorgabe der Routenplanung; eine Karte lädt WHERE nicht.')}
				</p>
			</div>
			<div class="sw-footer">
				<span class="spacer"></span>
				<button type="button" class="btn" onclick={() => (isOpen = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="btn primary"
					onclick={async () => {
						await commit();
						isOpen = null;
					}}>{t('Sichern')}</button
				>
			</div>
		</div>
	</div>
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
	/* Right column of the template: group heading in blue, the buttons below it. */
	.selection {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		max-width: 30rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
	}
	h2 {
		margin: 0.6rem 0 0.15rem;
		font-size: 0.95rem;
		color: var(--accent);
	}
	h2:first-child {
		margin-top: 0;
	}
	.entry {
		padding: 0.28rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		text-align: center;
		cursor: pointer;
	}
	.entry:hover {
		background: var(--highlight);
		border-color: var(--accent);
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.84rem;
		color: var(--text);
		padding: 0.1rem 0;
	}
	.foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 30rem;
		margin-top: 0.9rem;
		padding-top: 0.6rem;
		border-top: 1px solid var(--border);
	}
	.spacer {
		flex: 1;
	}
	.btn {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
	}
	.btn.primary {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.btn:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0.3rem 0;
	}
	.caption {
		min-width: 15rem;
		font-size: 0.84rem;
		color: var(--muted);
	}
	.text {
		flex: 1;
		min-width: 0;
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
	}
	.numeric {
		width: 5rem;
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
	}
	.small {
		font-size: 0.78rem;
		color: var(--muted-2);
	}
	.sw-content h3 {
		margin: 0.4rem 0 0.3rem;
		font-size: 0.9rem;
		color: var(--accent);
	}
	.sw-window .sw-content {
		padding: 0.75rem 0.9rem;
	}
</style>
