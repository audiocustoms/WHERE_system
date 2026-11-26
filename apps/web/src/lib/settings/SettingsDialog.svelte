<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '../escape.svelte';
	import { api, errorText, type SettingsArea, type SettingsRow, type SettingsValues } from '../api';
	import { askCancel, askConfirm } from '../confirm.svelte';
	import '../search/search-window.css';

	interface Props {
		/** The list the entry belongs to. */
		area: SettingsArea;
		/** `null` = new entry. */
		row: SettingsRow | null;
		onsaved: () => void;
		oncancel: () => void;
	}

	let { area, row, onsaved, oncancel }: Props = $props();

	// The mask starts with the values handed over; another entry replaces the
	// dialog completely, therefore these are deliberately not derived values.
	// svelte-ignore state_referenced_locally
	const start = row;

	let values = $state<SettingsValues>({ ...(start?.values ?? {}) });
	let saving = $state(false);
	let error = $state<string | null>(null);

	function name(): string {
		const column = area.fields[0]?.column;
		const value = column ? values[column] : null;
		return value && value.trim() ? value : area.title;
	}

	async function save() {
		saving = true;
		error = null;
		try {
			if (row) await api.settings.update(area.key, row.id, values);
			else await api.settings.create(area.key, values);
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	async function remove() {
		if (!row) return;
		const ok = await askConfirm({
			title: t('Eintrag löschen'),
			message: t('„{name}“ wird aus der Liste entfernt. Verweise darauf in Artikeln oder Adressen behalten ihre Nummer.', { name: name() }),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		saving = true;
		try {
			await api.settings.remove(area.key, row.id);
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** `Abbrechen` asks only when something was really changed. */
	async function cancel() {
		if (
			changed() &&
			!(await askCancel(t(row ? 'Das Bearbeiten des Eintrags' : 'Das Anlegen des Eintrags')))
		)
			return;
		oncancel();
	}

	function changed(): boolean {
		const previous: SettingsValues = start?.values ?? {};
		return area.fields.some(
			(field) => (values[field.column] ?? '') !== (previous[field.column] ?? '')
		);
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void cancel()));
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label={area.title}
		style="width: min(40rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{row ? t('{area}:', { area: area.title }) : t('Neuer Eintrag in {area}:', { area: area.title })}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={cancel}>×</button>
		</div>

		<div class="sw-content">
			{#if error}
				<p class="sw-error">{error}</p>
			{/if}
			<p class="sw-small">{area.description}</p>

			<div class="sw-form">
				{#each area.fields as field (field.column)}
					<label class="sw-row">
						<span class="sw-label">{field.label}</span>
						{#if field.kind === 'bool'}
							<span class="values">
								<label class="check">
									<input
										type="checkbox"
										checked={values[field.column] === 'true'}
										onchange={(e) =>
											(values[field.column] = e.currentTarget.checked ? 'true' : 'false')}
									/>
									{values[field.column] === 'true' ? t('Ja') : t('Nein')}
								</label>
							</span>
						{:else if field.kind === 'date'}
							<input
								type="date"
								value={values[field.column] ?? ''}
								oninput={(e) => (values[field.column] = e.currentTarget.value || null)}
							/>
						{:else if field.kind === 'integer' || field.kind === 'number'}
							<input
								type="number"
								step={field.kind === 'integer' ? '1' : 'any'}
								value={values[field.column] ?? ''}
								oninput={(e) => (values[field.column] = e.currentTarget.value || null)}
							/>
						{:else}
							<input
								value={values[field.column] ?? ''}
								oninput={(e) => (values[field.column] = e.currentTarget.value)}
							/>
						{/if}
					</label>
				{/each}
			</div>
		</div>

		<div class="sw-footer">
			{#if row}
				<button class="sw-button delete" type="button" onclick={remove} disabled={saving}>
					{t('Eintrag löschen')}
				</button>
			{/if}
			<span class="spacer"></span>
			<button class="sw-button" type="button" onclick={cancel} disabled={saving}>{t('Abbrechen')}</button>
			<button class="sw-button sw-primary" type="button" onclick={save} disabled={saving}>
				{saving ? t('Sichern …') : t('Sichern')}
			</button>
		</div>
	</div>
</div>

<style>
	.values {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.check {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.85rem;
		color: var(--muted);
	}
	.sw-form input {
		width: 100%;
		min-width: 0;
	}
	.delete {
		color: var(--bad);
	}
	.spacer {
		flex: 1;
	}
</style>
