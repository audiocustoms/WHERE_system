<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type PeopleFlag, type PeopleListEntry } from '$core/api';
	import { askConfirm } from '$core/confirm.svelte';

	/**
	 * The flags of a location: how many people of which position a `Vst`, a `Probe`
	 * or a `TE` needs in this house. Every location plans its own crew, so the rows
	 * shown here belong to `location` alone.
	 */
	interface Props {
		/** The location the rows belong to. */
		location: PeopleListEntry;
		/** Maintained positions, in their master-data order. */
		positions: string[];
		/** Only the administration may change the numbers. */
		mayEdit: boolean;
	}

	let { location, positions, mayEdit }: Props = $props();

	/** The position value that does not care which employee covers the row. */
	const NOT_RELEVANT = '*';

	let flags = $state<PeopleFlag[]>([]);
	let selected = $state<number | null>(null);
	/** Rows of the selected flag in this location. */
	let rows = $state<{ position: string; value: string }[]>([]);
	let creating = $state(false);
	let newCode = $state('');
	let newName = $state('');
	let busy = $state(false);
	let failure = $state<string | null>(null);

	const flag = $derived(flags.find((entry) => entry.id === selected) ?? null);

	onMount(load);

	async function load() {
		try {
			flags = await api.people.flags.list();
			if (selected !== null && !flags.some((entry) => entry.id === selected)) {
				selected = null;
			}
			selected = selected ?? flags[0]?.id ?? null;
			loadRows();
		} catch (problem) {
			failure = errorText(problem);
		}
	}

	/** Fills the rows from the stored staffing of this location. */
	function loadRows() {
		rows =
			flag?.staffing
				.filter((row) => row.location_id === location.id)
				.map((row) => ({
					position: row.position,
					value: String(row.needed)
				})) ?? [];
	}

	/** A flag was chosen in the list. */
	function pick(id: string) {
		selected = id === '' ? null : Number(id);
		loadRows();
		failure = null;
	}

	/** Positions this flag could still take beside the rows it has. */
	function freePositions(row: { position: string }): string[] {
		const taken = rows
			.map((entry) => entry.position.trim().toLowerCase())
			.filter((entry) => entry !== '' && entry !== row.position.trim().toLowerCase());
		return positions.filter((entry) => !taken.includes(entry.trim().toLowerCase()));
	}

	async function save() {
		if (flag === null) return;
		busy = true;
		failure = null;
		try {
			const staffing = rows
				.filter((row) => row.position.trim() !== '' && Number(row.value) > 0)
				.map((row) => ({
					location_id: location.id,
					position: row.position.trim(),
					needed: Number(row.value)
				}));
			await api.people.flags.setStaffing(flag.id, location.id, staffing);
			await load();
		} catch (problem) {
			failure = errorText(problem);
		} finally {
			busy = false;
		}
	}

	function addRow() {
		rows = [...rows, { position: '', value: '1' }];
	}

	async function removeRow(index: number) {
		rows = rows.filter((_, position) => position !== index);
		await save();
	}

	async function create() {
		if (newCode.trim() === '' || newName.trim() === '') return;
		busy = true;
		failure = null;
		try {
			const created = await api.people.flags.create(newCode.trim(), newName.trim());
			creating = false;
			newCode = '';
			newName = '';
			flags = await api.people.flags.list();
			pick(String(created.id));
		} catch (problem) {
			failure = errorText(problem);
		} finally {
			busy = false;
		}
	}

	async function remove() {
		if (flag === null) return;
		const confirmed = await askConfirm({
			title: t('Flag löschen'),
			message: t('„{code}“ wird mit seinen Angaben für alle Standorte gelöscht.', {
				code: flag.code
			}),
			confirmLabel: t('Löschen')
		});
		if (!confirmed) return;
		busy = true;
		failure = null;
		try {
			await api.people.flags.remove(flag.id);
			selected = null;
			await load();
		} catch (problem) {
			failure = errorText(problem);
		} finally {
			busy = false;
		}
	}

	async function presets() {
		busy = true;
		failure = null;
		try {
			await api.people.flags.presets();
			await load();
		} catch (problem) {
			failure = errorText(problem);
		} finally {
			busy = false;
		}
	}
</script>

<div class="flags">
	<div class="head">
		<span class="title">{t('Flags')}</span>
		{#if mayEdit}
			<button type="button" class="sw-link small" disabled={busy} onclick={presets}
				>{t('Vorlagen')}</button
			>
			<button type="button" class="sw-link small" disabled={busy} onclick={() => (creating = !creating)}
				>{t('Neu')}</button
			>
			{#if flag !== null}
				<button type="button" class="sw-link small danger" disabled={busy} onclick={remove}
					>{t('Löschen')}</button
				>
			{/if}
		{/if}
	</div>

	{#if creating && mayEdit}
		<div class="create">
			<input
				class="code"
				maxlength="20"
				placeholder={t('Kürzel')}
				aria-label={t('Kürzel des neuen Flags')}
				bind:value={newCode}
				disabled={busy}
			/>
			<input
				placeholder={t('Bezeichnung')}
				maxlength="100"
				aria-label={t('Bezeichnung des neuen Flags')}
				bind:value={newName}
				disabled={busy}
				onkeydown={(event) => {
					if (event.key === 'Enter') void create();
				}}
			/>
			<button type="button" class="sw-button small" disabled={busy} onclick={create}>{t('Anlegen')}</button>
		</div>
	{/if}

	<select
		class="flagChoice"
		aria-label={t('Flag')}
		disabled={busy || flags.length === 0}
		value={selected === null ? '' : String(selected)}
		onchange={(event) => pick(event.currentTarget.value)}
	>
		{#each flags as entry (entry.id)}
			<option value={String(entry.id)}>{entry.code}· {entry.name}</option>
		{/each}
	</select>

	{#if flag !== null}
		<div class="rowHead">
			<span>{t('Position')}</span>
			<span>{t('Bedarf')}</span>
			<span></span>
		</div>
		{#each rows as row, index (index)}
			<div class="row">
				<select
					aria-label={t('Position in {flag}', { flag: flag.code })}
					disabled={!mayEdit || busy}
					bind:value={row.position}
					onchange={save}
				>
					<option value="">—</option>
					<option value={NOT_RELEVANT}>{t('nicht relevant')}</option>
					{#each freePositions(row) as position (position)}
						<option value={position}>{position}</option>
					{/each}
					{#if row.position !== '' && row.position !== NOT_RELEVANT && !positions.includes(row.position)}
						<option value={row.position}>{row.position}</option>
					{/if}
				</select>
				<input
					type="number"
					min="0"
					max="99"
					step="1"
					inputmode="numeric"
					aria-label={t('Bedarf {flag} {position}', {
						flag: flag.code,
						position: row.position
					})}
					disabled={!mayEdit || busy}
					bind:value={row.value}
					onchange={save}
				/>
				{#if mayEdit}
					<button
						type="button"
						class="drop"
						title={t('Zeile entfernen')}
						aria-label={t('Zeile entfernen')}
						disabled={busy}
						onclick={() => void removeRow(index)}>−</button
					>
				{/if}
			</div>
		{/each}
		{#if mayEdit}
			<button type="button" class="sw-link small" disabled={busy} onclick={addRow}
				>{t('+ Position')}</button
			>
		{/if}
	{/if}

	{#if failure}<p class="error">{failure}</p>{/if}
</div>

<style>
	.flags {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		margin-top: 0.9rem;
		padding-top: 0.7rem;
		border-top: 1px solid var(--menu-band-border);
	}
	.head {
		display: flex;
		align-items: baseline;
		gap: 0.6rem;
	}
	.title {
		flex: 1;
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.create {
		display: flex;
		gap: 0.3rem;
	}
	.create input {
		min-width: 0;
		padding: 0.12rem 0.3rem;
		border: 1px solid var(--menu-band-border);
		border-radius: 2px;
		background: var(--surface);
		color: var(--label);
		font-family: inherit;
		font-size: 0.78rem;
	}
	.create .code {
		flex: 0 0 5rem;
	}
	.create input:not(.code) {
		flex: 1 1 auto;
	}
	.flagChoice {
		padding: 0.12rem 0.3rem;
		border: 1px solid var(--menu-band-border);
		border-radius: 2px;
		background: var(--surface);
		color: var(--label);
		font-family: inherit;
		font-size: 0.78rem;
	}
	.rowHead,
	.row {
		display: grid;
		grid-template-columns: 1fr 4rem 1.6rem;
		gap: 0.3rem;
		align-items: center;
	}
	.rowHead {
		font-size: 0.72rem;
		color: var(--muted);
	}
	.row select,
	.row input {
		min-width: 0;
		padding: 0.12rem 0.3rem;
		border: 1px solid var(--menu-band-border);
		border-radius: 2px;
		background: var(--surface);
		color: var(--label);
		font-family: inherit;
		font-size: 0.78rem;
	}
	.row input {
		text-align: right;
	}
	.drop {
		border: none;
		background: none;
		color: var(--muted);
		cursor: pointer;
		font-size: 0.9rem;
		line-height: 1;
	}
	.drop:hover {
		color: var(--bad);
	}
	.error {
		margin: 0.2rem 0 0;
		color: var(--bad);
		font-size: 0.78rem;
	}
</style>
