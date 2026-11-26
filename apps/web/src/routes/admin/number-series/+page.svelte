<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type NumberSeries, type NumberSeriesInput } from '../../../lib/api';

	let circles = $state<NumberSeries[]>([]);
	let loading = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);
	let hint = $state<string | null>(null);

	let editKey = $state<string | null>(null);
	let form = $state<NumberSeriesInput | null>(null);

	async function load() {
		loading = true;
		error = null;
		try {
			circles = await api.masterData.numberSeries();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			loading = false;
		}
	}

	function edit(k: NumberSeries) {
		editKey = k.key;
		form = {
			name: k.name,
			prefix: k.prefix,
			format: k.format,
			digits: k.digits,
			start_value: k.start_value,
			step: k.step,
			yearly_reset: k.yearly_reset,
			active: k.active,
			current_value: k.current_value
		};
		error = null;
		hint = null;
	}

	async function save() {
		if (!editKey || !form) return;
		saving = true;
		error = null;
		try {
			const updated = await api.masterData.numberSeriesUpdate(editKey, form);
			hint = t('„{name}“ gespeichert — nächste Nummer: {number}', {
				name: updated.name,
				number: updated.next_number ?? '—'
			});
			editKey = null;
			form = null;
			await load();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			saving = false;
		}
	}

	async function reset(k: NumberSeries) {
		if (
			!confirm(
				t(
					'Zähler von „{name}“ auf den Startwert zurücksetzen?\n\nDie nächste vergebene Nummer ist danach {start}. Bereits vergebene Nummern werden nicht geprüft.',
					{ name: k.name, start: k.start_value }
				)
			)
		)
			return;
		error = null;
		hint = null;
		try {
			const k2 = await api.masterData.numberSeriesReset(k.key);
			hint = t('„{name}“ zurückgesetzt — nächste Nummer: {number}', {
				name: k2.name,
				number: k2.next_number ?? '—'
			});
			await load();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	onMount(load);
</script>

<svelte:head><title>{t('Nummernkreise — WHERE')}</title></svelte:head>

<h1>{t('Nummernkreise')}</h1>

<div class="notice">
	<strong>{t('Format:')}</strong>
	{t(
		'Der Platzhalter {number} wird auf die angegebene Stellenzahl mit führenden Nullen aufgefüllt; {year}, {month} und {day} setzen das Datum ein.',
		{ number: '{number}', year: '{year}', month: '{month}', day: '{day}' }
	)}
	{t('Beispiel: Präfix {prefix}, Format {format}, Stellen 4 → {sample}.', {
		prefix: 'RE-',
		format: '{year}-{number}',
		sample: 'RE-2026-0042'
	})}
	{t('Das Präfix steht immer vorn. Bei Jahres-Neustart beginnt der Kreis im neuen Jahr wieder beim Startwert.')}
</div>

<table>
	<thead>
		<tr>
			<th>{t('Nummernkreis')}</th>
			<th>{t('Nächste Nummer')}</th>
			<th class="numeric">{t('Aktuell')}</th>
			<th class="numeric">{t('Start')}</th>
			<th class="numeric">{t('Schritt')}</th>
			<th>{t('Aktiv')}</th>
			<th></th>
		</tr>
	</thead>
	<tbody>
		{#each circles as k (k.key)}
			{#if editKey === k.key && form}
				<tr class="edit">
					<td>
						<input class="name" bind:value={form.name} maxlength="120" />
						<span class="key">{k.key}</span>
					</td>
					<td>
						<div class="fields">
							<label>{t('Präfix')}<input class="short" bind:value={form.prefix} maxlength="16" /></label>
							<label>{t('Format')}<input class="medium" bind:value={form.format} maxlength="64" /></label>
							<label>{t('Stellen')}<input class="mini" type="number" min="0" max="12" bind:value={form.digits} /></label>
						</div>
						<div class="fields">
							<label>{t('Startwert')}<input class="mini" type="number" min="0" bind:value={form.start_value} /></label>
							<label>{t('Schrittweite')}<input class="mini" type="number" min="1" bind:value={form.step} /></label>
							<label class="check">
								<input type="checkbox" bind:checked={form.yearly_reset} /> {t('Jahres-Neustart')}
							</label>
						</div>
					</td>
					<td class="numeric">
						<input class="mini" type="number" min="0" bind:value={form.current_value} />
					</td>
					<td class="numeric">—</td>
					<td class="numeric">—</td>
					<td><input type="checkbox" bind:checked={form.active} /></td>
					<td class="actions">
						<button onclick={save} disabled={saving}>{t('speichern')}</button>
						<button class="link" onclick={() => ((editKey = null), (form = null))}>{t('abbrechen')}</button>
					</td>
				</tr>
			{:else}
				<tr class:inactive={!k.active}>
					<td>
						{k.name}
						{#if k.legacy_source}
							<span class="source" title={t('Herkunft in der 4D-Struktur')}>{k.legacy_source}</span>
						{/if}
					</td>
					<td><code>{k.next_number ?? '—'}</code></td>
					<td class="numeric">{k.current_value || '—'}</td>
					<td class="numeric">{k.start_value}</td>
					<td class="numeric">{k.step}</td>
					<td>
						{#if k.active}
							{t('ja')}{#if k.yearly_reset}<span class="small">{t(', je Jahr')}</span>{/if}
						{:else}
							{t('nein')}
						{/if}
					</td>
					<td class="actions">
						<button class="link" onclick={() => edit(k)}>{t('bearbeiten')}</button>
						<button class="link" onclick={() => reset(k)}>{t('zurücksetzen')}</button>
					</td>
				</tr>
			{/if}
		{:else}
			<tr><td colspan="7">{t('Keine Nummernkreise angelegt.')}</td></tr>
		{/each}
	</tbody>
</table>

<p class="footnote">
	{loading
		? t('Lade …')
		: t(
				'{n} Nummernkreise · Herkunft in der 4D-Struktur steht als kleine Zeile unter der Bezeichnung.',
				{ n: circles.length }
			)}
</p>

{#if error}
	<p class="bad">{error}</p>
{/if}
{#if hint}
	<p class="good">{hint}</p>
{/if}

<style>
	.notice {
		background: var(--notice-bg);
		border: 1px solid var(--notice-border);
		border-radius: 6px;
		padding: 0.7rem 0.9rem;
		font-size: 0.85rem;
		max-width: 56rem;
		margin-bottom: 1.25rem;
	}
	.bad {
		color: var(--bad);
	}
	.good {
		color: var(--good);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		background: var(--surface);
	}
	th,
	td {
		text-align: left;
		padding: 0.45rem 0.6rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.9rem;
		vertical-align: top;
	}
	th.numeric,
	td.numeric {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	td.actions {
		white-space: nowrap;
	}
	td.actions button + button {
		margin-left: 0.6rem;
	}
	tr.inactive td {
		color: var(--muted-2);
	}
	tr.edit {
		background: var(--surface-2);
	}
	.key,
	.source {
		display: block;
		font-size: 0.72rem;
		color: var(--muted-2);
		font-family: ui-monospace, Consolas, monospace;
	}
	.small {
		font-size: 0.75rem;
		color: var(--muted-2);
	}
	code {
		font-size: 0.85rem;
	}
	input {
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font: inherit;
	}
	.fields {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		margin-bottom: 0.4rem;
	}
	.fields label {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.72rem;
		color: var(--muted);
	}
	.fields label.check {
		flex-direction: row;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.8rem;
		padding-top: 1rem;
	}
	input.name {
		min-width: 12rem;
	}
	input.medium {
		min-width: 9rem;
	}
	input.short {
		min-width: 4.5rem;
	}
	input.mini {
		width: 5rem;
	}
	button {
		padding: 0.4rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
	button:disabled {
		cursor: not-allowed;
		color: var(--muted-2);
	}
	button.link {
		border: none;
		background: none;
		padding: 0;
		color: var(--accent);
	}
	.footnote {
		font-size: 0.8rem;
		color: var(--muted-2);
		margin-top: 0.75rem;
	}
</style>
