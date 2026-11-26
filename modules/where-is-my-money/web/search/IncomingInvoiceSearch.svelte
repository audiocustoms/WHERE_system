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

	const BASES = [
		{ value: 'invoice_date', text: t('Rechnungsdatum') },
		{ value: 'due_date', text: t('Fälligkeit') },
		{ value: 'entry_date', text: t('Erfassungsdatum') }
	];

	let tab = $state<'features' | 'period' | 'positions'>('features');
	let bookedStatus = $state('any');
	let paidStatus = $state('any');

	let documentNo = $state('0');
	let amount = $state('0,00');
	let invoiceNo = $state('');
	let supplier = $state('');
	let supplierNo = $state('0');
	let invoiceDate = $state('');
	let dueFrom = $state('');
	let dueTo = $state('');
	let invoiceInfo = $state('');

	let base = $state('invoice_date');
	let year = $state<number | null>(null);
	let quarter = $state<number | null>(null);
	let month = $state<number | null>(null);

	let position = $state('');
	let articleNo = $state('0');
	let extraInfo = $state('');

	let note = $state(false);
</script>

<SearchWindow title={t('Suchen')} width="24rem" bandLabel={t('Vorgang')} bandText={t('Rechnungseingangsbuch')} {onclose}>
	<nav class="sw-tabs" aria-label={t('Register')}>
		<button class:active={tab === 'features'} type="button" onclick={() => (tab = 'features')}>
			{t('Merkmale')}
		</button>
		<button class:active={tab === 'period'} type="button" onclick={() => (tab = 'period')}>
			{t('Zeitraum')}
		</button>
		<button class:active={tab === 'positions'} type="button" onclick={() => (tab = 'positions')}>
			{t('Positionen')}
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
				<span class="sw-label">{t('Beleg Nr.')}</span>
				<input bind:value={documentNo} inputmode="numeric" />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Betrag')}</span>
				<input bind:value={amount} inputmode="decimal" />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Rechnungsnr')}</span>
				<input bind:value={invoiceNo} />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Lieferant')}</span>
				<input bind:value={supplier} />
				<button class="sw-extra" type="button" title={t('Lieferant wählen')}>+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Lieferanten Nr.')}</span>
				<input bind:value={supplierNo} inputmode="numeric" />
				<button class="sw-extra" type="button" title={t('Lieferant wählen')}>+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Rechnungsdatum')}</span>
				<input bind:value={invoiceDate} placeholder="00.00.0000" />
				<button class="sw-extra" type="button" title={t('Datum übernehmen')}>+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Fälligkeit von')}</span>
				<input bind:value={dueFrom} placeholder="00.00.0000" />
			</div>
			<div class="sw-row">
				<span class="sw-label"></span>
				<span class="sw-small">{t('bis')}</span>
				<input bind:value={dueTo} placeholder="00.00.0000" />
				<button class="sw-extra" type="button" title={t('Datum übernehmen')}>+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Rechnungsinfo')}</span>
				<input bind:value={invoiceInfo} />
			</div>
		</div>
	{:else if tab === 'period'}
		<div class="sw-form">
			<PeriodFields bind:year bind:quarter bind:month bases={BASES} bind:base />
		</div>
	{:else}
		<div class="sw-form">
			<div class="sw-row">
				<span class="sw-label">{t('Position')}</span>
				<input bind:value={position} />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('ArtikelNr.')}</span>
				<input bind:value={articleNo} inputmode="numeric" />
			</div>
			<div class="sw-row">
				<span class="sw-label">{t('Zusatzinformationen')}</span>
				<input bind:value={extraInfo} />
			</div>
		</div>
	{/if}

	{#if note}
		<p class="sw-note">
			{t('Die Suche im Rechnungseingangsbuch ist in WHERE noch nicht angebunden — der Eingangsrechnungs- stamm fehlt bisher. Die Maske bildet das Original ab.')}
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>{t('Abbrechen')}</button>
		<button class="sw-button" type="button" onclick={() => (note = true)}>{t('Sucheditor')}</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>{t('Suchen')}</button>
	{/snippet}
</SearchWindow>
