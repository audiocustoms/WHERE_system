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

<SearchWindow title="Rechnung suchen" width="24rem" bandLabel="Vorgang" bandText="Rechnungsausgangsbuch" {onclose}>
	<nav class="sw-tabs" aria-label="Register">
		<button class:active={tab === 'features'} type="button" onclick={() => (tab = 'features')}>
			Merkmale
		</button>
		<button class:active={tab === 'period'} type="button" onclick={() => (tab = 'period')}>
			Zeitraum
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
				<span class="sw-label">Rechnung Nr.</span>
				<input bind:value={invoiceNo} inputmode="numeric" />
			</div>
			<div class="sw-row">
				<span class="sw-label">Auftrag Nr.</span>
				<input bind:value={orderNo} />
			</div>
			<div class="sw-row">
				<span class="sw-label">AU Kunde</span>
				<button class="sw-extra round" type="button" title="Auswahl aufheben" onclick={() => (orderCustomer = '')}>
					⊘
				</button>
				<input bind:value={orderCustomer} />
				<button class="sw-extra" type="button" title="Kunde wählen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">RE Adresse</span>
				<input bind:value={invoiceAddress} />
				<button class="sw-extra" type="button" title="Adresse wählen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">Betrag</span>
				<input bind:value={amount} inputmode="decimal" />
			</div>
			<div class="sw-row">
				<span class="sw-label">Veranstaltung</span>
				<input bind:value={event} />
			</div>
			<div class="sw-row">
				<span class="sw-label">RE Datum</span>
				<input bind:value={invoiceDate} placeholder="00.00.0000" />
				<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label">Dispo Beginn</span>
				<select class="narrow" bind:value={dispoKind} aria-label="Dispo-Art">
					<option value="von">Von</option>
					<option value="ab">ab</option>
				</select>
				<input bind:value={dispoFrom} placeholder="00.00.0000" />
				<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
			</div>
			<div class="sw-row">
				<span class="sw-label"></span>
				<span class="sw-slot"></span>
				<span class="sw-small">Bis</span>
				<input bind:value={dispoTo} placeholder="00.00.0000" />
				<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
			</div>
			<div class="sw-row">
				<button class="sw-extra round" type="button" title="Suchoptionen" onclick={() => (more = !more)}>
					+
				</button>
				<button class="sw-link" type="button" onclick={() => (more = !more)}>
					{more ? 'weitere Suchoptionen ausblenden' : 'weitere Suchoptionen einblenden'}
				</button>
			</div>
			{#if more}
				<p class="sw-small">
					Die Zusatzoptionen des Originals (Zahlungsziel, Mahnstatus, Sachbearbeiter …) folgen mit dem
					Rechnungsstamm.
				</p>
			{/if}
		</div>

		<p class="sw-heading">Suche in den Positionen</p>

		<div class="sw-form">
			<div class="sw-row">
				<span class="sw-label">Position</span>
				<input bind:value={position} />
			</div>
			<div class="sw-row">
				<span class="sw-label">ArtikelNr.</span>
				<input bind:value={articleNo} inputmode="numeric" />
			</div>
		</div>
	{:else}
		<div class="sw-form">
			<PeriodFields bind:year bind:quarter bind:month />
			<div class="sw-row">
				<span class="sw-label">Bearbeiter</span>
				<select bind:value={processor} aria-label="Bearbeiter">
					<option value="alle">Alle Bearbeiter</option>
				</select>
			</div>
		</div>
	{/if}

	{#if note}
		<p class="sw-note">
			Die Rechnungssuche ist in WHERE noch nicht angebunden — der Rechnungsstamm fehlt bisher. Die
			Maske bildet das Original ab.
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>Abbrechen</button>
		<button class="sw-button" type="button" onclick={() => (note = true)}>Sucheditor</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>Suchen</button>
	{/snippet}
</SearchWindow>
