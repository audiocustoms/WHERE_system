<script lang="ts">
	import { escapeWindows } from '$core/escape.svelte';
	import { untrack } from 'svelte';
	import { api, ApiError, errorText, type Employee, type Shift } from '$core/api';
	import '$core/search/search-window.css';
	import { PAPERS, toCsv, toPdf, toXlsx, tableCsv, tableXlsx } from './roster-export';
	import { MONTHS, toDate, buildReport, loadMonth, monthRange } from './roster-report';
	import { planRows, dispoRows, save } from './roster-exchange';

	/**
	 * „Import / Export" — the window beside the print button of the board.
	 *
	 * The export hands the month out as CSV, XLSX or PDF: as the printed sheet, as the
	 * disposition or as the plan. The import takes files back in — the disposition and the
	 * plan as the module writes them, and the xlsx disposition of PeM — and asks the same
	 * questions for all of them: the month, a preview before anything is written, whether
	 * the month is emptied first and whether events without a time come along.
	 */
	interface Props {
		/** First day the board shows — the month is taken from it. */
		from: string;
		employees: Employee[];
		shifts: Shift[];
		colors: { sick: string; vacation: string };
		/** Hours one day of vacation and one day of sickness count. */
		absence: { sick: number; vacation: number };
		/** Tints of the master data; the sub-department wins over the department. */
		departmentColors: Map<string, string>;
		subdepartmentColors: Map<string, string>;
		/** A month was written into the plan. */
		onimported: () => void;
		onclose: () => void;
	}

	let {
		from,
		employees,
		shifts,
		colors,
		absence,
		departmentColors,
		subdepartmentColors,
		onimported,
		onclose
	}: Props = $props();

	const anchor = untrack(() => toDate(from));

	/** What an export hands out. */
	type ExportScope = 'druck' | 'dispo' | 'dienste';
	/** What an import takes back. */
	type ImportScope = 'dispo' | 'dienste' | 'pem';

	/** What a file carries, before anything is written. */
	interface Preview {
		rows: number;
		days: number;
		/** What the rows belong to: the locations of a disposition, the people of a plan. */
		groups: { name: string; rows: number }[];
		warnings: string[];
		existing: number;
		alreadyImported: boolean;
		/** Singular of what the rows belong to: `Standort` or `Person`. */
		unit: string;
	}

	let monthChoice = $state(String(anchor.getMonth() + 1));
	let yearChoice = $state(String(anchor.getFullYear()));
	let paper = $state('a3-quer');
	let scope = $state<ExportScope>('druck');
	let intake = $state<ImportScope>('dispo');
	let file = $state<File | null>(null);
	let showPreview = $state(true);
	// A whole month is at stake: the sheet replaces it unless the planner says otherwise.
	let replace = $state(true);
	let withoutTime = $state(true);
	let busy = $state(false);
	let error = $state('');
	let result = $state('');
	let notes = $state<string[]>([]);
	let preview = $state<Preview | null>(null);

	/** `Persönlicher Dienstplan senden`: the window with the employee list. */
	let icsOpen = $state(false);
	/** The employees whose calendar is sent. */
	let calendarChoice = $state<number[]>([]);
	let sending = $state(false);
	let sent = $state('');
	let sendError = $state('');
	/** Who was left out and why. */
	let notSent = $state<string[]>([]);

	const YEARS = Array.from({ length: 11 }, (_, index) => anchor.getFullYear() - 5 + index);
	const sheet = $derived(PAPERS.find((entry) => entry.value === paper) ?? PAPERS[0]);
	const period = $derived(monthRange(Number(yearChoice), Number(monthChoice)));
	const periodLabel = $derived(`${MONTHS[Number(monthChoice) - 1]} ${yearChoice}`);

	/** The month as the print and the PDF show it. */
	function report(data: Awaited<ReturnType<typeof loadMonth>>, part: 'all' | 'dispo' | 'staff') {
		return buildReport({
			period,
			data,
			employees,
			shifts,
			colors,
			absence,
			paper: { width: sheet.width, height: sheet.height },
			staffColors: { subdepartment: subdepartmentColors, department: departmentColors },
			part
		});
	}

	function part(): 'all' | 'dispo' | 'staff' {
		return scope === 'dispo' ? 'dispo' : scope === 'dienste' ? 'staff' : 'all';
	}

	async function exportSheet(kind: 'csv' | 'xlsx' | 'pdf') {
		busy = true;
		error = '';
		result = '';
		notes = [];
		try {
			const data = await loadMonth(period);
			const name = `${
				scope === 'dispo' ? 'dispo' : scope === 'dienste' ? 'dienste' : 'dienstplan'
			}-${period.from.slice(0, 7)}`;
			if (kind === 'pdf') {
				save(`${name}.pdf`, toPdf(report(data, part())), 'application/pdf');
			} else if (scope === 'druck') {
				const print = report(data, 'all');
				if (kind === 'csv') save(`${name}.csv`, toCsv(print), 'text/csv;charset=utf-8');
				else
					save(
						`${name}.xlsx`,
						toXlsx(print),
						'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
					);
			} else if (scope === 'dispo') {
				const table = dispoRows(
					Number(yearChoice),
					Number(monthChoice),
					data.dispo,
					data.locations
				);
				toTable(table, name, kind, 'Dispo');
				result = `${table.rows.length} Zeilen`;
				notes = table.notes;
			} else {
				// Everybody the plan may hold a cell for, active or not: an import
				// replaces the month, so a missing person would lose their days.
				const people = await api.people.list();
				const table = planRows(
					Number(yearChoice),
					Number(monthChoice),
					data.cells,
					people,
					shifts
				);
				toTable(table, name, kind, 'Dienste');
				result = `${table.rows.length} Zeilen`;
				notes = table.notes;
			}
		} catch (problem) {
			error = errorText(problem);
		} finally {
			busy = false;
		}
	}

	function toTable(
		table: { head: string[]; rows: string[][] },
		name: string,
		kind: 'csv' | 'xlsx',
		sheetName: string
	) {
		if (kind === 'csv')
			save(`${name}.csv`, tableCsv(table.head, table.rows), 'text/csv;charset=utf-8');
		else
			save(
				`${name}.xlsx`,
				tableXlsx(table.head, table.rows, sheetName),
				'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
			);
	}

	/** A picked file or another month makes a preview stale. */
	function reset() {
		preview = null;
		error = '';
		result = '';
		notes = [];
	}

	function chooseFile(event: Event) {
		file = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;
		reset();
	}

	/** Reads the file and names what it carries; nothing is written. */
	async function scan() {
		if (file === null) return;
		busy = true;
		reset();
		try {
			const bytes = await file.arrayBuffer();
			const month = Number(monthChoice);
			const year = Number(yearChoice);
			if (intake === 'pem') {
				const scan = await api.people.duty.dispo.scan(bytes, month, year, !withoutTime);
				preview = {
					rows: scan.locations.reduce((total, place) => total + place.events.length, 0),
					days: scan.days,
					groups: scan.locations.map((place) => ({ name: place.name, rows: place.events.length })),
					warnings: scan.warnings,
					existing: scan.existing_entries,
					alreadyImported: scan.already_imported,
					unit: 'Standort'
				};
			} else {
				const dispo = intake === 'dispo';
				const scan = await api.people.duty.exchange.scan(
					dispo ? 'dispo' : 'plan',
					bytes,
					month,
					year,
					dispo ? !withoutTime : false
				);
				preview = {
					rows: scan.rows,
					days: scan.days,
					groups: scan.groups,
					warnings: scan.warnings,
					existing: scan.existing_entries,
					alreadyImported: scan.already_imported,
					unit: dispo ? 'Standort' : 'Person'
				};
			}
		} catch (problem) {
			preview = null;
			error = errorText(problem);
		} finally {
			busy = false;
		}
	}

	/** Writes the file into the chosen month. */
	async function write() {
		if (file === null) return;
		const recognized = preview?.rows ?? 0;
		busy = true;
		error = '';
		result = '';
		notes = [];
		try {
			const bytes = await file.arrayBuffer();
			const month = Number(monthChoice);
			const year = Number(yearChoice);
			if (intake === 'pem') {
				const report = await api.people.duty.dispo.import(bytes, month, year, replace, !withoutTime);
				notes = report.warnings;
				result = `${recognized} Termine gelesen, ${report.entries_created} geschrieben`;
				if (report.flags_created > 0) {
					result += `, ${report.flags_created} Flags angelegt`;
				}
			} else {
				const report =
					intake === 'dispo'
						? await api.people.duty.exchange.dispo(bytes, month, year, replace, !withoutTime)
						: await api.people.duty.exchange.plan(bytes, month, year, replace);
				notes = report.warnings;
				result = `${report.rows} Zeilen gelesen, ${report.written} geschrieben`;
			}
			preview = null;
			onimported();
		} catch (problem) {
			error = importError(problem);
		} finally {
			busy = false;
		}
	}

	/** A refused duplicate is the one error the window explains itself. */
	function importError(problem: unknown): string {
		if (problem instanceof ApiError && problem.status === 409) {
			return `Diese Dispo steht für ${periodLabel} bereits drin. Zum Neuschreiben „Einträge dieses Monats ersetzen“ anhaken.`;
		}
		return errorText(problem);
	}

	/** `Importieren` reads the file first; without the preview switch it writes right away. */
	async function run() {
		if (preview !== null) {
			await write();
			return;
		}
		await scan();
		if (!showPreview && preview !== null) await write();
	}

	/** The employees with an address, as the selection offers them. */
	const withAddress = $derived(employees.filter((person) => (person.email ?? '').trim() !== ''));

	function toggleChoice(id: number | null, checked: boolean) {
		if (id == null) return;
		calendarChoice = checked ? [...calendarChoice, id] : calendarChoice.filter((entry) => entry !== id);
	}

	/** Sends one calendar per chosen employee and says who was left out. */
	async function sendCalendar() {
		sending = true;
		sendError = '';
		sent = '';
		notSent = [];
		try {
			const report = await api.people.duty.ical.send(period.from, period.to, calendarChoice);
			sent =
				report.sent === 1 ? '1 Dienstplan gesendet' : `${report.sent} Dienstpläne gesendet`;
			notSent = report.skipped.map(
				(entry) => `${entry.name}: ${entry.reason}`
			);
			icsOpen = false;
			calendarChoice = [];
		} catch (problem) {
			sendError = errorText(problem);
		} finally {
			sending = false;
		}
	}


	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>


<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) onclose();
	}}
></div>

<div class="window" role="dialog" aria-modal="true" aria-label="Import / Export">
	<header class="head">
		<h2>Import / Export</h2>
		<div class="month">
			<label for="tausch-monat">Monat</label>
			<select id="tausch-monat" bind:value={monthChoice} onchange={reset}>
				{#each MONTHS as name, index (name)}
					<option value={String(index + 1)}>{name}</option>
				{/each}
			</select>
			<label for="tausch-jahr">Jahr</label>
			<select id="tausch-jahr" bind:value={yearChoice} onchange={reset}>
				{#each YEARS as year (year)}
					<option value={String(year)}>{year}</option>
				{/each}
			</select>
		</div>
		<button type="button" class="close" aria-label="Schließen" onclick={onclose}>×</button>
	</header>

	<div class="block">
		<h3>Export</h3>
		<div class="choice">
			<select bind:value={scope} aria-label="Umfang des Exports">
				<option value="druck">Wie Druckansicht</option>
				<option value="dispo">Nur Dispo inkl. Standorte</option>
				<option value="dienste">Nur Dienste</option>
			</select>
			<select bind:value={paper} aria-label="Papier">
				{#each PAPERS as entry (entry.value)}
					<option value={entry.value}>{entry.label}</option>
				{/each}
			</select>
			<button type="button" onclick={() => void exportSheet('csv')} disabled={busy}>CSV</button>
			<button type="button" onclick={() => void exportSheet('xlsx')} disabled={busy}>XLSX</button>
			<button type="button" onclick={() => void exportSheet('pdf')} disabled={busy}>PDF</button>
		</div>
	</div>

	<div class="block">
		<h3>Import</h3>
		<div class="choice">
			<select bind:value={intake} onchange={reset} aria-label="Umfang des Imports">
				<option value="dispo">Dispo inkl. Standorte</option>
				<option value="dienste">Dienste</option>
				<option value="pem">Dispo (PeM)</option>
			</select>
			<input
				type="file"
				accept={intake === 'pem' ? '.xlsx' : '.csv,.xlsx'}
				aria-label="Datei"
				onchange={chooseFile}
			/>
			<button
				type="button"
				onclick={() => void run()}
				disabled={busy || file === null || (preview?.alreadyImported === true && !replace)}
			>
				{showPreview && preview === null ? 'Datei prüfen' : 'Importieren'}
			</button>
		</div>

		<div class="toggles">
			<label>
				<input type="checkbox" bind:checked={showPreview} />
				Vorschau zeigen, bevor geschrieben wird
			</label>
			<label>
				<input type="checkbox" bind:checked={replace} />
				Einträge dieses Monats ersetzen
			</label>
			{#if intake !== 'dienste'}
				<label>
					<input type="checkbox" bind:checked={withoutTime} />
					Zeilen ohne Uhrzeit ebenfalls übernehmen
				</label>
			{/if}
		</div>

		{#if preview !== null}
			<p class="summary">
				{preview.rows} Zeilen · {preview.days} Tage · {preview.groups.length}
				{preview.groups.length === 1 ? preview.unit : `${preview.unit}e`} · {preview.existing}
				vorhanden
			</p>
			{#if preview.alreadyImported}
				<p class="bad">
					Diese Datei wurde für {periodLabel} bereits geschrieben — erneut nur mit „Einträge dieses
					Monats ersetzen“.
				</p>
			{/if}
			<ul class="groups">
				{#each preview.groups as group (group.name)}
					<li>
						<span class="name" title={group.name}>{group.name}</span>
						<span class="count">{group.rows}</span>
					</li>
				{/each}
			</ul>
			{#each preview.warnings as warning (warning)}
				<p class="warning">{warning}</p>
			{/each}
		{/if}
	</div>

	<div class="block">
		<h3>ICS</h3>
		<div class="choice">
			<button type="button" onclick={() => (icsOpen = true)} disabled={busy}>
				Persönlicher Dienstplan senden
			</button>
			{#if sent !== ''}<span class="good">{sent}</span>{/if}
		</div>

		{#if icsOpen}
			<ul class="people">
				{#each withAddress as person (person.id)}
					<li>
						<label>
							<input
								type="checkbox"
								checked={calendarChoice.includes(person.id ?? -1)}
								onchange={(event) =>
									toggleChoice(person.id, event.currentTarget.checked)}
							/>
							{person.last_name}, {person.first_name}
						</label>
					</li>
				{:else}
					<li class="empty">Kein Mitarbeiter mit eMail-Adresse.</li>
				{/each}
			</ul>
			<div class="choice">
				<button
					type="button"
					onclick={() => void sendCalendar()}
					disabled={sending || calendarChoice.length === 0}>{sending ? 'Sende …' : 'Senden'}</button
				>
				<button type="button" onclick={() => (icsOpen = false)}>Abbrechen</button>
			</div>
		{/if}

		{#if sendError}<p class="bad">{sendError}</p>{/if}
		{#if notSent.length > 0}
			<ul class="notes">
				{#each notSent as entry (entry)}
					<li>{entry}</li>
				{/each}
			</ul>
		{/if}
	</div>

	{#if error}<p class="bad">{error}</p>{/if}
	{#if result}<p class="good">{result}</p>{/if}
	{#if notes.length > 0}
		<ul class="notes">
			{#each notes as note, number (number)}
				<li>{note}</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(15, 23, 42, 0.55);
		z-index: 40;
	}
	.window {
		position: fixed;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: min(36rem, 94vw);
		max-height: 88vh;
		padding: 16px 20px;
		overflow: auto;
		background: var(--where-surface, #fff);
		color: var(--where-label, #111);
		border-radius: 12px;
		box-shadow: 0 18px 50px rgba(15, 23, 42, 0.35);
		z-index: 41;
		color-scheme: light;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.head h2 {
		margin: 0;
		font-size: 18px;
	}
	.month {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-left: auto;
		font-size: 13px;
	}
	.month select,
	.choice select {
		font-family: inherit;
		font-size: 13px;
		padding: 2px 4px;
	}
	.close {
		font-size: 20px;
		line-height: 1;
		padding: 2px 10px;
	}
	.block {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding-top: 10px;
		border-top: 1px solid var(--menu-band-border, #cbd5e1);
	}
	.block h3 {
		margin: 0;
		font-size: 14px;
	}
	.choice {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}
	.choice input[type='file'] {
		flex: 1 1 10rem;
		min-width: 0;
		font-size: 13px;
	}
	.toggles {
		display: flex;
		flex-direction: column;
		gap: 2px;
		font-size: 13px;
	}
	.toggles label {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.summary {
		margin: 0;
		font-size: 13px;
	}
	.groups {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 1px 10px;
		margin: 0;
		padding: 0;
		list-style: none;
		max-height: 11rem;
		overflow: auto;
		font-size: 13px;
	}
	.groups li {
		display: contents;
	}
	.groups .name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.groups .count {
		text-align: right;
		opacity: 0.75;
	}
	.bad {
		margin: 0;
		color: #b91c1c;
		font-size: 13px;
	}
	.good {
		margin: 0;
		font-size: 13px;
	}
	.warning {
		margin: 0;
		font-size: 13px;
		color: #b45309;
	}
	.notes {
		margin: 0;
		padding-left: 1.2rem;
		font-size: 13px;
		color: #b45309;
	}
	/* The list of employees for the calendar mail. */
	.people {
		margin: 0;
		padding: 0;
		list-style: none;
		max-height: 12rem;
		overflow: auto;
		font-size: 13px;
	}
	.people label {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 1px 2px;
	}
	.people .empty {
		opacity: 0.7;
	}
</style>
