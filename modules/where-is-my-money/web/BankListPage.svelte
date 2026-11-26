<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type Bank,
		type BankInput,
		type BankSyncReport
	} from '$core/api';
	import { askConfirm } from '$core/confirm.svelte';
	import '$core/search/search-window.css';

	/** The list is fetched in one go — it has about 4,700 rows. */
	const ALL = 20000;

	/** The input window for one bank. */
	interface BankDialog {
		title: string;
		id: number | null;
		bic: string;
		name: string;
		country: string;
		blz: string;
		city: string;
		alt_name: string;
	}

	/** The window behind "Quelle…" (source): another URL or a pasted list. */
	interface SyncDialog {
		url: string;
		csv: string;
		running: boolean;
		message: string | null;
	}

	let rows = $state<Bank[]>([]);
	let total = $state(0);
	let query = $state('');
	let load = $state(true);
	let error = $state<string | null>(null);
	let choice = $state<number | null>(null);
	let dialog = $state<BankDialog | null>(null);
	let sync = $state<SyncDialog | null>(null);
	let report = $state<BankSyncReport | null>(null);
	let syncUrl = $state('');
	/** The built-in source while no custom one is configured. */
	let defaultUrl = $state('');
	let syncRunning = $state(false);

	onMount(async () => {
		await loadAll();
		await loadSource();
	});

	async function loadAll() {
		load = true;
		error = null;
		try {
			const list = await api.banks.list({ q: query || undefined, limit: ALL });
			rows = list.rows;
			total = list.total;
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	/** The last used source is offered so that `Update` takes it up. */
	async function loadSource() {
		try {
			const source = await api.banks.source();
			defaultUrl = source.url;
		} catch {
			defaultUrl = '';
		}
		try {
			const prefs = await api.prefs.load('banken.sync.');
			syncUrl = prefs.values['banken.sync.url'] ?? '';
		} catch {
			syncUrl = '';
		}
	}

	/** `Update`: fetch the list and import it — without an intermediate window. */
	async function refresh() {
		syncRunning = true;
		error = null;
		report = null;
		try {
			const result = await api.banks.sync({ url: syncUrl || defaultUrl || null });
			report = result;
			await loadAll();
		} catch (e) {
			error = errorText(e);
		} finally {
			syncRunning = false;
		}
	}

	function search() {
		choice = null;
		void loadAll();
	}

	/** "Land / PLZ / Ort" of the template: country, postal code and city in one column. */
	function place(bank: Bank): string {
		return [bank.country, bank.blz, bank.city].filter((part): part is string => !!part).join(' ');
	}

	function createNew() {
		dialog = {
			title: t('Neue Bank'),
			id: null,
			bic: '',
			name: '',
			country: 'DEU',
			blz: '',
			city: '',
			alt_name: ''
		};
	}

	function edit() {
		const bank = rows.find((row) => row.id === choice);
		if (!bank) return;
		dialog = {
			title: t('Bank ändern'),
			id: bank.id,
			bic: bank.bic,
			name: bank.name,
			country: bank.country ?? '',
			blz: bank.blz ?? '',
			city: bank.city ?? '',
			alt_name: bank.alt_name ?? ''
		};
	}

	async function commit() {
		if (!dialog) return;
		const entry = dialog;
		if (!entry.bic.trim() || !entry.name.trim()) return;
		const body: BankInput = {
			bic: entry.bic,
			name: entry.name,
			country: entry.country.trim() || null,
			blz: entry.blz.trim() || null,
			city: entry.city.trim() || null,
			alt_name: entry.alt_name.trim() || null
		};
		dialog = null;
		error = null;
		try {
			if (entry.id === null) await api.banks.create(body);
			else await api.banks.update(entry.id, body);
			await loadAll();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function remove() {
		const bank = rows.find((row) => row.id === choice);
		if (!bank) return;
		const ok = await askConfirm({
			title: t('Löschen'),
			message: t('Bank „{name}“ ({bic}) löschen? Adressen, die den Code tragen, behalten ihn.', {
				name: bank.name,
				bic: bank.bic
			}),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		error = null;
		try {
			await api.banks.remove(bank.id);
			choice = null;
			await loadAll();
		} catch (e) {
			error = errorText(e);
		}
	}

	function openSource() {
		report = null;
		sync = { url: syncUrl || defaultUrl, csv: '', running: false, message: null };
	}

	async function synchronize() {
		if (!sync) return;
		const target = sync;
		if (!target.url.trim() && !target.csv.trim()) {
			sync = { ...target, message: t('Bitte eine URL eintragen oder eine Liste einfügen.') };
			return;
		}
		sync = { ...target, running: true, message: null };
		error = null;
		try {
			const result = await api.banks.sync({
				url: target.url.trim() || null,
				csv: target.csv.trim() || null
			});
			report = result;
			const own = target.url.trim();
			syncUrl = own === defaultUrl ? '' : own;
			if (own) await api.prefs.save('banken.sync.url', own);
			sync = {
				...target,
				running: false,
				message: t('{read} Zeilen gelesen: {inserted} neu, {updated} geändert, {unchanged} unverändert, {skipped} ohne BIC oder Namen.', {
					read: result.read,
					inserted: result.inserted,
					updated: result.updated,
					unchanged: result.unchanged,
					skipped: result.skipped
				})
			};
			await loadAll();
		} catch (e) {
			sync = { ...target, running: false, message: errorText(e) };
		}
	}
</script>

<svelte:head><title>{t('Banken (SWIFT/BIC) — WHERE')}</title></svelte:head>

<h1>{t('Banken (SWIFT/BIC)')}</h1>

<div class="toolbar">
	<input
		class="find"
		type="search"
		placeholder={t('Find (Code, Bank, Ort, BLZ)')}
		value={query}
		oninput={(e) => (query = e.currentTarget.value)}
		onkeydown={(e) => {
			if (e.key === 'Enter') search();
		}}
	/>
	<button type="button" class="button" onclick={search} disabled={load}>{t('Suchen')}</button>
	<span class="spacer"></span>
	<button
		type="button"
		class="button"
		onclick={openSource}
		title={t('Andere Quelle wählen oder Liste einfügen')}>{t('Quelle…')}</button
	>
	<button type="button" class="button primary" onclick={refresh} disabled={syncRunning}>
		{syncRunning ? t('Lädt …') : t('Update')}
	</button>
</div>

{#if report}
	<p class="report">
		{t('{read} Zeilen gelesen: {inserted} neu, {updated} geändert, {unchanged} unverändert, {skipped} ohne BIC oder Namen — Quelle: {source}', {
			read: report.read,
			inserted: report.inserted,
			updated: report.updated,
			unchanged: report.unchanged,
			skipped: report.skipped,
			source: report.source
		})}
	</p>
{/if}

<div class="list">
	<table>
		<thead>
			<tr>
				<th class="bic">{t('SWIFT/BIC')}</th>
				<th class="place">{t('Land / PLZ / Ort')}</th>
				<th>{t('Bank')}</th>
				<th>{t('Alternativer Name')}</th>
			</tr>
		</thead>
		<tbody>
			{#each rows as bank (bank.id)}
				<tr
					class:selected={choice === bank.id}
					onclick={() => (choice = choice === bank.id ? null : bank.id)}
					ondblclick={() => {
						choice = bank.id;
						edit();
					}}
				>
					<td class="bic">{bank.bic}</td>
					<td class="place">{place(bank)}</td>
					<td>{bank.name}</td>
					<td>{bank.alt_name ?? ''}</td>
				</tr>
			{:else}
				<tr><td colspan="4">{load ? t('Lade …') : t('Keine Bank gefunden.')}</td></tr>
			{/each}
		</tbody>
	</table>
</div>

<div class="footer">
	<span class="counter">
		{rows.length < total
			? t('{shown} von {total}', { shown: rows.length, total })
			: t('{n} Einträge', { n: total })}
	</span>
	<span class="spacer"></span>
	<div class="button-group" role="group" aria-label={t('Zeile')}>
		<button type="button" class="square" title={t('Neue Bank')} onclick={createNew}>+</button>
		<button type="button" class="square" title={t('Ändern')} disabled={choice === null} onclick={edit}
			>✎</button
		>
		<button type="button" class="square" title={t('Löschen')} disabled={choice === null} onclick={remove}
			>−</button
		>
	</div>
	<span class="spacer"></span>
	<a class="button" href="/">{t('Schließen')}</a>
</div>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if dialog}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={dialog.title}
			style="width: min(30rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{dialog.title}</span>
				<button
					class="sw-close"
					type="button"
					title={t('Schließen')}
					onclick={() => (dialog = null)}>×</button
				>
			</div>
			<div class="sw-content">
				<label class="field">
					{t('SWIFT/BIC')}
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="text"
						autofocus
						maxlength="20"
						placeholder={t('z. B. AABAFI22XXX')}
						value={dialog.bic}
						oninput={(e) => (dialog!.bic = e.currentTarget.value.toUpperCase())}
						onkeydown={(e) => {
							if (e.key === 'Enter') commit();
						}}
					/>
				</label>
				<label class="field">
					{t('Bank')}
					<input
						class="text"
						maxlength="255"
						value={dialog.name}
						oninput={(e) => (dialog!.name = e.currentTarget.value)}
					/>
				</label>
				<div class="pair">
					<label class="field">
						{t('Land (ISO-3)')}
						<input
							class="text"
							maxlength="10"
							value={dialog.country}
							oninput={(e) => (dialog!.country = e.currentTarget.value.toUpperCase())}
						/>
					</label>
					<label class="field">
						{t('Bankleitzahl')}
						<input
							class="text"
							maxlength="20"
							value={dialog.blz}
							oninput={(e) => (dialog!.blz = e.currentTarget.value)}
						/>
					</label>
				</div>
				<label class="field">
					{t('Ort')}
					<input
						class="text"
						maxlength="255"
						value={dialog.city}
						oninput={(e) => (dialog!.city = e.currentTarget.value)}
					/>
				</label>
				<label class="field">
					{t('Alternativer Name')}
					<input
						class="text"
						maxlength="255"
						value={dialog.alt_name}
						oninput={(e) => (dialog!.alt_name = e.currentTarget.value)}
					/>
				</label>
			</div>
			<div class="sw-footer">
				<span class="spacer"></span>
				<button type="button" class="button" onclick={() => (dialog = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="button primary"
					disabled={!dialog.bic.trim() || !dialog.name.trim()}
					onclick={commit}>{dialog.id === null ? t('Anlegen') : t('Sichern')}</button
				>
			</div>
		</div>
	</div>
{/if}

{#if sync}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={t('Liste aktualisieren')}
			style="width: min(36rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{t('Liste aktualisieren')}</span>
				<button class="sw-close" type="button" title={t('Schließen')} onclick={() => (sync = null)}
					>×</button
				>
			</div>
			<div class="sw-content">
				<label class="field">
					{t('Quelle (URL)')}
					<input
						class="text"
						placeholder={t('https://… (leer = eingebaute Quelle)')}
						value={sync.url}
						oninput={(e) => (sync!.url = e.currentTarget.value)}
					/>
				</label>
				<p class="small">
					{t('Leer lassen für die eingebaute Quelle (Deutsche Bundesbank, Bankleitzahlen-Download); der Server liest die Seite und lädt die aktuelle CSV-Datei. Spalten werden nach ihrer Überschrift erkannt (BIC/SWIFT, Bank/Name/Bezeichnung, BLZ/PLZ, Ort, Land, Alternativer Name); ohne Überschrift wird die deutsche BLZ-Datei angenommen.')}
				</p>
				<label class="field">
					{t('… oder eine Liste einfügen')}
					<textarea
						class="text"
						rows="6"
						placeholder={t('bic;name;blz;city;country;altname')}
						value={sync.csv}
						oninput={(e) => (sync!.csv = e.currentTarget.value)}
					></textarea>
				</label>
				{#if sync.message}
					<p class="message">{sync.message}</p>
				{/if}
			</div>
			<div class="sw-footer">
				<span class="spacer"></span>
				<button type="button" class="button" onclick={() => (sync = null)}>{t('Schließen')}</button>
				<button
					type="button"
					class="button primary"
					disabled={sync.running}
					onclick={synchronize}>{sync.running ? t('Lädt …') : t('Synchronisieren')}</button
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
	.toolbar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 62rem;
		margin-bottom: 0.5rem;
	}
	.find {
		flex: 1;
		max-width: 22rem;
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
	}
	.report {
		max-width: 62rem;
		margin: 0 0 0.5rem;
		padding: 0.4rem 0.55rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		font-size: 0.78rem;
		color: var(--muted);
	}
	.list {
		max-width: 62rem;
		max-height: 34rem;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.82rem;
	}
	thead th {
		position: sticky;
		top: 0;
		z-index: 1;
		padding: 0.35rem 0.5rem;
		text-align: left;
		font-weight: 600;
		color: var(--accent);
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
	}
	tbody td {
		padding: 0.25rem 0.5rem;
		border-bottom: 1px solid var(--surface-3);
	}
	tbody tr:nth-child(even) {
		background: var(--surface-2);
	}
	tbody tr:hover {
		background: var(--highlight);
	}
	tbody tr.selected {
		background: var(--highlight);
		outline: 1px solid var(--accent);
		outline-offset: -1px;
	}
	th.bic,
	td.bic {
		width: 12rem;
		font-family: var(--mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace);
	}
	th.place,
	td.place {
		width: 14rem;
	}
	.footer {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 62rem;
		margin-top: 0.5rem;
	}
	.counter {
		font-size: 0.78rem;
		color: var(--muted-2);
		font-variant-numeric: tabular-nums;
	}
	.spacer {
		flex: 1;
	}
	.button-group {
		display: flex;
		gap: 0.2rem;
	}
	.square {
		width: 1.9rem;
		height: 1.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.9rem;
		line-height: 1;
		cursor: pointer;
	}
	.square:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.button {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
		text-decoration: none;
	}
	.button:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.button.primary {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.button.primary:disabled {
		border-color: var(--border);
		color: var(--muted-2);
		font-weight: 400;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 0.5rem;
		font-size: 0.78rem;
		color: var(--muted);
	}
	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}
	.text {
		width: 100%;
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.86rem;
	}
	textarea.text {
		resize: vertical;
	}
	.small {
		margin: -0.2rem 0 0.6rem;
		font-size: 0.74rem;
		color: var(--muted-2);
	}
	.message {
		margin: 0.2rem 0;
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		font-size: 0.8rem;
		color: var(--text);
	}
	.sw-window .sw-content {
		padding: 0.75rem 0.9rem;
	}
</style>
