<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type Place,
		type Tenant,
		type WarehouseAdmin,
		type WarehouseBin,
		type WarehouseLocation
	} from '../../../lib/api';
	import { askConfirm } from '../../../lib/confirm.svelte';
	import '../../../lib/search/search-window.css';

	/**
	 * Warehouses and storage bins (`img/Admin Menü/Lager und Lagerplätze`).
	 *
	 * Only with a selected tenant can a warehouse be assigned and a storage bin
	 * be picked — before that the right side stays empty, as in the template.
	 */
	type Tab = 'lager' | 'symetrisch' | 'selbst' | 'orte';

	type Toggle = {
		zugriff: boolean;
		prioritaet: boolean;
		such_std: boolean;
		entnahme: boolean;
		anmelden: boolean;
		fremd: boolean;
		konzern: boolean;
	};

	const TABS: { key: Tab; label: string }[] = [
		{ key: 'lager', label: t('Lager') },
		{ key: 'symetrisch', label: t('Lagerplatz (Symetric)') },
		{ key: 'selbst', label: t('Lagerplatz (Self defined)') },
		{ key: 'orte', label: t('Orte') }
	];

	/** The seven switches of the warehouse table, in the order of the template. */
	const TOGGLES: { key: keyof Toggle; label: string }[] = [
		{ key: 'zugriff', label: t('Zugriff') },
		{ key: 'prioritaet', label: t('Priorität') },
		{ key: 'such_std', label: t('Such-Std.') },
		{ key: 'entnahme', label: t('Entnahme') },
		{ key: 'anmelden', label: t('Anmelden') },
		{ key: 'fremd', label: t('Fremd') },
		{ key: 'konzern', label: t('Konzerninternes Lager oder ohne Dispo') }
	];

	const STORAGE = 'where.warehouse-mandant';

	let tab = $state<Tab>('lager');
	let tenants = $state<Tenant[]>([]);
	let mandant = $state<number | null>(null);
	let lager = $state<WarehouseAdmin[]>([]);
	let gewaehltesLager = $state<number | null>(null);
	let lagerplaetze = $state<WarehouseLocation[]>([]);
	let bins = $state<WarehouseBin[]>([]);
	let load = $state(true);
	let error = $state<string | null>(null);
	let message = $state<string | null>(null);

	/** Places of the house — they belong to no tenant. */
	let orte = $state<Place[]>([]);

	const currentWarehouse = $derived(lager.find((l) => l.id === gewaehltesLager) ?? null);

	onMount(async () => {
		try {
			tenants = await api.tenants.list();
			const remembered = Number(localStorage.getItem(STORAGE) ?? '');
			const start = tenants.find((m) => m.id === remembered) ?? tenants[0];
			if (start) await mandantWaehlen(start.id);
			orte = await api.places.list();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			load = false;
		}
	});

	// ------------------------------------------------------------- Places

	async function ortNeu() {
		error = null;
		try {
			await api.places.create({ name: 'Neuer Ort', note: null, active: true });
			orte = await api.places.list();
			changed(t('Ort angelegt — Namen bitte anpassen.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function saveOrt(p: Place, aenderung: Partial<Place>) {
		const neu = { ...p, ...aenderung };
		const name = neu.name.trim();
		if (!name) {
			error = t('Der Name des Ortes darf nicht leer sein.');
			return;
		}
		error = null;
		try {
			const saved = await api.places.update(p.id, {
				name,
				note: neu.note,
				active: neu.active
			});
			orte = orte.map((x) => (x.id === saved.id ? saved : x));
			changed();
		} catch (e) {
			error = errorText(e, true);
			orte = await api.places.list();
		}
	}

	async function ortLoeschen(p: Place) {
		const ok = await askConfirm({
			title: t('Ort löschen'),
			message: t('Ort „{name}" wirklich löschen?', { name: p.name }),
			confirmLabel: t('Ja, löschen'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		error = null;
		try {
			await api.places.remove(p.id);
			orte = await api.places.list();
			changed(t('Ort gelöscht.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function mandantWaehlen(id: number) {
		mandant = id;
		localStorage.setItem(STORAGE, String(id));
		gewaehltesLager = null;
		lagerplaetze = [];
		bins = [];
		error = null;
		message = null;
		await loadWarehouses();
	}

	async function loadWarehouses() {
		if (mandant === null) {
			lager = [];
			return;
		}
		try {
			lager = await api.warehouses.list(mandant);
			if (gewaehltesLager !== null && !lager.some((l) => l.id === gewaehltesLager)) {
				gewaehltesLager = null;
			}
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function lagerWaehlen(id: number) {
		gewaehltesLager = id;
		await loadPlaces();
	}

	async function loadPlaces() {
		if (gewaehltesLager === null) {
			lagerplaetze = [];
			bins = [];
			return;
		}
		try {
			const [plaetze, free] = await Promise.all([
				api.warehouses.locations(gewaehltesLager),
				api.warehouses.bins(gewaehltesLager)
			]);
			lagerplaetze = plaetze;
			bins = free;
		} catch (e) {
			error = errorText(e, true);
		}
	}

	function changed(text = t('Gespeichert.')) {
		message = text;
		setTimeout(() => (message = null), 2500);
	}

	// -------------------------------------------------------- Warehouses

	async function lagerNeu() {
		if (mandant === null) return;
		error = null;
		try {
			const createNew = await api.warehouses.create(mandant, 'Neues Lager');
			await loadWarehouses();
			gewaehltesLager = createNew.id;
			await loadPlaces();
			changed(t('Lager angelegt — Namen bitte anpassen.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	/** A check or the name — always the whole record, as the mask shows it. */
	async function saveWarehouse(w: WarehouseAdmin, aenderung: Partial<WarehouseAdmin>) {
		const createNew = { ...w, ...aenderung };
		const name = (createNew.name ?? '').trim();
		if (!name) {
			error = t('Der Name des Lagers darf nicht leer sein.');
			return;
		}
		error = null;
		try {
			const saved = await api.warehouses.update(w.id, {
				name,
				zugriff: createNew.zugriff,
				prioritaet: createNew.prioritaet,
				such_std: createNew.such_std,
				entnahme: createNew.entnahme,
				anmelden: createNew.anmelden,
				fremd: createNew.fremd,
				konzern: createNew.konzern
			});
			lager = lager.map((l) => (l.id === saved.id ? saved : l));
			changed();
		} catch (e) {
			error = errorText(e, true);
			await loadWarehouses();
		}
	}

	/** A warehouse from the legacy application carries no tenant — it is assigned here. */
	async function assignWarehouse(w: WarehouseAdmin) {
		if (mandant === null) return;
		error = null;
		try {
			const saved = await api.warehouses.assign(w.id, mandant);
			lager = lager.map((l) => (l.id === saved.id ? saved : l));
			changed(t('Lager dem Mandanten zugeordnet.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function lagerLoeschen() {		const w = currentWarehouse;
		if (!w) return;
		const ok = await askConfirm({
			title: t('Lager löschen'),
			message: t('Lager „{name}“ wirklich löschen?', { name: w.name ?? '' }),
			confirmLabel: t('Ja, löschen'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		error = null;
		try {
			await api.warehouses.remove(w.id);
			gewaehltesLager = null;
			await loadWarehouses();
			lagerplaetze = [];
			bins = [];
			changed(t('Lager gelöscht.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	// --------------------------------------------- Storage bin (Symetric)

	async function newSlot() {
		if (gewaehltesLager === null) return;
		error = null;
		try {
			await api.warehouses.locationCreate(gewaehltesLager, 'Neuer Lagerplatz');
			await loadPlaces();
			changed(t('Lagerplatz angelegt.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function saveSlot(p: WarehouseLocation, aenderung: Partial<WarehouseLocation>) {
		if (gewaehltesLager === null) return;
		const createNew = { ...p, ...aenderung };
		error = null;
		try {
			const saved = await api.warehouses.locationUpdate(gewaehltesLager, p.id, {
				name: createNew.name ?? '',
				prefix: createNew.prefix ?? '',
				values: createNew.values ?? ''
			});
			lagerplaetze = lagerplaetze.map((x) => (x.id === saved.id ? saved : x));
			changed();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function deleteSlot(p: WarehouseLocation) {
		if (gewaehltesLager === null) return;
		const ok = await askConfirm({
			title: t('Lagerplatz löschen'),
			message: t('Lagerplatz „{name}“ wirklich löschen?', { name: p.name ?? '' }),
			confirmLabel: t('Ja, löschen'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		error = null;
		try {
			await api.warehouses.locationRemove(gewaehltesLager, p.id);
			await loadPlaces();
			changed(t('Lagerplatz gelöscht.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	// ----------------------------------------- Storage bin (Self defined)

	async function binNeu() {
		if (gewaehltesLager === null) return;
		error = null;
		try {
			await api.warehouses.binCreate(gewaehltesLager, 'Neuer Lagerplatz');
			await loadPlaces();
			changed(t('Lagerplatz angelegt.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function saveBin(b: WarehouseBin, label: string) {
		if (gewaehltesLager === null) return;
		error = null;
		try {
			const saved = await api.warehouses.binUpdate(gewaehltesLager, b.id, label);
			bins = bins.map((x) => (x.id === saved.id ? saved : x));
			changed();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function binLoeschen(b: WarehouseBin) {
		if (gewaehltesLager === null) return;
		const ok = await askConfirm({
			title: t('Lagerplatz löschen'),
			message: t('Lagerplatz „{name}“ wirklich löschen?', { name: b.label ?? '' }),
			confirmLabel: t('Ja, löschen'),
			cancelLabel: t('Abbrechen')
		});
		if (!ok) return;
		error = null;
		try {
			await api.warehouses.binRemove(gewaehltesLager, b.id);
			await loadPlaces();
			changed(t('Lagerplatz gelöscht.'));
		} catch (e) {
			error = errorText(e, true);
		}
	}
</script>

<h1>{t('Lager & Lagerplätze')}</h1>

<nav class="tabs" aria-label={t('Bereiche')}>
	{#each TABS as entry (entry.key)}
		<button
			type="button"
			class="tab"
			class:active={tab === entry.key}
			onclick={() => (tab = entry.key)}>{entry.label}</button
		>
	{/each}
</nav>

{#if tab === 'orte'}
	<div class="window">
		<div class="side" aria-label={t('Ort')}>
			<div class="head">{t('Orte')}</div>
			<ul>
				{#each orte as p (p.id)}
					<li>
						<button type="button" class:active={false} onclick={() => saveOrt(p, { active: !p.active })}>
							{p.name}{#if !p.active}<span class="small">{t('inaktiv')}</span>{/if}
						</button>
					</li>
				{:else}
					<li class="empty">{t('Kein Ort vorhanden.')}</li>
				{/each}
			</ul>
		</div>

		<div class="content">
			<div class="scroll-area">
				<table>
					<thead>
						<tr>
							<th>{t('Ort')}</th>
							<th>{t('Notiz')}</th>
							<th class="narrow">{t('aktiv')}</th>
							<th class="narrow"></th>
						</tr>
					</thead>
					<tbody>
						{#each orte as p (p.id)}
							<tr>
								<td>
									<input
										value={p.name}
										onblur={(e) => {
											const value = e.currentTarget.value;
											if (value !== p.name) saveOrt(p, { name: value });
										}}
									/>
								</td>
								<td>
									<input
										value={p.note ?? ''}
										onblur={(e) => {
											const value = e.currentTarget.value.trim();
											if (value !== (p.note ?? '')) saveOrt(p, { note: value || null });
										}}
									/>
								</td>
								<td class="narrow">
									<input
										type="checkbox"
										checked={p.active}
										onchange={(e) => saveOrt(p, { active: e.currentTarget.checked })}
									/>
								</td>
								<td class="narrow">
									<button
										type="button"
										class="square"
										title={t('Ort löschen')}
										onclick={() => ortLoeschen(p)}>−</button
									>
								</td>
							</tr>
						{:else}
							<tr><td colspan="4">{t('Noch kein Ort — „+" legt einen an.')}</td></tr>
						{/each}
					</tbody>
				</table>
			</div>

			<div class="foot-row">
				<span class="counter">{t('{n} Orte', { n: orte.length })}</span>
				<span class="spacer"></span>
				<div class="button-group" role="group" aria-label={t('Ort')}>
					<button type="button" class="square" title={t('Neuer Ort')} onclick={ortNeu}>+</button>
				</div>
			</div>
		</div>
	</div>
{:else if tab === 'lager'}
	<div class="window">
		<div class="side" aria-label={t('Mandant')}>
			<div class="head">{t('Mandant')}</div>
			<ul>
				{#each tenants as m (m.id)}
					<li>
						<button
							type="button"
							class:active={m.id === mandant}
							onclick={() => mandantWaehlen(m.id)}
							title={`${m.name} [${m.short_name ?? '—'}]`}
						>
							{m.name}
						</button>
					</li>
				{:else}
					<li class="empty">{t('Kein Mandant vorhanden.')}</li>
				{/each}
			</ul>
		</div>

		<div class="content">
			{#if mandant === null}
				<div class="scroll-area"></div>
			{:else}
				<div class="scroll-area">
					<table>
						<thead>
							<tr>
								<th class="name">{t('Lager')}</th>
								{#each TOGGLES as s (s.key)}
									<th class="check" title={s.label}>{s.label}</th>
								{/each}
								<th class="id">{t('ID')}</th>
							</tr>
						</thead>
						<tbody>
							{#each lager as w (w.id)}
								<tr class:marked={w.id === gewaehltesLager} onclick={() => lagerWaehlen(w.id)}>
									<td class="name">
										<input
											value={w.name ?? ''}
											onclick={(e) => e.stopPropagation()}
											onblur={(e) => {
												const value = e.currentTarget.value.trim();
												if (value !== (w.name ?? '')) saveWarehouse(w, { name: value });
											}}
											onkeydown={(e) => {
												if (e.key === 'Enter') e.currentTarget.blur();
											}}
										/>
										{#if w.is_default}<span class="default" title={t('Vorgabelager')}>{t('Vorgabe')}</span>{/if}
										{#if w.mandant_id === null}
											<button
												type="button"
												class="assign"
												title={t('Diesem Mandanten zuordnen')}
												onclick={(e) => {
													e.stopPropagation();
													assignWarehouse(w);
												}}>{t('zuordnen')}</button
											>
										{/if}
									</td>
									{#each TOGGLES as s (s.key)}
										<td class="check">
											<input
												type="checkbox"
												checked={w[s.key]}
												onclick={(e) => e.stopPropagation()}
												onchange={(e) => saveWarehouse(w, { [s.key]: e.currentTarget.checked })}
												aria-label={t('{label} für {name}', {
													label: s.label,
													name: w.name ?? ''
												})}
											/>
										</td>
									{/each}
									<td class="id">{w.id}</td>
								</tr>
							{:else}
								<tr
									><td colspan={TOGGLES.length + 2}>
										{load
											? t('Lade …')
											: t('Noch kein Lager für diesen Mandanten — „+" legt eines an.')}
									</td></tr
								>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}

			<div class="foot-row">
				<span class="counter">
					{#if mandant !== null}{t('{n} Lager', { n: lager.length })}{/if}
				</span>
				<span class="spacer"></span>
				<div class="button-group" role="group" aria-label={t('Lager')}>
					<button
						type="button"
						class="square"
						title={t('Neues Lager')}
						disabled={mandant === null}
						onclick={lagerNeu}>+</button
					>
					<button
						type="button"
						class="square"
						title={t('Lager löschen')}
						disabled={!currentWarehouse}
						onclick={lagerLoeschen}>−</button
					>
				</div>
			</div>
		</div>
	</div>
{:else}
	<div class="window">
		<div class="side" aria-label={t('Lager')}>
			<div class="head">{t('Lager')}</div>
			<ul>
				{#each lager as w (w.id)}
					<li>
						<button
							type="button"
							class:active={w.id === gewaehltesLager}
							onclick={() => lagerWaehlen(w.id)}
						>
							{w.name ?? t('Lager {id}', { id: w.id })}
						</button>
					</li>
				{:else}
					<li class="empty">{t('Kein Lager vorhanden.')}</li>
				{/each}
			</ul>
		</div>

		<div class="content">
			{#if gewaehltesLager === null}
				<div class="scroll-area"></div>
			{:else if tab === 'symetrisch'}
				<div class="scroll-area">
					<table>
						<thead>
							<tr>
								<th>{t('Lagerplatz')}</th>
								<th class="short-code">{t('Kürzel')}</th>
								<th>{t('Werte')}</th>
								<th class="narrow"></th>
							</tr>
						</thead>
						<tbody>
							{#each lagerplaetze as p (p.id)}
								<tr>
									<td>
										<input
											value={p.name ?? ''}
											onblur={(e) => {
												const value = e.currentTarget.value;
												if (value !== (p.name ?? '')) saveSlot(p, { name: value });
											}}
										/>
									</td>
									<td class="short-code">
										<input
											value={p.prefix ?? ''}
											onblur={(e) => {
												const value = e.currentTarget.value;
												if (value !== (p.prefix ?? '')) saveSlot(p, { prefix: value });
											}}
										/>
									</td>
									<td>
										<input
											value={p.values ?? ''}
											onblur={(e) => {
												const value = e.currentTarget.value;
												if (value !== (p.values ?? '')) saveSlot(p, { values: value });
											}}
										/>
									</td>
									<td class="narrow">
										<button
											type="button"
											class="square"
											title={t('Lagerplatz löschen')}
											onclick={() => deleteSlot(p)}>−</button
										>
									</td>
								</tr>
							{:else}
								<tr
									><td colspan="4">{t('Noch kein Lagerplatz — „+" legt einen an.')}</td></tr
								>
							{/each}
						</tbody>
					</table>
				</div>
			{:else}
				<div class="scroll-area">
					<ul class="free">
						{#each bins as b (b.id)}
							<li>
								<input
									value={b.label ?? ''}
									onblur={(e) => {
										const value = e.currentTarget.value.trim();
										if (value && value !== (b.label ?? '')) saveBin(b, value);
									}}
								/>
								<button
									type="button"
									class="square"
									title={t('Lagerplatz löschen')}
									onclick={() => binLoeschen(b)}>−</button
								>
							</li>
						{:else}
							<li class="empty">{t('Noch kein Lagerplatz — „+" legt einen an.')}</li>
						{/each}
					</ul>
				</div>
			{/if}

			<div class="foot-row">
				<span class="counter">
					{#if gewaehltesLager !== null}
						{t('{n} Lagerplätze', {
							n: tab === 'symetrisch' ? lagerplaetze.length : bins.length
						})}
					{/if}
				</span>
				<span class="spacer"></span>
				<label class="input-help">
					<input type="checkbox" disabled />
					{t('Eingabehilfe aktiv')}
				</label>
				<span class="spacer"></span>
				<div class="button-group" role="group" aria-label={t('Lagerplatz')}>
					<button
						type="button"
						class="square"
						title={t('Neuer Lagerplatz')}
						disabled={gewaehltesLager === null}
						onclick={tab === 'symetrisch' ? newSlot : binNeu}>+</button
					>
				</div>
			</div>
		</div>
	</div>
{/if}

{#if error}
	<p class="message bad">{error}</p>
{/if}
{#if message}
	<p class="message good">{message}</p>
{/if}

<style>
	.tabs {
		display: flex;
		gap: 0.15rem;
		border-bottom: 1px solid var(--menu-band-border);
		margin-bottom: 0.6rem;
	}
	.tab {
		border: 1px solid transparent;
		border-bottom: none;
		background: none;
		padding: 0.3rem 0.9rem;
		font: inherit;
		font-size: 0.84rem;
		color: var(--muted);
		cursor: pointer;
		border-radius: 4px 4px 0 0;
	}
	.tab.active {
		background: var(--surface);
		border-color: var(--menu-band-border);
		color: var(--text);
		margin-bottom: -1px;
		font-weight: 600;
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

	/* Window like the mask: the list on the left, the table on the right. */
	.window {
		display: grid;
		grid-template-columns: 14rem minmax(0, 1fr);
		border: 1px solid var(--menu-band-border);
		border-radius: 4px;
		background: var(--surface);
		overflow: hidden;
		max-width: 74rem;
	}
	.side {
		border-right: 1px solid var(--menu-band-border);
		background: var(--surface-2);
		display: flex;
		flex-direction: column;
		min-height: 20rem;
		max-height: 32rem;
	}
	.side .head {
		padding: 0.3rem 0.5rem;
		font-size: 0.78rem;
		font-weight: 700;
		color: var(--muted);
		text-align: center;
		background: var(--surface-3);
		border-bottom: 1px solid var(--menu-band-border);
	}
	.side ul {
		list-style: none;
		margin: 0;
		padding: 0;
		overflow: auto;
	}
	.side li button {
		display: block;
		width: 100%;
		padding: 0.25rem 0.5rem;
		border: 0;
		background: none;
		font: inherit;
		font-size: 0.8rem;
		color: var(--text);
		text-align: left;
		cursor: pointer;
	}
	.side li button:hover {
		background: var(--menu-band-hover);
	}
	.side li button.active {
		background: var(--highlight);
		font-weight: 700;
	}
	.side .empty {
		padding: 0.4rem 0.5rem;
		font-size: 0.78rem;
		color: var(--muted-2);
	}

	.content {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 20rem;
	}
	.scroll-area {
		flex: 1 1 auto;
		max-height: 32rem;
		overflow: auto;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}
	thead th {
		position: sticky;
		top: 0;
		z-index: 1;
		padding: 0.3rem 0.45rem;
		font-weight: 600;
		color: var(--accent);
		background: var(--surface-2);
		border-bottom: 1px solid var(--menu-band-border);
		text-align: left;
		white-space: nowrap;
	}
	th.check,
	td.check {
		width: 3.6rem;
		text-align: center;
	}
	th.id,
	td.id {
		width: 4rem;
		text-align: right;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}
	th.name {
		min-width: 12rem;
	}
	th.short-code,
	td.short-code {
		width: 8rem;
	}
	th.narrow,
	td.narrow {
		width: 2.5rem;
		text-align: center;
	}
	tbody td {
		padding: 0.15rem 0.35rem;
		border-bottom: 1px solid var(--surface-3);
	}
	tbody tr:nth-child(even) {
		background: var(--surface-2);
	}
	tbody tr:hover {
		background: var(--highlight);
	}
	tbody tr.marked {
		outline: 1px solid var(--accent);
		outline-offset: -1px;
	}
	td input:not([type='checkbox']) {
		width: 100%;
		padding: 0.15rem 0.3rem;
		font: inherit;
		font-size: 0.8rem;
		color: var(--text);
		background: var(--surface);
		border: 1px solid transparent;
		border-radius: 3px;
	}
	td input:not([type='checkbox']):hover {
		border-color: var(--menu-band-border);
	}
	td input:not([type='checkbox']):focus {
		border-color: var(--accent);
		outline: none;
	}
	.default {
		margin-left: 0.35rem;
		padding: 0 0.3rem;
		font-size: 0.68rem;
		color: var(--accent);
		border: 1px solid var(--accent);
		border-radius: 3px;
	}
	.assign {
		margin-left: 0.35rem;
		padding: 0 0.35rem;
		font: inherit;
		font-size: 0.7rem;
		color: var(--muted);
		background: var(--surface-2);
		border: 1px dashed var(--menu-band-border);
		border-radius: 3px;
		cursor: pointer;
	}
	.assign:hover {
		color: var(--accent);
		border-color: var(--accent);
	}

	.free {
		list-style: none;
		margin: 0;
		padding: 0.3rem;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.free li {
		display: flex;
		gap: 0.3rem;
		align-items: center;
	}
	.free li.empty {
		font-size: 0.8rem;
		color: var(--muted-2);
	}
	.free input {
		flex: 1;
		padding: 0.2rem 0.35rem;
		font: inherit;
		font-size: 0.8rem;
		color: var(--text);
		background: var(--surface);
		border: 1px solid var(--menu-band-border);
		border-radius: 3px;
	}

	.foot-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.35rem 0.5rem;
		border-top: 1px solid var(--menu-band-border);
		background: var(--menu-band-1);
	}
	.counter {
		font-size: 0.76rem;
		color: var(--muted-2);
		font-variant-numeric: tabular-nums;
	}
	.spacer {
		flex: 1;
	}
	.input-help {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.76rem;
		color: var(--muted);
	}
	.button-group {
		display: flex;
		gap: 0.2rem;
	}
	.square {
		width: 1.9rem;
		height: 1.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.9rem;
		line-height: 1;
		cursor: pointer;
	}
	.square:disabled {
		color: var(--muted-2);
		cursor: default;
	}
</style>
