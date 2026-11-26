<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '$core/escape.svelte';
	import { api, errorText, type PeopleListEntry } from '$core/api';
	import '$core/search/search-window.css';
	import ScheduleLocationFlags from './ScheduleLocationFlags.svelte';

	/**
	 * „Einstellungen" of a location of the disposition: the properties that the
	 * plan generator follows, and the flags with the crew this house needs for them.
	 * A click on the name of a location row of the board opens it.
	 */
	interface Props {
		/** The location the window belongs to. */
		location: PeopleListEntry;
		/** Maintained positions, in their master-data order. */
		positions: string[];
		/** Only the administration may change the settings. */
		mayEdit: boolean;
		/** The settings were written; the board reloads them. */
		onsaved: () => void;
		/** The window was closed without saving. */
		onclose: () => void;
	}

	let { location, positions, mayEdit, onsaved, onclose }: Props = $props();

	/** `false` = the house is run without a foreman. */
	// svelte-ignore state_referenced_locally
	let needsForeman = $state(location.needs_foreman ?? true);
	let busy = $state(false);
	let error = $state<string | null>(null);

	async function save() {
		busy = true;
		error = null;
		try {
			await api.people.lists.setLocationForeman(location.id, needsForeman);
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = false;
		}
	}


	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>


<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label={t('Einstellungen {location}', { location: location.name })}
		style="width: min(30rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Einstellungen')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<div class="content">
			<p class="location">{location.name}</p>

			<label class="entry">
				<input
					type="checkbox"
					checked={needsForeman}
					disabled={!mayEdit || busy}
					onchange={(event) => (needsForeman = event.currentTarget.checked)}
				/>
				<span>{t('Meister benötigt')}</span>
			</label>

			<ScheduleLocationFlags {location} {positions} {mayEdit} />

			{#if error}<p class="error">{error}</p>{/if}
		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={onclose}>{t('Schließen')}</button>
			{#if mayEdit}
				<button class="sw-button sw-primary" type="button" disabled={busy} onclick={save}
					>{t('Speichern')}</button
				>
			{/if}
		</div>
	</div>
</div>

<style>
	.content {
		padding: 0.7rem 0.85rem 0.85rem;
		max-height: min(70vh, 34rem);
		overflow: auto;
	}
	.location {
		margin: 0 0 0.6rem;
		padding: 0.3rem 0.55rem;
		border-radius: 4px;
		background: color-mix(in srgb, var(--accent) 10%, transparent);
		color: var(--accent);
		font-size: 0.82rem;
		font-weight: 600;
	}
	.entry {
		display: flex;
		align-items: flex-start;
		gap: 0.45rem;
	}
	.entry input {
		margin-top: 0.15rem;
	}
	.entry span {
		font-size: 0.85rem;
	}
	.error {
		margin: 0.6rem 0 0;
		color: var(--bad);
		font-size: 0.82rem;
	}
</style>
