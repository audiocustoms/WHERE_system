<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import SearchWindow from '$core/search/SearchWindow.svelte';
	import PeriodFields from '$core/search/PeriodFields.svelte';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	const STATUS = [
		{ value: 'any', text: t('egal') },
		{ value: 'yes', text: t('ja') },
		{ value: 'no', text: t('nein') }
	];

	let tab = $state<'features' | 'period'>('features');
	let bookedStatus = $state('any');
	let paidStatus = $state('any');

	let invoiceNo = $state('0');
	let orderNo = $state('');
	let orderCustomer = $state('');
	let invoiceAddress = $state('');
	let amount = $state('0,00');
	let event = $state('');
	let invoiceDate = $state('');
	let dispoKind = $state('von');
	let dispoFrom = $state('');
	let dispoTo = $state('');
	let more = $state(false);
	let position = $state('');
	let articleNo = $state('0');

	let year = $state<number | null>(null);
	let quarter = $state<number | null>(null);
	let month = $state<number | null>(null);
	let processor = $state('alle');

	let note = $state(false);
</script>

<SearchWindow title={t('Rechnung suchen')} width="24rem" bandLabel={t('Vorgang')} bandText={t('Rechnungsausgangsbuch')} {onclose}>
	<nav class="sw-tabs" aria-label={t('Register')}>
		<button class:active={tab === 'features'} type="button" onclick={() => (tab = 'features')}>
			{t('Merkmale')}
		</button>
		<button class:active={tab === 'period'} type="button" onclick={() => (tab = 'period')}>
			{t('Zeitraum')}
		</button>
	</nav>

	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">{t('Gebucht Status')}</span>
			<select bind:value={bookedStatus} aria-label={t('Gebucht Status')}>
				{#each STATUS as s (s.value)}
					<option value={s.value}>{s.text}</option>
				{/each}
			</select>
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Bezahlt Status')}</span>
			<select bind:value={paidStatus} aria-label={t('Bezahlt Status')}>
				{#each STATUS as s (s.value)}
					<option value={s.value}>{s.text}</option>
				{/each}
			</select>
		</div>
	</div>

	{#if tab === 'features'}
		<div class="sw-form">
			<div class="sw-row">
				<span class="sw-label">{t('Rechnung Nr.')}</span>
				<input bind:value={invoiceNo} inputmode="numeric" />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Auftrag Nr.')}</span>
				<input bind:value={orderNo} />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('AU Kunde')}</span>
				<button class="sw-extra round" type="button" title={t('Auswahl aufheben')} onclick={() => (orderCustomer = '')}>
					⊘
				</button>
				<input bind:value={orderCustomer} />
				<button class="sw-extra" type="button" title={t('Kunde wählen')}>+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('RE Adresse')}</span>
				<input bind:value={invoiceAddress} />
				<button class="sw-extra" type="button" title={t('Adresse wählen')}>+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Betrag')}</span>
				<input bind:value={amount} inputmode="decimal" />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Veranstaltung')}</span>
				<input bind:value={event} />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('RE Datum')}</span>
				<input bind:value={invoiceDate} placeholder="00.00.0000" />
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
					{t('Die Zusatzoptionen des Originals (Zahlungsziel, Mahnstatus, Sachbearbeiter …) folgen mit dem Rechnungsstamm.')}
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
	{:else}
		<div class="sw-form">
			<PeriodFields bind:year bind:quarter bind:month />
			<div class="sw-row">
				<span class="sw-label">{t('Bearbeiter')}</span>
				<select bind:value={processor} aria-label={t('Bearbeiter')}>
					<option value="alle">{t('Alle Bearbeiter')}</option>
				</select>
			</div>
		</div>
	{/if}

	{#if note}
		<p class="sw-note">
			{t('Die Rechnungssuche ist in WHERE noch nicht angebunden — der Rechnungsstamm fehlt bisher. Die Maske bildet das Original ab.')}
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>{t('Abbrechen')}</button>
		<button class="sw-button" type="button" onclick={() => (note = true)}>{t('Sucheditor')}</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>{t('Suchen')}</button>
	{/snippet}
</SearchWindow>
