<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import SearchWindow from './SearchWindow.svelte';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	const MODI = [
		{ value: 'contains', text: t('Suchtext enthält…') },
		{ value: 'starts_with', text: t('Suchtext beginnt mit…') },
		{ value: 'equals', text: t('Suchtext ist genau…') }
	];

	const DATUM_MODI = [
		{ value: 'equals', text: t('Ist gleich') },
		{ value: 'before', text: t('ist kleiner als') },
		{ value: 'after', text: t('ist größer als') }
	];

	const TYPES = [
		{ value: 'none', text: t('Keine Typsuche') },
		{ value: 'contract', text: t('Vertrag') },
		{ value: 'offer', text: t('Angebot') },
		{ value: 'invoice', text: t('Rechnung') },
		{ value: 'other', text: t('Sonstige') }
	];

	let modus = $state('contains');
	let title = $state('');
	let address = $state('');
	let customerNo = $state('0');
	let changeMode = $state('equals');
	let changeDate = $state('');
	let createdMode = $state('equals');
	let createdDate = $state('');
	let dokumententyp = $state('none');
	let more = $state(false);
	let note = $state(false);
</script>

<SearchWindow
	title={t('Suchen')}
	width="26rem"
	bandLabel={t('Vorgang')}
	bandText={t('Dokumentenliste')}
	modeOptions={MODI}
	modeValue={modus}
	onmodechange={(value) => (modus = value)}
	{onclose}
>
	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">{t('Titel')}</span>
			<input bind:value={title} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Adresse')}</span>
			<input bind:value={address} />
			<button class="sw-extra" type="button" title={t('Adresse wählen')}>+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Kunden Nr.')}</span>
			<input bind:value={customerNo} inputmode="numeric" />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Änderungsdatum')}</span>
			<select class="operator" bind:value={changeMode} aria-label={t('Änderungsdatum')}>
				{#each DATUM_MODI as m (m.value)}
					<option value={m.value}>{m.text}</option>
				{/each}
			</select>
			<input bind:value={changeDate} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title={t('Datum übernehmen')}>+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Angelegt Datum')}</span>
			<select class="operator" bind:value={createdMode} aria-label={t('Angelegt Datum')}>
				{#each DATUM_MODI as m (m.value)}
					<option value={m.value}>{m.text}</option>
				{/each}
			</select>
			<input bind:value={createdDate} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title={t('Datum übernehmen')}>+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Dokumententyp')}</span>
			<select bind:value={dokumententyp} aria-label={t('Dokumententyp')}>
				{#each TYPES as t (t.value)}
					<option value={t.value}>{t.text}</option>
				{/each}
			</select>
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
				{t('Die Zusatzoptionen des Originals (Sachbearbeiter, Ablageort, Volltext …) folgen mit der Dokumentenablage.')}
			</p>
		{/if}
	</div>

	{#if note}
		<p class="sw-note">
			{t('Die Dokumentensuche ist in WHERE noch nicht angebunden — die Dokumentenablage fehlt bisher. Die Maske bildet das Original ab.')}
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>{t('Abbrechen')}</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>{t('Suchen')}</button>
	{/snippet}
</SearchWindow>
