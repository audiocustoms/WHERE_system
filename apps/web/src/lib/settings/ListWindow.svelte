<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '../escape.svelte';
	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type SettingsArea,
		type SettingsField,
		type SettingsRow
	} from '../api';
	import { askConfirm } from '../confirm.svelte';
	import '../search/search-window.css';
	import SettingsDialog from './SettingsDialog.svelte';

	interface Props {
		area: SettingsArea;
		onclose: () => void;
		/** Reports changes to the page so that it can reload. */
		onchange?: () => void;
	}

	let { area, onclose, onchange = () => {} }: Props = $props();

	let rows = $state<SettingsRow[]>([]);
	let choice = $state<SettingsRow | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let dialog = $state<{ row: SettingsRow | null } | null>(null);

	async function load() {
		loading = true;
		error = null;
		try {
			const list = await api.settings.list(area.key);
			rows = list.rows;
			choice = rows.find((row) => row.id === choice?.id) ?? null;
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	onMount(load);

	function value(row: SettingsRow, field: SettingsField): string {
		const value = row.values[field.column];
		if (value === null || value === undefined || value === '') return '';
		if (field.kind === 'bool') return value === 'true' ? t('Ja') : t('Nein');
		if (field.kind === 'date') {
			const [year, month, day] = value.split('-');
			return year ? `${day}.${month}.${year}` : value;
		}
		if (field.kind === 'number') return value.replace('.', ',');
		return value;
	}

	async function loeschen() {
		if (!choice) return;
		const nameField = area.name_column;
		const name = choice.values[nameField] ?? area.title;
		const ok = await askConfirm({
			title: t('{area} löschen', { area: area.title }),
			message: t('„{name}“ wird aus der Liste entfernt. Verweise darauf behalten ihre Nummer.', {
				name
			}),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		try {
			await api.settings.remove(area.key, choice.id);
			choice = null;
			await load();
			onchange();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function saved() {
		dialog = null;
		await load();
		onchange();
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window lf"
		role="dialog"
		aria-modal="true"
		aria-label={area.title}
		style="width: min(46rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{area.title}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<div class="lf-body">
			<div class="lf-list">
				{#if error}
					<p class="sw-error">{error}</p>
				{/if}
				<table>
					<thead>
						<tr>
							{#each area.fields as field (field.column)}
								<th class:number={field.kind === 'integer' || field.kind === 'number'}>
									{field.label}
								</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each rows as row (row.id)}
							<tr
								class:active={choice?.id === row.id}
								tabindex="0"
								role="button"
								onclick={() => (choice = row)}
								ondblclick={() => (dialog = { row })}
								onkeydown={(e) => {
									if (e.key === 'Enter') choice = row;
								}}
							>
								{#each area.fields as field (field.column)}
									<td class:number={field.kind === 'integer' || field.kind === 'number'}>
										{value(row, field)}
									</td>
								{/each}
							</tr>
						{:else}
							<tr class="empty">
								<td colspan={area.fields.length}>
									{loading ? t('lade …') : t('Noch kein Eintrag — „Neuer Eintrag“ legt den ersten an.')}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<div class="lf-buttons">
				<button type="button" onclick={() => (dialog = { row: null })}>{t('Neuer Eintrag')}</button>
				<button type="button" disabled={!choice} onclick={() => choice && (dialog = { row: choice })}>
					{t('Umbenennen')}
				</button>
				<button type="button" disabled={!choice} onclick={loeschen}>{t('Löschen')}</button>
				<span class="spacer"></span>
				<button type="button" onclick={onclose}>{t('Schließen')}</button>
			</div>
		</div>

		<p class="lf-footer">
			{t('Einträge:')} {rows.length}
			<span class="lf-note">{area.description}</span>
		</p>
	</div>
</div>

{#if dialog}
	<SettingsDialog
		area={area}
		row={dialog.row}
		onsaved={saved}
		oncancel={() => (dialog = null)}
	/>
{/if}

<style>
	.lf {
		max-height: 80vh;
	}
	.lf-body {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 9.5rem;
		gap: 0.5rem;
		padding: 0.5rem;
		min-height: 16rem;
		max-height: calc(80vh - 6rem);
	}
	.lf-list {
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.83rem;
	}
	th,
	td {
		padding: 0.22rem 0.45rem;
		text-align: left;
		border-bottom: 1px solid var(--surface-3);
		white-space: nowrap;
	}
	th {
		position: sticky;
		top: 0;
		background: var(--surface-2);
		color: var(--muted);
		font-size: 0.76rem;
		font-weight: 600;
		z-index: 1;
	}
	th.number,
	td.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	tbody tr {
		cursor: pointer;
	}
	tbody tr:hover {
		background: var(--surface-2);
	}
	tbody tr.active {
		background: var(--highlight);
	}
	tr.empty td {
		color: var(--muted-2);
		padding: 0.6rem 0.45rem;
	}
	.lf-buttons {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.lf-buttons button {
		padding: 0.32rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
		cursor: pointer;
	}
	.lf-buttons button:hover:not(:disabled) {
		background: var(--highlight);
	}
	.lf-buttons button:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.lf-buttons .spacer {
		flex: 1;
	}
	.lf-footer {
		display: flex;
		gap: 1rem;
		align-items: baseline;
		margin: 0;
		padding: 0.3rem 0.6rem 0.45rem;
		border-top: 1px solid var(--border);
		font-size: 0.76rem;
		color: var(--muted);
	}
	.lf-note {
		margin-left: auto;
		color: var(--muted-2);
		text-align: right;
		max-width: 60%;
	}
</style>
