<script lang="ts">
	/**
	 * Personal settings of the signed-in user: the language of the interface.
	 *
	 * The choice belongs to the browser (like the design switch) and takes effect
	 * at once — the window stays open so the labels can be read in the new
	 * language straight away.
	 */
	import { escapeWindows } from '../escape.svelte';
	import { LANGUAGES, hasChosenLocale, locale, setLocale, t, type Locale } from '../i18n.svelte';
	import '../search/search-window.css';
	import { personalSettings } from './personalSettings.svelte';

	function close(): void {
		personalSettings.hide();
	}

	/** `Ändern` on a language: it is applied immediately, the window only reports it. */
	function choose(value: string): void {
		setLocale(value as Locale);
		// texts that live outside the components (menu data, month names) are
		// translated when the module loads, so the language takes a reload
		location.reload();
	}

	$effect(() => escapeWindows(() => void close()));
</script>

<div class="sw-backdrop" role="presentation">
	<div class="sw-window" role="dialog" aria-modal="true" aria-label={t('Persönliche Einstellungen')} style="width: min(34rem, 100%)">
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Persönliche Einstellungen')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={close}>×</button>
		</div>

		<div class="sw-content">
			<div class="sw-form">
				<label class="sw-row">
					<span class="sw-label">{t('Sprache der Oberfläche')}</span>
					<select
						class="sw-choice"
						value={hasChosenLocale() ? locale() : 'auto'}
						onchange={(event) => {
							const value = event.currentTarget.value;
							if (value !== 'auto') choose(value);
						}}
					>
						{#if !hasChosenLocale()}
							<option value="auto">{t('Automatisch (Systemsprache)')}</option>
						{/if}
						{#each LANGUAGES as language (language.value)}
							<option value={language.value}>{language.text}</option>
						{/each}
					</select>
				</label>
			</div>
		</div>

		<div class="sw-footer">
			<span class="spacer"></span>
			<button class="sw-button sw-primary" type="button" onclick={close}>{t('Schließen')}</button>
		</div>
	</div>
</div>
