<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type Stocktake, type StocktakeDetail, type StocktakeLine } from '../../lib/api';
	import { askConfirm } from '../../lib/confirm.svelte';
	import { auth } from '../../lib/auth.svelte';
	import AccessDenied from '../../lib/AccessDenied.svelte';
	import { decimalText, decimalValue } from '../../lib/format';
	import { RIGHT } from '../../lib/rights';

	let stocktakes = $state<Stocktake[]>([]);
	let detail = $state<StocktakeDetail | null>(null);
	let warehouses = $state<
		{ id: number; name: string | null; is_default: boolean; zugriff: boolean }[]
	>([]);
	let warehouseId = $state<number | null>(null);
	let name = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let message = $state<string | null>(null);
	/** What is typed into `Ist`, per line; saved on blur. */
	let counted = $state<Record<number, string>>({});

	const mayCount = $derived(auth.hasRight(RIGHT.mayEditArticle));
	const open = $derived(detail?.stocktake.state === 'open');

	const STATE_TEXT: Record<string, string> = {
		open: t('läuft'),
		closed: t('abgeschlossen'),
		cancelled: t('abgebrochen')
	};

	function amount(value: string | null): string {
		return value === null ? '—' : decimalText(value);
	}

	async function load() {
		loading = true;
		error = null;
		try {
			stocktakes = await api.stocktaking.list();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			loading = false;
		}
	}

	async function loadWarehouses() {
		try {
			// Only warehouses released for bookings can be counted.
			warehouses = (await api.masterData.warehouses()).filter((l) => l.zugriff);
			const start = warehouses.find((l) => l.is_default) ?? warehouses[0];
			warehouseId = start?.id ?? null;
			proposeName();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	function proposeName() {
		const lager = warehouses.find((l) => l.id === warehouseId);
		const heute = new Date().toLocaleDateString('de-DE');
		name = lager
			? t('Inventur {warehouse} {date}', { warehouse: lager.name ?? '', date: heute })
			: '';
	}

	async function choose(id: number) {
		error = null;
		message = null;
		try {
			detail = await api.stocktaking.get(id);
			const werte: Record<number, string> = {};
			for (const line of detail.lines) {
				werte[line.id] = line.counted_quantity === null ? '' : decimalText(line.counted_quantity);
			}
			counted = werte;
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function start() {
		if (warehouseId === null) return;
		error = null;
		message = null;
		try {
			const neu = await api.stocktaking.start({
				warehouse_id: warehouseId,
				name: name.trim(),
				note: null
			});
			await load();
			await choose(neu.id);
			message = t('Inventur {name} gestartet.', { name: neu.name });
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function save(line: StocktakeLine) {
		if (!detail || !open) return;
		const wert = decimalValue(counted[line.id] ?? '');
		const vorher = line.counted_quantity === null ? null : decimalText(line.counted_quantity);
		const gleich =
			(wert === null && line.counted_quantity === null) ||
			(wert !== null && vorher !== null && Number(wert) === Number(decimalValue(vorher)));
		if (gleich) return;
		error = null;
		try {
			await api.stocktaking.count(detail.stocktake.id, line.id, { counted: wert, note: null });
			await choose(detail.stocktake.id);
			await load();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function finish() {
		if (!detail) return;
		const offen = detail.lines.filter(
			(l) => l.difference !== null && Number(l.difference) !== 0
		).length;
		const ok = await askConfirm({
			title: t('Inventur abschließen'),
			message: t('{count} Differenzen als Korrektur buchen?', { count: offen }),
			confirmLabel: t('Ja, abschließen'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		error = null;
		message = null;
		try {
			const fertig = await api.stocktaking.finish(detail.stocktake.id);
			await load();
			await choose(fertig.id);
			message = t('Inventur abgeschlossen.');
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function cancel() {
		if (!detail) return;
		const ok = await askConfirm({
			title: t('Inventur abbrechen'),
			message: t('Inventur „{name}" ohne Buchung abbrechen?', { name: detail.stocktake.name }),
			confirmLabel: t('Ja, abbrechen'),
			cancelLabel: t('Zurück')
		});
		if (!ok) return;
		error = null;
		message = null;
		try {
			const abgebrochen = await api.stocktaking.cancel(detail.stocktake.id);
			await load();
			await choose(abgebrochen.id);
			message = t('Inventur abgebrochen.');
		} catch (e) {
			error = errorText(e, true);
		}
	}

	onMount(async () => {
		await loadWarehouses();
		await load();
		if (stocktakes.length) await choose(stocktakes[0].id);
	});
</script>

{#if !auth.hasRight(RIGHT.mayReadArticles)}
	<AccessDenied
		pageName={t('Inventur')}
		right={RIGHT.mayReadArticles}
		rightLabel={t('Artikel lesen')}
	/>
{:else}
	<h1>{t('Inventur')}</h1>

	<div class="window">
		<div class="side">
			<div class="head">{t('Zählungen')}</div>
			<ul>
				{#each stocktakes as s (s.id)}
					<li>
						<button
							type="button"
							class:active={detail?.stocktake.id === s.id}
							onclick={() => choose(s.id)}
							title={`${s.name} · ${STATE_TEXT[s.state] ?? s.state}`}
						>
							<span>{s.name}</span>
							<span class="small">{s.counted}/{s.lines}</span>
						</button>
					</li>
				{:else}
					<li class="empty">{loading ? t('Lade …') : '—'}</li>
				{/each}
			</ul>
		</div>

		<div class="content">
			{#if mayCount}
				<div class="start">
					<label class="field">
						<span>{t('Lager')}</span>
						<select bind:value={warehouseId} onchange={proposeName}>
							{#each warehouses as l (l.id)}
								<option value={l.id}>{l.name ?? t('Lager {id}', { id: l.id })}</option>
							{/each}
						</select>
					</label>
					<label class="field wide">
						<span>{t('Bezeichnung')}</span>
						<input bind:value={name} />
					</label>
					<button class="primary" type="button" onclick={start} disabled={warehouseId === null}>
						{t('Inventur starten')}
					</button>
				</div>
			{/if}

			{#if detail}
				<div class="head-row">
					<strong>{detail.stocktake.name}</strong>
					<span class="small">
						{detail.stocktake.warehouse_name ?? ''} ·
						{STATE_TEXT[detail.stocktake.state] ?? detail.stocktake.state}
						{#if detail.stocktake.finished_at}· {detail.stocktake.finished_at.slice(0, 10)}{/if}
					</span>
				</div>

				<div class="scroll-area">
					<table>
						<thead>
							<tr>
								<th class="number">{t('Art.-Nr.')}</th>
								<th>{t('Artikel')}</th>
								<th>{t('Serien-Nr.')}</th>
								<th>{t('Lagerplatz')}</th>
								<th class="number">{t('Soll')}</th>
								<th class="number">{t('Ist')}</th>
								<th class="number">{t('Differenz')}</th>
								<th>{t('gezählt von')}</th>
							</tr>
						</thead>
						<tbody>
							{#each detail.lines as line (line.id)}
								<tr
									class:gone={line.device_gone}
									class:differs={line.difference !== null && Number(line.difference) !== 0}
								>
									<td class="number">{line.article_no ?? '—'}</td>
									<td>{line.article_name ?? '—'}</td>
									<td>{line.serial_no ?? '—'}</td>
									<td>{line.bin_location ?? '—'}</td>
									<td class="number">{amount(line.book_quantity)}</td>
									<td class="number">
										{#if open && mayCount}
											<input
												class="ist"
												bind:value={counted[line.id]}
												inputmode="decimal"
												onblur={() => save(line)}
												onkeydown={(e) => e.key === 'Enter' && save(line)}
											/>
										{:else}
											{amount(line.counted_quantity)}
										{/if}
									</td>
									<td class="number">{amount(line.difference)}</td>
									<td class="small">{line.counted_by ?? ''}</td>
								</tr>
							{:else}
								<tr
									><td colspan="8">{t('Keine Zeilen — in diesem Lager steht kein Material.')}</td
									></tr
								>
							{/each}
						</tbody>
					</table>
				</div>

				<div class="foot-row">
					<span class="small">
						{t('{counted} von {lines} gezählt · {differences} Differenzen', {
							counted: detail.stocktake.counted,
							lines: detail.stocktake.lines,
							differences: detail.stocktake.differences
						})}
					</span>
					<span class="spacer"></span>
					{#if open && mayCount}
						<button type="button" onclick={cancel}>{t('Abbrechen')}</button>
						<button class="primary" type="button" onclick={finish}>{t('Abschließen')}</button>
					{/if}
				</div>
			{:else}
				<p class="notice">—</p>
			{/if}
		</div>
	</div>

	{#if error}
		<p class="message bad">{error}</p>
	{/if}
	{#if message}
		<p class="message good">{message}</p>
	{/if}
{/if}

<style>
	.window {
		display: grid;
		grid-template-columns: 16rem minmax(0, 1fr);
		border: 1px solid var(--menu-band-border);
		border-radius: 4px;
		background: var(--surface);
		overflow: hidden;
	}
	.side {
		border-right: 1px solid var(--menu-band-border);
		background: var(--surface-2);
		padding: 0.5rem 0;
	}
	.side .head {
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--muted-2);
		padding: 0 0.75rem 0.4rem;
	}
	.side ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.side button {
		display: flex;
		justify-content: space-between;
		gap: 0.5rem;
		width: 100%;
		border: none;
		background: none;
		font: inherit;
		font-size: 0.82rem;
		text-align: left;
		padding: 0.3rem 0.75rem;
		cursor: pointer;
		color: var(--text);
	}
	.side button.active {
		background: var(--highlight);
		font-weight: 600;
	}
	.side .empty {
		padding: 0.3rem 0.75rem;
		color: var(--muted-2);
		font-size: 0.82rem;
	}
	.content {
		padding: 0.75rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		min-width: 0;
	}
	.start {
		display: flex;
		align-items: flex-end;
		gap: 0.6rem;
		flex-wrap: wrap;
		padding-bottom: 0.6rem;
		border-bottom: 1px solid var(--menu-band-border);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.78rem;
	}
	.field.wide {
		flex: 1;
		min-width: 14rem;
	}
	.head-row {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.scroll-area {
		max-height: 60vh;
		overflow: auto;
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		text-align: left;
		padding: 0.25rem 0.4rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.82rem;
		white-space: nowrap;
	}
	th.number,
	td.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	td :global(.ist) {
		width: 5rem;
		text-align: right;
	}
	tr.differs td {
		background: var(--highlight);
	}
	tr.gone td {
		color: var(--muted-2);
		text-decoration: line-through;
	}
	.foot-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		border-top: 1px solid var(--menu-band-border);
		padding-top: 0.5rem;
	}
	.spacer {
		flex: 1;
	}
	.small {
		font-size: 0.78rem;
		color: var(--muted);
	}
	.notice {
		color: var(--muted);
		font-size: 0.85rem;
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
