<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '$core/escape.svelte';
	import { onMount } from 'svelte';
	import { api, errorText, type DispoEntry, type PeopleFlag } from '$core/api';
	import '$core/search/search-window.css';

	/**
	 * The events of one location on one day („Großes Haus am 01.12.2026").
	 *
	 * Every event is its own line with its own time, so both stay editable: add
	 * a line with `+`, remove it with `−`, change time and text and save. The
	 * whole day is written at once, which keeps the order of the lines.
	 */
	interface Props {
		/** Location the window belongs to. */
		location: { id: number; name: string };
		/** The day (`YYYY-MM-DD` and its German label). */
		day: { iso: string; weekday: string; day: string };
		/** The stored entries of that day, in their order. */
		entries: DispoEntry[];
		/** Whether the user may change the plan. */
		mayEdit: boolean;
		/** Called on close; `changed` tells the board to reload. */
		onclose: (changed: boolean) => void;
	}

	let { location, day, entries, mayEdit, onclose }: Props = $props();

	/** `Do., 03.12.2026` from the ISO day. */
	const date = $derived(
		`${t(day.weekday)}., ${day.iso.slice(8, 10)}.${day.iso.slice(5, 7)}.${day.iso.slice(0, 4)}`
	);

	interface DispoRow {
		id: number | null;
		time: string;
		title: string;
		/** Code of the flag, `''` for none. */
		flag: string;
		/** Set aside by the planner: kept, but it asks for nobody. */
		ignored: boolean;
	}

	/** The flags of the master data, for the selector of every line. */
	let flagList = $state<PeopleFlag[]>([]);

	onMount(async () => {
		try {
			flagList = await api.people.flags.list();
		} catch {
			flagList = [];
		}
	});

	function initialRows(): DispoRow[] {
		const stored = entries.map((entry) => ({
			id: entry.id,
			time: entry.starts_at ?? '',
			title: entry.title,
			flag: entry.flag ?? '',
			ignored: entry.ignored
		}));
		return stored.length > 0
			? stored
			: [{ id: null, time: '', title: '', flag: '', ignored: false }];
	}

	let rows = $state<DispoRow[]>(initialRows());
	let saving = $state(false);
	let error = $state<string | null>(null);
	let changed = $state(false);

	function add() {
		rows = [...rows, { id: null, time: '', title: '', flag: '', ignored: false }];
	}

	function remove(index: number) {
		rows = rows.filter((_, position) => position !== index);
	}

	/** Rows the planner really filled in. */
	function filled(): DispoRow[] {
		return rows.filter((row) => row.title.trim() !== '');
	}

	async function save() {
		saving = true;
		error = null;
		try {
			await api.people.schedule.dispo.setDay(
				location.id,
				day.iso,
				filled().map((row) => ({
					id: row.id,
					starts_at: row.time.trim(),
					title: row.title.trim(),
					flag: row.flag === '' ? null : row.flag,
					ignored: row.ignored
				}))
			);
			changed = true;
			onclose(true);
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	function close() {
		onclose(changed);
	}


	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void close()));
</script>


<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window dispo-window"
		role="dialog"
		aria-modal="true"
		aria-label={t('Dispo {location} am {day}.', {
			location: location.name,
			day: `${t(day.weekday)} ${day.day}`
		})}
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title" title={location.name}>{t('Dispo —')} {location.name}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={close}
				>×</button
			>
		</div>

		<div class="sw-content">
			<div class="sw-row">
				<span class="sw-label">{t('Tag')}</span>
				<span class="value">{date}</span>
			</div>

			<hr class="sw-divider" />
			<span class="sw-heading">{t('Termine')}</span>

			{#each rows as row, index (index)}
				<div class="sw-row event">
					<span class="sw-label time">
						<input
							type="text"
							maxlength="5"
							placeholder={t('HH:MM')}
							title={t('Uhrzeit — 8, 830 und 8.30 werden zu 08:00, 08:30')}
							aria-label={t('Uhrzeit des {n}. Termins', { n: index + 1 })}
							bind:value={row.time}
							disabled={!mayEdit}
						/>
					</span>
					<input
						type="text"
						maxlength="200"
						placeholder={t('Was findet statt?')}
						aria-label={t('Text des {n}. Termins', { n: index + 1 })}
						bind:value={row.title}
						disabled={!mayEdit}
					/>
					<select
						class="flag"
						title={t('Flag des Termins')}
						aria-label={t('Flag des {n}. Termins', { n: index + 1 })}
						bind:value={row.flag}
						disabled={!mayEdit}
					>
						<option value="">{t('— ohne Flag —')}</option>
						{#each flagList as flag (flag.id)}
							<option value={flag.code}>{flag.code} · {flag.name}</option>
						{/each}
					</select>
					{#if mayEdit}
						<button
							type="button"
							class="sw-extra round"
							title={t('Termin entfernen')}
							aria-label={t('Termin {n} entfernen', { n: index + 1 })}
							onclick={() => remove(index)}>−</button
						>
					{/if}
				</div>
			{/each}

			{#if mayEdit}
				<div class="sw-row">
					<span class="sw-label"></span>
					<button type="button" class="sw-link" onclick={add}>{t('+ Termin hinzufügen')}</button>
				</div>
			{/if}

			{#if error}
				<p class="sw-error">{error}</p>
			{/if}
		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={close}>{t('Schließen')}</button>
			{#if mayEdit}
				<button
					class="sw-button sw-primary"
					type="button"
					disabled={saving}
					onclick={save}>{t('Speichern')}</button
				>
			{/if}
		</div>
	</div>
</div>

<style>
	.dispo-window {
		width: min(44rem, 100%);
	}
	.event .time {
		flex: 0 0 5rem;
	}
	.event .time input {
		width: 100%;
	}
	.event .flag {
		flex: 0 0 9rem;
		min-width: 0;
	}
</style>
