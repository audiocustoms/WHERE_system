<script lang="ts">
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

	// Local copy: changes must not take effect outside until saved.
	// svelte-ignore state_referenced_locally
	let a = $state<Article>(normalize(JSON.parse(JSON.stringify(initial)) as Article));

	/** Older answers do not know the tabs „Lieferanten"/„Pyro" yet. */
	function normalize(article: Article): Article {
		article.suppliers ??= [];
		article.pyro ??= emptyPyro();
		// Make the decimal strings of the API („2574.0000") readable, the value stays the same.
		for (const s of article.suppliers) s.purchase_price = shortNumber(s.purchase_price);
		const p = article.pyro as unknown as Record<string, unknown>;
		for (const key of [
			'lateral_safety_distance',
			'vertical_safety_distance',
			'ascending_height',
			'sound_pressure_level',
			'sound_pressure_range',
			'delay_time',
			'effect_duration',
			'neq',
			'un_points'
		]) {
			if (typeof p[key] === 'string') p[key] = shortNumber(p[key]);
		}
		return article;
	}

	/** „2574.0000" → „2574", „1.2500" → „1.25"; unknown texts stay. */
	function shortNumber(value: string | null): string | null {
		if (value === null) return null;
		const n = Number(value);
		if (!Number.isFinite(n)) return value;
		return String(n);
	}

	let textTab = $state<'kurz' | 'lang' | 'tech' | 'liefer' | 'pack' | 'webshop'>('kurz');
	let lowerTab = $state<'masse' | 'warengruppe' | 'dguv' | 'rest'>('masse');
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
		onsave(a);
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
			supplierError = 'Dieser Lieferant steht bereits in der Liste.';
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
		short: 'Kurz',
		long: 'Lang',
		tech: 'Tech. Details',
		liefer: 'Lieferumfang',
		pack: 'Packinfo',
		webshop: 'WebShop'
	} as const;
</script>

<form onsubmit={submit} class="mask" class:in-window={imFenster}>
	<header class="title-bar">
		<div>
			<h1>Artikel</h1>
			<span class="number">{a.article_no} /</span>
		</div>
		<div class="meta">
			{#if istNeu}
				<span class="new">neuer Artikel</span>
			{:else}
				<span>Erstellt: {a.erstellt_am ? new Date(a.erstellt_am).toLocaleString('de-DE') : '—'}</span>
				<span>Letzte Änderung: {a.changed_at ? new Date(a.changed_at).toLocaleString('de-DE') : '—'}</span>
			{/if}
		</div>
	</header>

	<nav class="tabs" aria-label="Artikelbereiche">
		<button type="button" class="tab" class:active={tab === 'artikel'} onclick={() => (tab = 'artikel')}>
			Artikel
		</button>
		{#if pricingEnabled}
			<button
				type="button"
				class="tab"
				class:active={tab === 'miete'}
				disabled={!a.rental}
				title={a.rental ? 'Miete' : 'Artikel ist nicht als Miete markiert'}
				onclick={() => (tab = 'miete')}
			>
				Miete
			</button>
			<button
				type="button"
				class="tab"
				class:active={tab === 'verkauf'}
				disabled={!a.sale}
				title={a.sale ? 'Verkauf' : 'Artikel ist nicht als Verkauf markiert'}
				onclick={() => (tab = 'verkauf')}
			>
				Verkauf
			</button>
		{/if}
		<button
			type="button"
			class="tab"
			class:active={tab === 'lagerbestand'}
			onclick={() => (tab = 'lagerbestand')}
		>
			Lagerbestand
		</button>
		<button
			type="button"
			class="tab"
			class:active={tab === 'lieferanten'}
			onclick={() => (tab = 'lieferanten')}
		>
			Lieferanten
		</button>
		<button type="button" class="tab" class:active={tab === 'pyro'} onclick={() => (tab = 'pyro')}>
			Pyro
		</button>
	</nav>

	<!-- Without write permission all fields of the tabs are locked; the tabs
	     themselves stay clickable, so the article can be read. -->
	{#if !schreibbar}
		<p class="read-only">
			Nur Lesen — zum Bearbeiten dieses Artikels fehlt das Recht
			<strong>Artikel editieren</strong>.
		</p>
	{/if}
	<fieldset class="content" disabled={!schreibbar}>
	{#if tab === 'lagerbestand'}
		<div class="tab-content">
			<StockLevel
				articleId={a.id}
				articleNo={a.article_no}
				articleUsage={{ inventory: a.inventory, rental: pricingEnabled && a.rental, sale: pricingEnabled && a.sale }}
			/>
		</div>
	{:else if tab === 'miete'}
		<ArticleRental bind:pricing={a.pricing} />
	{:else if tab === 'verkauf'}
		<ArticleSale bind:pricing={a.pricing} />
		{:else if tab === 'lieferanten'}
			<div class="tab-content suppliers">
				<p class="notice">
					Lieferanten sind Adressen mit dem Marker „Lieferant". Der Einkaufspreis gilt nur für
					diesen Artikel.
				</p>

				<table class="table">
					<thead>
						<tr>
							<th>Nr.</th>
							<th>Lieferant</th>
							<th class="num">EK-Preis</th>
							<th class="num">Lieferzeit (Tage)</th>
							<th>Favorit</th>
							<th>Verkaufsartikel</th>
							<th>Art.-Lief.-Nr.</th>
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
										aria-label="EK-Preis"
									/>
								</td>
								<td class="num">
									<input
										class="num-field"
										type="number"
										min="0"
										bind:value={s.delivery_days}
										aria-label="Lieferzeit in Tagen"
									/>
								</td>
								<td class="center">
									<input type="checkbox" bind:checked={s.favorite} aria-label="Favorit" />
								</td>
								<td class="center">
									<input
										type="checkbox"
										bind:checked={s.sales_article}
										aria-label="Verkaufsartikel"
									/>
								</td>
								<td>
									<input bind:value={s.supplier_article_no} aria-label="Artikelnummer des Lieferanten" />
								</td>
								<td>
									<button type="button" class="remove" title="Lieferant entfernen" onclick={() => removeSupplier(i)}>
										×
									</button>
								</td>
							</tr>
						{:else}
							<tr><td colspan="8">Noch kein Lieferant zugeordnet.</td></tr>
						{/each}
					</tbody>
				</table>

				<div class="row add">
					<select bind:value={selectedSupplier} onfocus={loadSuppliers} aria-label="Lieferant wählen">
						<option value="">— Lieferant wählen —</option>
						{#each supplierChoice ?? [] as l (l.id)}
							<option value={String(l.id)}>{l.no ? `${l.no} — ` : ''}{l.name}</option>
						{/each}
					</select>
					<button type="button" onclick={addSupplier} disabled={!selectedSupplier}>
						Hinzufügen
					</button>
				</div>
				{#if supplierError}
					<p class="error">{supplierError}</p>
				{/if}
			</div>
		{:else if tab === 'pyro'}
			<div class="tab-content pyro">
				<p class="notice">
					Pyrotechnische Kennwerte — sie werden von der Artikelsuche (Register „Pyro") und später
					vom Dispositions- und Sicherheitsnachweis gelesen.
				</p>
				<div class="columns">
					<section class="left">
						<fieldset>
							<legend>Stoff und Einstufung</legend>
							<label class="field row"><span>UN-Nr.</span><input bind:value={a.pyro.un_number} /></label>
							<label class="field row"><span>UN-Bezeichnung</span><input bind:value={a.pyro.un_description} /></label>
							<label class="field row"><span>Gefahrklasse</span><input bind:value={a.pyro.hazard_class} placeholder="z. B. 1.4G" /></label>
							<label class="field row"><span>Kaliber (mm)</span><input type="number" min="0" bind:value={a.pyro.caliber} /></label>
							<label class="field row"><span>Kategorie</span><input bind:value={a.pyro.category} /></label>
							<label class="field row"><span>Klassifizierung</span><input bind:value={a.pyro.classification} /></label>
							<label class="field row"><span>BAM-ID</span><input bind:value={a.pyro.bam_id} /></label>
							<label class="field row"><span>Zulassungsnummer</span><input bind:value={a.pyro.approval_number} /></label>
						</fieldset>

						<fieldset>
							<legend>Transport</legend>
							<label class="field row"><span>Verpackungsgruppe</span><input bind:value={a.pyro.packing_group} /></label>
							<label class="field row"><span>Transportkategorie</span><input bind:value={a.pyro.transport_category} /></label>
							<label class="field row"><span>Tunnelbeschränkung</span><input bind:value={a.pyro.tunnel_restriction_code} /></label>
							<label class="field row">
								<span>NEQ (netto)</span>
								<span class="double-field">
									<input inputmode="decimal" bind:value={a.pyro.neq} />
									<input class="unit" bind:value={a.pyro.neq_unit} placeholder="kg" aria-label="Einheit der NEQ" />
								</span>
							</label>
							<label class="field row"><span>UN-Punkte</span><input inputmode="decimal" bind:value={a.pyro.un_points} /></label>
						</fieldset>
					</section>

					<section class="right">
						<fieldset>
							<legend>Sicherheitsabstände (m)</legend>
							<label class="field row"><span>Seitlich</span><input inputmode="decimal" bind:value={a.pyro.lateral_safety_distance} /></label>
							<label class="field row"><span>Vertikal</span><input inputmode="decimal" bind:value={a.pyro.vertical_safety_distance} /></label>
							<label class="field row"><span>Steighöhe</span><input inputmode="decimal" bind:value={a.pyro.ascending_height} /></label>
						</fieldset>

						<fieldset>
							<legend>Wirkung</legend>
							<label class="field row"><span>Schalldruckpegel (dB)</span><input inputmode="decimal" bind:value={a.pyro.sound_pressure_level} /></label>
							<label class="field row"><span>Schalldruckbereich (m)</span><input inputmode="decimal" bind:value={a.pyro.sound_pressure_range} /></label>
							<label class="field row"><span>Verzögerungszeit (s)</span><input inputmode="decimal" bind:value={a.pyro.delay_time} /></label>
							<label class="field row"><span>Effektdauer (s)</span><input inputmode="decimal" bind:value={a.pyro.effect_duration} /></label>
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
					<span>Artikel Nr.</span>
					<input type="number" bind:value={a.article_no} min="0" />
				</label>
				<label class="field short">
					<span>Inventar Nr.</span>
					<input bind:value={a.inventory_no} />
				</label>
				<label class="field short">
					<span>Sprache</span>
					<select>
						{#each languages as s (s.id)}
							<option value={s.id}>{s.language}</option>
						{/each}
					</select>
				</label>
			</div>

			<label class="field"><span>Name <em>*</em></span><input bind:value={a.name} required /></label>
			<label class="field"><span>Kurzname</span><input bind:value={a.short_name} /></label>
			<label class="field"><span>Matchcode</span><input bind:value={a.match_code} /></label>
			<label class="field"><span>Einheit</span><input bind:value={a.unit} /></label>

			<div class="block">
				<nav class="tabs small" aria-label="Textblöcke">
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
				<nav class="tabs small" aria-label="Artikeldetails">
					<button type="button" class="tab" class:active={lowerTab === 'masse'} onclick={() => (lowerTab = 'masse')}>Maße &amp; Werte</button>
					<button type="button" class="tab" class:active={lowerTab === 'warengruppe'} onclick={() => (lowerTab = 'warengruppe')}>Warengruppe</button>
					<button type="button" class="tab" class:active={lowerTab === 'dguv'} onclick={() => (lowerTab = 'dguv')}>DGUV</button>
					<button type="button" class="tab" class:active={lowerTab === 'rest'} onclick={() => (lowerTab = 'rest')}>Bemerkungen</button>
				</nav>

				{#if lowerTab === 'masse'}
					<div class="dimensions">
						<div class="group">
							<h3>netto</h3>
							<div class="triple">
								<label class="field"><span>B (cm)</span><input bind:value={a.width_net} inputmode="decimal" /></label>
								<label class="field"><span>H (cm)</span><input bind:value={a.height_net} inputmode="decimal" /></label>
								<label class="field"><span>T (cm)</span><input bind:value={a.depth_net} inputmode="decimal" /></label>
							</div>
							<label class="field"><span>Volumen (m³)</span><input bind:value={a.volume_net} inputmode="decimal" /></label>
							<label class="field"><span>Gewicht (kg)</span><input bind:value={a.weight_net} inputmode="decimal" /></label>
						</div>
						<div class="group">
							<h3>brutto</h3>
							<div class="triple">
								<label class="field"><span>B (cm)</span><input bind:value={a.width_gross} inputmode="decimal" /></label>
								<label class="field"><span>H (cm)</span><input bind:value={a.height_gross} inputmode="decimal" /></label>
								<label class="field"><span>T (cm)</span><input bind:value={a.depth_gross} inputmode="decimal" /></label>
							</div>
							<label class="field"><span>Volumen (m³)</span><input bind:value={a.volume_gross} inputmode="decimal" /></label>
							<label class="field"><span>Gewicht (kg)</span><input bind:value={a.weight_gross} inputmode="decimal" /></label>
						</div>
						<div class="group wide">
							<h3>Betrieb</h3>
							<div class="pair">
								<label class="field"><span>Betriebsspannung von (V)</span><input bind:value={a.voltage_from} inputmode="decimal" /></label>
								<label class="field"><span>bis (V)</span><input bind:value={a.voltage_to} inputmode="decimal" /></label>
							</div>
							<div class="pair">
								<label class="field"><span>Stromaufnahme (A)</span><input bind:value={a.current_draw} inputmode="decimal" /></label>
								<label class="field"><span>elektr. Leistung (kW)</span><input bind:value={a.power} inputmode="decimal" /></label>
							</div>
							<div class="pair">
								<label class="field"><span>Aufbauzeit (min)</span><input type="number" bind:value={a.setup_minutes} min="0" /></label>
								<label class="field"><span>Abbauzeit (min)</span><input type="number" bind:value={a.teardown_minutes} min="0" /></label>
							</div>
							<div class="pair">
								<label class="field"><span>Check-Out-Zeit (sek)</span><input type="number" bind:value={a.check_out_seconds} min="0" /></label>
								<label class="field"><span>Check-In-Zeit (sek)</span><input type="number" bind:value={a.check_in_seconds} min="0" /></label>
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
							<p class="notice">Keine Warengruppen angelegt.</p>
						{/each}
					</div>
				{:else if lowerTab === 'dguv'}
					<div class="dguv">
						<div class="pair">
							<label class="field"><span>Schutzklasse</span><input type="number" bind:value={a.inspection_class} min="0" max="3" /></label>
							<label class="field"><span>Prüfintervall (Monate)</span><input type="number" bind:value={a.inspection_interval_months} min="0" /></label>
							<label class="field"><span>DGUV-Prüfzeit (min)</span><input type="number" bind:value={a.inspection_minutes} min="0" /></label>
						</div>
						<h3>Werkstattintervalle</h3>
						<label class="check"><input type="checkbox" bind:checked={a.workshop_by_usage_days} /> nach Einsatztagen</label>
						<label class="field narrow"><span>Max.</span><input bind:value={a.workshop_max_usage_days} inputmode="decimal" /></label>
						<label class="check"><input type="checkbox" bind:checked={a.workshop_fixed_intervals} /> nach festen Intervallen</label>
						<label class="field narrow">
							<span>Einheit</span>
							<select
								value={a.workshop_by_months ? 'monate' : 'tage'}
								onchange={(e) => {
									const v = e.currentTarget.value;
									a.workshop_by_days = v === 'tage';
									a.workshop_by_months = v === 'monate';
								}}
							>
								<option value="tage">nach Tagen</option>
								<option value="monate">nach Monaten</option>
							</select>
						</label>
						<div class="pair">
							<label class="field narrow"><span>Tage</span><input type="number" bind:value={a.workshop_days} min="0" /></label>
							<label class="field narrow"><span>Monate</span><input type="number" bind:value={a.workshop_months} min="0" /></label>
						</div>
					</div>
				{:else}
					<p class="notice">Suchindex, Bemerkungen und CAD sind im Original eigene Unterformulare und noch nicht umgesetzt.</p>
				{/if}
			</div>
		</section>

		<!-- ----------------------------------------------------- right column -->
		<section class="right">
			<!-- The article picture stands above `Kategorie`; a click opens it in full size. -->
			<ArticlePicture articleId={a.id} {schreibbar} />

			<fieldset>
				<legend>Kategorie</legend>
				<label class="check"><input type="checkbox" bind:checked={a.active} /> Artikel aktiv</label>
				<div class="pair">
					<label class="check"><input type="checkbox" bind:checked={a.inventory} /> Inventar</label>
					{#if pricingEnabled}
						<label class="check"><input type="checkbox" bind:checked={a.rental} /> Miete</label>
					{/if}
				</div>
				<div class="pair">
					{#if pricingEnabled}
						<label class="check"><input type="checkbox" bind:checked={a.sale} /> Verkauf</label>
					{/if}
					<label class="check"><input type="checkbox" bind:checked={a.text_sync} /> Textsync</label>
				</div>
			</fieldset>

			<fieldset>
				<legend>Bestandsführung</legend>
				<label class="check"><input type="checkbox" bind:checked={a.stock_tracking} /> Aktiv</label>
				<div class="pair">
					<label class="check"><input type="checkbox" bind:checked={a.no_own_stock} /> ohne Eigenbestand</label>
					<label class="check"><input type="checkbox" bind:checked={a.individual_items} /> mit Einzelerfassung</label>
				</div>
				<div class="pair">
					<label class="check"><input type="checkbox" bind:checked={a.allocation_control} /> Dispositionskontrolle</label>
					<label class="check"><input type="checkbox" bind:checked={a.virtual_article} /> virtueller Artikel</label>
				</div>
			</fieldset>

			<fieldset>
				<legend>Herstellerangaben</legend>
				<label class="field row"><span>EAN-Code</span><input bind:value={a.ean_code} /></label>
				<label class="field row"><span>Herst.-Art.-Nr.</span><input bind:value={a.manufacturer_no} /></label>
				<label class="field row"><span>Hersteller</span><input bind:value={a.manufacturer} list="hersteller-liste" /></label>
				<datalist id="hersteller-liste">
					{#each manufacturer as name (name)}<option value={name}></option>{/each}
				</datalist>
				<label class="field row"><span>Herstellerland</span><input bind:value={a.manufacturer_country} /></label>
				<label class="field row"><span>UVP / EVP brutto</span><input bind:value={a.retail_price} inputmode="decimal" placeholder="0,00" /></label>
				<label class="field row"><span>Zolltarif-Nr. Export</span><input bind:value={a.customs_tariff_export} /></label>
				<label class="field row"><span>Zolltarif-Nr. Import</span><input bind:value={a.customs_tariff_import} /></label>
			</fieldset>

			<fieldset>
				<legend>weitere Informationen</legend>
				<label class="field row"><span>Versicherungswert netto</span><input bind:value={a.insurance_value_net} inputmode="decimal" placeholder="0,00" /></label>
				<label class="field row"><span>Versicherungswert brutto</span><input bind:value={a.insurance_value_gross} inputmode="decimal" placeholder="0,00" /></label>
				<label class="field row"><span>TeamAgenda-Gruppe</span><input bind:value={a.team_group} /></label>
				<label class="field row"><span>Provisionsklasse</span><input bind:value={a.commission_class} placeholder="-- Keine Angabe --" /></label>
			</fieldset>
		</section>
	</div>
	{/if}
	</fieldset>

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
				onclick={confirmCancel('Das Bearbeiten des Artikels', oncancel)}
				disabled={busy}>{schreibbar ? 'Abbrechen' : 'Schließen'}</button
			>
			{#if schreibbar}
				<button type="submit" class="primary" disabled={busy}
					>{busy ? 'Sichern …' : 'Sichern'}</button
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
	/* In the window the content of the tabs scrolls, not the whole window. */
	.mask.in-window .content {
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
