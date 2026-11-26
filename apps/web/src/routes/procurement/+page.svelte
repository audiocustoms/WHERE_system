<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type PurchasingSuggestion } from '../../lib/api';
	import { auth } from '../../lib/auth.svelte';
	import AccessDenied from '../../lib/AccessDenied.svelte';
	import { decimalText } from '../../lib/format';
	import { RIGHT } from '../../lib/rights';

	let rows = $state<PurchasingSuggestion[]>([]);
	let days = $state(90);
	let warehouse = $state<number | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let status = $state<string | null>(null);
	/** Selected lines: `article_id-warehouse_id`. */
	let picked = $state<Set<string>>(new Set());
	let ordering = $state(false);

	const WINDOWS = [30, 90, 365];

	const mayWrite = $derived(auth.hasRight(RIGHT.mayEditArticle));

	const shown = $derived(
		warehouse === null ? rows : rows.filter((r) => r.warehouse_id === warehouse)
	);
	const warehouses = $derived(
		[...new Map(rows.map((r) => [r.warehouse_id, r.warehouse_name ?? ''])).entries()].map(
			([id, name]) => ({ id, name })
		)
	);

	function key(row: PurchasingSuggestion): string {
		return `${row.article_id}-${row.warehouse_id}`;
	}

	function toggle(row: PurchasingSuggestion) {
		const k = key(row);
		const next = new Set(picked);
		if (next.has(k)) {
			next.delete(k);
		} else {
			next.add(k);
		}
		picked = next;
	}

	function toggleAll() {
		const all = shown.every((row) => picked.has(key(row)));
		picked = all ? new Set() : new Set(shown.map(key));
	}

	/** One order per supplier — mixing suppliers in one order would be wrong. */
	async function orderPicked() {
		const chosen = rows.filter((row) => picked.has(key(row)));
		if (chosen.length === 0) return;
		ordering = true;
		error = null;
		status = null;
		try {
			const groups = new Map<number | null, PurchasingSuggestion[]>();
			for (const row of chosen) {
				const list = groups.get(row.supplier_no) ?? [];
				list.push(row);
				groups.set(row.supplier_no, list);
			}
			const numbers: string[] = [];
			for (const [supplier, list] of groups) {
				const order = await api.purchaseOrders.create({
					supplier_no: supplier,
					ordered_at: null,
					expected_at: null,
					note: null,
					lines: list.map((row) => ({
						article_id: row.article_id,
						quantity: row.missing,
						unit_price: null,
						note: null
					}))
				});
				numbers.push(order.number);
			}
			status = t('Bestellung angelegt: {numbers}', { numbers: numbers.join(', ') });
			picked = new Set();
			await load();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			ordering = false;
		}
	}

	function amount(value: string): string {
		return decimalText(value);
	}

	async function load() {
		loading = true;
		error = null;
		try {
			rows = await api.procurement.suggestions(days);
		} catch (e) {
			error = errorText(e, true);
		} finally {
			loading = false;
		}
	}

	onMount(load);
</script>

{#if !auth.hasRight(RIGHT.mayReadArticles)}
	<AccessDenied
		pageName={t('Beschaffung')}
		right={RIGHT.mayReadArticles}
		rightLabel={t('Artikel lesen')}
	/>
{:else}
	<h1>{t('Beschaffung')}</h1>

	<div class="toolbar">
		<label class="field">
			<span>{t('Lager')}</span>
			<select bind:value={warehouse}>
				<option value={null}>{t('Alle Lager')}</option>
				{#each warehouses as w (w.id)}
					<option value={w.id}>{w.name}</option>
				{/each}
			</select>
		</label>
		<label class="field">
			<span>{t('Verbrauch')}</span>
			<select bind:value={days} onchange={load}>
				{#each WINDOWS as d (d)}
					<option value={d}>{t('{days} Tage', { days: d })}</option>
				{/each}
			</select>
		</label>
		<span class="spacer"></span>
		<span class="small">{loading ? t('Lade …') : t('{n} Positionen', { n: shown.length })}</span>
		{#if mayWrite}
			<button class="primary" type="button" onclick={orderPicked} disabled={ordering || picked.size === 0}>
				{t('Bestellen …')}
			</button>
		{/if}
	</div>

	<div class="scroll-area">
		<table>
			<thead>
				<tr>
					<th class="check">
						<input
							type="checkbox"
							checked={shown.length > 0 && shown.every((row) => picked.has(key(row)))}
							onchange={toggleAll}
							aria-label={t('Alle')}
						/>
					</th>
					<th class="number">{t('Art.-Nr.')}</th>
					<th>{t('Artikel')}</th>
					<th>{t('Lager')}</th>
					<th class="number">{t('Verfügbar')}</th>
					<th class="number">{t('Meldebestand')}</th>
					<th class="number">{t('Fehlmenge')}</th>
					<th class="number">{t('Verbrauch')}</th>
					<th class="number">{t('Lieferzeit')}</th>
					<th>{t('Lieferant')}</th>
					<th>{t('Lieferanten-Art.-Nr.')}</th>
				</tr>
			</thead>
			<tbody>
				{#each shown as r (`${r.article_id}-${r.warehouse_id}`)}
					<tr class:picked={picked.has(key(r))}>
						<td class="check">
							{#if mayWrite}
								<input
									type="checkbox"
									checked={picked.has(key(r))}
									onchange={() => toggle(r)}
									aria-label={r.article_name ?? ''}
								/>
							{/if}
						</td>
						<td class="number">{r.article_no ?? '—'}</td>
						<td>{r.article_name ?? '—'}</td>
						<td>{r.warehouse_name ?? '—'}</td>
						<td class="number">{amount(r.available)}</td>
						<td class="number">{amount(r.reorder_level)}</td>
						<td class="number missing">{amount(r.missing)}</td>
						<td class="number">{amount(r.used)}</td>
						<td class="number">
							{r.lead_time_days === null ? '—' : t('{days} Tage', { days: r.lead_time_days })}
						</td>
						<td>{r.supplier_name ?? '—'}</td>
						<td>{r.supplier_article_no ?? '—'}</td>
					</tr>
				{:else}
					<tr><td colspan="11">—</td></tr>
				{/each}
			</tbody>
		</table>
	</div>

	{#if error}
		<p class="message bad">{error}</p>
	{/if}
	{#if status}
		<p class="message good">{status}</p>
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
	.scroll-area {
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
		padding: 0.3rem 0.45rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.82rem;
		white-space: nowrap;
	}
	th.number,
	td.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	td.missing {
		font-weight: 600;
	}
	th.check,
	td.check {
		width: 1.6rem;
		padding-right: 0;
	}
	th.check input,
	td.check input {
		width: auto;
		margin: 0;
	}
	tr.picked {
		background: var(--surface-2);
	}
	.small {
		font-size: 0.78rem;
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
