<script lang="ts">
	import { onMount } from 'svelte';
	import SearchWindow from '$core/search/SearchWindow.svelte';
	import { api, errorText } from '$core/api';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let warehouse = $state<number | null>(null);
	let warehouseList = $state<{ id: number; name: string | null; is_default: boolean }[]>([]);
	let packageSlipNo = $state('0');
	let orderNo = $state('0');
	let customer = $state('');
	let event = $state('');
	let packageSlipDate = $state('');
	let packDate = $state('');
	let deliveryDate = $state('');
	let articleNo = $state('0');
	let serialNumber = $state('');
	let includeArchive = $state(false);
	let note = $state(false);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			warehouseList = await api.masterData.warehouses();
			warehouse = warehouseList.find((w) => w.is_default)?.id ?? warehouseList[0]?.id ?? null;
		} catch (e) {
			error = errorText(e);
		}
	});
</script>

<SearchWindow title="Packschein suchen" width="24rem" bandLabel="Vorgang" bandText="Packscheinliste" {onclose}>
	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">Lager</span>
			<select bind:value={warehouse} aria-label="Lager">
				{#each warehouseList as w (w.id)}
					<option value={w.id}>{w.name ?? `Lager ${w.id}`}</option>
				{:else}
					<option value={null}>— kein Lager —</option>
				{/each}
			</select>
		</div>
	</div>

	<hr class="sw-divider" />

	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">Packschein Nr.</span>
			<input bind:value={packageSlipNo} inputmode="numeric" />
		</div>
		<div class="sw-row">
			<span class="sw-label">Auftrag Nr.</span>
			<input bind:value={orderNo} inputmode="numeric" />
		</div>
		<div class="sw-row">
			<span class="sw-label">Kunde</span>
			<input bind:value={customer} />
			<button class="sw-extra" type="button" title="Kunde wählen">+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">Veranstaltung</span>
			<input bind:value={event} />
		</div>
		<div class="sw-row">
			<span class="sw-label">Packscheindatum</span>
			<input bind:value={packageSlipDate} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">Packtermin</span>
			<input bind:value={packDate} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
		</div>
		<div class="sw-row">
			<span class="sw-label">Liefer-/Abholtermin</span>
			<input bind:value={deliveryDate} placeholder="00.00.0000" />
			<button class="sw-extra" type="button" title="Datum übernehmen">+</button>
		</div>
	</div>

	<p class="sw-heading">Suche in den Positionen</p>

	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">Artikel Nr.</span>
			<input bind:value={articleNo} inputmode="numeric" />
		</div>
		<div class="sw-row">
			<span class="sw-label">Seriennummer</span>
			<input bind:value={serialNumber} />
		</div>
	</div>

	<hr class="sw-divider" />

	<div class="sw-row indented">
		<label class="sw-check">
			<input type="checkbox" bind:checked={includeArchive} />
			Inklusive Archiv
		</label>
	</div>

	{#if error}
		<p class="sw-error">{error}</p>
	{/if}
	{#if note}
		<p class="sw-note">
			Die Packscheinsuche ist in WHERE noch nicht angebunden — der Packschein- und Auftragsstamm fehlt
			bisher. Die Maske bildet das Original ab.
		</p>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>Abbrechen</button>
		<button class="sw-button" type="button" onclick={() => (note = true)}>Sucheditor</button>
		<button class="sw-button sw-primary" type="button" onclick={() => (note = true)}>Suchen</button>
	{/snippet}
</SearchWindow>
