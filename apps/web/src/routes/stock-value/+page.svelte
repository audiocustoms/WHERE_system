<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type StockValueLine } from '../../lib/api';
	import { auth } from '../../lib/auth.svelte';
	import AccessDenied from '../../lib/AccessDenied.svelte';
	import { decimalText } from '../../lib/format';
	import { RIGHT } from '../../lib/rights';

	let rows = $state<StockValueLine[]>([]);
	let warehouse = $state<number | null>(null);
	let warehouses = $state<{ id: number; name: string | null }[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);

	const amount = $derived(rows.reduce((sum, r) => sum + Number(r.amount), 0));
	const value = $derived(rows.reduce((sum, r) => sum + Number(r.value), 0));

	async function load() {
		loading = true;
		error = null;
		try {
			rows = await api.stockValue.report(warehouse);
		} catch (e) {
			error = errorText(e, true);
		} finally {
			loading = false;
		}
	}

	function money(value: number): string {
		return value.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	}

	onMount(async () => {
		try {
			warehouses = await api.masterData.warehouses();
		} catch {
			warehouses = [];
		}
		await load();
	});
</script>

{#if !auth.hasRight(RIGHT.mayReadArticles)}
	<AccessDenied
		pageName={t('Bestandswert')}
		right={RIGHT.mayReadArticles}
		rightLabel={t('Artikel lesen')}
	/>
{:else}
	<h1>{t('Bestandswert')}</h1>

	<div class="toolbar">
		<label class="field">
			<span>{t('Lager')}</span>
			<select bind:value={warehouse} onchange={load}>
				<option value={null}>{t('Alle Lager')}</option>
				{#each warehouses as w (w.id)}
					<option value={w.id}>{w.name ?? t('Lager {id}', { id: w.id })}</option>
				{/each}
			</select>
		</label>
		<span class="spacer"></span>
		<span class="small">{loading ? t('Lade …') : t('{n} Positionen', { n: rows.length })}</span>
	</div>

	<div class="scroll-area">
		<table>
			<thead>
				<tr>
					<th>{t('Lager')}</th>
					<th>{t('Warengruppe')}</th>
					<th class="number">{t('Artikel')}</th>
					<th class="number">{t('Menge')}</th>
					<th class="number">{t('Wert')}</th>
				</tr>
			</thead>
			<tbody>
				{#each rows as r (`${r.warehouse_id}-${r.group_name}`)}
					<tr>
						<td>{r.warehouse_name ?? '—'}</td>
						<td>{r.group_name || '—'}</td>
						<td class="number">{r.articles}</td>
						<td class="number">{decimalText(r.amount)}</td>
						<td class="number">{money(Number(r.value))}</td>
					</tr>
				{:else}
					<tr><td colspan="5">—</td></tr>
				{/each}
			</tbody>
			{#if rows.length > 0}
				<tfoot>
					<tr>
						<th></th>
						<th>{t('Summe')}</th>
						<th></th>
						<th class="number">{decimalText(String(amount))}</th>
						<th class="number">{money(value)}</th>
					</tr>
				</tfoot>
			{/if}
		</table>
	</div>

	{#if error}
		<p class="message bad">{error}</p>
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
	tfoot th {
		border-top: 2px solid var(--menu-band-border);
		border-bottom: none;
	}
	.message {
		margin: 0.5rem 0 0;
		font-size: 0.82rem;
	}
	.message.bad {
		color: var(--bad);
	}
</style>
