<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from './escape.svelte';
	import { api, errorText, type Device, type Place, type StockRow } from './api';
	import { decimalText, decimalValue, quantity as quantityText } from './format';

	/** Which in-house movement the window books. */
	export type StockActionMode = 'issue' | 'transfer' | 'return' | 'consume';

	interface Props {
		mode: StockActionMode;
		articleId: number;
		device: Device;
		stock: StockRow[];
		places: Place[];
		/** Text for the line under the container after the booking. */
		onfertig: (text: string) => void;
		onschliessen: () => void;
	}

	let { mode, articleId, device, stock, places, onfertig, onschliessen }: Props = $props();

	const TITLES: Record<StockActionMode, string> = {
		issue: t('Ausgeben'),
		transfer: t('Umlegen'),
		return: t('Zurücknehmen'),
		consume: t('Verbrauchen')
	};

	/** `true` when the row carries an amount that can be split. */
	const splittable = $derived((Number(device.quantity ?? '1') || 1) > 1);

	/** Warehouses the material can go to — the current one is no target. */
	const targets = $derived(
		stock.filter((l) => l.stock_id !== device.stock_id && l.zugriff)
	);

	// svelte-ignore state_referenced_locally
	let placeId = $state<number | null>(places.find((p) => p.active)?.id ?? null);
	// svelte-ignore state_referenced_locally
	let stockId = $state<number | null>(targets[0]?.stock_id ?? null);
	// svelte-ignore state_referenced_locally
	let amount = $state(decimalText(device.quantity) || '1');
	let until = $state('');
	let note = $state('');
	// svelte-ignore state_referenced_locally
	let condition = $state(device.condition ?? '');
	let saving = $state(false);
	let error = $state<string | null>(null);

	function close() {
		onschliessen();
	}

	async function book() {
		saving = true;
		error = null;
		try {
			const menge = splittable ? amount.trim() || null : null;
			if (mode === 'issue' && placeId !== null) {
				await api.article.issueDevice(articleId, device.id, {
					place_id: placeId,
					quantity: decimalValue(menge),
					until: until.trim() || null,
					note: note.trim() || null
				});
				const ort = places.find((p) => p.id === placeId)?.name ?? '';
				onfertig(t('Ausgegeben: {place}', { place: ort }));
			} else if (mode === 'transfer' && stockId !== null) {
				await api.article.transferDevice(articleId, device.id, {
					stock_id: stockId,
					quantity: decimalValue(menge),
					note: note.trim() || null
				});
				const lager = stock.find((l) => l.stock_id === stockId)?.stock_name ?? '';
				onfertig(t('Umgelegt nach {stock}', { stock: lager }));
			} else if (mode === 'return') {
				await api.article.returnDevice(articleId, device.id, {
					condition: condition.trim() || null,
					note: note.trim() || null
				});
				onfertig(t('Zurückgenommen.'));
			} else if (mode === 'consume') {
				await api.article.consumeDevice(articleId, device.id, {
					quantity: decimalValue(amount.trim() || null) ?? '0',
					note: note.trim() || null
				});
				onfertig(t('Verbraucht: {amount}', { amount: amount.trim() }));
			}
			close();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			saving = false;
		}
	}

	const ready = $derived(
		mode === 'return' ||
			(mode === 'issue' && placeId !== null) ||
			(mode === 'transfer' && stockId !== null) ||
			mode === 'consume'
	);

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void close()));
</script>

<div class="backdrop" role="presentation">
	<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="lb-titel">
		<header>
			<h2 id="lb-titel">{TITLES[mode]}</h2>
			<span class="device">
				{t('ID')} {device.id}{#if device.serial_no} · {device.serial_no}{/if}
			</span>
			<button class="close" type="button" onclick={close} title={t('Schließen')}>✕</button>
		</header>

		{#if mode === 'return'}
			<div class="fields">
				<p class="state">
					{device.place_name ?? '—'}{#if device.issued_until} · {t('bis')} {device.issued_until}{/if}
				</p>
				<label class="row">
					<span>{t('Zustand')}</span>
					<input bind:value={condition} />
				</label>
			</div>
		{:else}
			<div class="fields">
				{#if mode === 'issue'}
					<label class="row">
						<span>{t('Ort')}</span>
						<select bind:value={placeId}>
							{#each places.filter((p) => p.active) as p (p.id)}
								<option value={p.id}>{p.name}</option>
							{/each}
						</select>
					</label>
					<label class="row">
						<span>{t('bis')}</span>
						<input type="date" bind:value={until} />
					</label>
				{:else if mode === 'transfer'}
					<label class="row">
						<span>{t('Lager')}</span>
						<select bind:value={stockId}>
							{#each targets as l (l.stock_id)}
								<option value={l.stock_id}>{l.stock_name}</option>
							{/each}
						</select>
					</label>
				{/if}
				{#if splittable || mode === 'consume'}
					<label class="row">
						<span>{t('Menge')}</span>
						<input bind:value={amount} inputmode="decimal" />
					</label>
					<span class="small">{t('von {total}', { total: quantityText(device.quantity) })}</span>
				{/if}
			</div>
		{/if}

		<label class="block">
			<span>{t('Notiz')}</span>
			<input bind:value={note} />
		</label>

		{#if error}
			<p class="bad">{error}</p>
		{/if}

		<footer>
			<button type="button" onclick={close}>{t('Abbrechen')}</button>
			<button class="primary" type="button" onclick={book} disabled={saving || !ready}>
				{saving ? t('Buche …') : TITLES[mode]}
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
		padding: 4rem 1rem;
		z-index: 50;
		overflow: auto;
	}
	.dialog {
		background: var(--surface);
		border-radius: 8px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.3);
		width: min(24rem, 100%);
		padding: 0 1rem 1rem;
	}
	header {
		display: flex;
		align-items: baseline;
		gap: 0.6rem;
		border-bottom: 1px solid var(--border);
		padding: 0.75rem 0 0.5rem;
	}
	h2 {
		font-size: 1rem;
		margin: 0;
	}
	.device,
	.small,
	.state {
		color: var(--muted);
		font-size: 0.82rem;
	}
	.state {
		margin: 0.75rem 0 0;
	}
	.close {
		margin-left: auto;
		border: none;
		background: none;
		font-size: 1rem;
		cursor: pointer;
		color: var(--muted);
	}
	.fields {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin: 0.75rem 0 0;
	}
	.row {
		display: grid;
		grid-template-columns: 6rem 1fr;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.85rem;
	}
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		margin-top: 0.75rem;
		font-size: 0.85rem;
	}
	.bad {
		color: var(--bad);
		margin: 0.6rem 0 0;
		font-size: 0.82rem;
	}
	footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 1rem;
	}
</style>
