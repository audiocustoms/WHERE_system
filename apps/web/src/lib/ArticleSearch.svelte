<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import SearchWindow from './search/SearchWindow.svelte';
	import {
		api,
		errorText,
		type ArticleSearchInput,
		type ArticleType,
		type SavedSearch,
		type SetOperator,
		type SearchIndexNode,
		type ArticleGroup
	} from './api';

	interface Props {
		/** Last applied criteria — the mask starts with them. */
		criteria: ArticleSearchInput;
		onsearch: (criteria: ArticleSearchInput) => void;
		onclose: () => void;
	}

	let { criteria, onsearch, onclose }: Props = $props();

	/** „Artikeltyp" selection — labels taken from the legacy language package. */
	const ARTICLE_TYPES: { value: ArticleType; text: string }[] = [
		{ value: 'active', text: t('Alle Artikel (Aktiv)') },
		{ value: 'inactive', text: t('Alle inaktiven Artikel') },
		{ value: 'all', text: t('Alle aktiven und inaktiven Artikel') }
	];

	const OPERATORS: { value: SetOperator; text: string }[] = [
		{ value: 'contains', text: t('enthält') },
		{ value: 'contains_any', text: t('oder enthält') },
		{ value: 'contains_all', text: t('und enthält') },
		{ value: 'contains_none', text: t('enthält nicht') }
	];

	/** Number fields of the pyro tab as text, so that „1,5" stays typable. */
	type PyroText = {
		un_number: string;
		hazard_class: string;
		ce_nr: string;
		classification: string;
		category: string;
		approval_number: string;
		packing_group: string;
		tunnel_code: string;
		transport_category: string;
		neq_from: string;
		neq_to: string;
		un_points: string;
		lateral_distance_from: string;
		lateral_distance_to: string;
		vertical_distance_from: string;
		vertical_distance_to: string;
		ascending_height_from: string;
		ascending_height_to: string;
		sound_pressure_level_from: string;
		sound_pressure_level_to: string;
		sound_pressure_range_from: string;
		sound_pressure_range_to: string;
		delay_time_from: string;
		delay_time_to: string;
		effect_duration_from: string;
		effect_duration_to: string;
	};

	function emptyPyro(): PyroText {
		return {
			un_number: '',
			hazard_class: '',
			ce_nr: '',
			classification: '',
			category: '',
			approval_number: '',
			packing_group: '',
			tunnel_code: '',
			transport_category: '',
			neq_from: '',
			neq_to: '',
			un_points: '',
			lateral_distance_from: '',
			lateral_distance_to: '',
			vertical_distance_from: '',
			vertical_distance_to: '',
			ascending_height_from: '',
			ascending_height_to: '',
			sound_pressure_level_from: '',
			sound_pressure_level_to: '',
			sound_pressure_range_from: '',
			sound_pressure_range_to: '',
			delay_time_from: '',
			delay_time_to: '',
			effect_duration_from: '',
			effect_duration_to: ''
		};
	}

	/** Number from a German or English decimal separator. */
	function numberText(value: string): number | null {
		const s = value.trim().replace(',', '.');
		if (s === '') return null;
		const n = Number(s);
		return Number.isFinite(n) ? n : null;
	}

	function text(value: string | undefined): string | undefined {
		const s = (value ?? '').trim();
		return s === '' ? undefined : s;
	}

	let tab = $state<'allgemein' | 'pyro'>('allgemein');
	// svelte-ignore state_referenced_locally
	let article_type = $state<ArticleType>(criteria.article_type ?? 'active');
	// svelte-ignore state_referenced_locally
	let name = $state(criteria.name ?? '');
	// svelte-ignore state_referenced_locally
	let inDescriptions = $state(criteria.include_descriptions ?? false);
	// svelte-ignore state_referenced_locally
	let articleNo = $state(criteria.article_no ? String(criteria.article_no) : '');
	// svelte-ignore state_referenced_locally
	let inventoryNo = $state(criteria.inventory_no ?? '');
	// svelte-ignore state_referenced_locally
	let serialNo = $state(criteria.serial_no ?? '');
	// svelte-ignore state_referenced_locally
	let withBundle = $state(criteria.with_bundle ?? false);
	// svelte-ignore state_referenced_locally
	let article_groups = $state<number[]>([...(criteria.article_groups ?? [])]);
	// svelte-ignore state_referenced_locally
	let warengruppenOp = $state<SetOperator>(criteria.article_groups_op ?? 'contains');
	// svelte-ignore state_referenced_locally
	let searchIndex = $state<string[]>([...(criteria.search_index ?? [])]);
	// svelte-ignore state_referenced_locally
	let suchindexOp = $state<SetOperator>(criteria.search_index_op ?? 'contains');
	let supplierInput = $state('');
	// svelte-ignore state_referenced_locally
	let suppliers = $state<string[]>([...(criteria.suppliers ?? [])]);
	// svelte-ignore state_referenced_locally
	let manufacturer = $state(criteria.manufacturer ?? '');
	// svelte-ignore state_referenced_locally
	let owner = $state(criteria.owner ?? '');
	// svelte-ignore state_referenced_locally
	let excludeVirtual = $state(criteria.exclude_virtual ?? false);
	// svelte-ignore state_referenced_locally
	let pyro = $state<PyroText>({ ...emptyPyro(), ...alsText(criteria.pyro) });

	let saved = $state<SavedSearch[]>([]);
	let savedSearchKey = $state<string>('');
	let sucheditor = $state(false);
	let saveName = $state('');
	let selection = $state<'warengruppe' | 'suchindex' | null>(null);
	let articleGroupList = $state<ArticleGroup[]>([]);
	let searchIndexList = $state<SearchIndexNode[]>([]);
	let error = $state<string | null>(null);
	let hint = $state<string | null>(null);
	let busy = $state(false);

	/** Turn pyro criteria into text fields (empty = not set). */
	function alsText(source: object | null | undefined): Partial<PyroText> {
		const target: Record<string, string> = {};
		for (const [key, value] of Object.entries(source ?? {})) {
			if (value === null || value === undefined) continue;
			target[key] = String(value).replace('.', ',');
		}
		return target as Partial<PyroText>;
	}

	/** Fields that are sent as a number (range); all others are text. */
	const PYRO_NUMBERS = new Set([
		'neq_von',
		'neq_bis',
		'un_punkte',
		'seitlicher_abstand_von',
		'seitlicher_abstand_bis',
		'senkrechter_abstand_von',
		'senkrechter_abstand_bis',
		'steighoehe_von',
		'steighoehe_bis',
		'schalldruckpegel_von',
		'schalldruckpegel_bis',
		'schalldruckweite_von',
		'schalldruckweite_bis',
		'verzoegerungszeit_von',
		'verzoegerungszeit_bis',
		'effektdauer_von',
		'effektdauer_bis'
	]);

	function buildCriteria(): ArticleSearchInput {
		const p: Record<string, string | number> = {};
		for (const [key, value] of Object.entries(pyro)) {
			const s = value.trim();
			if (s === '') continue;
			if (PYRO_NUMBERS.has(key)) {
				const n = numberText(s);
				if (n !== null) p[key] = n;
			} else {
				p[key] = s;
			}
		}
		return {
			article_type,
			name: text(name),
			include_descriptions: inDescriptions,
			article_no: numberText(articleNo),
			inventory_no: text(inventoryNo),
			serial_no: text(serialNo),
			with_bundle: withBundle,
			article_groups,
			article_groups_op: warengruppenOp,
			search_index: searchIndex,
			search_index_op: suchindexOp,
			suppliers,
			manufacturer: text(manufacturer),
			owner: text(owner),
			exclude_virtual: excludeVirtual,
			pyro: Object.keys(p).length ? (p as ArticleSearchInput['pyro']) : null
		};
	}

	function addSupplier() {
		const value = supplierInput.trim();
		if (!value) return;
		if (!suppliers.includes(value)) suppliers = [...suppliers, value];
		supplierInput = '';
	}

	function search() {
		error = null;
		try {
			onsearch(buildCriteria());
		} catch (e) {
			error = errorText(e);
		}
	}

	async function loadSearches() {
		try {
			saved = await api.article.savedSearches();
		} catch (e) {
			error = errorText(e);
		}
	}

	function chooseSavedSearch(value: string) {
		savedSearchKey = value;
		const hits = saved.find((s) => String(s.id) === value);
		if (!hits) return;
		const k = hits.criteria;
		article_type = k.article_type ?? 'active';
		name = k.name ?? '';
		inDescriptions = k.include_descriptions ?? false;
		articleNo = k.article_no ? String(k.article_no) : '';
		inventoryNo = k.inventory_no ?? '';
		serialNo = k.serial_no ?? '';
		withBundle = k.with_bundle ?? false;
		article_groups = [...(k.article_groups ?? [])];
		warengruppenOp = k.article_groups_op ?? 'contains';
		searchIndex = [...(k.search_index ?? [])];
		suchindexOp = k.search_index_op ?? 'contains';
		suppliers = [...(k.suppliers ?? [])];
		manufacturer = k.manufacturer ?? '';
		owner = k.owner ?? '';
		excludeVirtual = k.exclude_virtual ?? false;
		pyro = { ...emptyPyro(), ...alsText(k.pyro as Record<string, unknown>) };
		saveName = hits.name;
		hint = t('Vorgang „{name}" geladen', { name: hits.name });
	}

	async function save() {
		const value = saveName.trim();
		if (!value) {
			error = t('Bitte einen Namen für die Suche eingeben.');
			return;
		}
		busy = true;
		error = null;
		try {
			const saved = await api.article.saveSearch(value, buildCriteria());
			savedSearchKey = String(saved.id);
			hint = t('Suche „{name}" gespeichert', { name: saved.name });
			await loadSearches();
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = false;
		}
	}

	async function remove(id: number) {
		busy = true;
		try {
			await api.article.deleteSearch(id);
			if (savedSearchKey === String(id)) savedSearchKey = '';
			await loadSearches();
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = false;
		}
	}

	async function openPicker(was: 'warengruppe' | 'suchindex') {
		selection = was;
		error = null;
		try {
			if (was === 'warengruppe' && articleGroupList.length === 0) {
				articleGroupList = await api.masterData.articleGroups();
			}
			if (was === 'suchindex' && searchIndexList.length === 0) {
				searchIndexList = await api.masterData.searchIndex();
			}
		} catch (e) {
			error = errorText(e);
		}
	}

	function articleGroupName(id: number): string {
		return articleGroupList.find((w) => w.id === id)?.name ?? `#${id}`;
	}

	/** Indentation by parent node, so the tree stays visible. */
	function depth(list: { id: number; parent_id: number | null }[], entry: { parent_id: number | null }): number {
		let t = 0;
		let parent = entry.parent_id;
		while (parent && t < 8) {
			const parents = list.find((x) => x.id === parent);
			if (!parents) break;
			parent = parents.parent_id;
			t++;
		}
		return t;
	}

	onMount(() => {
		void loadSearches();
	});

	/** Process selection of the dark band: saved searches plus „neu". */
	const processOptions = $derived([
		{ value: '', text: t('— neu —') },
		...saved.map((s) => ({ value: String(s.id), text: s.name }))
	]);
</script>

<SearchWindow
	title={t('Artikelsuche')}
	width="32rem"
	bandLabel={t('Vorgang')}
	bandOptions={processOptions}
	bandValue={savedSearchKey}
	onbandchange={chooseSavedSearch}
	{onclose}
>
		<nav class="tabs">
			<button class:active={tab === 'allgemein'} type="button" onclick={() => (tab = 'allgemein')}>
				{t('Allgemein')}
			</button>
			<button class:active={tab === 'pyro'} type="button" onclick={() => (tab = 'pyro')}>
				{t('Pyro')}
			</button>
		</nav>

		{#if tab === 'allgemein'}
			<div class="form">
				<div class="row">
					<span class="label">{t('Artikeltyp')}</span>
					<select bind:value={article_type}>
						{#each ARTICLE_TYPES as t (t.value)}
							<option value={t.value}>{t.text}</option>
						{/each}
					</select>
				</div>
			</div>

			<hr />

			<div class="form">
				<div class="row">
					<span class="label">{t('Name')}</span>
					<div class="stack">
						<input bind:value={name} onkeydown={(e) => e.key === 'Enter' && search()} />
						<label class="check">
							<input type="checkbox" bind:checked={inDescriptions} />
							{t('inkl. in Artikelbeschreibungen')}
						</label>
					</div>
				</div>
				<div class="row">
					<span class="label">{t('Artikel Nr.')}</span>
					<input class="narrow" bind:value={articleNo} inputmode="numeric" placeholder="0" />
				</div>
				<div class="row">
					<span class="label">{t('Inventar Nr.')}</span>
					<input bind:value={inventoryNo} />
				</div>
				<div class="row">
					<span class="label">{t('Serien-Nr.')}</span>
					<input bind:value={serialNo} />
				</div>
				<div class="row">
					<span class="label">{t('Bundlematerial')}</span>
					<label class="check">
						<input type="checkbox" bind:checked={withBundle} />
						{t('Zugehöriges mit anzeigen')}
					</label>
				</div>
			</div>

			<hr />

			<div class="form">
				<div class="row">
					<span class="label">{t('Warengruppe')}</span>
					<select class="operator" bind:value={warengruppenOp}>
						{#each OPERATORS as o (o.value)}
							<option value={o.value}>{o.text}</option>
						{/each}
					</select>
					<button type="button" onclick={() => openPicker('warengruppe')}>{t('Index wählen')}</button>
				</div>
				{#if article_groups.length}
					<div class="chips">
						{#each article_groups as id (id)}
							<span class="chip">
								{articleGroupName(id)}
								<button
									type="button"
									title={t('Entfernen')}
									onclick={() => (article_groups = article_groups.filter((w) => w !== id))}>×</button
								>
							</span>
						{/each}
					</div>
				{/if}

				<div class="row">
					<span class="label">{t('Suchindex')}</span>
					<select class="operator" bind:value={suchindexOp}>
						{#each OPERATORS as o (o.value)}
							<option value={o.value}>{o.text}</option>
						{/each}
					</select>
					<button type="button" onclick={() => openPicker('suchindex')}>{t('Index wählen')}</button>
				</div>
				{#if searchIndex.length}
					<div class="chips">
						{#each searchIndex as value (value)}
							<span class="chip">
								{value}
								<button
									type="button"
									title={t('Entfernen')}
									onclick={() => (searchIndex = searchIndex.filter((s) => s !== value))}>×</button
								>
							</span>
						{/each}
					</div>
				{/if}

				<div class="row">
					<span class="label">{t('Lieferant')}</span>
					<input
						bind:value={supplierInput}
						placeholder={t('Name oder Adress-Nr.')}
						onkeydown={(e) => e.key === 'Enter' && addSupplier()}
					/>
					<button type="button" title={t('Weiteren Lieferanten hinzufügen')} onclick={addSupplier}>
						+
					</button>
				</div>
				{#if suppliers.length > 1}
					<div class="chips">
						{#each suppliers as value (value)}
							<span class="chip">
								{value}
								<button
									type="button"
									title={t('Entfernen')}
									onclick={() => (suppliers = suppliers.filter((l) => l !== value))}>×</button
								>
							</span>
						{/each}
					</div>
				{/if}

				<div class="row">
					<span class="label">{t('Hersteller')}</span>
					<input bind:value={manufacturer} />
				</div>
				<div class="row">
					<span class="label">{t('Eigentümer')}</span>
					<input bind:value={owner} />
				</div>
			</div>

			<hr />

			<div class="form">
				<div class="row">
					<span class="label">{t('Option')}</span>
					<label class="check">
						<input type="checkbox" bind:checked={excludeVirtual} />
						{t('exkl. virtueller Artikel')}
					</label>
				</div>
			</div>
		{:else}
			<div class="form pyro">
				<div class="row">
					<span class="label">{t('UN Nummer')}</span>
					<input bind:value={pyro.un_number} />
				</div>
				<div class="row">
					<span class="label">{t('Gefahrgutklasse')}</span>
					<input bind:value={pyro.hazard_class} />
				</div>
				<div class="row">
					<span class="label">{t('Masse Gefahrstoff (NEQ)')}</span>
					<input class="narrow" bind:value={pyro.neq_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.neq_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
				<div class="row">
					<span class="label">{t('UN Punkte Faktor')}</span>
					<input class="narrow" bind:value={pyro.un_points} inputmode="decimal" />
				</div>
				<div class="row">
					<span class="label">{t('CE-Nr.')}</span>
					<input bind:value={pyro.ce_nr} />
				</div>
				<div class="row">
					<span class="label">{t('Klassifizierung')}</span>
					<input bind:value={pyro.classification} />
				</div>
				<div class="row">
					<span class="label">{t('Kategorie')}</span>
					<input bind:value={pyro.category} />
				</div>
				<div class="row">
					<span class="label">{t('Zulassungsnummer')}</span>
					<input bind:value={pyro.approval_number} />
				</div>
				<div class="row">
					<span class="label">{t('Verpackungsgruppe')}</span>
					<input bind:value={pyro.packing_group} />
				</div>
				<div class="row">
					<span class="label">{t('Tunnelbegrenzungscode')}</span>
					<input bind:value={pyro.tunnel_code} />
				</div>
				<div class="row">
					<span class="label">{t('Beförderungskategorie')}</span>
					<input bind:value={pyro.transport_category} />
				</div>
				<div class="row">
					<span class="label">{t('Seitlicher Sicherheitsabstand')}</span>
					<input class="narrow" bind:value={pyro.lateral_distance_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.lateral_distance_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
				<div class="row">
					<span class="label">{t('Senkrechter Sicherheitsabstand')}</span>
					<input class="narrow" bind:value={pyro.vertical_distance_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.vertical_distance_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
				<div class="row">
					<span class="label">{t('Steighöhe')}</span>
					<input class="narrow" bind:value={pyro.ascending_height_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.ascending_height_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
				<div class="row">
					<span class="label">{t('Schalldruckpegel')}</span>
					<input class="narrow" bind:value={pyro.sound_pressure_level_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.sound_pressure_level_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
				<div class="row">
					<span class="label">{t('Schalldruckreichweite')}</span>
					<input class="narrow" bind:value={pyro.sound_pressure_range_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.sound_pressure_range_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
				<div class="row">
					<span class="label">{t('Verzögerungszeit')}</span>
					<input class="narrow" bind:value={pyro.delay_time_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.delay_time_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
				<div class="row">
					<span class="label">{t('Effektdauer')}</span>
					<input class="narrow" bind:value={pyro.effect_duration_from} inputmode="decimal" placeholder={t('von')} />
					<input class="narrow" bind:value={pyro.effect_duration_to} inputmode="decimal" placeholder={t('bis')} />
				</div>
			</div>
		{/if}

		{#if error}
			<p class="bad">{error}</p>
		{/if}
		{#if hint}
			<p class="good">{hint}</p>
		{/if}

		{#if sucheditor}
			<section class="editor">
				<h3>{t('Sucheditor')}</h3>
				<div class="row">
					<span class="label">{t('Name')}</span>
					<input bind:value={saveName} placeholder={t('z. B. Licht ohne Ersatzteile')} />
					<button type="button" onclick={save} disabled={busy}>{t('Speichern')}</button>
				</div>
				{#if saved.length}
					<ul>
						{#each saved as s (s.id)}
							<li>
								<button type="button" class="name" onclick={() => chooseSavedSearch(String(s.id))}>
									{s.name}
								</button>
								<button type="button" onclick={() => remove(s.id)} disabled={busy}>{t('Löschen')}</button>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="small">{t('Noch keine Suche gespeichert.')}</p>
				{/if}
			</section>
		{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>{t('Abbrechen')}</button>
		<button class="sw-button" type="button" onclick={() => (sucheditor = !sucheditor)}>
			{sucheditor ? t('Editor schließen') : t('Sucheditor')}
		</button>
		<button class="sw-button sw-primary" type="button" onclick={search}>{t('Suchen')}</button>
	{/snippet}
</SearchWindow>

{#if selection === 'warengruppe'}
	<SearchWindow title={t('Warengruppe wählen')} width="24rem" onclose={() => (selection = null)}>
		<ul class="tree">
			{#each articleGroupList as w (w.id)}
				<li style="padding-left: {depth(articleGroupList, w) * 1.1}rem">
					<label class="check">
						<input
							type="checkbox"
							checked={article_groups.includes(w.id)}
							onchange={(e) =>
								(article_groups = e.currentTarget.checked
									? [...article_groups, w.id]
									: article_groups.filter((x) => x !== w.id))}
						/>
						{w.name ?? `#${w.id}`}
						<span class="small">{t('{count} Artikel', { count: w.article_count })}</span>
					</label>
				</li>
			{:else}
				<li class="small">{t('Keine Warengruppen angelegt.')}</li>
			{/each}
		</ul>

		{#snippet actions()}
			<button class="sw-button sw-primary" type="button" onclick={() => (selection = null)}>
				{t('Übernehmen')}
			</button>
		{/snippet}
	</SearchWindow>
{/if}

{#if selection === 'suchindex'}
	<SearchWindow title={t('Suchindex wählen')} width="24rem" onclose={() => (selection = null)}>
		<ul class="tree">
			{#each searchIndexList as k (k.id)}
				<li style="padding-left: {depth(searchIndexList, k) * 1.1}rem">
					<label class="check">
						<input
							type="checkbox"
							checked={searchIndex.includes(k.name ?? '')}
							onchange={(e) => {
								const value = k.name ?? '';
								searchIndex = e.currentTarget.checked
									? [...searchIndex, value]
									: searchIndex.filter((x) => x !== value);
							}}
						/>
						{k.name ?? `#${k.id}`}
					</label>
				</li>
			{:else}
				<li class="small">{t('Kein Suchindex angelegt.')}</li>
			{/each}
		</ul>

		{#snippet actions()}
			<button class="sw-button sw-primary" type="button" onclick={() => (selection = null)}>
				{t('Übernehmen')}
			</button>
		{/snippet}
	</SearchWindow>
{/if}

<style>
	.tabs {
		display: flex;
		gap: 0.25rem;
		border-bottom: 1px solid var(--border);
		margin-bottom: 0.6rem;
	}
	.tabs button {
		border: 1px solid transparent;
		border-bottom: none;
		background: none;
		padding: 0.35rem 0.8rem;
		font-size: 0.8rem;
		cursor: pointer;
		color: var(--muted);
		border-radius: 4px 4px 0 0;
	}
	.tabs button.active {
		background: var(--surface);
		border-color: var(--border);
		color: var(--text);
		font-weight: 600;
		margin-bottom: -1px;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
	}
	.label {
		flex: 0 0 9.5rem;
		color: var(--text);
	}
	.stack {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		flex: 1;
	}
	input,
	select {
		flex: 1;
		min-width: 0;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		font-size: 0.8rem;
		font-family: inherit;
		background: var(--surface);
	}
	input.narrow {
		flex: 0 0 5.5rem;
	}
	select.operator {
		flex: 0 0 8rem;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		white-space: nowrap;
	}
	.check input {
		flex: 0 0 auto;
		width: auto;
	}
	hr {
		border: none;
		border-top: 1px solid var(--border);
		margin: 0.55rem 0;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		margin: 0 0 0.1rem 9.9rem;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		background: var(--surface-3);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 0.1rem 0.45rem;
		font-size: 0.75rem;
	}
	.chip button {
		border: none;
		background: none;
		cursor: pointer;
		color: var(--muted);
		padding: 0;
		line-height: 1;
	}
	.editor {
		border-top: 1px solid var(--border);
		margin-top: 0.6rem;
		padding-top: 0.4rem;
	}
	.editor h3 {
		font-size: 0.8rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--muted-2);
		margin: 0.2rem 0 0.4rem;
	}
	.editor ul {
		list-style: none;
		margin: 0.4rem 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.editor li {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
	}
	.editor .name {
		flex: 1;
		text-align: left;
		border: none;
		background: none;
		color: var(--accent);
		cursor: pointer;
		padding: 0;
	}
	.tree {
		list-style: none;
		margin: 0.5rem 0;
		padding: 0;
		max-height: 22rem;
		overflow: auto;
	}
	.tree li {
		padding: 0.15rem 0;
		font-size: 0.8rem;
	}
	.row button {
		padding: 0.3rem 0.8rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-2);
		color: var(--text);
		cursor: pointer;
		font-size: 0.8rem;
	}
	.bad {
		color: var(--bad);
		font-size: 0.8rem;
		margin: 0.5rem 0 0;
	}
	.good {
		color: var(--good);
		font-size: 0.8rem;
		margin: 0.5rem 0 0;
	}
	.small {
		color: var(--muted-2);
		font-size: 0.75rem;
	}
</style>
