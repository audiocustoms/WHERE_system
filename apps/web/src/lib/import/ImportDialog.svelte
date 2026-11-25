<script lang="ts">
	import { escapeWindows } from '../escape.svelte';
	import {
		api,
		emptyAddress,
		errorText,
		type AddressInput,
		type ArticleImportRow
	} from '../api';
	import { askCancel } from '../confirm.svelte';
	import { auth } from '../auth.svelte';
	import { modules } from '../modules.svelte';
	import { RIGHT } from '../rights';
	import { closeImport, importTarget } from './importStore.svelte';
	import { fieldsFor, fieldWith, type ImportField, type ImportTarget } from './importFields';
	import { KODIERUNGEN, SEPARATORS, dekodiere, liesCsv, detectSeparator } from './csv';
	import { isEmpty, convert } from './values';
	import '../search/search-window.css';
	import './import-window.css';

	/** One column of the file together with its link. */
	interface Column {
		/** Label from the first row of the file. */
		label: string;
		/** Key of the target field (`importFields.ts`) or `null`. */
		field: string | null;
	}

	interface Report {
		created: number;
		total: number;
		failed: { row: number; message: string }[];
	}

	const startTarget = importTarget();
	let target = $state<ImportTarget>(startTarget);
	let filename = $state<string | null>(null);
	let fileSize = $state(0);
	/** The bytes are kept: another encoding reads the same file again. */
	let rohdaten = $state<Uint8Array | null>(null);
	/** The encoding the file is read with (key of the list). */
	let encoding = $state('utf-8');
	let separatorChoice = $state('auto');
	let erkannterTrenner = $state(';');
	let columns = $state<Column[]>([]);
	let rows = $state<string[][]>([]);
	let fieldSearch = $state('');
	/** Clicked on the field: the next click on a column links it. */
	let gewaehlt = $state<string | null>(null);
	/** Key of the field currently dragged (for browsers without `dataTransfer`). */
	let dragged = $state<string | null>(null);
	let overColumn = $state<number | null>(null);
	let overWindow = $state(false);
	let laeuft = $state(false);
	let error = $state<string | null>(null);
	let report = $state<Report | null>(null);
	/** After a run: the file has already been taken over. */
	let imported = $state(0);

	const mayImport = $derived(
		target === 'addresses'
			? auth.hasRight(RIGHT.mayCreateAddress)
			: auth.hasRight(RIGHT.mayCreateArticle)
	);

	/** Linked columns — only those are imported. */
	const verknuepft = $derived(columns.filter((column) => column.field).length);

	const encodingNote = $derived(KODIERUNGEN.find((k) => k.key === encoding)?.note);

	const separatorLabel = $derived(
		SEPARATORS.find((t) => t.value === erkannterTrenner)?.label ?? erkannterTrenner
	);

	/** The fields of the target, filtered by groups and by the search. */
	const fieldGroups = $derived.by(() => {
		const search = fieldSearch.trim().toLowerCase();
		const matching = fieldsFor(target).filter(
			(field) =>
				(!field.module || modules.isInstalled(field.module)) &&
				(!search ||
					field.label.toLowerCase().includes(search) ||
					field.key.toLowerCase().includes(search))
		);
		const groups: { name: string; fields: ImportField[] }[] = [];
		for (const field of matching) {
			const last = groups.at(-1);
			if (last && last.name === field.group) last.fields.push(field);
			else groups.push({ name: field.group, fields: [field] });
		}
		return groups;
	});

	/** Fields that already lie on a column. */
	const occupiedFields = $derived(new Set(columns.map((column) => column.field).filter(Boolean)));

	// ---------------------------------------------------------------- File

	async function chooseFile(file: File) {
		filename = file.name;
		fileSize = file.size;
		rohdaten = new Uint8Array(await file.arrayBuffer());
		report = null;
		error = null;
		imported = 0;
		read(false);
	}

	/**
	 * Reads the bytes again — after a change of the encoding the mapping is kept
	 * (the columns are the same), after a change of the separator it is not (the
	 * columns are different).
	 */
	function read(keepMapping: boolean) {
		if (!rohdaten) return;
		const alterStand = keepMapping ? columns.map((column) => column.field) : [];
		error = null;
		const text = dekodiere(
			rohdaten,
			KODIERUNGEN.find((entry) => entry.key === encoding)?.decoding ?? 'utf-8'
		);
		erkannterTrenner = separatorChoice === 'auto' ? detectSeparator(text) : separatorChoice;
		const table = liesCsv(text, erkannterTrenner);
		if (!table.length) {
			columns = [];
			rows = [];
			error = 'Die Datei enthält keine Zeilen.';
			return;
		}
		const width = table.reduce((max, row) => Math.max(max, row.length), 0);
		const fill = (row: string[]) => {
			const values = row.slice(0, width);
			while (values.length < width) values.push('');
			return values;
		};
		columns = fill(table[0]).map((label, index) => ({
			label: label.trim() || `Spalte ${index + 1}`,
			field: alterStand[index] ?? null
		}));
		rows = table.slice(1).map(fill);
	}

	/** Change the separator: the columns of the file change, so map again. */
	function changeSeparator(value: string) {
		separatorChoice = value;
		read(false);
	}

	/** Change the target: the other target has other fields, so map again. */
	function changeTarget(value: string) {
		target = value === 'articles' ? 'articles' : 'addresses';
		for (const column of columns) column.field = null;
		gewaehlt = null;
		report = null;
		imported = 0;
	}

	// -------------------------------------------------------------- Mapping

	/** A field always lies on one column only. */
	function assign(column: number, field: string | null) {
		if (column < 0 || column >= columns.length) return;
		if (field === null) {
			columns[column].field = null;
			return;
		}
		for (const others of columns) if (others.field === field) others.field = null;
		columns[column].field = field;
		gewaehlt = null;
	}

	/** Click on a column: link, unlink or "pick up" the field. */
	function columnClick(index: number) {
		if (gewaehlt) assign(index, gewaehlt);
		else if (columns[index].field) assign(index, null);
	}

	function fieldClick(key: string) {
		gewaehlt = gewaehlt === key ? null : key;
	}

	function alleLoesen() {
		for (const column of columns) column.field = null;
		gewaehlt = null;
	}

	function fieldDrag(event: DragEvent, key: string) {
		dragged = key;
		event.dataTransfer?.setData('text/plain', key);
		if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
	}

	function overTarget(event: DragEvent, index: number) {
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
		overColumn = index;
	}

	function stored(event: DragEvent, index: number) {
		// A file may also land on a column — then it is loaded.
		if (event.dataTransfer?.files?.[0]) {
			void fileDropped(event);
			return;
		}
		event.preventDefault();
		event.stopPropagation();
		overColumn = null;
		const key = event.dataTransfer?.getData('text/plain') || dragged;
		dragged = null;
		if (key) assign(index, key);
	}

	/** Drag a CSV file onto the area. */
	async function fileDropped(event: DragEvent) {
		const file = event.dataTransfer?.files?.[0];
		if (!file) return;
		event.preventDefault();
		overWindow = false;
		await chooseFile(file);
	}

	// --------------------------------------------------------------- Import

	/** Sets a value in the record; `pricing.x` reaches into the sub-object. */
	function setzePfad(record: Record<string, unknown>, path: string, value: unknown) {
		const parts = path.split('.');
		let target = record;
		for (const part of parts.slice(0, -1)) {
			const naechste = target[part];
			if (!naechste || typeof naechste !== 'object') return;
			target = naechste as Record<string, unknown>;
		}
		target[parts[parts.length - 1]] = value;
	}

	async function runImport() {
		if (!verknuepft || laeuft) return;
		laeuft = true;
		error = null;
		report = null;
		try {
			// Articles: the template of the mask delivers all defaults (category,
			// prices, texts). The article number stays at 0 so that the API assigns
			// the next free one — unless the file brings one along.
			const template = target === 'articles' ? await api.article.new() : null;
			const saetze: unknown[] = [];
			/** File row per sent record — the API counts only the sent ones. */
			const fileRow: number[] = [];
			const rejected: { row: number; message: string }[] = [];

			rows.forEach((row, index) => {
				const record = template
					? ({ ...structuredClone(template), article_no: 0 } as Record<string, unknown>)
					: (emptyAddress() as unknown as Record<string, unknown>);
				/** The product group stands beside the record, not inside it: the API
				    resolves the name and creates the group if it does not exist yet. */
				let warengruppe: string | null = null;
				const maengel: string[] = [];
				for (let i = 0; i < columns.length; i += 1) {
					const field = fieldWith(target, columns[i].field);
					if (!field) continue;
					const text = row[i] ?? '';
					if (isEmpty(text)) continue;
					const result = convert(field.kind, text);
					if (!result.ok) {
						maengel.push(`${field.label}: ${result.reason}`);
						continue;
					}
					if (field.key === 'warengruppe') warengruppe = String(result.value);
					else setzePfad(record, field.key, result.value);
				}
				// The row number of the file: the first row holds the column names.
				const number = index + 2;
				if (maengel.length) rejected.push({ row: number, message: maengel.join(' · ') });
				else {
					saetze.push(target === 'articles' ? { article: record, warengruppe } : record);
					fileRow.push(number);
				}
			});

			const answer = saetze.length
				? target === 'articles'
					? await api.import.articles(saetze as ArticleImportRow[])
					: await api.import.addresses(saetze as AddressInput[])
				: { created: 0, failed: [] };

			report = {
				created: answer.created,
				total: rows.length,
				failed: [
					...rejected,
					...answer.failed.map((entry) => ({
						row: fileRow[entry.row - 1] ?? entry.row,
						message: entry.message
					}))
				].sort((a, b) => a.row - b.row)
			};
			imported = answer.created;
		} catch (e) {
			error = errorText(e, true);
		} finally {
			laeuft = false;
		}
	}

	// --------------------------------------------------------------- Closing

	/** With a loaded file the user is asked — as with every mask that holds input. */
	async function close() {
		if (filename && !imported) {
			if (!(await askCancel('Den Import'))) return;
		}
		const createNew = imported;
		closeImport();
		// After a run the list behind the window shows the new records.
		if (createNew > 0 && ['/addresses', '/articles'].includes(location.pathname)) location.reload();
	}

	function fileField(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (file) void chooseFile(file);
	}

	const size = (bytes: number) =>
		bytes >= 1024 * 1024
			? `${(bytes / (1024 * 1024)).toLocaleString('de-DE', { maximumFractionDigits: 1 })} MB`
			: `${Math.max(1, Math.round(bytes / 1024))} KB`;
</script>

	/** `Esc` closes the import window like `Abbrechen` — including its confirmation. */
	$effect(() => escapeFenster(() => void schliessen()));

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window imp-window"
		role="dialog"
		aria-modal="true"
		aria-label="Import-Manager"
		tabindex="-1"
		style="width: min(82rem, 100%)"
		ondragover={(event) => {
			// Only a dragged file highlights the area.
			if (!event.dataTransfer?.types.includes('Files')) return;
			event.preventDefault();
			overWindow = true;
		}}
		ondragleave={() => (overWindow = false)}
		ondrop={fileDropped}
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">Import-Manager</span>
			<button class="sw-close" type="button" title="Schließen" onclick={close}>×</button>
		</div>

		<div class="imp-content">
			<div class="imp-toolbar">
				<label class="sw-button file">
					Datei auswählen
					<input type="file" accept=".csv,.txt,.tsv,text/csv,text/plain" onchange={fileField} />
				</label>
				<span class="imp-small">
					{#if filename}
						{filename} · {size(fileSize)} · {columns.length} Spalten · {rows.length} Zeilen
						<button class="sw-link" type="button" onclick={alleLoesen} disabled={!verknuepft}>
							Zuordnung löschen
						</button>
					{:else}
						CSV-Datei wählen oder hierher ziehen — die erste Zeile liefert die Spaltennamen.
					{/if}
				</span>
				<span class="imp-right">
					{#if imported > 0 && report}
						<span class="imp-small">Zuletzt übernommen: {imported} Sätze</span>
					{/if}
					<button
						class="sw-button sw-primary"
						type="button"
						onclick={runImport}
						disabled={!verknuepft || laeuft || !mayImport}
						title={!mayImport
							? 'Für den Import fehlt das Recht zum Anlegen'
							: !verknuepft
								? 'Zuerst ein Feld auf eine Spalte ziehen'
								: 'Die verknüpften Spalten importieren'}
					>
						{laeuft ? 'Import läuft …' : imported > 0 ? 'Erneut importieren' : 'Import starten'}
					</button>
				</span>
			</div>

			<div class="imp-options">
				<label class="imp-option">
					<span>Zeichenkodierung</span>
					<select
						value={encoding}
						onchange={(e) => {
							encoding = e.currentTarget.value;
							read(true);
						}}
						disabled={!rohdaten}
					>
						{#each KODIERUNGEN as entry (entry.key)}
							<option value={entry.key}>{entry.label}</option>
						{/each}
					</select>
				</label>
				<label class="imp-option">
					<span>Feldtrennung</span>
					<select value={separatorChoice} onchange={(e) => changeSeparator(e.currentTarget.value)} disabled={!rohdaten}>
						{#each SEPARATORS as entry (entry.value)}
							<option value={entry.value}>{entry.label}</option>
						{/each}
					</select>
				</label>
				<div class="imp-option">
					<span>Erkannt</span>
					<span class="imp-small">{separatorLabel}{encodingNote ? ` · ${encodingNote}` : ''}</span>
				</div>
				<div class="imp-option" style="flex: 1 1 12rem">
					<span>Ziel</span>
					<span class="imp-small">
						{#if target === 'addresses'}
							Ohne Typ-Spalte werden die Sätze als Kunden angelegt; fehlende Kunden- und
							Lieferantennummern kommen aus dem Nummernkreis.
						{:else}
							Ohne Artikel-Nr. in der Datei vergibt die Anwendung die nächste freie Nummer.
						{/if}
					</span>
				</div>
			</div>

			{#if !mayImport}
				<p class="imp-error">
					Für den Import nach „{target === 'addresses' ? 'Adressen' : 'Artikel'}“ fehlt das Recht
					zum Anlegen — die Datei lässt sich prüfen, aber nicht übernehmen.
				</p>
			{/if}

			<div class="imp-areas" class:over={overWindow} role="presentation">
				<!-- --------------------------------------- Columns of the file (row 1) -->
				<div class="imp-left">
					{#if !columns.length}
						<div class="imp-empty">
							<stark>Keine Datei geladen</stark>
							Mit <b>Datei auswählen</b> eine CSV-Datei öffnen — oder sie auf dieses Fenster
							ziehen. Links stehen dann die Spaltennamen der ersten Zeile; ziehen Sie ein Feld
							von rechts daneben, um es zu importieren.
						</div>
					{:else}
						<div class="imp-scroll-area">
							<table class="imp-table">
								<thead>
									<tr>
										<th class="imp-column">Importierte Daten</th>
										<th class="imp-linked">Verbunde Felder</th>
									</tr>
								</thead>
								<tbody>
									{#each columns as column, index (index)}
										<tr class:occupied={!!column.field}>
											<td class="imp-column" title={column.label}>{column.label}</td>
											<td class="imp-linked">
												<button
													type="button"
													class="imp-target"
													class:occupied={!!column.field}
													class:over={overColumn === index}
													title={column.field
														? 'Klicken löst die Verknüpfung'
														: 'Zielfeld hierher ziehen'}
													ondragover={(event) => overTarget(event, index)}
													ondragleave={() => {
														if (overColumn === index) overColumn = null;
													}}
													ondrop={(event) => stored(event, index)}
													onclick={() => columnClick(index)}
												>
													{#if column.field}
														{fieldWith(target, column.field)?.label ?? column.field}
														<span class="imp-remove" aria-hidden="true">×</span>
													{:else if dragged}
														Feld hierher ziehen
													{:else}
														<span aria-hidden="true">—</span>
													{/if}
												</button>
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</div>

				<!-- ---------------------------------------- Fields of the target mask -->
				<div class="imp-right-column">
					<div class="imp-header">
						<span>Ziel</span>
						<select value={target} onchange={(e) => changeTarget(e.currentTarget.value)} aria-label="Importziel">
							<option value="addresses">Adressen</option>
							<option value="articles">Artikel</option>
						</select>
						<span class="imp-right">{fieldGroups.reduce((n, g) => n + g.fields.length, 0)} Felder</span>
					</div>
					<input
						class="imp-field-search"
						type="search"
						placeholder="Feld suchen …"
						bind:value={fieldSearch}
						aria-label="Feld suchen"
					/>
					<div class="imp-field-list" role="listbox" aria-label="Felder des Ziels" aria-multiselectable="false">
						{#each fieldGroups as group (group.name)}
							<div class="imp-group">{group.name}</div>
							{#each group.fields as field (field.key)}
								<button
									type="button"
									class="imp-field"
									class:selected={gewaehlt === field.key}
									class:occupied={occupiedFields.has(field.key)}
									role="option"
									aria-selected={gewaehlt === field.key}
									draggable="true"
									title={`${field.label} — auf eine Spalte ziehen`}
									ondragstart={(event) => fieldDrag(event, field.key)}
									ondragend={() => {
										dragged = null;
										overColumn = null;
									}}
									onclick={() => fieldClick(field.key)}
								>
									<span class="imp-type">{field.kind}</span>
									<span class="imp-fieldname">{field.label}</span>
								</button>
							{/each}
						{/each}
						{#if !fieldGroups.length}
							<p class="imp-small" style="padding: 0.4rem 0.5rem">Kein Feld passt zu „{fieldSearch}“.</p>
						{/if}
					</div>
				</div>
			</div>

			{#if error}
				<p class="imp-error">{error}</p>
			{/if}

			{#if report}
				<div class="imp-report" class:ok={!report.failed.length}>
					<strong>
						{report.created} von {report.total}
						{report.total === 1 ? 'Zeile' : 'Zeilen'} angelegt
						{#if report.failed.length}· {report.failed.length} nicht übernommen{/if}
					</strong>
					{#if report.failed.length}
						<ul>
							{#each report.failed as entry, index (index)}
								<li>
									<span class="imp-line">Zeile {entry.row}:</span>
									{entry.message}
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/if}

			<div class="imp-footer">
				<span class="imp-status">
					{#if columns.length}
						<b>{verknuepft}</b>
						{verknuepft === 1 ? 'Spalte' : 'Spalten'} verknüpft · <b>{rows.length}</b>
						{rows.length === 1 ? 'Zeile' : 'Zeilen'} in der Datei
					{:else}
						Nur verknüpfte Spalten werden importiert.
					{/if}
				</span>
				<span class="imp-right">
					<button class="sw-button" type="button" onclick={close}>Schließen</button>
					<button
						class="sw-button sw-primary"
						type="button"
						onclick={runImport}
						disabled={!verknuepft || laeuft || !mayImport}
					>
						{laeuft ? 'Import läuft …' : 'Import starten'}
					</button>
				</span>
			</div>
		</div>
	</div>
</div>

<style>
	/* The `Datei auswählen` button is a label for the hidden field. */
	.file {
		position: relative;
		overflow: hidden;
		display: inline-flex;
		align-items: center;
		flex: 0 0 auto;
		white-space: nowrap;
	}

	.file input {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}

	/* `sw-button` grows inside a button row — here it sits in a tool bar. */
	.imp-toolbar :global(.sw-button),
	.imp-footer :global(.sw-button) {
		flex: 0 0 auto;
	}

		/* A disabled button has to look like one too (`sw-button` knows no state). */
		.imp-toolbar :global(.sw-button:disabled),
		.imp-footer :global(.sw-button:disabled) {
			opacity: 0.5;
			cursor: default;
			box-shadow: none;
		}

	/* While a file is dragged the area highlights. */
	.imp-areas.over {
		outline: 2px dashed var(--accent);
		outline-offset: -3px;
		background: var(--highlight);
	}

	.imp-field.occupied .imp-fieldname {
		color: var(--muted);
	}

	.imp-field.occupied::after {
		content: '✓';
		margin-left: auto;
		color: var(--accent);
	}

	.imp-small :global(.sw-link):disabled {
		color: var(--muted-2);
		cursor: default;
		text-decoration: none;
	}
</style>
