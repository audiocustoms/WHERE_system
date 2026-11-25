<script lang="ts">
	import SearchWindow from './SearchWindow.svelte';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	const MODI = [
		{ value: 'contains', text: 'Suchtext enthält…' },
		{ value: 'starts_with', text: 'Suchtext beginnt mit…' },
		{ value: 'equals', text: 'Suchtext ist genau…' }
	];

	const DATUM_MODI = [
		{ value: 'equals', text: 'Ist gleich' },
		{ value: 'before', text: 'ist kleiner als' },
		{ value: 'after', text: 'ist größer als' }
	];

	const TYPES = [
		{ value: 'none', text: 'Keine Typsuche' },
		{ value: 'contract', text: 'Vertrag' },
		{ value: 'offer', text: 'Angebot' },
		{ value: 'invoice', text: 'Rechnung' },
		{ value: 'other', text: 'Sonstige' }
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
	title="Suchen"
	width="26rem"
	bandLabel="Vorgang"
	bandText="Dokumentenliste"
	modeOptions={MODI}
	modeValue={modus}
	onmodechange={(value) => (modus = value)}
	{onclose}
>
	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">Titel</span>
			<input bind:value={title} />
		</div>
		<div class="sw-row">
			<span class="sw-label">Adresse</span>
			<input bind:value={address} />
			<button class="sw-extra" type="button" title="Adresse wählen">+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">Kunden Nr.</span>
			<input bind:value={customerNo} inputmode="numeric" />
		</div>
		<div class="sw-row">
			<span class="sw-label">Änderungsdatum</span>
			<select class="operator" bind:value={changeMode} aria-label="Änderungsdatum">
				{#each DATUM_MODI as m (m.value)}
					<option value={m.value}>{m.text}</option>
				{/each}
			</select>
			<input bind:value={changeDate} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">Angelegt Datum</span>
			<select class="operator" bind:value={createdMode} aria-label="Angelegt Datum">
				{#each DATUM_MODI as m (m.value)}
					<option value={m.value}>{m.text}</option>
				{/each}
			</select>
			<input bind:value={createdDate} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">Dokumententyp</span>
			<select bind:value={dokumententyp} aria-label="Dokumententyp">
				{#each TYPES as t (t.value)}
					<option value={t.value}>{t.text}</option>
				{/each}
			</select>
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
				Die Zusatzoptionen des Originals (Sachbearbeiter, Ablageort, Volltext …) folgen mit der
				Dokumentenablage.
			</p>
		{/if}
	</div>

	{#if note}
		<p class="sw-note">
			Die Dokumentensuche ist in WHERE noch nicht angebunden — die Dokumentenablage fehlt bisher. Die
			Maske bildet das Original ab.
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>Abbrechen</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>Suchen</button>
	{/snippet}
</SearchWindow>
