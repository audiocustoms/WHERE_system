<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import SearchWindow from '$core/search/SearchWindow.svelte';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let orderNo = $state('');
	let customer = $state('');
	let event = $state('');
	let date = $state('');
	let dispoKind = $state('von');
	let dispoFrom = $state('');
	let dispoTo = $state('');
	let includeArchive = $state(false);
	let includeCancelled = $state(false);
	let more = $state(false);
	let position = $state('');
	let articleNo = $state('0');
	let note = $state(false);
</script>

<SearchWindow title={t('Angebot/Auftrag suchen')} width="24rem" bandLabel={t('Vorgang')} bandText={t('Auftragsliste')} {onclose}>
	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">{t('AuftragsNr.')}</span>
			<input bind:value={orderNo} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Kunde')}</span>
			<button class="sw-extra round" type="button" title={t('Auswahl aufheben')} onclick={() => (customer = '')}>
				⊘
			</button>
			<input bind:value={customer} />
			<button class="sw-extra" type="button" title={t('Kunde wählen')}>+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Veranstaltung')}</span>
			<input bind:value={event} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Datum')}</span>
			<input bind:value={date} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title={t('Datum übernehmen')}>+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Dispo Beginn')}</span>
			<select class="narrow" bind:value={dispoKind} aria-label={t('Dispo-Art')}>
				<option value="von">{t('Von')}</option>
				<option value="ab">{t('ab')}</option>
			</select>
			<input bind:value={dispoFrom} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title={t('Datum übernehmen')}>+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label"></span>
			<span class="sw-slot"></span>
			<span class="sw-small">{t('Bis')}</span>
			<input bind:value={dispoTo} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title={t('Datum übernehmen')}>+</button>
		</div>
		<div class="sw-row indented">
			<label class="sw-check">
				<input type="checkbox" bind:checked={includeArchive} />
				{t('Inklusive Archiv')}
			</label>
		</div>
		<div class="sw-row indented">
			<label class="sw-check">
				<input type="checkbox" bind:checked={includeCancelled} />
				{t('Inklusive Stornos')}
			</label>
		</div>
		<div class="sw-row">
			<button class="sw-extra round" type="button" title={t('Suchoptionen')} onclick={() => (more = !more)}>
				+
			</button>
			<button class="sw-link" type="button" onclick={() => (more = !more)}>
				{more ? t('weitere Suchoptionen ausblenden') : t('weitere Suchoptionen einblenden')}
			</button>
		</div>
		{#if more}
			<p class="sw-small">
				{t('Die Zusatzoptionen des Originals (Status, Sachbearbeiter, Zahlungsziel …) folgen mit dem Auftragsstamm.')}
			</p>
		{/if}
	</div>

	<p class="sw-heading">{t('Suche in den Positionen')}</p>

	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">{t('Position')}</span>
			<input bind:value={position} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('ArtikelNr.')}</span>
			<input bind:value={articleNo} inputmode="numeric" />
		</div>
	</div>

	{#if note}
		<p class="sw-note">
			{t('Die Auftragssuche ist in WHERE noch nicht angebunden — der Auftragsstamm fehlt bisher. Die Maske bildet das Original ab.')}
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>{t('Abbrechen')}</button>
		<button class="sw-button" type="button" onclick={() => (note = true)}>{t('Sucheditor')}</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>{t('Suchen')}</button>
	{/snippet}
</SearchWindow>
