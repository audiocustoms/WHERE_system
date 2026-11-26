<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type ArticleList,
		type Device,
		type PurchaseOrder,
		type PurchaseOrderDetail,
		type PurchaseOrderLine,
		type StockRow
	} from '../../lib/api';
	import { auth } from '../../lib/auth.svelte';
	import AccessDenied from '../../lib/AccessDenied.svelte';
	import { askConfirm } from '../../lib/confirm.svelte';
	import { escapeWindows } from '../../lib/escape.svelte';
	import { decimalText, decimalValue } from '../../lib/format';
	import GoodsReceipt from '../../lib/GoodsReceipt.svelte';
	import { RIGHT } from '../../lib/rights';

	let orders = $state<PurchaseOrder[]>([]);
	let detail = $state<PurchaseOrderDetail | null>(null);
	let stateFilter = $state<string | null>('open');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let status = $state<string | null>(null);

	/** New order: supplier, delivery date and the first line. */
	let creating = $state(false);
	let newSupplier = $state('');
	let newExpected = $state('');
	let newArticleNo = $state('');
	let newQuantity = $state('1');
	let found = $state<ArticleList | null>(null);
	let saving = $state(false);

	const mayWrite = $derived(auth.hasRight(RIGHT.mayEditArticle));

	const STATE_TEXT: Record<string, string> = {
		open: t('offen'),
		done: t('abgeschlossen'),
		cancelled: t('storniert')
	};

	async function load() {
		loading = true;
		error = null;
		try {
			orders = await api.purchaseOrders.list(stateFilter);
			if (detail) {
				if (orders.some((o) => o.id === detail!.order.id)) {
					detail = await api.purchaseOrders.get(detail.order.id);
				} else {
					detail = null;
				}
			}
		} catch (e) {
			error = errorText(e, true);
		} finally {
			loading = false;
		}
	}

	/** Editable header of the chosen order (the mask saves it as a whole). */
	let editSupplier = $state('');
	let editExpected = $state('');
	let editNote = $state('');

	function loadHeader() {
		editSupplier = detail?.order.supplier_no === null || detail === null
			? ''
			: String(detail.order.supplier_no);
		editExpected = detail?.order.expected_at ?? '';
		editNote = detail?.order.note ?? '';
	}

	async function saveHeader() {
		if (!detail) return;
		error = null;
		try {
			const raw = editSupplier.trim() ? Number.parseInt(editSupplier.trim(), 10) : null;
			await api.purchaseOrders.updateHeader(detail.order.id, {
				supplier_no: raw !== null && Number.isFinite(raw) ? raw : null,
				expected_at: editExpected.trim() || null,
				note: editNote.trim() || null
			});
			status = t('Bestellung geändert.');
			await load();
			await pick(detail.order.id);
			loadHeader();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function pick(id: number) {
		error = null;
		status = null;
		try {
			detail = await api.purchaseOrders.get(id);
			loadHeader();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function findArticle() {
		error = null;
		found = null;
		const no = newArticleNo.trim();
		if (!no) return;
		try {
			const page = await api.article.list({ q: no, active: 'active', limit: 10 });
			found =
				page.items.find((a) => String(a.article_no) === no) ?? page.items[0] ?? null;
		} catch (e) {
			error = errorText(e, true);
		}
	}

	function resetNew() {
		creating = false;
		newSupplier = '';
		newExpected = '';
		newArticleNo = '';
		newQuantity = '1';
		found = null;
	}

	async function create() {
		if (!found) return;
		saving = true;
		error = null;
		try {
			const raw = newSupplier.trim() ? Number.parseInt(newSupplier.trim(), 10) : null;
			const order = await api.purchaseOrders.create({
				supplier_no: raw !== null && Number.isFinite(raw) ? raw : null,
				ordered_at: null,
				expected_at: newExpected.trim() || null,
				note: null,
				lines: [
					{
						article_id: found.id,
						quantity: decimalValue(newQuantity) ?? '1',
						unit_price: null,
						note: null
					}
				]
			});
			status = t('Bestellung {number} angelegt.', { number: order.number });
			resetNew();
			await load();
			await pick(order.id);
		} catch (e) {
			error = errorText(e, true);
		} finally {
			saving = false;
		}
	}

	async function removeLine(lineId: number) {
		if (!detail) return;
		const ok = await askConfirm({
			title: t('Bestellung'),
			message: t('Position entfernen?'),
			confirmLabel: t('Ja, entfernen'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		try {
			await api.purchaseOrders.removeLine(detail.order.id, lineId);
			await load();
			await pick(detail.order.id);
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function closeOrder() {
		if (!detail) return;
		const ok = await askConfirm({
			title: t('Bestellung'),
			message: t('Bestellung {number} abschließen?', { number: detail.order.number }),
			confirmLabel: t('Ja, abschließen'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		try {
			await api.purchaseOrders.close(detail.order.id);
			status = t('Bestellung abgeschlossen.');
			await load();
			await pick(detail.order.id);
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function cancelOrder() {
		if (!detail) return;
		const ok = await askConfirm({
			title: t('Bestellung'),
			message: t('Bestellung {number} stornieren?', { number: detail.order.number }),
			confirmLabel: t('Ja, stornieren'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		try {
			await api.purchaseOrders.cancel(detail.order.id);
			status = t('Bestellung storniert.');
			await load();
			await pick(detail.order.id);
		} catch (e) {
			error = errorText(e, true);
		}
	}

	/** The goods receipt mask of an article, opened for one order line. */
	let receipt = $state<{
		lineId: number;
		articleId: number;
		articleNo: number;
		stock: StockRow[];
		devices: Device[];
		usage: { inventory: boolean; rental: boolean; sale: boolean };
	} | null>(null);

	/** Opens the goods receipt of the article — the same mask as in the article window. */
	async function openReceipt(line: PurchaseOrderLine) {
		error = null;
		status = null;
		try {
			const [level, article] = await Promise.all([
				api.article.stock(line.article_id),
				api.article.get(line.article_id)
			]);
			receipt = {
				lineId: line.id,
				articleId: line.article_id,
				articleNo: line.article_no ?? line.article_id,
				stock: level.stock,
				devices: level.devices,
				usage: {
					inventory: article.inventory,
					rental: article.rental,
					sale: article.sale
				}
			};
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function afterReceipt() {
		if (!detail) return;
		await load();
		await pick(detail.order.id);
	}

	function amount(value: string | null): string {
		return value === null ? '—' : decimalText(value);
	}

	onMount(load);

	// `Esc` closes the dialog like `Abbrechen`.
	$effect(() => {
		if (creating) return escapeWindows(resetNew);
	});
</script>

{#if !auth.hasRight(RIGHT.mayReadArticles)}
	<AccessDenied
		pageName={t('Bestellungen')}
		right={RIGHT.mayReadArticles}
		rightLabel={t('Artikel lesen')}
	/>
{:else}
	<h1>{t('Bestellungen')}</h1>

	<div class="toolbar">
		<label class="field">
			<span>{t('Status')}</span>
			<select bind:value={stateFilter} onchange={load}>
				<option value={null}>{t('Alle')}</option>
				<option value="open">{t('offen')}</option>
				<option value="done">{t('abgeschlossen')}</option>
				<option value="cancelled">{t('storniert')}</option>
			</select>
		</label>
		<span class="spacer"></span>
		<span class="small">{loading ? t('Lade …') : t('{n} Bestellungen', { n: orders.length })}</span>
		{#if mayWrite}
			<button class="primary" type="button" onclick={() => (creating = true)}>
				{t('Bestellung +')}
			</button>
		{/if}
	</div>

	<div class="columns">
		<div class="list">
			<table>
				<thead>
					<tr>
						<th class="number">{t('Nr.')}</th>
						<th>{t('Bestellt am')}</th>
						<th>{t('Lieferant')}</th>
						<th class="number">{t('Positionen')}</th>
						<th class="number">{t('Offen')}</th>
						<th>{t('Status')}</th>
					</tr>
				</thead>
				<tbody>
					{#each orders as o (o.id)}
						<tr
							class:selected={detail?.order.id === o.id}
							onclick={() => pick(o.id)}
							onkeydown={(e) => e.key === 'Enter' && pick(o.id)}
							tabindex="0"
						>
							<td class="number">{o.number}</td>
							<td>{o.ordered_at}</td>
							<td>{o.supplier_name ?? '—'}</td>
							<td class="number">{o.lines}</td>
							<td class="number">{amount(o.missing)}</td>
							<td>{STATE_TEXT[o.state] ?? o.state}</td>
						</tr>
					{:else}
						<tr><td colspan="6">—</td></tr>
					{/each}
				</tbody>
			</table>
		</div>

		<aside class="detail">
			{#if detail}
				<h2>
					{t('Bestellung')} {detail.order.number}
					<span class="state">{STATE_TEXT[detail.order.state] ?? detail.order.state}</span>
				</h2>
				<div class="fields">
					{#if mayWrite && detail.order.state === 'open'}
						<label class="row">
							<span>{t('Lieferant')}</span>
							<input type="number" bind:value={editSupplier} placeholder={t('Adress-Nr.')} />
						</label>
						<label class="row">
							<span>{t('Liefertermin')}</span>
							<input type="date" bind:value={editExpected} />
						</label>
						<label class="row">
							<span>{t('Notiz')}</span>
							<input bind:value={editNote} />
						</label>
						<div class="row">
							<span></span>
							<button type="button" onclick={saveHeader}>{t('Sichern')}</button>
						</div>
					{:else}
						<div class="row"><span>{t('Lieferant')}</span><strong>{detail.order.supplier_name ?? '—'}</strong></div>
						<div class="row"><span>{t('Liefertermin')}</span><strong>{detail.order.expected_at ?? '—'}</strong></div>
						{#if detail.order.note}
							<div class="row"><span>{t('Notiz')}</span><strong>{detail.order.note}</strong></div>
						{/if}
					{/if}
					<div class="row"><span>{t('Bestellt am')}</span><strong>{detail.order.ordered_at}</strong></div>
					<div class="row"><span>{t('Bestellt')}</span><strong>{amount(detail.order.ordered)}</strong></div>
					<div class="row"><span>{t('Geliefert')}</span><strong>{amount(detail.order.received)}</strong></div>
					<div class="row"><span>{t('Offen')}</span><strong>{amount(detail.order.missing)}</strong></div>
				</div>

				<table class="lines">
					<thead>
						<tr>
							<th class="number">{t('Pos.')}</th>
							<th class="number">{t('Art.-Nr.')}</th>
							<th>{t('Artikel')}</th>
							<th class="number">{t('Bestellt')}</th>
							<th class="number">{t('Geliefert')}</th>
							<th class="number">{t('Offen')}</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each detail.lines as l (l.id)}
							<tr>
								<td class="number">{l.line_no}</td>
								<td class="number">{l.article_no ?? '—'}</td>
								<td>{l.article_name ?? '—'}</td>
								<td class="number">{amount(l.quantity)}</td>
								<td class="number">{amount(l.received)}</td>
								<td class="number open">{amount(l.open)}</td>
								<td>
									{#if mayWrite && detail.order.state === 'open'}
										{#if decimalText(l.open) !== '0'}
											<button class="link" type="button" onclick={() => openReceipt(l)}>
												{t('Wareneingang …')}
											</button>
										{/if}
										{#if decimalText(l.received) === '0'}
											<button class="link" type="button" onclick={() => removeLine(l.id)}>
												{t('entfernen')}
											</button>
										{/if}
									{/if}
								</td>
							</tr>
						{:else}
							<tr><td colspan="7">—</td></tr>
						{/each}
					</tbody>
				</table>

				{#if mayWrite && detail.order.state === 'open'}
					<footer>
						<button type="button" onclick={cancelOrder}>{t('Stornieren')}</button>
						<button class="primary" type="button" onclick={closeOrder}>{t('Abschließen')}</button>
					</footer>
				{/if}
			{:else}
				<p class="small">—</p>
			{/if}
		</aside>
	</div>

	{#if creating}
		<div class="backdrop" role="presentation">
			<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="bo-titel">
				<header>
					<h2 id="bo-titel">{t('Bestellung +')}</h2>
					<button class="close" type="button" onclick={resetNew} title={t('Schließen')}>✕</button>
				</header>
				<div class="fields">
					<label class="row">
						<span>{t('Lieferant')}</span>
						<input type="number" bind:value={newSupplier} placeholder={t('Adress-Nr.')} />
					</label>
					<label class="row">
						<span>{t('Liefertermin')}</span>
						<input type="date" bind:value={newExpected} />
					</label>
					<label class="row">
						<span>{t('Art.-Nr.')}</span>
						<input bind:value={newArticleNo} onblur={findArticle} />
					</label>
					<div class="row">
						<span>{t('Artikel')}</span>
						<strong>{found ? `${found.article_no} ${found.name ?? ''}` : '—'}</strong>
					</div>
					<label class="row">
						<span>{t('Menge')}</span>
						<input bind:value={newQuantity} inputmode="decimal" />
					</label>
				</div>
				<footer>
					<button type="button" onclick={resetNew}>{t('Abbrechen')}</button>
					<button class="primary" type="button" onclick={create} disabled={saving || !found}>
						{t('Anlegen')}
					</button>
				</footer>
			</div>
		</div>
	{/if}

	{#if error}
		<p class="message bad">{error}</p>
	{/if}
	{#if status}
		<p class="message good">{status}</p>
	{/if}

	{#if receipt}
		<GoodsReceipt
			articleId={receipt.articleId}
			articleNo={receipt.articleNo}
			stock={receipt.stock}
			devices={receipt.devices}
			defaultUsage={{ ...receipt.usage, used_sale: false }}
			pickOrderLine={receipt.lineId}
			onfertig={afterReceipt}
			onschliessen={() => (receipt = null)}
		/>
	{/if}
{/if}

<style>
	.toolbar {
		display: flex;
		align-items: flex-end;
		gap: 0.75rem;
		flex-wrap: wrap;
		margin-bottom: 0.6rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.78rem;
	}
	.spacer {
		flex: 1;
	}
	.small {
		font-size: 0.78rem;
		color: var(--muted);
	}
	.columns {
		display: grid;
		grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
		gap: 0.9rem;
		align-items: start;
	}
	@media (max-width: 900px) {
		.columns {
			grid-template-columns: 1fr;
		}
	}
	.list {
		max-height: 70vh;
		overflow: auto;
		border: 1px solid var(--menu-band-border);
		border-radius: 4px;
		background: var(--surface);
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		text-align: left;
		padding: 0.28rem 0.45rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.8rem;
		white-space: nowrap;
	}
	th.number,
	td.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	td.open {
		font-weight: 600;
	}
	tbody tr {
		cursor: pointer;
	}
	tbody tr.selected {
		background: var(--surface-2);
	}
	.detail {
		border: 1px solid var(--menu-band-border);
		border-radius: 4px;
		background: var(--surface);
		padding: 0.6rem 0.75rem 0.75rem;
	}
	.detail h2 {
		font-size: 0.95rem;
		margin: 0 0 0.5rem;
		display: flex;
		gap: 0.5rem;
		align-items: baseline;
	}
	.state {
		font-size: 0.75rem;
		color: var(--muted);
		font-weight: 400;
	}
	.row {
		display: grid;
		grid-template-columns: 8rem minmax(0, 1fr);
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8rem;
		margin-bottom: 0.3rem;
	}
	.lines {
		margin-top: 0.6rem;
	}
	button {
		padding: 0.35rem 0.9rem;
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
		font-size: 0.78rem;
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
		margin-top: 0.75rem;
	}
	input,
	select {
		padding: 0.28rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font: inherit;
		width: 100%;
		box-sizing: border-box;
	}
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
		width: min(30rem, 100%);
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
	.close {
		margin-left: auto;
		border: none;
		background: none;
		font-size: 1rem;
		cursor: pointer;
		color: var(--muted);
	}
	.message {
		margin: 0.5rem 0 0;
		font-size: 0.82rem;
	}
	.message.bad {
		color: var(--bad);
	}
	.message.good {
		color: var(--good);
	}
</style>
