<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api, errorText, type Dashboard } from '../lib/api';
	import { auth } from '../lib/auth.svelte';
	import { RIGHT } from '../lib/rights';
	import { quantity, seit, count } from '../lib/format';
	import { openAddressEdit } from '../lib/addresses/addressDialogStore.svelte';
	import { modules } from '../lib/modules.svelte';

	/** Rental/sale metrics belong to `WHERE - does my stuff go?`. */
	const pricingEnabled = $derived(modules.isInstalled('go'));

	let data = $state<Dashboard | null>(null);
	let health = $state<{ status: string; version: string } | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let updated = $state<string | null>(null);

	async function load() {
		loading = true;
		error = null;
		try {
			const [overview, status] = await Promise.all([api.dashboard(), api.health()]);
			data = overview;
			health = status;
			updated = new Intl.DateTimeFormat('de-DE', {
				hour: '2-digit',
				minute: '2-digit',
				second: '2-digit'
			}).format(new Date());
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	onMount(load);

	// Without the core application the dashboard stays empty: `WHERE - are my people?`
	// starts at its board — the schedule if the account may read it, otherwise at the
	// employee list.
	$effect(() => {
		if (!modules.ready || modules.isInstalled('stuff') || !modules.isInstalled('people')) return;
		void goto(auth.hasRight(RIGHT.mayReadDispo) ? '/people/schedule' : '/people', {
			replaceState: true
		});
	});

	/** The address mask expects the full record, the overview only the short form. */
	async function openAddress(id: number) {
		try {
			openAddressEdit(await api.addresses.get(id));
		} catch (e) {
			error = errorText(e);
		}
	}

	/** Longest bar of the distributions; 1 prevents a division by zero. */
	const groupMaximum = $derived(Math.max(1, ...(data?.groups ?? []).map((g) => g.articles)));
	const classMaximum = $derived(
		Math.max(1, ...(data?.pyro.hazard_classes ?? []).map((k) => k.count))
	);

	/** Bar width in percent, at least a visible remainder. */
	function share(value: number, maximum: number): string {
		return `${Math.max(2, Math.round((value / maximum) * 100))}%`;
	}

	function openArticle(id: number) {
		void goto(`/articles/${id}`);
	}

	/** Articles of this hazard class via the pyro search of the article list. */
	function searchPyro(className: string) {
		void goto(`/articles?pyro=${encodeURIComponent(JSON.stringify({ hazard_class: className }))}`);
	}
</script>

<header class="page-head">
	<h1>{t('Zentrale')}</h1>
	<div class="status">
		{#if error}
			<span class="badge bad">{t('nicht erreichbar')}</span>
		{:else if health}
			<span class="badge good">{t('verbunden · v{version}', { version: health.version })}</span>
		{:else}
			<span class="badge">{t('verbinde …')}</span>
		{/if}
		<button type="button" onclick={load} disabled={loading}>
			{loading ? t('Lade …') : t('Aktualisieren')}
		</button>
	</div>
</header>

{#if error}
	<div class="notice">
		<strong>{t('Die Übersicht konnte nicht geladen werden.')}</strong>
		<p>{error}</p>
	</div>
{:else if !data}
	<p class="empty">{t('Kennzahlen werden geladen …')}</p>
{:else}
	<section class="metrics">
		<article class="metric">
			<h2>{t('Artikel')}</h2>
			<p class="value">{count(data.articles.total)}</p>
			<p class="detail">
				{t('{active} active · {inactive} inactive', {
					active: count(data.articles.active),
					inactive: count(data.articles.inactive)
				})}
				{#if data.articles.virtual_articles}
					{t('· {n} Dienstleistung', { n: count(data.articles.virtual_articles) })}
				{/if}
			</p>
			<p class="detail">
				{#if data.articles.changed_last_30_days}
					{t('{n} in 30 Tagen bearbeitet', { n: count(data.articles.changed_last_30_days) })}
				{:else}
					{t('seit 30 Tagen unverändert')}
				{/if}
			</p>
			<a class="footnote" href="/articles">{t('Zur Artikelliste')}</a>
		</article>

		<article class="metric">
			<h2>{t('Adressen')}</h2>
			<p class="value">{count(data.addresses.total)}</p>
			<p class="detail">
				{t('{customers} Kunden · {suppliers} Lieferanten', {
					customers: count(data.addresses.customers),
					suppliers: count(data.addresses.suppliers)
				})}
			</p>
			<p class="detail">
				{t('{interested} Interessenten · {locations} Locations', {
					interested: count(data.addresses.interested),
					locations: count(data.addresses.locations)
				})}
				{#if data.addresses.vip}
					{t('· {n} VIP', { n: count(data.addresses.vip) })}
				{/if}
			</p>
			<a class="footnote" href="/addresses">{t('Zur Adressliste')}</a>
		</article>

		{#if pricingEnabled}
			<article class="metric">
				<h2>{t('Bestand Miete')}</h2>
				<p class="value">{quantity(data.stock.rental)}</p>
				<p class="detail">
					{t('Verkauf: {n} Einheiten', { n: quantity(data.stock.sale) })}
				</p>
				<p class="detail">
					{t('{reserved} reserviert · {withdrawn} entnommen', {
						reserved: quantity(data.stock.reserved),
						withdrawn: quantity(data.stock.withdrawn)
					})}
				</p>
				<a class="footnote" href="/articles">{t('Zum Bestand')}</a>
			</article>
		{/if}

		<article class="metric">
			<h2>{t('Geräte')}</h2>
			<p class="value">{count(data.stock.devices)}</p>
			<p class="detail">
				{t('{n} im Bestand', {
					n: count(Math.max(0, data.stock.devices - data.stock.discarded_devices))
				})}
				{#if data.stock.discarded_devices}
					{t('· {n} ausgesondert', { n: count(data.stock.discarded_devices) })}
				{/if}
			</p>
			<p class="detail">{t('Geräte und Chargen aus dem Wareneingang')}</p>
		</article>
	</section>

	<div class="grid">
		<section class="card">
			<h2>{t('Zuletzt bearbeitete Artikel')}</h2>
			{#if data.recent_articles.length}
				<ul class="list">
					{#each data.recent_articles as artikel (artikel.id)}
						<li>
							<button type="button" onclick={() => openArticle(artikel.id)}>
								<span class="no">{artikel.article_no}</span>
								<span class="name">{artikel.name ?? '—'}</span>
								<span class="meta">{artikel.group ?? t('ohne Warengruppe')}</span>
								<span class="time">{seit(artikel.changed_at)}</span>
							</button>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="empty">{t('Noch keine Artikel angelegt.')}</p>
			{/if}
		</section>

		<section class="card">
			<h2>{t('Neueste Adressen')}</h2>
			{#if data.recent_addresses.length}
				<ul class="list">
					{#each data.recent_addresses as address (address.id)}
						<li>
							<button
								type="button"
								onclick={() => openAddress(address.id)}
								title={t('Adresse öffnen')}
							>
								<span class="no">{address.customer_no ?? '—'}</span>
								<span class="name">{address.name ?? '—'}</span>
								<span class="meta">{address.city ?? ''}</span>
								<span class="time" class:inactive={!address.active}>
									{address.active ? t('aktiv') : t('inaktiv')}
								</span>
							</button>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="empty">{t('Noch keine Adressen angelegt.')}</p>
			{/if}
			<a class="footnote" href="/addresses">{t('Alle Adressen')}</a>
		</section>

		<section class="card">
			<h2>{t('Warengruppen')}</h2>
			{#if data.groups.length}
				<ul class="bars">
					{#each data.groups as group (group.id)}
						<li>
							<a href={`/articles?group=${group.id}`} title={t('Artikel dieser Warengruppe zeigen')}>
								<span class="label">{group.name ?? t('Gruppe {id}', { id: group.id })}</span>
								<span class="track">
									<span
										class="fill"
										style:width={share(group.articles, groupMaximum)}
									></span>
								</span>
								<span class="count">{count(group.articles)}</span>
							</a>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="empty">{t('Keine Warengruppen angelegt.')}</p>
			{/if}
			<a class="footnote" href="/admin/articles">{t('Warengruppen pflegen')}</a>
		</section>

		<section class="card">
			<h2>{t('Lager')}</h2>
			{#if data.warehouses.length}
				<table>
					<thead>
						<tr>
							<th>{t('Lager')}</th>
							{#if pricingEnabled}
								<th class="count">{t('Miete')}</th>
								<th class="count">{t('Verkauf')}</th>
							{/if}
						</tr>
					</thead>
					<tbody>
						{#each data.warehouses as lager (lager.id)}
							<tr>
								<td>{lager.name}</td>
								{#if pricingEnabled}
									<td class="count">{quantity(lager.rental)}</td>
									<td class="count">{quantity(lager.sale)}</td>
								{/if}
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<p class="empty">{t('Kein Lager angelegt.')}</p>
			{/if}
		</section>

		<section class="card">
			<h2>{t('Pyro-Register')}</h2>
			<p class="detail">
				{t('{n} Artikel mit UN-Nr.', { n: count(data.pyro.articles) })}
				{#if data.pyro.articles_with_hazard_class}
					{t('· {n} mit Gefahrklasse', {
						n: count(data.pyro.articles_with_hazard_class)
					})}
				{/if}
			</p>
			{#if data.pyro.hazard_classes.length}
				<ul class="bars">
					{#each data.pyro.hazard_classes as className (className.label)}
						<li>
							<button
								type="button"
								onclick={() => searchPyro(className.label)}
								title={t('Artikel mit dieser Gefahrklasse zeigen')}
							>
								<span class="label">{className.label}</span>
								<span class="track">
									<span class="fill" style:width={share(className.count, classMaximum)}
									></span>
								</span>
								<span class="count">{count(className.count)}</span>
							</button>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="empty">{t('Kein Artikel mit pyrotechnischen Kennwerten.')}</p>
			{/if}
		</section>

		<section class="card">
			<h2>{t('Datenqualität')}</h2>
			<ul class="list compact">
				<li>
					<a href="/articles?group=0" title={t('Artikel ohne Warengruppe zeigen')}>
						<span class="name">{t('Artikel ohne Warengruppe')}</span>
						<span class="count">{count(data.quality.articles_without_group)}</span>
					</a>
				</li>
				{#if pricingEnabled}
					<li>
						<span class="row">
							<span class="name">{t('Artikel ohne Mietpreis')}</span>
							<span class="count">{count(data.quality.articles_without_rental_price)}</span>
						</span>
					</li>
				{/if}
				<li>
					<span class="row">
						<span class="name">{t('Artikel ohne Bestand')}</span>
						<span class="count">{count(data.quality.articles_without_stock)}</span>
					</span>
				</li>
				<li>
					<span class="row">
						<span class="name">{t('Adressen ohne E-Mail')}</span>
						<span class="count">{count(data.quality.addresses_without_email)}</span>
					</span>
				</li>
				<li>
					<span class="row">
						<span class="name">{t('Adressen ohne Kunden-/Lieferantennummer')}</span>
						<span class="count">{count(data.quality.addresses_without_number)}</span>
					</span>
				</li>
			</ul>
			<p class="detail">{t('Die Zahl ist ein Hinweis auf fehlende Pflege, kein Fehler.')}</p>
		</section>
	</div>

	{#if updated}
		<p class="stamp">
			{t('Stand {time} · alle Angaben aus der laufenden Datenbank', { time: updated })}
		</p>
	{/if}
{/if}

<style>
	.page-head {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}
	.status {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}
	.badge {
		padding: 0.15rem 0.5rem;
		border-radius: 10px;
		font-size: 0.78rem;
		color: var(--muted);
		background: var(--surface-3);
	}
	.badge.good {
		color: var(--good);
		background: color-mix(in srgb, var(--good) 12%, transparent);
	}
	.badge.bad {
		color: var(--bad);
		background: color-mix(in srgb, var(--bad) 12%, transparent);
	}
	.status button {
		padding: 0.3rem 0.8rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--text);
		cursor: pointer;
	}
	.status button:disabled {
		cursor: default;
		opacity: 0.6;
	}

	.notice {
		border: 1px solid var(--border);
		border-left: 3px solid var(--bad);
		border-radius: 4px;
		padding: 0.6rem 0.8rem;
		background: var(--surface);
	}
	.notice p {
		margin: 0.3rem 0 0;
		font-size: 0.85rem;
		color: var(--muted);
	}

	/* Four metrics on top. */
	.metrics {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}
	@media (max-width: 1100px) {
		.metrics {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	.metric {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 0.75rem 0.9rem;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.metric h2 {
		margin: 0;
		font-size: 0.8rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--muted);
	}
	.metric .value {
		margin: 0.15rem 0;
		font-size: 1.9rem;
		font-weight: 600;
		line-height: 1.1;
		font-variant-numeric: tabular-nums;
	}
	.metric .detail {
		margin: 0;
		font-size: 0.8rem;
		color: var(--muted);
	}

	/* Two columns for the detail cards. */
	.grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem;
		align-items: start;
	}
	@media (max-width: 1000px) {
		.grid {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	.card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 0.75rem 0.9rem 0.85rem;
	}
	.card h2 {
		margin: 0 0 0.55rem;
		font-size: 0.95rem;
	}
	.footnote {
		display: inline-block;
		margin-top: 0.55rem;
		font-size: 0.78rem;
		color: var(--accent);
		text-decoration: none;
	}
	.footnote:hover {
		text-decoration: underline;
	}
	.detail {
		margin: 0 0 0.4rem;
		font-size: 0.82rem;
		color: var(--muted);
	}
	.empty {
		color: var(--muted-2);
		font-size: 0.85rem;
		margin: 0.2rem 0;
	}

	/* Lists of row buttons: the whole row is the click target. */
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.list li + li {
		border-top: 1px solid var(--surface-3);
	}
	.list button,
	.list a,
	.list .row {
		display: grid;
		grid-template-columns: 4.5rem minmax(0, 1fr) minmax(0, 8rem) auto;
		align-items: baseline;
		gap: 0.5rem;
		width: 100%;
		padding: 0.32rem 0.3rem;
		border: none;
		border-radius: 3px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.85rem;
		text-align: left;
		text-decoration: none;
	}
	.list button,
	.list a {
		cursor: pointer;
	}
	.list button:hover,
	.list a:hover,
	.list button:focus-visible,
	.list a:focus-visible {
		background: var(--surface-3);
	}
	.list.compact a,
	.list.compact .row {
		grid-template-columns: minmax(0, 1fr) auto;
	}
	.no {
		color: var(--muted);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta {
		color: var(--muted-2);
		font-size: 0.78rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.time {
		color: var(--muted);
		font-size: 0.78rem;
		white-space: nowrap;
	}
	.time.inactive {
		color: var(--muted-2);
		font-style: italic;
	}
	.count {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	/* Distributions as bars. */
	.bars {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.bars li + li {
		margin-top: 0.2rem;
	}
	.bars a,
	.bars button {
		display: grid;
		grid-template-columns: 9rem minmax(0, 1fr) 3rem;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.15rem 0.3rem;
		border: none;
		border-radius: 3px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
	}
	.bars a:hover,
	.bars button:hover,
	.bars a:focus-visible,
	.bars button:focus-visible {
		background: var(--surface-3);
	}
	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.track {
		height: 0.55rem;
		border-radius: 3px;
		background: var(--surface-3);
		overflow: hidden;
	}
	.fill {
		display: block;
		height: 100%;
		background: var(--accent);
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}
	th,
	td {
		text-align: left;
		padding: 0.25rem 0.3rem;
		border-bottom: 1px solid var(--surface-3);
	}
	th {
		color: var(--muted);
		font-weight: 600;
	}
	th.count,
	td.count {
		text-align: right;
	}

	.stamp {
		margin: 0.75rem 0 0;
		font-size: 0.75rem;
		color: var(--muted-2);
	}
</style>
