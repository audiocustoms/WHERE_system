<script lang="ts">
	import SearchWindow from '$core/search/SearchWindow.svelte';
	import PeriodFields from '$core/search/PeriodFields.svelte';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	const STATUS = [
		{ value: 'any', text: 'egal' },
		{ value: 'yes', text: 'ja' },
		{ value: 'no', text: 'nein' }
	];

	const BASES = [
		{ value: 'invoice_date', text: 'Rechnungsdatum' },
		{ value: 'due_date', text: 'Fälligkeit' },
		{ value: 'entry_date', text: 'Erfassungsdatum' }
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

<SearchWindow title="Suchen" width="24rem" bandLabel="Vorgang" bandText="Rechnungseingangsbuch" {onclose}>
	<nav class="sw-tabs" aria-label="Register">
		<button class:active={tab === 'features'} type="button" onclick={() => (tab = 'features')}>
			Merkmale
		</button>
		<button class:active={tab === 'period'} type="button" onclick={() => (tab = 'period')}>
			Zeitraum
		</button>
		<button class:active={tab === 'positions'} type="button" onclick={() => (tab = 'positions')}>
			Positionen
		</button>
	</nav>

	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">Gebucht Status</span>
			<select bind:value={bookedStatus} aria-label="Gebucht Status">
				{#each STATUS as s (s.value)}
					<option value={s.value}>{s.text}</option>
				{/each}
			</select>
		</div>
		<div class="sw-row">
			<span class="sw-label">Bezahlt Status</span>
			<select bind:value={paidStatus} aria-label="Bezahlt Status">
				{#each STATUS as s (s.value)}
					<option value={s.value}>{s.text}</option>
				{/each}
			</select>
		</div>
	</div>

	{#if tab === 'features'}
		<div class="sw-form">
			<div class="sw-row">
				<span class="sw-label">Beleg Nr.</span>
				<input bind:value={documentNo} inputmode="numeric" />
			</div>
			<div class="sw-row">
				<span class="sw-label">Betrag</span>
				<input bind:value={amount} inputmode="decimal" />
			</div>
			<div class="sw-row">
				<span class="sw-label">Rechnungsnr</span>
				<input bind:value={invoiceNo} />
			</div>
			<div class="sw-row">
				<span class="sw-label">Lieferant</span>
				<input bind:value={supplier} />
				<button class="sw-extra" type="button" title="Lieferant wählen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">Lieferanten Nr.</span>
				<input bind:value={supplierNo} inputmode="numeric" />
				<button class="sw-extra" type="button" title="Lieferant wählen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">Rechnungsdatum</span>
				<input bind:value={invoiceDate} placeholder="00.00.0000" />
				<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">Fälligkeit von</span>
				<input bind:value={dueFrom} placeholder="00.00.0000" />
			</div>
			<div class="sw-row">
				<span class="sw-label"></span>
				<span class="sw-small">bis</span>
				<input bind:value={dueTo} placeholder="00.00.0000" />
				<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">Rechnungsinfo</span>
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
				<span class="sw-label">Position</span>
				<input bind:value={position} />
			</div>
			<div class="sw-row">
				<span class="sw-label">ArtikelNr.</span>
				<input bind:value={articleNo} inputmode="numeric" />
			</div>
			<div class="sw-row">
				<span class="sw-label">Zusatzinformationen</span>
				<input bind:value={extraInfo} />
			</div>
		</div>
	{/if}

	{#if note}
		<p class="sw-note">
			Die Suche im Rechnungseingangsbuch ist in WHERE noch nicht angebunden — der Eingangsrechnungs-
			stamm fehlt bisher. Die Maske bildet das Original ab.
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>Abbrechen</button>
		<button class="sw-button" type="button" onclick={() => (note = true)}>Sucheditor</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>Suchen</button>
	{/snippet}
</SearchWindow>
