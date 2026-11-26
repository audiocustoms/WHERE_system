<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '$core/escape.svelte';
	import { api, errorText, type MasterArea, type MasterReport } from '$core/api';
	import '$core/search/search-window.css';
	import {
		saveSheet,
		listRows,
		staffRows,
		shiftRows
	} from './master-data-exchange';

	/**
	 * „Import / Export" of the master data.
	 *
	 * The window belongs to one area — the lists of the employee mask, the employees or
	 * the shifts — and asks the same questions for all of them: export as CSV, and for the
	 * import a preview before anything is written and whether records the file names again
	 * are overwritten. Nothing is ever deleted.
	 */
	interface Props {
		area: MasterArea;
		/** What the window calls its area, e.g. `Abteilungen und Positionen`. */
		title: string;
		/** File name of the export without extension. */
		filename: string;
		/** Something was written; the page reloads. */
		onimported: () => void;
		onclose: () => void;
	}

	let { area, title, filename, onimported, onclose }: Props = $props();

	let file = $state<File | null>(null);
	let showPreview = $state(true);
	// Master data is not a month: nothing is overwritten unless the user says so.
	let replace = $state(false);
	let busy = $state(false);
	let error = $state('');
	let result = $state('');
	let preview = $state<MasterReport | null>(null);

	function reset() {
		preview = null;
		error = '';
		result = '';
	}

	function chooseFile(event: Event) {
		file = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;
		reset();
	}

	/** Writes the master data of this area as the sheet the import reads. */
	async function exportSheet() {
		busy = true;
		error = '';
		result = '';
		try {
			if (area === 'lists') {
				const [departments, subdepartments, positions, locations] = await Promise.all([
					api.people.lists.list('departments'),
					api.people.lists.list('subdepartments'),
					api.people.lists.list('positions'),
					api.people.lists.list('locations')
				]);
				const sheet = listRows(departments, subdepartments, positions, locations);
				saveSheet(filename, sheet);
				result = t('{n} Einträge', { n: sheet.rows.length });
			} else if (area === 'employees') {
				const people = await api.people.list();
				const sheet = staffRows(people);
				saveSheet(filename, sheet);
				result = t('{n} Personen', { n: sheet.rows.length });
			} else {
				const shifts = await api.people.shifts.list();
				const sheet = shiftRows(shifts);
				saveSheet(filename, sheet);
				result = t('{n} Schichten', { n: sheet.rows.length });
			}
		} catch (problem) {
			error = errorText(problem);
		} finally {
			busy = false;
		}
	}

	/** Reads the file and names what it carries; nothing is written. */
	async function scan() {
		if (file === null) return;
		busy = true;
		reset();
		try {
			preview = await api.people.masterdata.scan(area, await file.arrayBuffer(), replace);
		} catch (problem) {
			preview = null;
			error = errorText(problem);
		} finally {
			busy = false;
		}
	}

	/** Writes the file into the master data. */
	async function write() {
		if (file === null) return;
		busy = true;
		error = '';
		result = '';
		preview = null;
		try {
			const report = await api.people.masterdata.import(
				area,
				await file.arrayBuffer(),
				replace
			);
			const parts = [
				t('{n} Zeilen gelesen', { n: report.rows }),
				t('{n} neu', { n: report.created })
			];
			if (report.changed > 0) parts.push(t('{n} überschrieben', { n: report.changed }));
			if (report.unchanged > 0) parts.push(t('{n} unverändert', { n: report.unchanged }));
			result = parts.join(', ');
			error = report.warnings.join('\n');
			onimported();
		} catch (problem) {
			error = errorText(problem);
		} finally {
			busy = false;
		}
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

<div class="window" role="dialog" aria-modal="true" aria-label={t('Import / Export')}>
	<header class="head">
		<h2>{t('Import / Export')}</h2>
		<span class="subject">{title}</span>
		<button type="button" class="close" aria-label={t('Schließen')} onclick={onclose}>×</button>
	</header>

	<div class="block">
		<h3>{t('Export')}</h3>
		<div class="choice">
			<span class="note">{filename}{t('.csv')}</span>
			<button type="button" onclick={() => void exportSheet()} disabled={busy}>{t('CSV')}</button>
		</div>
	</div>

	<div class="block">
		<h3>{t('Import')}</h3>
		<div class="choice">
			<input type="file" accept=".csv,.xlsx" aria-label={t('Datei')} onchange={chooseFile} />
			<button
				type="button"
				onclick={() => void run()}
				disabled={busy || file === null}
			>
				{showPreview && preview === null ? t('Datei prüfen') : t('Importieren')}
			</button>
		</div>

		<div class="toggles">
			<label>
				<input type="checkbox" bind:checked={showPreview} />
				{t('Vorschau zeigen, bevor geschrieben wird')}
			</label>
			<label>
				<input type="checkbox" bind:checked={replace} onchange={reset} />
				{t('Vorhandene Einträge überschreiben')}
			</label>
		</div>

		{#if preview !== null}
			<p class="summary">
				{t('{rows} Zeilen · {created} neu · {changed} geändert · {unchanged} unverändert', {
					rows: preview.rows,
					created: preview.created,
					changed: preview.changed,
					unchanged: preview.unchanged
				})}
			</p>
			{#each preview.warnings as warning (warning)}
				<p class="warning">{warning}</p>
			{/each}
		{/if}
	</div>

	{#if error}<p class="bad">{error}</p>{/if}
	{#if result}<p class="good">{result}</p>{/if}
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
		width: min(34rem, 94vw);
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
	.subject {
		margin-left: auto;
		font-size: 13px;
		color: var(--muted, #64748b);
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
	.choice button {
		font-family: inherit;
		font-size: 13px;
		padding: 3px 12px;
	}
	.note {
		font-size: 13px;
		color: var(--muted, #64748b);
	}
	.toggles {
		display: flex;
		flex-direction: column;
		gap: 4px;
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
	.warning {
		margin: 0;
		font-size: 12px;
		color: #92400e;
	}
	.bad {
		margin: 0;
		font-size: 13px;
		color: #b4564f;
		white-space: pre-line;
	}
	.good {
		margin: 0;
		font-size: 13px;
		color: #1f7a3d;
	}
</style>
