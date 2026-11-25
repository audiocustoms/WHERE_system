<script lang="ts">
	import { escapeWindows } from './escape.svelte';
	import { api, errorText, type Device, type StockRow } from './api';
	import { askConfirm } from './confirm.svelte';
	import { quantity } from './format';
	import { modules } from './modules.svelte';

	/** Rental/sale exists only with `WHERE - does my stuff go?`. */
	const pricingEnabled = $derived(modules.isInstalled('go'));

	interface Props {
		articleId: number;
		articleNo: number;
		stock: StockRow[];
		/** Already recorded devices — for „autom. hoch zählen" and the next device ID. */
		devices: Device[];
		/** Preset of the usage areas from the flags of the article. */
		defaultUsage: { inventory: boolean; rental: boolean; sale: boolean; used_sale: boolean };
		/** The devices this goods receipt has created (batch: one with an amount). */
		onfertig: (devices: Device[]) => void;
		onschliessen: () => void;
	}

	let {
		articleId,
		articleNo,
		stock,
		devices,
		defaultUsage,
		onfertig,
		onschliessen
	}: Props = $props();

	let serialNo = $state('');
	let withoutSerial = $state(false);
	let autoIncrement = $state(false);
	// svelte-ignore state_referenced_locally
	let stockId = $state<number>(
		(stock.find((l) => l.is_default) ?? stock[0])?.stock_id ?? 1
	);
	let binLocation = $state('');
	let condition = $state('');
	let purchasePrice = $state('');
	let purchaseDate = $state(new Date().toISOString().slice(0, 10));
	let supplier = $state<number | null>(null);
	let inventoryNo = $state('');
	let reason = $state('');
	// Pre-filled from the article flags, freely changeable afterwards.
	// svelte-ignore state_referenced_locally
	let usage = $state({
		inventory: defaultUsage.inventory,
		rental: defaultUsage.rental,
		sale: defaultUsage.sale,
		used_sale: defaultUsage.used_sale
	});
	let amount = $state('1');
	let saving = $state(false);
	let error = $state<string | null>(null);
	let hint = $state<string | null>(null);

	/** Already recorded devices — for „autom. hoch zählen" and the next device ID. */
	let extra = $state<Device[]>([]);

	/** Devices from the mask plus those already recorded in this dialog (without duplicates). */
	function allDevices(): Device[] {
		const map = new Map<number, Device>();
		for (const g of devices) map.set(g.id, g);
		for (const g of extra) map.set(g.id, g);
		return [...map.values()];
	}

	/** Next free serial number: highest numeric suffix + 1, suffix length stays. */
	function nextSerialNumber(): string {
		let best: { header: string; numberText: number; digits: number } | null = null;
		for (const g of allDevices()) {
			const hits = /^(.*?)(\d+)$/.exec(g.serial_no ?? '');
			if (!hits) continue;
			const numberText = Number(hits[2]);
			if (!best || numberText >= best.numberText) {
				best = { header: hits[1], numberText, digits: hits[2].length };
			}
		}
		if (!best) return '';
		return best.header + String(best.numberText + 1).padStart(best.digits, '0');
	}

	function applyProposal() {
		serialNo = nextSerialNumber();
	}

	/** Closing the mask only after a confirmation — otherwise recorded inputs are lost. */
	async function close() {
		const ok = await askConfirm({
			title: 'Wareneingang',
			message: 'Wareneingang schließen? Nicht erfasste Eingaben gehen verloren.',
			confirmLabel: 'Ja, schließen',
			cancelLabel: 'Weiter erfassen'
		});
		if (ok) onschliessen();
	}

	function nextDeviceId(): number {
		return allDevices().reduce((max, g) => Math.max(max, g.id), 0) + 1;
	}

	async function record() {
		saving = true;
		error = null;
		hint = null;
		try {
			const result = await api.article.goodsReceipt(articleId, {
				serial_no: withoutSerial ? null : serialNo.trim() || null,
				without_serial: withoutSerial,
				stock_id: stockId,
				binLocation: binLocation.trim() || null,
				condition: condition.trim() || null,
				purchasePrice: purchasePrice.trim() || null,
				purchaseDate: purchaseDate.trim() || null,
				supplier_no: supplier,
				inventory_no: inventoryNo.trim() || null,
				reason: reason.trim() || null,
				inventory: usage.inventory,
				rental: usage.rental,
				sale: usage.sale,
				used_sale: usage.used_sale,
			quantity: amount.trim() || null
			});
			extra = [...extra, ...result.devices];
			onfertig(result.devices);
			const naechste = nextSerialNumber();
			const first = result.devices[0];
			const last = result.devices.at(-1);
			if (result.devices.length > 1) {
				hint = `${result.devices.length} Geräte erfasst (${first?.serial_no} – ${last?.serial_no})${
					autoIncrement && naechste ? ` — als Nächstes: ${naechste}` : ''
				}`;
			} else if (first) {
				hint = `Gerät ${first.id} erfasst (Menge ${quantity(result.quantity)})${
					autoIncrement && naechste ? ` — als Nächstes: ${naechste}` : ''
				}`;
			}
			if (autoIncrement) {
				serialNo = naechste;
			} else {
				serialNo = '';
				inventoryNo = '';
			}
		} catch (e) {
			error = errorText(e, true);
		} finally {
			saving = false;
		}
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void close()));
</script>

<div class="backdrop" role="presentation">
	<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="we-titel">
		<header>
			<h2 id="we-titel">Wareneingang erfassen</h2>
			<span class="article">{articleNo}</span>
			<button class="close" type="button" onclick={close} title="Schließen">✕</button>
		</header>

		<div class="top-row">
			<label class="field">
				<span>Serien-Nr.</span>
				<input bind:value={serialNo} disabled={withoutSerial} placeholder={withoutSerial ? '—' : 'Seriennummer'} />
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={withoutSerial} /> ohne Seriennummernerfassung
			</label>
			<label class="check">
				<input type="checkbox" bind:checked={autoIncrement} disabled={withoutSerial} /> autom. hoch
				zählen
			</label>
			{#if autoIncrement && nextSerialNumber() && !serialNo}
				<button class="link" type="button" onclick={applyProposal}>
					Vorschlag übernehmen: {nextSerialNumber()}
				</button>
			{/if}
		</div>

		<h3>Erfassungsoptionen</h3>

		<p class="small" style="margin: 0 0 0.5rem">
			Menge mit Seriennummern: so viele Geräte werden angelegt, die Seriennummer zählt weiter
			(<code>…-001</code> → <code>…-002</code>). Mit „ohne Seriennummernerfassung" bleibt es ein
			Satz, dessen Menge den Zugang bildet.
		</p>

		<div class="grid">
			<div class="left">
				<label class="row">
					<span>Lager</span>
					<select bind:value={stockId}>
						{#each stock as l (l.stock_id)}
							<option value={l.stock_id}>{l.stock_name}</option>
						{/each}
					</select>
				</label>
				<label class="row"><span>Lagerplatz</span><input bind:value={binLocation} /></label>
				<label class="row"><span>Zustand</span><input bind:value={condition} /></label>
				<label class="row">
					<span>Kaufpreis</span>
					<input bind:value={purchasePrice} inputmode="decimal" placeholder="0,00" />
				</label>
				<label class="row"><span>Kaufdatum</span><input type="date" bind:value={purchaseDate} /></label>
				<label class="row">
					<span>Lieferant</span>
					<input type="number" bind:value={supplier} placeholder="Adress-Nr." />
				</label>
				<label class="row"><span>Inventar Nr.</span><input bind:value={inventoryNo} /></label>
				<label class="row">
					<span>Menge</span>
					<input
				bind:value={amount}
						inputmode="numeric"
						title="Mit Seriennummern: Anzahl der Geräte (die Nummern zählen weiter). Ohne Seriennummern: Menge der Charge."
					/>
				</label>
				<label class="row wide">
					<span>Grund der manuellen Erfassung</span>
					<input bind:value={reason} placeholder="z. B. Lieferschein Nr. …" />
				</label>
			</div>

			<div class="right">
				<div class="row">
					<span>nächste Geräte-ID</span>
					<strong>{nextDeviceId()}</strong>
					<span class="small">wird automatisch vergeben</span>
				</div>
				<fieldset>
					<legend>Einsatzbereiche — mehrere möglich</legend>
					<label class="check"><input type="checkbox" bind:checked={usage.inventory} /> Inventar</label>
					{#if pricingEnabled}
						<label class="check"><input type="checkbox" bind:checked={usage.rental} /> Miete</label>
						<label class="check"><input type="checkbox" bind:checked={usage.sale} /> Verkauf</label>
						<label class="check">
							<input type="checkbox" bind:checked={usage.used_sale} /> Gebrauchtverkauf
						</label>
					{/if}
				</fieldset>
			</div>
		</div>

		{#if error}
			<p class="bad">{error}</p>
		{/if}		{#if hint}
			<p class="good">{hint}</p>
		{/if}

		<footer>
			<button type="button" onclick={close}>Schließen</button>
			<button class="primary" type="button" onclick={record} disabled={saving}>
				{saving ? 'Erfasse …' : 'SN erfassen'}
			</button>
		</footer>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(27, 31, 36, 0.45);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding: 3rem 1rem;
		z-index: 50;
		overflow: auto;
	}
	.dialog {
		background: var(--surface);
		border-radius: 8px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.3);
		width: min(46rem, 100%);
		padding: 0 1rem 1rem;
	}
	header {
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
		border-bottom: 1px solid var(--border);
		padding: 0.75rem 0 0.5rem;
	}
	h2 {
		font-size: 1.05rem;
		margin: 0;
	}
	.article {
		color: var(--muted);
		font-size: 0.85rem;
	}
	.close {
		margin-left: auto;
		border: none;
		background: none;
		font-size: 1rem;
		cursor: pointer;
		color: var(--muted);
	}
	h3 {
		font-size: 0.85rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--muted-2);
		margin: 1rem 0 0.5rem;
	}
	.top-row {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: 0.75rem;
		padding: 0.75rem 0 0;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		font-size: 0.8rem;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.8rem;
		white-space: nowrap;
	}
	.check input {
		width: auto;
		flex: 0 0 auto;
	}
	.grid {
		display: grid;
		grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
		gap: 1.25rem;
	}
	@media (max-width: 720px) {
		.grid {
			grid-template-columns: 1fr;
		}
	}
	.row {
		display: grid;
		grid-template-columns: 11rem minmax(0, 1fr);
		align-items: center;
		gap: 0.5rem;
		font-size: 0.82rem;
		margin-bottom: 0.35rem;
	}
	.row.wide {
		grid-template-columns: 11rem minmax(0, 1fr);
	}
	.right .row {
		grid-template-columns: auto 1fr;
	}
	.small {
		color: var(--muted-2);
		font-size: 0.72rem;
	}
	input,
	select {
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font: inherit;
		width: 100%;
		box-sizing: border-box;
	}
	fieldset {
		border: 1px solid var(--surface-3);
		border-radius: 6px;
		padding: 0.5rem 0.75rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	legend {
		font-size: 0.75rem;
		color: var(--muted);
		padding: 0 0.25rem;
	}
	.bad {
		color: var(--bad);
	}
	.good {
		color: var(--good);
	}
	button {
		padding: 0.4rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
	button.link {
		border: none;
		background: none;
		color: var(--accent);
		padding: 0;
		font-size: 0.8rem;
	}
	button.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--surface);
	}
	button:disabled {
		cursor: not-allowed;
		opacity: 0.6;
	}
	footer {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		margin-top: 1rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--surface-3);
	}
</style>
