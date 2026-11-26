<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type HeadingEntry,
		type OrderMacro,
		type TimelineEntry
	} from '$core/api';
	import { askConfirm } from '$core/confirm.svelte';
	import '$core/search/search-window.css';

	/** The five tabs from `img/Admin Menü/Aufträge/Auftragseinstellungen_*.PNG`. */
	type Tab = 'rules' | 'times' | 'schedule' | 'headings' | 'macros';

	const TABS: { key: Tab; label: string }[] = [
		{ key: 'rules', label: t('Regeln') },
		{ key: 'times', label: t('Zeiten') },
		{ key: 'schedule', label: t('Zeitplan') },
		{ key: 'headings', label: t('Überschriften') },
		{ key: 'macros', label: t('Makros') }
	];

	/** An input window for one macro. */
	interface MacroDialog {
		title: string;
		id: number | null;
		label: string;
		for_sale: boolean;
		active: boolean;
		body: string;
	}

	let tab = $state<Tab>('rules');
	let load = $state(true);
	let error = $state<string | null>(null);

	let prefs = $state<Record<string, string>>({});
	let saved = $state<Record<string, string>>({});

	let timeline = $state<TimelineEntry[]>([]);
	let headings = $state<HeadingEntry[]>([]);
	let macros = $state<OrderMacro[]>([]);
	let macroChoice = $state<number | null>(null);
	let macroDialog = $state<MacroDialog | null>(null);

	onMount(loadAll);

	async function loadAll() {
		load = true;
		error = null;
		try {
			const loaded = await api.prefs.load('order.');
			prefs = { ...loaded.values };
			saved = { ...loaded.values };
			timeline = await api.orders.timeline();
			headings = await api.orders.headings();
			macros = await api.orders.macros();
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	function bool(key: string): boolean {
		return prefs[key] === 'true';
	}

	function text(key: string): string {
		return prefs[key] ?? '';
	}

	function setPref(key: string, value: string) {
		prefs = { ...prefs, [key]: value };
	}

	function setPrefBool(key: string, value: boolean) {
		setPref(key, value ? 'true' : 'false');
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
				const next = prefs[key] ?? '';
				if (previous !== next) await api.prefs.save(key, next === '' ? null : next);
			}
			await loadAll();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function cancel() {
		if (changed && !(await askConfirm({
			title: t('Abbrechen'),
			message: t('Das Bearbeiten der Auftragseinstellungen wirklich abbrechen? Nicht gesicherte Eingaben gehen verloren.'),
			confirmLabel: t('Ja, abbrechen'),
			cancelLabel: t('Weiter bearbeiten')
		})))
			return;
		await loadAll();
	}

	async function toggleRow(row: TimelineEntry) {
		error = null;
		try {
			await api.orders.saveTimeline(row.key, { label: row.label, shown: !row.shown });
			timeline = await api.orders.timeline();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function renameHeading(row: HeadingEntry, patch: Partial<HeadingEntry>) {
		error = null;
		try {
			await api.orders.saveHeading(row.number, {
				shown: patch.shown ?? row.shown,
				label: patch.label ?? row.label
			});
			headings = await api.orders.headings();
		} catch (e) {
			error = errorText(e);
		}
	}

	function newMacro() {
		macroDialog = {
			title: t('Neues Makro'),
			id: null,
			label: '',
			for_sale: false,
			active: true,
			body: ''
		};
	}

	function editMacro() {
		const macro = macros.find((entry) => entry.id === macroChoice);
		if (!macro) return;
		macroDialog = {
			title: t('Makro ändern'),
			id: macro.id,
			label: macro.label,
			for_sale: macro.for_sale,
			active: macro.active,
			body: macro.body ?? ''
		};
	}

	async function saveMacro() {
		if (!macroDialog) return;
		const dialog = macroDialog;
		if (!dialog.label.trim()) return;
		macroDialog = null;
		error = null;
		const input = {
			label: dialog.label.trim(),
			for_sale: dialog.for_sale,
			active: dialog.active,
			body: dialog.body.trim() || null
		};
		try {
			if (dialog.id === null) await api.orders.createMacro(input);
			else await api.orders.updateMacro(dialog.id, input);
			macros = await api.orders.macros();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function deleteMacro() {
		const macro = macros.find((entry) => entry.id === macroChoice);
		if (!macro) return;
		const ok = await askConfirm({
			title: t('Löschen'),
			message: t('Makro „{label}“ löschen?', { label: macro.label }),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		error = null;
		try {
			await api.orders.removeMacro(macro.id);
			macroChoice = null;
			macros = await api.orders.macros();
		} catch (e) {
			error = errorText(e);
		}
	}
</script>

<svelte:head><title>{t('Auftragseinstellungen — WHERE')}</title></svelte:head>

<h1>{t('Auftragseinstellungen')}</h1>

<div class="tab" role="tablist" aria-label={t('Auftragseinstellungen')}>
	{#each TABS as entry (entry.key)}
		<button
			type="button"
			role="tab"
			aria-selected={tab === entry.key}
			class:active={tab === entry.key}
			onclick={() => (tab = entry.key)}>{entry.label}</button
		>
	{/each}
</div>

<section class="box" aria-label={tab}>
	{#if load}
		<p class="small">{t('Lade…')}</p>
	{:else if tab === 'rules'}
		<h2>{t('Regeln für die Angebotswiderbearbeitung')}</h2>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.dispo-pflicht')}
				onchange={(e) => setPrefBool('order.regeln.dispo-pflicht', e.currentTarget.checked)}
			/>
			{t('Angebot muss disponiert sein zum Beauftragen')}
		</label>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.nur-auftraege-packschein')}
				onchange={(e) =>
					setPrefBool('order.regeln.nur-auftraege-packschein', e.currentTarget.checked)}
			/>
			{t('Nur Aufträge zum Packschein erstellen zulassen')}
		</label>
		<p class="note">{t('Angebote werden ignoriert')}</p>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.angebote-direkt-abrechnen')}
				onchange={(e) =>
					setPrefBool('order.regeln.angebote-direkt-abrechnen', e.currentTarget.checked)}
			/>
			{t('Angebote können direkt abgerechnet werden')}
		</label>
		<p class="note">{t('Angebote müssen nicht erst beauftragt werden um sie abzurechnen')}</p>
		<label class="check indented">
			<input
				type="checkbox"
				checked={bool('order.regeln.nicht-aktuelle-abrechnen')}
				onchange={(e) =>
					setPrefBool('order.regeln.nicht-aktuelle-abrechnen', e.currentTarget.checked)}
			/>
			{t('Abrechnung von nicht-aktuellen Angeboten zulassen')}
		</label>
		<p class="note indented">
			{t('Angebote müssen nicht den Status „aktuelle Version“ haben um sie abzurechnen')}
		</p>

		<h2>{t('Artikeldiposition')}</h2>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.dispo-aktivieren')}
				onchange={(e) => setPrefBool('order.regeln.dispo-aktivieren', e.currentTarget.checked)}
			/>
			{t('Dispo bei Neuanlage von Angeboten immer aktivieren')}
		</label>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.freie-artikel-fremd')}
				onchange={(e) => setPrefBool('order.regeln.freie-artikel-fremd', e.currentTarget.checked)}
			/>
			{t('Freie Artikel immer als Fremdanmietung/-einkauf behandeln')}
		</label>

		<h2>{t('Personaldisposition')}</h2>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.personal-keine-mehrfachbuchung')}
				onchange={(e) =>
					setPrefBool('order.regeln.personal-keine-mehrfachbuchung', e.currentTarget.checked)}
			/>
			{t('Keine Mehrfachbuchung von Personal zur selben Zeit zulassen')}
		</label>

		<h2>{t('Druckoptionen setzen bei Neuanlage')}</h2>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.druck.einzelpreise')}
				onchange={(e) => setPrefBool('order.regeln.druck.einzelpreise', e.currentTarget.checked)}
			/>
			{t('Einzelpreise drucken')}
		</label>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.druck.gesamtpreise')}
				onchange={(e) => setPrefBool('order.regeln.druck.gesamtpreise', e.currentTarget.checked)}
			/>
			{t('Gesamtpreise drucken von Artikeln/Personal/Transport/Kapiteln')}
		</label>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.regeln.druck.zwischensummen')}
				onchange={(e) => setPrefBool('order.regeln.druck.zwischensummen', e.currentTarget.checked)}
			/>
			{t('Gesamtpreise drucken von Zwischensummen')}
		</label>

		<h2>{t('Anzeige')}</h2>
		<div class="row">
			<a class="button" href="/agenda">{t('Bearbeiten')}</a>
			<span>{t('Auftragskategorien für TeamAgenda-Darstellung')}</span>
		</div>
	{:else if tab === 'times'}
		<h2>{t('Angebotsdatum')}</h2>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.angebotsdatum.aktuell')}
				onchange={(e) => setPrefBool('order.angebotsdatum.aktuell', e.currentTarget.checked)}
			/>
			{t('Angebotsdatum bei jedem Sichern auf aktuelles Datum setzen')}
		</label>

		<h2>{t('Einsatztage-Berechnung')}</h2>
		<label class="check">
			<input
				type="radio"
				name="einsatztage"
				checked={text('order.einsatztage.regel') !== '24h'}
				onchange={() => setPref('order.einsatztage.regel', 'tagesregel')}
			/>
			{t('Tagesregel')}
		</label>
		<p class="note">{t('Jeder angefangene Tag zählt als Einsatztag')}</p>
		<label class="check">
			<input
				type="radio"
				name="einsatztage"
				checked={text('order.einsatztage.regel') === '24h'}
				onchange={() => setPref('order.einsatztage.regel', '24h')}
			/>
			{t('24h Regel')}
		</label>
		<p class="note">
			{t('Die Tage werden nach 24h Einheiten berechnet ab der Einsatzzeit. Einsätze unter 24h werden als 1 Tag berechnet.')}
		</p>

		<h2>{t('Angebotsablauffrist (Miete)')}</h2>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.ablauffrist.miete.active')}
				onchange={(e) => setPrefBool('order.ablauffrist.miete.active', e.currentTarget.checked)}
			/>
			{t('Angebotsablauffrist bei Neuanlage aktivieren')}
		</label>
		<div class="row">
			<input
				class="number"
				type="number"
				min="0"
				value={text('order.ablauffrist.miete.nachdatum')}
				oninput={(e) => setPref('order.ablauffrist.miete.nachdatum', e.currentTarget.value)}
			/>
			<span>{t('(Tage) zur Berechnung der Angebotsablauffrist nach Angebotsdatum')}</span>
		</div>
		<div class="row">
			<input
				class="number"
				type="number"
				min="0"
				value={text('order.ablauffrist.miete.vorevent')}
				oninput={(e) => setPref('order.ablauffrist.miete.vorevent', e.currentTarget.value)}
			/>
			<span>{t('(Tage) zur Berechnung der Angebotsablauffrist vor Eventbeginn')}</span>
		</div>

		<h2>{t('Angebotsablauffrist (Verkauf)')}</h2>
		<label class="check">
			<input
				type="checkbox"
				checked={bool('order.ablauffrist.verkauf.active')}
				onchange={(e) => setPrefBool('order.ablauffrist.verkauf.active', e.currentTarget.checked)}
			/>
			{t('Angebotsablauffrist bei Neuanlage aktivieren')}
		</label>
		<div class="row">
			<input
				class="number"
				type="number"
				min="0"
				value={text('order.ablauffrist.verkauf.nachdatum')}
				oninput={(e) => setPref('order.ablauffrist.verkauf.nachdatum', e.currentTarget.value)}
			/>
			<span>{t('(Tage) zur Berechnung der Angebotsablauffrist nach Angebotsdatum')}</span>
		</div>

		<h2>{t('Zeiteinstellungen')}</h2>
		<div class="row">
			<input
				class="time"
				type="time"
				value={text('order.zeit.start')}
				oninput={(e) => setPref('order.zeit.start', e.currentTarget.value)}
			/>
			<span>+</span>
			<span>{t('Standard Startzeit')}</span>
			<input
				class="time"
				type="time"
				value={text('order.zeit.ende')}
				oninput={(e) => setPref('order.zeit.ende', e.currentTarget.value)}
			/>
			<span>+</span>
			<span>{t('Standard Endzeit beim Anlegen von AU und RE')}</span>
		</div>
		<div class="row">
			<input
				class="number"
				type="number"
				min="0"
				value={text('order.dispo.vor')}
				oninput={(e) => setPref('order.dispo.vor', e.currentTarget.value)}
			/>
			<span>{t('(Stunden) Dispostart vordatieren in Abhängigkeit der Eventstartzeit')}</span>
		</div>
		<div class="row">
			<input
				class="number"
				type="number"
				min="0"
				value={text('order.dispo.nach')}
				oninput={(e) => setPref('order.dispo.nach', e.currentTarget.value)}
			/>
			<span>{t('(Stunden) Dispoende nachdatieren in Abhängigkeit der Eventendzeit')}</span>
		</div>

		<h2>{t('Automatische Archivierung von Aufträgen/Packscheinen')}</h2>
		<div class="row">
			<input
				class="number"
				type="number"
				min="0"
				value={text('order.archiv.auftrag')}
				oninput={(e) => setPref('order.archiv.auftrag', e.currentTarget.value)}
			/>
			<span>{t('(Tage) nach Auftragsbeendigung/Stornierung den Auftrag automatisch archivieren')}</span>
		</div>
		<div class="row">
			<input
				class="number"
				type="number"
				min="0"
				value={text('order.archiv.packschein')}
				oninput={(e) => setPref('order.archiv.packschein', e.currentTarget.value)}
			/>
			<span>{t('(Tage) nach komplettem Check-In den Packschein automatisch archivieren')}</span>
		</div>
	{:else if tab === 'schedule'}
		<p class="intro">
			{t('Der folgende Zeitplan mit den ausgewählten Zeiten wird bei einer Neuanlage eines Auftrages/einer Rechnung standarmäßig angezeigt.')}
		</p>
		<table>
			<thead>
				<tr><th>{t('Timeline')}</th><th class="center">{t('Anzeigen')}</th></tr>
			</thead>
			<tbody>
				{#each timeline as row (row.key)}
					<tr>
						<td>{row.label}</td>
						<td class="center">
							<input
								type="checkbox"
								checked={row.shown}
								onchange={() => toggleRow(row)}
							/>
						</td>
					</tr>
				{:else}
					<tr><td colspan="2">{t('Kein Zeitplan hinterlegt.')}</td></tr>
				{/each}
			</tbody>
		</table>
	{:else if tab === 'headings'}
		<table>
			<thead>
				<tr><th>{t('Überschrift')}</th><th class="center">{t('Anzeigen')}</th><th>{t('Bezeichnung')}</th></tr>
			</thead>
			<tbody>
				{#each headings as row (row.number)}
					<tr>
						<td>{t('Überschrift')} {row.number}</td>
						<td class="center">
							<input
								type="checkbox"
								checked={row.shown}
								onchange={(e) => renameHeading(row, { shown: e.currentTarget.checked })}
							/>
						</td>
						<td>
							<input
								class="text"
								maxlength="120"
								value={row.label ?? ''}
								onchange={(e) => renameHeading(row, { label: e.currentTarget.value })}
							/>
						</td>
					</tr>
				{:else}
					<tr><td colspan="3">{t('Keine Überschriften hinterlegt.')}</td></tr>
				{/each}
			</tbody>
		</table>
	{:else}
		<table>
			<thead>
				<tr>
					<th>{t('Bezeichnung')}</th>
					<th class="center">{t('Verkauf')}</th>
					<th class="center">{t('Aktiv')}</th>
					<th>{t('Erstellt von')}</th>
				</tr>
			</thead>
			<tbody>
				{#each macros as macro (macro.id)}
					<tr
						class:selected={macroChoice === macro.id}
						onclick={() => (macroChoice = macroChoice === macro.id ? null : macro.id)}
						ondblclick={() => {
							macroChoice = macro.id;
							editMacro();
						}}
					>
						<td>{macro.label}</td>
						<td class="center">{macro.for_sale ? t('Ja') : t('Nein')}</td>
						<td class="center">{macro.active ? t('Ja') : t('Nein')}</td>
						<td>{macro.created_by ?? '—'}</td>
					</tr>
				{:else}
					<tr><td colspan="4">{t('Noch keine Makros.')}</td></tr>
				{/each}
			</tbody>
		</table>
		<div class="macro-foot">
			<div class="button-group" role="group" aria-label={t('Makro')}>
				<button type="button" class="square" title={t('Neues Makro')} onclick={newMacro}>+</button>
				<button
					type="button"
					class="square"
					title={t('Ändern')}
					disabled={macroChoice === null}
					onclick={editMacro}>✎</button
				>
				<button
					type="button"
					class="square"
					title={t('Löschen')}
					disabled={macroChoice === null}
					onclick={deleteMacro}>−</button
				>
			</div>
		</div>
	{/if}
</section>

<footer class="footer">
	<span class="small">
		{tab === 'rules' || tab === 'times'
			? t('Regeln und Zeiten sichern mit „Sichern“.')
			: t('Diese Listen sichern sofort.')}
	</span>
	<span class="spacer"></span>
	<button type="button" class="button" onclick={cancel} disabled={load}>{t('Abbrechen')}</button>
	<button
		type="button"
		class="button primary"
		onclick={commit}
		disabled={!changed}
		title={changed ? t('Änderungen speichern') : t('Nichts geändert')}>{t('Sichern')}</button
	>
</footer>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if macroDialog}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={macroDialog.title}
			style="width: min(26rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{macroDialog.title}</span>
				<button
					class="sw-close"
					type="button"
					title={t('Schließen')}
					onclick={() => (macroDialog = null)}>×</button
				>
			</div>
			<div class="sw-content">
				<label class="field">
					{t('Bezeichnung')}
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="text"
						autofocus
						maxlength="120"
						value={macroDialog.label}
						oninput={(e) => (macroDialog!.label = e.currentTarget.value)}
						onkeydown={(e) => {
							if (e.key === 'Enter') saveMacro();
						}}
					/>
				</label>
				<label class="check">
					<input
						type="checkbox"
						checked={macroDialog.for_sale}
						onchange={(e) => (macroDialog!.for_sale = e.currentTarget.checked)}
					/>
					{t('Verkauf')}
				</label>
				<label class="check">
					<input
						type="checkbox"
						checked={macroDialog.active}
						onchange={(e) => (macroDialog!.active = e.currentTarget.checked)}
					/>
					{t('Aktiv')}
				</label>
				<label class="field">
					{t('Schritte')}
					<textarea
						class="text"
						rows="4"
						placeholder={t('Was das Makro später ausführen soll')}
						value={macroDialog.body}
						oninput={(e) => (macroDialog!.body = e.currentTarget.value)}
					></textarea>
				</label>
			</div>
			<div class="sw-footer">
				<span class="spacer"></span>
				<button type="button" class="button" onclick={() => (macroDialog = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="button primary"
					disabled={!macroDialog.label.trim()}
					onclick={saveMacro}>{macroDialog.id === null ? t('Anlegen') : t('Sichern')}</button
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
	/* As in the address and article settings: one column, equal width. */
	.tab {
		display: flex;
		gap: 0.25rem;
		max-width: 30rem;
	}
	.tab button {
		padding: 0.3rem 0.6rem;
		border: 1px solid var(--border);
		border-bottom: none;
		border-radius: 3px 3px 0 0;
		background: var(--surface-2);
		color: var(--muted);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
	}
	.tab button.active {
		background: var(--surface);
		color: var(--accent);
		font-weight: 600;
	}
	.box {
		max-width: 30rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: 0 4px 4px 4px;
		background: var(--surface);
	}
	h2 {
		margin: 0.85rem 0 0.3rem;
		font-size: 0.95rem;
		color: var(--accent);
	}
	h2:first-child {
		margin-top: 0;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.84rem;
		color: var(--text);
		padding: 0.1rem 0;
	}
	.check.indented {
		margin-left: 1.6rem;
	}
	.note {
		margin: 0 0 0.15rem 1.6rem;
		font-size: 0.76rem;
		color: var(--muted-2);
		max-width: 26rem;
	}
	.intro {
		margin: 0 0 0.6rem;
		font-size: 0.84rem;
		color: var(--muted);
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin: 0.25rem 0;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.number {
		width: 4.5rem;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		text-align: right;
	}
	.time {
		width: 5.5rem;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.82rem;
	}
	th,
	td {
		padding: 0.22rem 0.35rem;
		border-bottom: 1px solid var(--surface-3);
		text-align: left;
	}
	th {
		font-weight: 600;
		color: var(--muted);
		border-bottom: 1px solid var(--border);
	}
	th.center,
	td.center {
		text-align: center;
		width: 4.5rem;
	}
	tbody tr:hover {
		background: var(--surface-2);
	}
	tbody tr.selected {
		background: var(--highlight);
	}
	.text {
		width: 100%;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
	}
	textarea.text {
		resize: vertical;
	}
	.macro-foot {
		margin-top: 0.5rem;
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
	.footer {
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
	.small {
		font-size: 0.76rem;
		color: var(--muted-2);
		max-width: 24rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 0.5rem;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.sw-window .sw-content {
		padding: 0.75rem 0.9rem;
	}
</style>
