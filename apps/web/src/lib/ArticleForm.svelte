<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount, type Snippet } from 'svelte';
	import {
		api,
		errorText,
		emptyPyro,
		type Article,
		type ArticleSupplier,
		type Language,
		type ArticleGroup
	} from './api';
	import { confirmCancel } from './confirm.svelte';
	import { decimalText, decimalValue } from './format';
	import StockLevel from './StockLevel.svelte';
	import ArticleRental from '$modules/where-does-my-stuff-go/web/ArticleRental.svelte';
	import ArticleSale from '$modules/where-does-my-stuff-go/web/ArticleSale.svelte';
	import ArticlePicture from './articles/ArticlePicture.svelte';
	import { modules } from './modules.svelte';

	interface Props {
		article: Article;
		article_groups: ArticleGroup[];
		languages: Language[];
		busy?: boolean;
		/** Without write permission all fields stay locked and `Sichern` is missing. */
		schreibbar?: boolean;
		/** In the window (`ArticleDialog`): no own frame, the content scrolls. */
		imFenster?: boolean;
		/** Optional content on the left of the footer (e.g. the file storage). */
		actions?: Snippet;
		onsave: (article: Article) => void | Promise<void>;
		oncancel: () => void;
	}

	let {
		article: initial,
		article_groups,
		languages,
		busy = false,
		schreibbar = true,
		imFenster = false,
		actions,
		onsave,
		oncancel
	}: Props = $props();

	/**
	 * Fields the database keeps as `numeric`. The mask shows them with a comma and
	 * without trailing zeros, `toApiDecimals` turns them back before saving.
	 */
	const DECIMAL_FIELDS = [
		'retail_price',
		'insurance_value_net',
		'insurance_value_gross',
		'width_net',
		'height_net',
		'depth_net',
		'volume_net',
		'weight_net',
		'width_gross',
		'height_gross',
		'depth_gross',
		'volume_gross',
		'weight_gross',
		'voltage_from',
		'voltage_to',
		'current_draw',
		'power',
		'workshop_max_usage_days'
	] as const;

	const PYRO_DECIMALS = [
		'lateral_safety_distance',
		'vertical_safety_distance',
		'ascending_height',
		'sound_pressure_level',
		'sound_pressure_range',
		'delay_time',
		'effect_duration',
		'neq',
		'un_points'
	] as const;

	const PRICE_DECIMALS = [
		'rental_price_per_day',
		'rental_base_price',
		'rental_cost_price',
		'rental_price_per_day_roll_days',
		'rental_base_price_roll_days',
		'rental_cost_price_roll_days',
		'rental_price_per_day_off_days',
		'rental_base_price_off_days',
		'rental_cost_price_off_days',
		'rental_price_weekend',
		'rental_cost_price_weekend',
		'deposit',
		'used_sale_price',
		'capital_binding',
		'sale_price_net',
		'sale_price_gross',
		'sale_retail_price',
		'sale_cost_price_net',
		'sale_markup_percent',
		'package_size',
		'reorder_level'
	] as const;

	/** Older answers do not know the tabs „Lieferanten"/„Pyro" yet. */
	function normalize(article: Article): Article {
		article.suppliers ??= [];
		article.pyro ??= emptyPyro();
		for (const key of DECIMAL_FIELDS) article[key] = decimalText(article[key]);
		for (const s of article.suppliers) s.purchase_price = decimalText(s.purchase_price);
		if (article.pyro) {
			const pyro = article.pyro as unknown as Record<string, string | null>;
			for (const key of PYRO_DECIMALS) pyro[key] = decimalText(pyro[key]);
		}
		if (article.pricing) {
			const pricing = article.pricing as unknown as Record<string, string | null>;
			for (const key of PRICE_DECIMALS) pricing[key] = decimalText(pricing[key]);
		}
		return article;
	}

	/** Back to what the API expects: the mask holds `11,4`, the column wants `11.4`. */
	function toApiDecimals(article: Article): Article {
		for (const key of DECIMAL_FIELDS) article[key] = decimalValue(article[key]);
		for (const s of article.suppliers) s.purchase_price = decimalValue(s.purchase_price);
		if (article.pyro) {
			const pyro = article.pyro as unknown as Record<string, string | null>;
			for (const key of PYRO_DECIMALS) pyro[key] = decimalValue(pyro[key]);
		}
		if (article.pricing) {
			const pricing = article.pricing as unknown as Record<string, string | null>;
			for (const key of PRICE_DECIMALS) pricing[key] = decimalValue(pricing[key]);
		}
		return article;
	}

	// Local copy: changes must not take effect outside until saved.
	// svelte-ignore state_referenced_locally
	let a = $state<Article>(normalize(JSON.parse(JSON.stringify(initial)) as Article));

	let textTab = $state<'kurz' | 'lang' | 'tech' | 'liefer' | 'pack' | 'webshop'>('kurz');
	let lowerTab = $state<'masse' | 'warengruppe' | 'dguv'>('masse');
	let tab = $state<'artikel' | 'miete' | 'verkauf' | 'lagerbestand' | 'lieferanten' | 'pyro'>('artikel');

	/**
	 * Rental and sale prices belong to `WHERE - does my stuff go?`. Without the
	 * module the tabs and the two category boxes are missing; existing legacy data
	 * stays untouched.
	 */
	const pricingEnabled = $derived(modules.isInstalled('go'));

	// Rental and sale are only accessible while the article is marked that way.
	$effect(() => {
		if (!pricingEnabled && (tab === 'miete' || tab === 'verkauf')) tab = 'artikel';
		if (tab === 'miete' && !a.rental) tab = 'artikel';
		if (tab === 'verkauf' && !a.sale) tab = 'artikel';
	});

	// svelte-ignore state_referenced_locally
	const istNeu = initial.id === null;

	function toggleArticleGroup(id: number) {
		if (a.article_groups.includes(id)) {
			a.article_groups = a.article_groups.filter((g) => g !== id);
		} else {
			a.article_groups = [...a.article_groups, id];
		}
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		// A copy: the mask keeps its German text (`11,4`) while the copy goes to the API.
		const copy = JSON.parse(JSON.stringify(a)) as Article;
		onsave(toApiDecimals(copy));
	}

	// --------------------------------------------------------- Tab „Lieferanten"
	/** Supplier addresses for the selection; `null` = not loaded yet. */
	let supplierChoice = $state<{ id: number; name: string; no: number | null }[] | null>(null);
	let selectedSupplier = $state<string>('');
	let supplierError = $state<string | null>(null);

	/** Manufacturers from the settings list (`Admin → Artikel → Hersteller`). */
	let manufacturer = $state<string[]>([]);
	onMount(async () => {
		try {
			const list = await api.settings.list('hersteller');
			manufacturer = list.rows
				.map((row) => row.values.Manufacturer?.trim())
				.filter((value): value is string => !!value);
		} catch {
			// Without a list the field stays a free text field.
		}
	});

	async function loadSuppliers() {
		if (supplierChoice) return;
		try {
			const page = await api.addresses.search({ address_type: 'suppliers', limit: 500 });
			supplierChoice = page.items.map((x) => ({
				id: x.id,
				name: x.name1 ?? `#${x.id}`,
				no: x.customer_no ?? x.supplier_no ?? null
			}));
		} catch (e) {
			supplierError = errorText(e);
		}
	}

	function addSupplier() {
		const id = Number(selectedSupplier);
		if (!Number.isInteger(id) || id <= 0) return;
		if (a.suppliers.some((s) => s.supplier_id === id)) {
			supplierError = t('Dieser Lieferant steht bereits in der Liste.');
			return;
		}
		const gewaehlt = supplierChoice?.find((x) => x.id === id);
		const createNew: ArticleSupplier = {
			supplier_id: id,
			supplier_no: gewaehlt?.no ?? null,
			name: gewaehlt?.name ?? null,
			position: a.suppliers.length + 1,
			purchase_price: null,
			delivery_days: null,
			favorite: false,
			sales_article: false,
			supplier_article_no: null
		};
		a.suppliers = [...a.suppliers, createNew];
		selectedSupplier = '';
		supplierError = null;
	}

	function removeSupplier(index: number) {
		a.suppliers = a.suppliers.filter((_, i) => i !== index);
	}

	const textFields = {
		short: t('Kurz'),
		long: t('Lang'),
		tech: t('Tech. Details'),
		liefer: t('Lieferumfang'),
		pack: t('Packinfo'),
		webshop: t('WebShop')
	} as const;
</script>

<form onsubmit={submit} class="mask" class:in-window={imFenster}>
	<header class="title-bar">
		<div>
			<h1>{t('Artikel')}</h1>
			<span class="number">{a.article_no} /</span>
		</div>
		<div class="meta">
			{#if istNeu}
				<span class="new">{t('neuer Artikel')}</span>
			{:else}
				<span>{t('Erstellt:')} {a.erstellt_am ? new Date(a.erstellt_am).toLocaleString('de-DE') : '—'}</span>
				<span>{t('Letzte Änderung:')} {a.changed_at ? new Date(a.changed_at).toLocaleString('de-DE') : '—'}</span>
			{/if}
		</div>
	</header>

	<nav class="tabs" aria-label={t('Artikelbereiche')}>
		<button type="button" class="tab" class:active={tab === 'artikel'} onclick={() => (tab = 'artikel')}>
			{t('Artikel')}
		</button>
		{#if pricingEnabled}
			<button
				type="button"
				class="tab"
				class:active={tab === 'miete'}
				disabled={!a.rental}
				title={a.rental ? t('Miete') : t('Artikel ist nicht als Miete markiert')}
				onclick={() => (tab = 'miete')}
			>
				{t('Miete')}
			</button>
			<button
				type="button"
				class="tab"
				class:active={tab === 'verkauf'}
				disabled={!a.sale}
				title={a.sale ? t('Verkauf') : t('Artikel ist nicht als Verkauf markiert')}
				onclick={() => (tab = 'verkauf')}
			>
				{t('Verkauf')}
			</button>
		{/if}
		<button
			type="button"
			class="tab"
			class:active={tab === 'lagerbestand'}
			onclick={() => (tab = 'lagerbestand')}
		>
			{t('Lagerbestand')}
		</button>
		<button
			type="button"
			class="tab"
			class:active={tab === 'lieferanten'}
			onclick={() => (tab = 'lieferanten')}
		>
			{t('Lieferanten')}
		</button>
		<button type="button" class="tab" class:active={tab === 'pyro'} onclick={() => (tab = 'pyro')}>
			{t('Pyro')}
		</button>
	</nav>

	<!-- Without write permission all fields of the tabs are locked; the tabs
	     themselves stay clickable, so the article can be read. -->
	{#if !schreibbar}
		<p class="read-only">
			{t('Nur Lesen — zum Bearbeiten dieses Artikels fehlt das Recht')}
			<strong>{t('Artikel editieren')}</strong>.
		</p>
	{/if}
	<!-- The wrapper scrolls, not the fieldset: Chromium makes a `fieldset` no real
	     scroll container, so its content is cut off without a bar. -->
	<div class="content-scroll">
	<fieldset class="content" disabled={!schreibbar}>
	{#if tab === 'lagerbestand'}
		<div class="tab-content">
			<StockLevel
				articleId={a.id}
				articleNo={a.article_no}
				articleUsage={{ inventory: a.inventory, rental: pricingEnabled && a.rental, sale: pricingEnabled && a.sale }}
				articleSwitches={{
					stock_tracking: a.stock_tracking,
					no_own_stock: a.no_own_stock,
					individual_items: a.individual_items
				}}
			/>
		</div>
	{:else if tab === 'miete'}
		<ArticleRental bind:pricing={a.pricing} />
	{:else if tab === 'verkauf'}
		<ArticleSale bind:pricing={a.pricing} />
		{:else if tab === 'lieferanten'}
			<div class="tab-content suppliers">
				<table class="table">
					<thead>
						<tr>
							<th>{t('Nr.')}</th>
							<th>{t('Lieferant')}</th>
							<th class="num">{t('EK-Preis')}</th>
							<th class="num">{t('Lieferzeit (Tage)')}</th>
							<th>{t('Favorit')}</th>
							<th>{t('Verkaufsartikel')}</th>
							<th>{t('Art.-Lief.-Nr.')}</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each a.suppliers as s, i (s.supplier_id)}
							<tr>
								<td class="number">{s.supplier_no ?? '—'}</td>
								<td>{s.name ?? `#${s.supplier_id}`}</td>
								<td class="num">
									<input
										class="num-field"
										inputmode="decimal"
										placeholder="0,00"
										bind:value={s.purchase_price}
										aria-label={t('EK-Preis')}
									/>
								</td>
								<td class="num">
									<input
										class="num-field"
										type="number"
										min="0"
										bind:value={s.delivery_days}
										aria-label={t('Lieferzeit in Tagen')}
									/>
								</td>
								<td class="center">
									<input type="checkbox" bind:checked={s.favorite} aria-label={t('Favorit')} />
								</td>
								<td class="center">
									<input
										type="checkbox"
										bind:checked={s.sales_article}
										aria-label={t('Verkaufsartikel')}
									/>
								</td>
								<td>
									<input bind:value={s.supplier_article_no} aria-label={t('Artikelnummer des Lieferanten')} />
								</td>
								<td>
									<button type="button" class="remove" title={t('Lieferant entfernen')} onclick={() => removeSupplier(i)}>
										×
									</button>
								</td>
							</tr>
						{:else}
							<tr><td colspan="8">{t('Noch kein Lieferant zugeordnet.')}</td></tr>
						{/each}
					</tbody>
				</table>

				<div class="row add">
					<select bind:value={selectedSupplier} onfocus={loadSuppliers} aria-label={t('Lieferant wählen')}>
						<option value="">{t('— Lieferant wählen —')}</option>
						{#each supplierChoice ?? [] as l (l.id)}
							<option value={String(l.id)}>{l.no ? `${l.no} — ` : ''}{l.name}</option>
						{/each}
					</select>
					<button type="button" onclick={addSupplier} disabled={!selectedSupplier}>
						{t('Hinzufügen')}
					</button>
				</div>
				{#if supplierError}
					<p class="error">{supplierError}</p>
				{/if}
			</div>
		{:else if tab === 'pyro'}
			<div class="tab-content pyro">
				<div class="columns">
					<section class="left">
						<fieldset>
							<legend>{t('Stoff und Einstufung')}</legend>
							<label class="field row"><span>{t('UN-Nr.')}</span><input bind:value={a.pyro.un_number} /></label>
							<label class="field row"><span>{t('UN-Bezeichnung')}</span><input bind:value={a.pyro.un_description} /></label>
							<label class="field row"><span>{t('Gefahrklasse')}</span><input bind:value={a.pyro.hazard_class} placeholder={t('z. B. 1.4G')} /></label>
							<label class="field row"><span>{t('Kaliber (mm)')}</span><input type="number" min="0" bind:value={a.pyro.caliber} /></label>
							<label class="field row"><span>{t('Kategorie')}</span><input bind:value={a.pyro.category} /></label>
							<label class="field row"><span>{t('Klassifizierung')}</span><input bind:value={a.pyro.classification} /></label>
							<label class="field row"><span>{t('BAM-ID')}</span><input bind:value={a.pyro.bam_id} /></label>
							<label class="field row"><span>{t('Zulassungsnummer')}</span><input bind:value={a.pyro.approval_number} /></label>
						</fieldset>

						<fieldset>
							<legend>{t('Transport')}</legend>
							<label class="field row"><span>{t('Verpackungsgruppe')}</span><input bind:value={a.pyro.packing_group} /></label>
							<label class="field row"><span>{t('Transportkategorie')}</span><input bind:value={a.pyro.transport_category} /></label>
							<label class="field row"><span>{t('Tunnelbeschränkung')}</span><input bind:value={a.pyro.tunnel_restriction_code} /></label>
							<label class="field row">
								<span>{t('NEQ (netto)')}</span>
								<span class="double-field">
									<input inputmode="decimal" bind:value={a.pyro.neq} />
									<input class="unit" bind:value={a.pyro.neq_unit} placeholder={t('kg')} aria-label={t('Einheit der NEQ')} />
								</span>
							</label>
							<label class="field row"><span>{t('UN-Punkte')}</span><input inputmode="decimal" bind:value={a.pyro.un_points} /></label>
						</fieldset>
					</section>

					<section class="right">
						<fieldset>
							<legend>{t('Sicherheitsabstände (m)')}</legend>
							<label class="field row"><span>{t('Seitlich')}</span><input inputmode="decimal" bind:value={a.pyro.lateral_safety_distance} /></label>
							<label class="field row"><span>{t('Vertikal')}</span><input inputmode="decimal" bind:value={a.pyro.vertical_safety_distance} /></label>
							<label class="field row"><span>{t('Steighöhe')}</span><input inputmode="decimal" bind:value={a.pyro.ascending_height} /></label>
						</fieldset>

						<fieldset>
							<legend>{t('Wirkung')}</legend>
							<label class="field row"><span>{t('Schalldruckpegel (dB)')}</span><input inputmode="decimal" bind:value={a.pyro.sound_pressure_level} /></label>
							<label class="field row"><span>{t('Schalldruckbereich (m)')}</span><input inputmode="decimal" bind:value={a.pyro.sound_pressure_range} /></label>
							<label class="field row"><span>{t('Verzögerungszeit (s)')}</span><input inputmode="decimal" bind:value={a.pyro.delay_time} /></label>
							<label class="field row"><span>{t('Effektdauer (s)')}</span><input inputmode="decimal" bind:value={a.pyro.effect_duration} /></label>
						</fieldset>
					</section>
				</div>
			</div>
		{:else}
	<div class="columns">
		<!-- ------------------------------------------------------ left column -->
		<section class="left">
			<div class="header-row">
				<label class="field short">
					<span>{t('Artikel Nr.')}</span>
					<input type="number" bind:value={a.article_no} min="0" />
				</label>
				<label class="field short">
					<span>{t('Inventar Nr.')}</span>
					<input bind:value={a.inventory_no} />
				</label>
				<label class="field short">
					<span>{t('Sprache')}</span>
					<select>
						{#each languages as s (s.id)}
							<option value={s.id}>{s.language}</option>
						{/each}
					</select>
				</label>
			</div>

			<label class="field"><span>{t('Name')} <em>*</em></span><input bind:value={a.name} required /></label>
			<label class="field"><span>{t('Kurzname')}</span><input bind:value={a.short_name} /></label>
			<label class="field"><span>{t('Matchcode')}</span><input bind:value={a.match_code} /></label>
			<label class="field"><span>{t('Einheit')}</span><input bind:value={a.unit} /></label>

			<div class="block">
				<nav class="tabs small" aria-label={t('Textblöcke')}>
					{#each Object.entries(textFields) as [key, label] (key)}
						<button
							type="button"
							class="tab"
							class:active={textTab === key}
							onclick={() => (textTab = key as typeof textTab)}>{label}</button
						>
					{/each}
				</nav>
				{#if textTab === 'kurz'}
					<textarea bind:value={a.texts.short} rows="9"></textarea>
				{:else if textTab === 'lang'}
					<textarea bind:value={a.texts.long} rows="9"></textarea>
				{:else if textTab === 'tech'}
					<textarea bind:value={a.texts.technical_details} rows="9"></textarea>
				{:else if textTab === 'liefer'}
					<textarea bind:value={a.texts.scope_of_delivery} rows="9"></textarea>
				{:else if textTab === 'pack'}
					<textarea bind:value={a.texts.packaging_info} rows="9"></textarea>
				{:else}
					<textarea bind:value={a.texts.webshop} rows="9"></textarea>
				{/if}
			</div>

			<div class="block">
				<nav class="tabs small" aria-label={t('Artikeldetails')}>
					<button type="button" class="tab" class:active={lowerTab === 'masse'} onclick={() => (lowerTab = 'masse')}>{t('Maße & Werte')}</button>
					<button type="button" class="tab" class:active={lowerTab === 'warengruppe'} onclick={() => (lowerTab = 'warengruppe')}>{t('Warengruppe')}</button>
					<button type="button" class="tab" class:active={lowerTab === 'dguv'} onclick={() => (lowerTab = 'dguv')}>{t('DGUV')}</button>
				</nav>

				{#if lowerTab === 'masse'}
					<div class="dimensions">
						<div class="group">
							<h3>{t('netto')}</h3>
							<div class="triple">
								<label class="field"><span>{t('B (cm)')}</span><input bind:value={a.width_net} inputmode="decimal" /></label>
								<label class="field"><span>{t('H (cm)')}</span><input bind:value={a.height_net} inputmode="decimal" /></label>
								<label class="field"><span>{t('T (cm)')}</span><input bind:value={a.depth_net} inputmode="decimal" /></label>
							</div>
							<label class="field"><span>{t('Volumen (m³)')}</span><input bind:value={a.volume_net} inputmode="decimal" /></label>
							<label class="field"><span>{t('Gewicht (kg)')}</span><input bind:value={a.weight_net} inputmode="decimal" /></label>
						</div>
						<div class="group">
							<h3>{t('brutto')}</h3>
							<div class="triple">
								<label class="field"><span>{t('B (cm)')}</span><input bind:value={a.width_gross} inputmode="decimal" /></label>
								<label class="field"><span>{t('H (cm)')}</span><input bind:value={a.height_gross} inputmode="decimal" /></label>
								<label class="field"><span>{t('T (cm)')}</span><input bind:value={a.depth_gross} inputmode="decimal" /></label>
							</div>
							<label class="field"><span>{t('Volumen (m³)')}</span><input bind:value={a.volume_gross} inputmode="decimal" /></label>
							<label class="field"><span>{t('Gewicht (kg)')}</span><input bind:value={a.weight_gross} inputmode="decimal" /></label>
						</div>
						<div class="group wide">
							<h3>{t('Betrieb')}</h3>
							<div class="pair">
								<label class="field"><span>{t('Betriebsspannung von (V)')}</span><input bind:value={a.voltage_from} inputmode="decimal" /></label>
								<label class="field"><span>{t('bis (V)')}</span><input bind:value={a.voltage_to} inputmode="decimal" /></label>
							</div>
							<div class="pair">
								<label class="field"><span>{t('Stromaufnahme (A)')}</span><input bind:value={a.current_draw} inputmode="decimal" /></label>
								<label class="field"><span>{t('elektr. Leistung (kW)')}</span><input bind:value={a.power} inputmode="decimal" /></label>
							</div>
							<div class="pair">
								<label class="field"><span>{t('Aufbauzeit (min)')}</span><input type="number" bind:value={a.setup_minutes} min="0" /></label>
								<label class="field"><span>{t('Abbauzeit (min)')}</span><input type="number" bind:value={a.teardown_minutes} min="0" /></label>
							</div>
							<div class="pair">
								<label class="field"><span>{t('Check-Out-Zeit (sek)')}</span><input type="number" bind:value={a.check_out_seconds} min="0" /></label>
								<label class="field"><span>{t('Check-In-Zeit (sek)')}</span><input type="number" bind:value={a.check_in_seconds} min="0" /></label>
							</div>
						</div>
					</div>
				{:else if lowerTab === 'warengruppe'}
					<div class="article-groups">
						{#each article_groups as w (w.id)}
							<label class="check">
								<input
									type="checkbox"
									checked={a.article_groups.includes(w.id)}
									onchange={() => toggleArticleGroup(w.id)}
								/>
								{w.name ?? `Gruppe ${w.id}`}
							</label>
						{:else}
							<p class="notice">{t('Keine Warengruppen angelegt.')}</p>
						{/each}
					</div>
				{:else if lowerTab === 'dguv'}
					<div class="dguv">
						<div class="pair">
							<label class="field"><span>{t('Schutzklasse')}</span><input type="number" bind:value={a.inspection_class} min="0" max="3" /></label>
							<label class="field"><span>{t('Prüfintervall (Monate)')}</span><input type="number" bind:value={a.inspection_interval_months} min="0" /></label>
							<label class="field"><span>{t('DGUV-Prüfzeit (min)')}</span><input type="number" bind:value={a.inspection_minutes} min="0" /></label>
						</div>
						<h3>{t('Werkstattintervalle')}</h3>
						<label class="check"><input type="checkbox" bind:checked={a.workshop_by_usage_days} /> {t('nach Einsatztagen')}</label>
						<label class="field narrow"><span>{t('Max.')}</span><input bind:value={a.workshop_max_usage_days} inputmode="decimal" /></label>
						<label class="check"><input type="checkbox" bind:checked={a.workshop_fixed_intervals} /> {t('nach festen Intervallen')}</label>
						<label class="field narrow">
							<span>{t('Einheit')}</span>
							<select
								value={a.workshop_by_months ? 'monate' : 'tage'}
								onchange={(e) => {
									const v = e.currentTarget.value;
									a.workshop_by_days = v === 'tage';
									a.workshop_by_months = v === 'monate';
								}}
							>
								<option value="tage">{t('nach Tagen')}</option>
								<option value="monate">{t('nach Monaten')}</option>
							</select>
						</label>
						<div class="pair">
							<label class="field narrow"><span>{t('Tage')}</span><input type="number" bind:value={a.workshop_days} min="0" /></label>
							<label class="field narrow"><span>{t('Monate')}</span><input type="number" bind:value={a.workshop_months} min="0" /></label>
						</div>
					</div>
				{/if}
			</div>
		</section>

		<!-- ----------------------------------------------------- right column -->
		<section class="right">
			<!-- The article picture stands above `Kategorie`; a click opens it in full size. -->
			<ArticlePicture articleId={a.id} {schreibbar} />

			<fieldset>
				<legend>{t('Kategorie')}</legend>
				<label class="check"><input type="checkbox" bind:checked={a.active} /> {t('Artikel aktiv')}</label>
				<div class="pair">
					<label class="check"><input type="checkbox" bind:checked={a.inventory} /> {t('Inventar')}</label>
					{#if pricingEnabled}
						<label class="check"><input type="checkbox" bind:checked={a.rental} /> {t('Miete')}</label>
					{/if}
				</div>
				<div class="pair">
					{#if pricingEnabled}
						<label class="check"><input type="checkbox" bind:checked={a.sale} /> {t('Verkauf')}</label>
					{/if}
					<label class="check"><input type="checkbox" bind:checked={a.text_sync} /> {t('Textsync')}</label>
				</div>
			</fieldset>

			<fieldset>
				<legend>{t('Bestandsführung')}</legend>
				<label class="check"><input type="checkbox" bind:checked={a.stock_tracking} /> {t('Aktiv')}</label>
				<div class="pair">
					<label class="check"><input type="checkbox" bind:checked={a.no_own_stock} /> {t('ohne Eigenbestand')}</label>
					<label class="check"><input type="checkbox" bind:checked={a.individual_items} /> {t('mit Einzelerfassung')}</label>
				</div>
				<div class="pair">
					<label class="check"><input type="checkbox" bind:checked={a.allocation_control} /> {t('Dispositionskontrolle')}</label>
					<label class="check"><input type="checkbox" bind:checked={a.virtual_article} /> {t('virtueller Artikel')}</label>
				</div>
			</fieldset>

			<fieldset>
				<legend>{t('Herstellerangaben')}</legend>
				<label class="field row"><span>{t('EAN-Code')}</span><input bind:value={a.ean_code} /></label>
				<label class="field row"><span>{t('Herst.-Art.-Nr.')}</span><input bind:value={a.manufacturer_no} /></label>
				<label class="field row"><span>{t('Hersteller')}</span><input bind:value={a.manufacturer} list="hersteller-liste" /></label>
				<datalist id="hersteller-liste">
					{#each manufacturer as name (name)}<option value={name}></option>{/each}
				</datalist>
				<label class="field row"><span>{t('Herstellerland')}</span><input bind:value={a.manufacturer_country} /></label>
				<label class="field row"><span>{t('UVP / EVP brutto')}</span><input bind:value={a.retail_price} inputmode="decimal" placeholder="0,00" /></label>
				<label class="field row"><span>{t('Zolltarif-Nr. Export')}</span><input bind:value={a.customs_tariff_export} /></label>
				<label class="field row"><span>{t('Zolltarif-Nr. Import')}</span><input bind:value={a.customs_tariff_import} /></label>
			</fieldset>

			<fieldset>
				<legend>{t('weitere Informationen')}</legend>
				<label class="field row"><span>{t('Versicherungswert netto')}</span><input bind:value={a.insurance_value_net} inputmode="decimal" placeholder="0,00" /></label>
				<label class="field row"><span>{t('Versicherungswert brutto')}</span><input bind:value={a.insurance_value_gross} inputmode="decimal" placeholder="0,00" /></label>
				<label class="field row"><span>{t('TeamAgenda-Gruppe')}</span><input bind:value={a.team_group} /></label>
				<label class="field row"><span>{t('Provisionsklasse')}</span><input bind:value={a.commission_class} placeholder={t('-- Keine Angabe --')} /></label>
			</fieldset>
		</section>
	</div>
	{/if}
	</fieldset>
	</div>

	<footer class="actions">
		<div class="actions-left">
			{#if actions}
				{@render actions()}
			{/if}
		</div>
		<div class="actions-right">
			<button
				type="button"
				class="secondary"
				onclick={confirmCancel(t('Das Bearbeiten des Artikels'), oncancel)}
				disabled={busy}>{schreibbar ? t('Abbrechen') : t('Schließen')}</button
			>
			{#if schreibbar}
				<button type="submit" class="primary" disabled={busy}
					>{busy ? t('Sichern …') : t('Sichern')}</button
				>
			{/if}
		</div>
	</footer>
</form>

<style>
	/* The fieldset locks the fields, but brings no look of its own. */
	.content {
		border: 0;
		margin: 0;
		padding: 0;
		min-inline-size: 0;
	}
	/* Locked fields shall also look that way, not only not react. */
	.content[disabled] {
		opacity: 0.66;
	}
	/* In the window the content of the tabs scrolls, not the whole window.
	   The scroller is the wrapper, not the fieldset: Chromium gives a `fieldset`
	   no scrollbar and keeps its `scrollTop` at 0, so the content would be cut. */
	.mask.in-window .content-scroll {
		flex: 1 1 auto;
		min-height: 0;
		overflow: auto;
	}
	.read-only {
		margin: 0;
		padding: 0.45rem 1rem;
		border-bottom: 1px solid var(--notice-border);
		background: var(--notice-bg);
		color: var(--muted);
		font-size: 0.8rem;
	}

	.mask {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		overflow: hidden;
	}
	/* In the window the frame (`article-window.css`) brings the surface with it. */
	.mask.in-window {
		border: 0;
		border-radius: 0;
		background: transparent;
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
		min-height: 0;
	}
	/* The head of the mask is, in the window, the band under the menu bar. */
	.mask.in-window .title-bar {
		align-items: center;
		padding: 0.3rem 0.6rem;
	}
	.mask.in-window .title-bar h1 {
		font-size: 1.02rem;
		font-weight: 700;
	}
	.mask.in-window .number {
		font-size: 0.82rem;
		font-variant-numeric: tabular-nums;
	}
	.title-bar {
		display: flex;
		flex: 0 0 auto;
		justify-content: space-between;
		align-items: flex-end;
		gap: 1rem;
		padding: 0.75rem 1rem;
		background: var(--menu-band-2);
		color: var(--text);
		border-bottom: 1px solid var(--menu-band-border);
	}
	.title-bar h1 {
		font-size: 1.6rem;
		margin: 0;
		display: inline;
	}
	.number {
		font-size: 1.2rem;
		margin-left: 0.75rem;
		color: var(--muted);
	}
	.meta {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		font-size: 0.75rem;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}
	.meta .new {
		font-weight: 600;
		color: var(--accent);
	}
	.tabs {
		display: flex;
		flex: 0 0 auto;
		gap: 0.25rem;
		padding: 0.4rem 1rem 0;
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
		flex-wrap: wrap;
	}
	.tabs.small {
		padding: 0 0 0.25rem;
		background: transparent;
		border-bottom: 1px solid var(--surface-3);
		margin-bottom: 0.4rem;
	}
	.tab {
		padding: 0.3rem 0.7rem;
		font-size: 0.82rem;
		border: 1px solid transparent;
		border-bottom: none;
		border-radius: 4px 4px 0 0;
		background: transparent;
		color: var(--muted);
		cursor: default;
		font-family: inherit;
	}
	button.tab {
		cursor: pointer;
	}
	button.tab:disabled {
		color: var(--muted-2);
		cursor: default;
		text-decoration: line-through;
	}
	.tab.active {
		background: var(--surface);
		border-color: var(--border);
		color: var(--text);
		font-weight: 600;
	}
	.columns {
		display: grid;
		grid-template-columns: minmax(0, 1.55fr) minmax(0, 1fr);
		gap: 1.25rem;
		padding: 1rem;
	}
	.tab-content {
		padding: 1rem;
	}
	@media (max-width: 900px) {
		.columns {
			grid-template-columns: 1fr;
		}
	}
	.header-row {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0.6rem;
		margin-bottom: 0.6rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.78rem;
		margin-bottom: 0.5rem;
	}
	.field span {
		color: var(--muted);
	}
	.field em {
		color: var(--bad);
		font-style: normal;
	}
	.field.row {
		display: grid;
		grid-template-columns: 11rem minmax(0, 1fr);
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.35rem;
	}
	.field.narrow {
		max-width: 8rem;
	}
	input,
	select,
	textarea {
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font-family: inherit;
		font-size: 0.85rem;
		width: 100%;
		box-sizing: border-box;
	}
	textarea {
		resize: vertical;
	}
	.block {
		margin-top: 0.75rem;
	}
	.dimensions {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem;
	}
	.dimensions .wide {
		grid-column: 1 / -1;
	}
	.group {
		border: 1px solid var(--surface-3);
		border-radius: 4px;
		padding: 0.5rem;
	}
	.group h3 {
		margin: 0 0 0.4rem;
		font-size: 0.8rem;
		color: var(--accent);
	}
	.triple {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0.4rem;
	}
	.pair {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.4rem;
	}
	fieldset {
		border: 1px solid var(--surface-3);
		border-radius: 4px;
		padding: 0.5rem 0.6rem 0.7rem;
		margin: 0 0 0.75rem;
	}
	legend {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--accent);
		padding: 0 0.3rem;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
		margin: 0.2rem 0;
	}
	.check input {
		width: auto;
	}
	.article-groups {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.notice {
		font-size: 0.8rem;
		color: var(--muted);
	}

	/* Tab „Lieferanten" */
	.table {
		width: 100%;
		border-collapse: collapse;
		margin: 0.5rem 0 0.75rem;
	}
	.table th,
	.table td {
		text-align: left;
		padding: 0.3rem 0.4rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.85rem;
	}
	.table th {
		font-weight: 600;
	}
	.table .num {
		text-align: right;
	}
	.table .center {
		text-align: center;
	}
	.table td.number {
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.table input:not([type='checkbox']) {
		width: 100%;
		padding: 0.2rem 0.3rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
	}
	.table .num-field {
		max-width: 7rem;
		text-align: right;
	}
	.remove {
		border: none;
		background: none;
		color: var(--bad);
		font-size: 1rem;
		cursor: pointer;
		padding: 0 0.25rem;
	}
	.add {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}
	.add select {
		min-width: 18rem;
		padding: 0.25rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
	}
	.add button {
		padding: 0.25rem 0.8rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-3);
		color: var(--text);
		cursor: pointer;
	}
	.add button:disabled {
		opacity: 0.55;
		cursor: default;
	}
	.error {
		color: var(--bad);
		font-size: 0.8rem;
	}

	/* Tab „Pyro": two columns with fields */
	.pyro .columns {
		padding: 0;
	}
	.pyro .unit {
		max-width: 4.5rem;
	}
	.double-field {
		display: flex;
		gap: 0.3rem;
	}
	.double-field .unit {
		flex: 0 0 4.5rem;
	}
	.actions {
		display: flex;
		flex: 0 0 auto;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem 1rem;
		background: var(--surface-2);
		border-top: 1px solid var(--border);
	}
	.actions button {
		padding: 0.45rem 1.1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
		font-family: inherit;
	}
	.actions button.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--surface);
		font-weight: 600;
	}
	.actions button:disabled {
		opacity: 0.6;
		cursor: default;
	}
</style>
