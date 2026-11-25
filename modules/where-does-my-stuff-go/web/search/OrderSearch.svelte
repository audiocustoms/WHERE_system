<script lang="ts">
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

<SearchWindow title="Angebot/Auftrag suchen" width="24rem" bandLabel="Vorgang" bandText="Auftragsliste" {onclose}>
	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">AuftragsNr.</span>
			<input bind:value={orderNo} />
		</div>
		<div class="sw-row">
			<span class="sw-label">Kunde</span>
			<button class="sw-extra round" type="button" title="Auswahl aufheben" onclick={() => (customer = '')}>
				⊘
			</button>
			<input bind:value={customer} />
			<button class="sw-extra" type="button" title="Kunde wählen">+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">Veranstaltung</span>
			<input bind:value={event} />
		</div>
		<div class="sw-row">
			<span class="sw-label">Datum</span>
			<input bind:value={date} placeholder="00.00.0000" />
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
		<div class="sw-row indented">
			<label class="sw-check">
				<input type="checkbox" bind:checked={includeArchive} />
				Inklusive Archiv
			</label>
		</div>
		<div class="sw-row indented">
			<label class="sw-check">
				<input type="checkbox" bind:checked={includeCancelled} />
				Inklusive Stornos
			</label>
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
				Die Zusatzoptionen des Originals (Status, Sachbearbeiter, Zahlungsziel …) folgen mit dem
				Auftragsstamm.
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

	{#if note}
		<p class="sw-note">
			Die Auftragssuche ist in WHERE noch nicht angebunden — der Auftragsstamm fehlt bisher. Die Maske
			bildet das Original ab.
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>Abbrechen</button>
		<button class="sw-button" type="button" onclick={() => (note = true)}>Sucheditor</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>Suchen</button>
	{/snippet}
</SearchWindow>
