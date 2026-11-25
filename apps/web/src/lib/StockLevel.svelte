<script lang="ts">
	import { onMount } from 'svelte';
	import { api, errorText, type Device, type DeviceInput, type StockRow } from './api';
	import GoodsReceipt from './GoodsReceipt.svelte';
	import InspectionReports from './articles/InspectionReports.svelte';
	import { modules } from './modules.svelte';

	interface Props {
		/** `null` for an article that is not saved yet. */
		articleId: number | null;
		articleNo: number;
		/** Flags of the article — pre-fill for the usage areas in the goods receipt. */
		articleUsage: { inventory: boolean; rental: boolean; sale: boolean };
	}

	let { articleId, articleNo, articleUsage }: Props = $props();

	/** Rental/sale exists only with `WHERE - does my stuff go?`. */
	const pricingEnabled = $derived(modules.isInstalled('go'));
	const stockColumns = $derived(pricingEnabled ? 8 : 6);

	let stock = $state<StockRow[]>([]);
	let devices = $state<Device[]>([]);
	let search = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let hint = $state<string | null>(null);
	let dialogOpen = $state(false);

	let selected = $state<Device | null>(null);
	let form = $state<DeviceInput | null>(null);
	let saving = $state(false);

	/** To the right of „Gerätedaten" stands „Prüfprotokolle". */
	let page = $state<'geraet' | 'protokolle'>('geraet');

	function quantity(value: string | null): string {
		if (value === null) return '—';
		const n = Number(value);
		if (!Number.isFinite(n)) return value;
		return new Intl.NumberFormat('de-DE', { maximumFractionDigits: 4 }).format(n);
	}

	function numberText(value: number | null): string {
		return value === null ? '—' : String(value);
	}

	function formFrom(g: Device): DeviceInput {
		return {
			serial_no: g.serial_no,
			inventory_no: g.inventory_no,
			inventory_no2: g.inventory_no2,
			stock_id: g.stock_id,
			binLocation: g.binLocation,
			condition: g.condition,
			inventory: g.inventory,
			rental: g.rental,
			sale: g.sale,
			used_sale: g.used_sale,
			discarded: g.discarded,
			purchasePrice: g.purchasePrice === null ? null : String(Number(g.purchasePrice)),
			purchaseDate: g.purchaseDate,
			supplier_no: g.supplier_no,
			owner: g.owner,
			warranty_months: g.warranty_months,
			ext_barcode: g.ext_barcode,
			yearBuilt: g.yearBuilt,
			note: g.note
		};
	}

	async function load() {
		if (articleId === null) return;
		loading = true;
		error = null;
		try {
			const stockResult = await api.article.stock(articleId, search || undefined);
			stock = stockResult.stock;
			devices = stockResult.devices;
			if (selected) {
				const refreshed = devices.find((g) => g.id === selected?.id) ?? null;
				selected = refreshed;
				form = refreshed ? formFrom(refreshed) : null;
			}
		} catch (e) {
			error = errorText(e, true);
		} finally {
			loading = false;
		}
	}

	function chooseDevice(g: Device) {
		selected = g;
		form = formFrom(g);
		error = null;
		hint = null;
		page = 'geraet';
	}

	function afterReceipt(geraete: Device[]) {
		const first = geraete[0];
		if (!first) return;
		hint =
			geraete.length > 1
				? `${geraete.length} Geräte im Wareneingang erfasst.`
				: `Gerät ${first.id} im Wareneingang erfasst.`;
		load().then(() => {
			const match = devices.find((x) => x.id === first.id);
			if (match) chooseDevice(match);
		});
	}

	async function saveDevice() {
		if (!articleId || !selected || !form) return;
		saving = true;
		error = null;
		hint = null;
		try {
			const updated = await api.article.updateDevice(articleId, selected.id, form);
			selected = updated;
			form = formFrom(updated);
			hint = 'Gerätedaten gespeichert.';
			await load();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			saving = false;
		}
	}

	async function remove() {
		if (!articleId || !selected) return;
		if (!confirm(`Gerät ${selected.id} (${selected.serial_no ?? 'ohne Seriennummer'}) endgültig entfernen?`))
			return;
		error = null;
		hint = null;
		try {
			await api.article.deleteDevice(articleId, selected.id);
			selected = null;
			form = null;
			hint = 'Gerät entfernt.';
			await load();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	onMount(load);
</script>

{#if articleId === null}
	<p class="notice">Der Lagerbestand lässt sich erfassen, sobald der Artikel gespeichert ist.</p>
{:else}
	<div class="toolbar">
		<button class="primary" type="button" onclick={() => (dialogOpen = true)}>
			Wareneingang erfassen
		</button>
		<div class="search">
			<input
				placeholder="Gerät suchen (Serien-Nr., Inventar-Nr., Lagerplatz, Zustand)"
				bind:value={search}
				onkeydown={(e) => e.key === 'Enter' && load()}
			/>
			<button type="button" onclick={load} disabled={loading}>{loading ? 'Lade …' : 'Suchen'}</button>
		</div>
	</div>

	<div class="columns">
		<div class="left">
			<table class="stock">
				<caption>Lager</caption>
				<thead>
					<tr>
						<th>Lager</th>
						<th class="number">Gesamt</th>
						{#if pricingEnabled}
							<th class="number">Miete</th>
							<th class="number">Verkauf</th>
						{/if}
						<th class="number">Inventar</th>
						<th class="number">Gebraucht</th>
						<th class="number">Reserviert</th>
						<th class="number">Verfügbar</th>
					</tr>
				</thead>
				<tbody>
					{#each stock as l (l.stock_id)}
						<tr>
							<td>
								{l.stock_name}{#if l.is_default}<span class="small">Standard</span>{/if}
							</td>
							<td class="number">{quantity(l.total)}</td>
							{#if pricingEnabled}
								<td class="number">{quantity(l.rental)}</td>
								<td class="number">{quantity(l.sale)}</td>
							{/if}
							<td class="number">{quantity(l.inventory)}</td>
							<td class="number">{quantity(l.used_sale)}</td>
							<td class="number">{quantity(l.reserved)}</td>
							<td class="number">{quantity(l.available)}</td>
						</tr>
					{:else}
						<tr><td colspan={stockColumns}>Keine Lager angelegt.</td></tr>
					{/each}
				</tbody>
			</table>

			<table class="devices">
				<caption>Geräte/Chargen ({devices.length})</caption>
				<thead>
					<tr>
						<th class="number">ID</th>
						<th>Serien-Nr.</th>
						<th>Inv-Nr.</th>
						<th class="number">Menge</th>
						<th>Einsatz</th>
						<th>Lager</th>
						<th>Lagerplatz</th>
						<th>Status</th>
					</tr>
				</thead>
				<tbody>
					{#each devices as g (g.id)}
						<tr
							class:selected={selected?.id === g.id}
							class:discarded={g.discarded}
							onclick={() => chooseDevice(g)}
							onkeydown={(e) => e.key === 'Enter' && chooseDevice(g)}
							tabindex="0"
						>
							<td class="number">{g.id}</td>
							<td>{g.serial_no ?? (g.without_serial ? '—' : '?')}</td>
							<td>{g.inventory_no ?? '—'}</td>
							<td class="number">{quantity(g.quantity)}</td>
							<td>{g.usage_text}</td>
							<td>{g.stock_name ?? '—'}</td>
							<td>{g.binLocation ?? '—'}</td>
							<td>{g.status}</td>
						</tr>
					{:else}
						<tr><td colspan="8">Noch keine Geräte erfasst — „Wareneingang erfassen".</td></tr>
					{/each}
				</tbody>
			</table>
		</div>

		<aside class="right">
			{#if form && selected}
				<nav class="device-tabs" aria-label="Gerät">
					<button
						type="button"
						class="tab-button"
						class:active={page === 'geraet'}
						onclick={() => (page = 'geraet')}>Gerätedaten</button
					>
					<button
						type="button"
						class="tab-button"
						class:active={page === 'protokolle'}
						onclick={() => (page = 'protokolle')}>Prüfprotokolle</button
					>
				</nav>

				{#if page === 'protokolle'}
					<h3>Prüfprotokolle <span class="id">ID {selected.id}</span></h3>
					<InspectionReports articleId={articleId} device={selected} />
				{:else}
					<h3>Gerätedaten <span class="id">ID {selected.id}</span></h3>
				<div class="fields">
					<label class="row"><span>Serien-Nr.</span><input bind:value={form.serial_no} /></label>
					<label class="row"><span>Inventar Nr.</span><input bind:value={form.inventory_no} /></label>
					<label class="row"><span>2. Inventar-Nr.</span><input bind:value={form.inventory_no2} /></label>
					<label class="row">
						<span>Lager</span>
						<select bind:value={form.stock_id}>
							{#each stock as l (l.stock_id)}
								<option value={l.stock_id}>{l.stock_name}</option>
							{/each}
						</select>
					</label>
					<label class="row"><span>Lagerplatz</span><input bind:value={form.binLocation} /></label>
					<label class="row"><span>Zustand</span><input bind:value={form.condition} /></label>
					<label class="row">
						<span>Kaufpreis</span>
						<input bind:value={form.purchasePrice} inputmode="decimal" placeholder="0,00" />
					</label>
					<label class="row">
						<span>Kaufdatum</span>
						<input type="date" bind:value={form.purchaseDate} />
					</label>
					<label class="row">
						<span>Lieferant</span>
						<input type="number" bind:value={form.supplier_no} />
					</label>
					<label class="row"><span>Eigentümer</span><input bind:value={form.owner} /></label>
					<label class="row">
						<span>Garantiezeit</span>
						<input type="number" bind:value={form.warranty_months} />
					</label>
					<label class="row"><span>ext. Barcode</span><input bind:value={form.ext_barcode} /></label>
					<label class="row">
						<span>Baujahr</span>
						<input type="number" bind:value={form.yearBuilt} />
					</label>
				</div>

				<fieldset>
					<legend>Einsatzbereiche — mehrere möglich</legend>
					<label class="check"><input type="checkbox" bind:checked={form.inventory} /> Inventar</label>
					{#if pricingEnabled}
						<label class="check"><input type="checkbox" bind:checked={form.rental} /> Miete</label>
						<label class="check"><input type="checkbox" bind:checked={form.sale} /> Verkauf</label>
						<label class="check">
							<input type="checkbox" bind:checked={form.used_sale} /> Gebrauchtverkauf
						</label>
					{/if}
				</fieldset>

				<label class="check off">
					<input type="checkbox" bind:checked={form.discarded} /> ausgesondert
				</label>

				<label class="block">
					<span>Bemerkungen</span>
					<textarea rows="3" bind:value={form.note}></textarea>
				</label>

				<div class="actions">
					<button type="button" class="danger" onclick={remove}>entfernen</button>
					<button class="primary" type="button" onclick={saveDevice} disabled={saving}>
						{saving ? 'Sichern …' : 'Sichern'}
					</button>
				</div>
				<p class="meta">
					Erfasst: {selected.recorded_at ?? '—'} · Menge {quantity(selected.quantity)}
					{#if selected.without_serial}· ohne Seriennummer{/if}
				</p>
				{/if}
			{:else}
				<p class="empty">
					Gerät in der Liste wählen, um Lagerplatz, Zustand, Kaufdaten und Einsatzbereich zu
					bearbeiten. Neue Geräte kommen über den Wareneingang.
				</p>
			{/if}
		</aside>
	</div>

	{#if error}
		<p class="bad">{error}</p>
	{/if}
	{#if hint}
		<p class="good">{hint}</p>
	{/if}

	{#if dialogOpen}
		<GoodsReceipt
			{articleId}
			{articleNo}
			{stock}
			{devices}
			defaultUsage={{ ...articleUsage, used_sale: false }}
			onfertig={afterReceipt}
			onschliessen={() => (dialogOpen = false)}
		/>
	{/if}
{/if}

<style>
	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
		flex-wrap: wrap;
	}
	.search {
		display: flex;
		gap: 0.4rem;
		flex: 1;
		min-width: 18rem;
	}
	.search input {
		flex: 1;
	}
	.notice,
	.empty {
		color: var(--muted);
		font-size: 0.85rem;
	}
	.bad {
		color: var(--bad);
	}
	.good {
		color: var(--good);
	}
	.columns {
		display: grid;
		grid-template-columns: minmax(0, 1.7fr) minmax(0, 1fr);
		gap: 1rem;
		align-items: start;
	}
	@media (max-width: 1000px) {
		.columns {
			grid-template-columns: 1fr;
		}
	}
	table {
		width: 100%;
		border-collapse: collapse;
		background: var(--surface);
		margin-bottom: 1rem;
	}
	caption {
		text-align: left;
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--muted);
		padding: 0 0 0.25rem;
	}
	th,
	td {
		text-align: left;
		padding: 0.3rem 0.45rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.85rem;
		white-space: nowrap;
	}
	th.number,
	td.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	tr.selected td {
		background: var(--highlight);
	}
	tr.discarded td {
		color: var(--muted-2);
	}
	tr[tabindex] {
		cursor: pointer;
	}
	.small {
		margin-left: 0.35rem;
		font-size: 0.7rem;
		color: var(--muted-2);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	aside {
		background: var(--surface-2);
		border: 1px solid var(--surface-3);
		border-radius: 6px;
		padding: 0.75rem;
	}
	h3 {
		font-size: 0.9rem;
		margin: 0 0 0.6rem;
	}
	h3 .id {
		font-weight: 400;
		color: var(--muted-2);
		font-size: 0.78rem;
		margin-left: 0.35rem;
	}
	/* Tabs „Gerätedaten" / „Prüfprotokolle" above the content of the right column. */
	.device-tabs {
		display: flex;
		gap: 0.25rem;
		margin-bottom: 0.6rem;
		border-bottom: 1px solid var(--border);
	}
	.tab-button {
		padding: 0.25rem 0.6rem;
		border: 1px solid transparent;
		border-bottom: none;
		border-radius: 4px 4px 0 0;
		background: none;
		color: var(--muted);
		font: inherit;
		font-size: 0.8rem;
		cursor: pointer;
		margin-bottom: -1px;
	}
	.tab-button.active {
		background: var(--surface);
		border-color: var(--border);
		color: var(--text);
		font-weight: 600;
	}
	.row {
		display: grid;
		grid-template-columns: 7.5rem minmax(0, 1fr);
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
		margin-bottom: 0.3rem;
	}
	.block {
		display: block;
		font-size: 0.8rem;
		margin-top: 0.5rem;
	}
	.block span {
		display: block;
		margin-bottom: 0.2rem;
	}
	input,
	select,
	textarea {
		padding: 0.28rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font: inherit;
		width: 100%;
		box-sizing: border-box;
	}
	fieldset {
		border: 1px solid var(--surface-3);
		border-radius: 6px;
		margin: 0.4rem 0;
		padding: 0.4rem 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	legend {
		font-size: 0.72rem;
		color: var(--muted);
		padding: 0 0.25rem;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.8rem;
	}
	.check input {
		width: auto;
	}
	.off {
		margin-top: 0.3rem;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
		margin-top: 0.6rem;
	}
	.meta {
		font-size: 0.72rem;
		color: var(--muted-2);
		margin: 0.5rem 0 0;
	}
	button {
		padding: 0.35rem 0.9rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
		font-size: 0.85rem;
	}
	button.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--surface);
	}
	button.danger {
		color: var(--bad);
	}
	button:disabled {
		cursor: not-allowed;
		opacity: 0.6;
	}
</style>
