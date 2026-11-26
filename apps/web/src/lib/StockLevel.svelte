<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type Device, type DeviceInput, type Place, type StockMovement, type StockRow } from './api';
	import { auth } from './auth.svelte';
	import { escapeWindows } from './escape.svelte';
	import { decimalText, decimalValue } from './format';
	import GoodsReceipt from './GoodsReceipt.svelte';
	import InspectionReports from './articles/InspectionReports.svelte';
	import StockAction, { type StockActionMode } from './StockAction.svelte';
	import { modules } from './modules.svelte';

	interface Props {
		/** `null` for an article that is not saved yet. */
		articleId: number | null;
		articleNo: number;
		/** Flags of the article — pre-fill for the usage areas in the goods receipt. */
		articleUsage: { inventory: boolean; rental: boolean; sale: boolean };
		/** The switches of `Bestandsführung` — they decide whether stock may be booked. */
		articleSwitches?: { stock_tracking: boolean; no_own_stock: boolean; individual_items: boolean };
	}

	let {
		articleId,
		articleNo,
		articleUsage,
		articleSwitches = { stock_tracking: true, no_own_stock: false, individual_items: false }
	}: Props = $props();

	/** Stock is booked only for an article that keeps its own stock and controls it. */
	const mayReceive = $derived(articleSwitches.stock_tracking && !articleSwitches.no_own_stock);

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

	/** Stock history of the chosen device (`"where_stock_movement"`). */
	let verlauf = $state<StockMovement[]>([]);

	/** Places of the house — targets of „Ausgeben". */
	let places = $state<Place[]>([]);

	/** Bin locations of the warehouse of the chosen device — suggestions for the field. */
	let binNames = $state<string[]>([]);

	/** `Meldebestand` of the article, as typed. */
	let reorderLevel = $state('');

	/** Open window of an in-house movement; `null` = closed. */
	let booking = $state<StockActionMode | null>(null);

	/** Marks of the device grid — the rows a bulk transfer takes along. */
	let marked = $state<Set<number>>(new Set());
	let bulkOpen = $state(false);
	let bulkStock = $state<number | null>(null);
	let bulkNote = $state('');
	let bulkSaving = $state(false);

	/** Warehouses a booking may move material into (`where_zugriff`). */
	const targets = $derived(stock.filter((l) => l.zugriff));

	const markedDevices = $derived(devices.filter((g) => marked.has(g.id)));

	// `Esc` closes the bulk window like `Abbrechen`.
	$effect(() => {
		if (bulkOpen) return escapeWindows(() => (bulkOpen = false));
	});

	function toggleMark(id: number) {
		const next = new Set(marked);
		if (next.has(id)) {
			next.delete(id);
		} else {
			next.add(id);
		}
		marked = next;
	}

	/** Opens the bulk window; the first released warehouse that is not the current one wins. */
	function openBulk() {
		bulkStock = targets.find((l) => l.stock_id !== selected?.stock_id)?.stock_id ?? null;
		bulkNote = '';
		bulkOpen = true;
	}

	async function transferMarked() {
		if (articleId === null || bulkStock === null || markedDevices.length === 0) return;
		bulkSaving = true;
		error = null;
		try {
			const result = await api.article.transferDevices(articleId, {
				devices: markedDevices.map((g) => g.id),
				stock_id: bulkStock,
				note: bulkNote.trim() || null
			});
			bulkOpen = false;
			marked = new Set();
			hint =
				result.failed.length === 0
					? t('{count} Geräte umgelegt.', { count: result.moved })
					: t('{moved} umgelegt, {failed} nicht möglich', {
							moved: result.moved,
							failed: result.failed.length
						});
			await load();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			bulkSaving = false;
		}
	}

	/** Warehouses a booking may move material into (`where_zugriff`). */

	/** The warehouse of the chosen device allows a withdrawal (`where_entnahme`). */
	const withdrawable = $derived(
		selected === null
			? true
			: stock.find((l) => l.stock_id === selected!.stock_id)?.entnahme ?? true
	);

	const KIND_TEXT: Record<string, string> = {
		receipt: t('Wareneingang'),
		change: t('Änderung'),
		move: t('Umlagerung'),
		issue: t('Ausgabe'),
		return: t('Rücknahme'),
		count: t('Inventur'),
		consume: t('Verbrauch'),
		retire: t('ausgesondert'),
		reinstate: t('Aussonderung zurückgenommen'),
		delete: t('entfernt')
	};

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

	/** Loads the history of one device; empty without a chosen device. */
	async function loadMovements(id: number | null) {
		if (articleId === null || id === null) {
			verlauf = [];
			return;
		}
		try {
			verlauf = await api.article.movements(articleId, id, 30);
		} catch {
			// Without the history the tab keeps working — the list just stays empty.
			verlauf = [];
		}
	}

	// The history follows the chosen device.
	$effect(() => {
		const id = selected?.id ?? null;
		void loadMovements(id);
	});

	/** `A-01`, or `A-01 → B-02` when the value changed. */
	function vonNach(from: string | null, to: string | null): string | null {
		if (from && to) return from === to ? to : `${from} → ${to}`;
		return to ?? from;
	}

	/** What the line is about: warehouse, place, bin, condition, quantity. */
	function bewegungText(m: StockMovement): string {
		const teile: string[] = [];
		const lager = vonNach(m.warehouse_from_name, m.warehouse_to_name);
		if (lager) teile.push(lager);
		const ort = vonNach(m.place_from_name, m.place_to_name);
		if (ort) teile.push(ort);
		const platz = vonNach(m.bin_from, m.bin_to);
		if (platz) teile.push(`${t('Lagerplatz')}: ${platz}`);
		const zustand = vonNach(m.condition_from, m.condition_to);
		if (zustand) teile.push(`${t('Zustand')}: ${zustand}`);
		if (m.quantity) teile.push(`${t('Menge')} ${quantity(m.quantity)}`);
		return teile.join(' · ');
	}

	/** `2026-11-25T21:37:41` → `25.11.2026, 21:37`. */
	function zeitpunkt(at: string): string {
		const date = new Date(at);
		if (Number.isNaN(date.getTime())) return at;
		return date.toLocaleString('de-DE', {
			day: '2-digit',
			month: '2-digit',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function formFrom(g: Device): DeviceInput {
		return {
			serial_no: g.serial_no,
			inventory_no: g.inventory_no,
			inventory_no2: g.inventory_no2,
			stock_id: g.stock_id,
			bin_location: g.bin_location,
			condition: g.condition,
			inventory: g.inventory,
			rental: g.rental,
			sale: g.sale,
			used_sale: g.used_sale,
			discarded: g.discarded,
			purchase_price: decimalText(g.purchase_price),
			purchase_date: g.purchase_date,
			supplier_no: g.supplier_no,
			owner: g.owner,
			warranty_months: g.warranty_months,
			ext_barcode: g.ext_barcode,
			year_built: g.year_built,
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
			reorderLevel = decimalText(stockResult.reorder_level);
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

	/** Places of the house; without them the issue window stays closed. */
	async function loadPlaces() {
		try {
			places = await api.places.list();
		} catch {
			places = [];
		}
	}

	/** Bin locations the administration maintained for the warehouse of the row. */
	async function loadBinNames(warehouse: number | null) {
		if (warehouse === null) {
			binNames = [];
			return;
		}
		try {
			binNames = await api.masterData.binLocations(warehouse);
		} catch {
			binNames = [];
		}
	}

	// The suggestions follow the warehouse chosen in the device data.
	$effect(() => {
		void loadBinNames(form?.stock_id ?? null);
	});

	function afterBooking(text: string) {
		hint = text;
		booking = null;
		load().then(() => loadMovements(selected?.id ?? null));
	}

	/** Saves the `Meldebestand` — it drives the reorder suggestion. */
	async function saveReorderLevel() {
		if (articleId === null) return;
		error = null;
		try {
			await api.article.setReorderLevel(articleId, decimalValue(reorderLevel));
			hint = t('Meldebestand gespeichert.');
		} catch (e) {
			error = errorText(e, true);
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
				? t('{count} Geräte im Wareneingang erfasst.', { count: geraete.length })
				: t('Gerät {id} im Wareneingang erfasst.', { id: first.id });
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
			const updated = await api.article.updateDevice(articleId, selected.id, {
				...form,
				purchase_price: decimalValue(form.purchase_price)
			});
			selected = updated;
			form = formFrom(updated);
			hint = t('Gerätedaten gespeichert.');
			await load();
			await loadMovements(updated.id);
		} catch (e) {
			error = errorText(e, true);
		} finally {
			saving = false;
		}
	}

	async function remove() {
		if (!articleId || !selected) return;
		if (
			!confirm(
				t('Gerät {id} ({serial}) endgültig entfernen?', {
					id: selected.id,
					serial: selected.serial_no ?? t('ohne Seriennummer')
				})
			)
		)
			return;
		error = null;
		hint = null;
		try {
			await api.article.deleteDevice(articleId, selected.id);
			selected = null;
			form = null;
			hint = t('Gerät entfernt.');
			await load();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	onMount(() => {
		void load();
		void loadPlaces();
	});
</script>

{#if articleId === null}
	<p class="notice">{t('Der Lagerbestand lässt sich erfassen, sobald der Artikel gespeichert ist.')}</p>
{:else}
	<div class="toolbar">
		<button class="primary" type="button" onclick={() => (dialogOpen = true)} disabled={!mayReceive}>
			{t('Wareneingang erfassen')}
		</button>
		<label class="level">
			<span>{t('Meldebestand')}</span>
			<input
				bind:value={reorderLevel}
				inputmode="decimal"
				onblur={saveReorderLevel}
				onkeydown={(e) => e.key === 'Enter' && saveReorderLevel()}
			/>
		</label>
		<div class="search">
			<input
				placeholder={t('Gerät suchen (Serien-Nr., Inventar-Nr., Lagerplatz, Zustand)')}
				bind:value={search}
				onkeydown={(e) => e.key === 'Enter' && load()}
			/>
			<button type="button" onclick={load} disabled={loading}>{loading ? t('Lade …') : t('Suchen')}</button>
		</div>
		<button
			type="button"
			onclick={openBulk}
			disabled={marked.size === 0 || targets.length === 0}
		>
			{t('Umlegen ({count}) …', { count: marked.size })}
		</button>
	</div>

	<div class="columns">
		<div class="left">
			<table class="stock">
				<caption>{t('Lager')}</caption>
				<thead>
					<tr>
						<th>{t('Lager')}</th>
						<th class="number">{t('Gesamt')}</th>
						{#if pricingEnabled}
							<th class="number">{t('Miete')}</th>
							<th class="number">{t('Verkauf')}</th>
						{/if}
						<th class="number">{t('Inventar')}</th>
						<th class="number">{t('Gebraucht')}</th>
						<th class="number">{t('Ausgegeben')}</th>
						<th class="number">{t('Verfügbar')}</th>
					</tr>
				</thead>
				<tbody>
					{#each stock as l (l.stock_id)}
						<tr>
							<td>
								{l.stock_name}{#if l.is_default}<span class="small">{t('Standard')}</span>{/if}
							</td>
							<td class="number">{quantity(l.total)}</td>
							{#if pricingEnabled}
								<td class="number">{quantity(l.rental)}</td>
								<td class="number">{quantity(l.sale)}</td>
							{/if}
							<td class="number">{quantity(l.inventory)}</td>
							<td class="number">{quantity(l.used_sale)}</td>
							<td class="number">{quantity(l.withdrawal)}</td>
							<td class="number">{quantity(l.available)}</td>
						</tr>
					{:else}
						<tr><td colspan={stockColumns}>{t('Keine Lager angelegt.')}</td></tr>
					{/each}
				</tbody>
			</table>

			<table class="devices">
				<caption>{t('Geräte/Chargen ({count})', { count: devices.length })}</caption>
				<thead>
					<tr>
						<th class="mark"></th>
						<th class="number">{t('ID')}</th>
						<th>{t('Serien-Nr.')}</th>
						<th>{t('Inv-Nr.')}</th>
						<th class="number">{t('Menge')}</th>
						<th>{t('Einsatz')}</th>
						<th>{t('Lager')}</th>
						<th>{t('Lagerplatz')}</th>
						<th>{t('Status')}</th>
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
							<td class="mark">
								<input
									type="checkbox"
									checked={marked.has(g.id)}
									onclick={(e) => e.stopPropagation()}
									onchange={() => toggleMark(g.id)}
									aria-label={g.serial_no ?? `${t('ID')} ${g.id}`}
								/>
							</td>
							<td class="number">{g.id}</td>
							<td>{g.serial_no ?? (g.without_serial ? '—' : '?')}</td>
							<td>{g.inventory_no ?? '—'}</td>
							<td class="number">{quantity(g.quantity)}</td>
							<td>{g.usage_text}</td>
							<td>{g.stock_name ?? '—'}</td>
							<td>{g.bin_location ?? '—'}</td>
							<td>
								{g.status}{#if g.issued_until}<span class="small">{t('bis')} {g.issued_until}</span>{/if}
							</td>
						</tr>
					{:else}
						<tr><td colspan="9">{t('Noch keine Geräte erfasst — „Wareneingang erfassen".')}</td></tr>
					{/each}
				</tbody>
			</table>
		</div>

		<aside class="right">
			{#if form && selected}
				<nav class="device-tabs" aria-label={t('Gerät')}>
					<button
						type="button"
						class="tab-button"
						class:active={page === 'geraet'}
						onclick={() => (page = 'geraet')}>{t('Gerätedaten')}</button
					>
					<button
						type="button"
						class="tab-button"
						class:active={page === 'protokolle'}
						onclick={() => (page = 'protokolle')}>{t('Prüfprotokolle')}</button
					>
				</nav>

				{#if page === 'protokolle'}
					<h3>{t('Prüfprotokolle')} <span class="id">{t('ID')} {selected.id}</span></h3>
					<InspectionReports articleId={articleId} device={selected} />
				{:else}
					<h3>{t('Gerätedaten')} <span class="id">{t('ID')} {selected.id}</span></h3>
				{#if !selected.discarded || selected.place_id !== null}
					<div class="booking">
						<button
							type="button"
							onclick={() => (booking = 'issue')}
							disabled={selected.discarded || selected.place_id !== null || !withdrawable}
						>
							{t('Ausgeben …')}
						</button>
						<button
							type="button"
							onclick={() => (booking = 'transfer')}
							disabled={selected.discarded || selected.place_id !== null || !withdrawable || targets.length === 0}
						>
							{t('Umlegen …')}
						</button>
						{#if selected.place_id !== null}
							<button type="button" onclick={() => (booking = 'return')}>
								{t('Zurücknehmen …')}
							</button>
						{/if}
						{#if selected.without_serial}
							<button
								type="button"
								onclick={() => (booking = 'consume')}
								disabled={selected.discarded || selected.place_id !== null || !withdrawable}
							>
								{t('Verbrauchen …')}
							</button>
						{/if}
					</div>
				{/if}
				<div class="fields">
					<label class="row"><span>{t('Serien-Nr.')}</span><input bind:value={form.serial_no} /></label>
					<label class="row"><span>{t('Inventar Nr.')}</span><input bind:value={form.inventory_no} /></label>
					<label class="row"><span>{t('2. Inventar-Nr.')}</span><input bind:value={form.inventory_no2} /></label>
					<label class="row">
						<span>{t('Lager')}</span>
						<select bind:value={form.stock_id}>
							{#each stock as l (l.stock_id)}
								{#if l.zugriff || l.stock_id === form.stock_id}
									<option value={l.stock_id}>{l.stock_name}</option>
								{/if}
							{/each}
						</select>
					</label>
					<label class="row">
						<span>{t('Lagerplatz')}</span>
						<input bind:value={form.bin_location} list="lagerplaetze" />
					</label>
					<label class="row"><span>{t('Zustand')}</span><input bind:value={form.condition} /></label>
					<label class="row">
						<span>{t('Kaufpreis')}</span>
						<input bind:value={form.purchase_price} inputmode="decimal" placeholder="0,00" />
					</label>
					<label class="row">
						<span>{t('Kaufdatum')}</span>
						<input type="date" bind:value={form.purchase_date} />
					</label>
					<label class="row">
						<span>{t('Lieferant')}</span>
						<input type="number" bind:value={form.supplier_no} />
					</label>
					<label class="row"><span>{t('Eigentümer')}</span><input bind:value={form.owner} /></label>
					<label class="row">
						<span>{t('Garantiezeit')}</span>
						<input type="number" bind:value={form.warranty_months} />
					</label>
					<label class="row"><span>{t('ext. Barcode')}</span><input bind:value={form.ext_barcode} /></label>
					<label class="row">
						<span>{t('Baujahr')}</span>
						<input type="number" bind:value={form.year_built} />
					</label>
				</div>

				<fieldset>
					<legend>{t('Einsatzbereiche — mehrere möglich')}</legend>
					<label class="check"><input type="checkbox" bind:checked={form.inventory} /> {t('Inventar')}</label>
					{#if pricingEnabled}
						<label class="check"><input type="checkbox" bind:checked={form.rental} /> {t('Miete')}</label>
						<label class="check"><input type="checkbox" bind:checked={form.sale} /> {t('Verkauf')}</label>
						<label class="check">
							<input type="checkbox" bind:checked={form.used_sale} /> {t('Gebrauchtverkauf')}
						</label>
					{/if}
				</fieldset>

					<label class="check off">
						<input type="checkbox" bind:checked={form.discarded} /> {t('inaktiv (ausgesondert)')}
					</label>

				<label class="block">
					<span>{t('Bemerkungen')}</span>
					<textarea rows="3" bind:value={form.note}></textarea>
				</label>

				<div class="actions">
					{#if auth.isAdmin}
						<button type="button" class="danger" onclick={remove} title={t('Nur die Administration löscht ein Gerät endgültig')}>
							{t('entfernen')}
						</button>
					{/if}
					<button class="primary" type="button" onclick={saveDevice} disabled={saving}>
						{saving ? t('Sichern …') : t('Sichern')}
					</button>
				</div>
				<p class="meta">
					{t('Erfasst:')} {selected.recorded_at ?? '—'} {t('· Menge')} {quantity(selected.quantity)}
					{#if selected.without_serial}{t('· ohne Seriennummer')}{/if}
				</p>

				<h3 class="verlauf-head">{t('Verlauf')} <span class="id">{verlauf.length}</span></h3>
				<ul class="movements">
					{#each verlauf as m (m.id)}
						<li>
							<span class="kind">{KIND_TEXT[m.kind] ?? m.kind}</span>
							<span class="detail">{bewegungText(m)}</span>
							<span class="when">{zeitpunkt(m.at)} · {m.by}</span>
						</li>
					{:else}
						<li class="none">—</li>
					{/each}
				</ul>
				{/if}
			{:else}
				<p class="empty">—</p>
			{/if}
		</aside>
	</div>

	{#if error}
		<p class="bad">{error}</p>
	{/if}
	{#if hint}
		<p class="good">{hint}</p>
	{/if}

	<datalist id="lagerplaetze">
		{#each binNames as name (name)}
			<option value={name}></option>
		{/each}
	</datalist>

	{#if bulkOpen}
		<div class="backdrop" role="presentation">
			<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="bulk-titel">
				<header>
					<h2 id="bulk-titel">{t('Umlegen ({count}) …', { count: marked.size })}</h2>
					<button class="close" type="button" onclick={() => (bulkOpen = false)} title={t('Schließen')}>
						✕
					</button>
				</header>
				<label class="row">
					<span>{t('Lager')}</span>
					<select bind:value={bulkStock}>
						{#each targets as l (l.stock_id)}
							<option value={l.stock_id}>{l.stock_name}</option>
						{/each}
					</select>
				</label>
				<label class="row">
					<span>{t('Notiz')}</span>
					<input bind:value={bulkNote} />
				</label>
				<footer>
					<button type="button" onclick={() => (bulkOpen = false)}>{t('Abbrechen')}</button>
					<button
						class="primary"
						type="button"
						onclick={transferMarked}
						disabled={bulkSaving || bulkStock === null}
					>
						{t('Umlegen')}
					</button>
				</footer>
			</div>
		</div>
	{/if}

	{#if dialogOpen}
		<GoodsReceipt
			{articleId}
			{articleNo}
			{stock}
			{devices}
			defaultUsage={{ ...articleUsage, used_sale: false }}
			individualItems={articleSwitches.individual_items}
			onfertig={afterReceipt}
			onschliessen={() => (dialogOpen = false)}
		/>
	{/if}

	{#if booking && selected}
		<StockAction
			mode={booking}
			{articleId}
			device={selected}
			{stock}
			{places}
			onfertig={afterBooking}
			onschliessen={() => (booking = null)}
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
	.level {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.82rem;
		white-space: nowrap;
	}
	.level input {
		width: 6rem;
		text-align: right;
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
	/* In-house movements of the chosen row: issue, move, take back. */
	.booking {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
		margin-bottom: 0.6rem;
	}
	.booking button {
		font-size: 0.82rem;
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
	.verlauf-head {
		margin: 0.9rem 0 0.35rem;
		font-size: 0.8rem;
		font-weight: 600;
	}
	.movements {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--border);
	}
	.movements li {
		display: grid;
		grid-template-columns: 9.5rem minmax(0, 1fr);
		gap: 0.1rem 0.5rem;
		padding: 0.3rem 0;
		border-bottom: 1px solid var(--border);
		font-size: 0.75rem;
	}
	.movements .kind {
		font-weight: 600;
	}
	.movements .detail {
		color: var(--text);
		overflow-wrap: anywhere;
	}
	.movements .when {
		grid-column: 2;
		color: var(--muted-2);
		font-size: 0.7rem;
	}
	.movements .none {
		grid-template-columns: minmax(0, 1fr);
		color: var(--muted);
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
	/* Mark column of the device grid (`Sammel-Umlagerung`). */
	th.mark,
	td.mark {
		width: 1.5rem;
		padding-right: 0;
	}
	td.mark input {
		width: auto;
		margin: 0;
	}
	/* Window of the bulk transfer. */
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
		width: min(26rem, 100%);
		padding: 0 1rem 1rem;
	}
	.dialog header {
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
		border-bottom: 1px solid var(--border);
		padding: 0.75rem 0 0.5rem;
		margin-bottom: 0.6rem;
	}
	.dialog h2 {
		font-size: 1rem;
		margin: 0;
	}
	.dialog .close {
		margin-left: auto;
		border: none;
		background: none;
		font-size: 1rem;
		cursor: pointer;
		color: var(--muted);
		padding: 0;
	}
	.dialog footer {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		margin-top: 0.75rem;
	}
</style>
