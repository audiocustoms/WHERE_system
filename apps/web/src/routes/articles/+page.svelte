<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		api,
		errorText,
		type ArticleGroup,
		type ArticleList,
		type ArticleSearchInput
	} from '../../lib/api';
	import ArticleSearch from '../../lib/ArticleSearch.svelte';
	import { openArticleEdit } from '../../lib/articles/articleDialogStore.svelte';
	import { auth } from '../../lib/auth.svelte';
	import { modules } from '../../lib/modules.svelte';
	import { RIGHT } from '../../lib/rights';
	import AccessDenied from '../../lib/AccessDenied.svelte';
	import {
		articleSearchFromParams,
		articleSearchParams,
		isArticleSearchEmpty
	} from '../../lib/search/articleSearchQuery';

	/** Rental/sale columns belong to `WHERE - does my stuff go?`. */
	const pricingEnabled = $derived(modules.isInstalled('go'));
	const columns = $derived(pricingEnabled ? 7 : 5);

	let items = $state<ArticleList[]>([]);
	let total = $state(0);
	let quickSearch = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);

	/** Criteria of the last applied mask search; `null` = plain list. */
	let criteria = $state<ArticleSearchInput | null>(null);
	let searchOpen = $state(false);

	/** Article groups of the left menu column (`img/Artikelliste.PNG`). */
	let groups = $state<ArticleGroup[]>([]);
	/** Chosen article group; `null` = „Alle Artikel zeigen". */
	let groupFilter = $state<number | null>(null);
	/** „Nur aktive / nur inaktive Artikel zeigen"; `all` = both. */
	let activeFilter = $state<'all' | 'active' | 'inactive'>('all');
	/** Collapsed article groups (their children stay hidden). */
	let zugeklappt = $state<number[]>([]);

	const ARTICLE_TYPE_TEXT: Record<string, string> = {
		active: 'Alle Artikel (Aktiv)',
		inactive: 'Alle inaktiven Artikel',
		all: 'Alle aktiven und inaktiven Artikel'
	};
	const OPERATOR_TEXT: Record<string, string> = {
		contains: 'enthält',
		contains_any: 'oder enthält',
		contains_all: 'und enthält',
		contains_none: 'enthält nicht'
	};

	async function load() {
		loading = true;
		error = null;
		try {
			const result = criteria
				? await api.article.search({ ...criteria, limit: 200 })
				: await api.article.list({
						limit: 200,
						q: quickSearch || undefined,
						group: groupFilter ?? undefined,
						active: activeFilter
					});
			items = result.items;
			total = result.total;
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	async function loadGroups() {
		try {
			groups = await api.masterData.articleGroups();
		} catch (e) {
			error = errorText(e);
		}
	}

	/** Closes the search mask and removes the ?search=1 from the address. */
	function closeSearch() {
		searchOpen = false;
		if (page.url.searchParams.has('search')) {
			void goto(page.url.pathname + (criteria ? articleSearchParams(criteria) : ''), {
				replace: true,
				reset: false
			});
		}
	}

	/** Without criteria the article list stays; otherwise it shows the hits. */
	function applySearch(k: ArticleSearchInput) {
		quickSearch = '';
		searchOpen = false;
		const query = isArticleSearchEmpty(k) ? '' : articleSearchParams(k);
		void goto(`/articles${query}`, { replace: true, reset: false });
	}

	function reset() {
		void goto('/articles', { replace: true, reset: false });
	}

	/**
	 * Build the address from the current menu choice: article group and active filter.
	 * If the address is already current (only the search text was new), it loads directly.
	 */
	function applySelection() {
		const query = new URLSearchParams();
		if (groupFilter !== null) query.set('group', String(groupFilter));
		if (activeFilter !== 'all') query.set('active', activeFilter);
		const s = query.toString();
		const target = `/articles${s ? `?${s}` : ''}`;
		if (target === `${page.url.pathname}${page.url.search}`) void load();
		else void goto(target, { replace: true, reset: false });
	}

	/** Quick search: replaces the mask criteria and removes them from the address. */
	function quick() {
		criteria = null;
		applySelection();
	}

	/** Choose an article group: show only its articles (with subgroups). */
	function selectGroup(id: number | null) {
		quickSearch = '';
		criteria = null;
		groupFilter = id;
		applySelection();
	}

	/** Show only active, only inactive or all articles. */
	function selectActive(value: 'all' | 'active' | 'inactive') {
		quickSearch = '';
		criteria = null;
		activeFilter = value;
		applySelection();
	}

	function istZugeklappt(id: number) {
		return zugeklappt.includes(id);
	}

	function toggle(id: number) {
		zugeklappt = istZugeklappt(id) ? zugeklappt.filter((g) => g !== id) : [...zugeklappt, id];
	}

	const groupsWithChildren = $derived(
		new Set(
			groups
				.map((g) => g.parent_id)
				.filter((id): id is number => id !== null && id !== undefined)
		)
	);

	const expandAll = () => (zugeklappt = []);
	const collapseAll = () => (zugeklappt = [...groupsWithChildren]);

	/** Tree from the flat list: roots on top, children below. */
	function children(von: number | null): ArticleGroup[] {
		return groups.filter((g) => (g.parent_id ?? null) === von);
	}

	/** A chosen group also applies to its subgroups. */
	function istVorfahr(ancestor: number, id: number): boolean {
		let current = groups.find((g) => g.id === id);
		while (current?.parent_id != null) {
			if (current.parent_id === ancestor) return true;
			const parent: number = current.parent_id;
			current = groups.find((g) => g.id === parent);
		}
		return false;
	}

	/** Short form of the applied criteria for the header row. */
	function criteriaList(): string[] {
		if (!criteria) return [];
		const parts: string[] = [];
		if (criteria.article_type && criteria.article_type !== 'active') {
			parts.push(ARTICLE_TYPE_TEXT[criteria.article_type] ?? criteria.article_type);
		}
		if (criteria.name) {
			parts.push(
				criteria.include_descriptions
					? `Name (inkl. Beschreibungen) „${criteria.name}"`
					: `Name „${criteria.name}"`
			);
		}
		if (criteria.article_no) parts.push(`Artikel Nr. ${criteria.article_no}`);
		if (criteria.inventory_no) parts.push(`Inventar Nr. „${criteria.inventory_no}"`);
		if (criteria.serial_no) parts.push(`Serien-Nr. „${criteria.serial_no}"`);
		if (criteria.with_bundle) parts.push('Bundlematerial mit Zugehörigem');
		if (criteria.article_groups?.length) {
			const op = OPERATOR_TEXT[criteria.article_groups_op ?? 'contains'];
			parts.push(`Warengruppe ${op} (${criteria.article_groups.length})`);
		}
		if (criteria.search_index?.length) {
			const op = OPERATOR_TEXT[criteria.search_index_op ?? 'contains'];
			parts.push(`Suchindex ${op} ${criteria.search_index.join(', ')}`);
		}
		if (criteria.suppliers?.length) parts.push(`Lieferant ${criteria.suppliers.join(', ')}`);
		if (criteria.manufacturer) parts.push(`Hersteller „${criteria.manufacturer}"`);
		if (criteria.owner) parts.push(`Eigentümer „${criteria.owner}"`);
		if (criteria.exclude_virtual) parts.push('exkl. virtueller Artikel');
		if (criteria.pyro) parts.push('Pyro-Kriterien');
		return parts;
	}

	/** Quantities come as a decimal string; „—" means: no booking present. */
	function quantity(value: string | null): string {
		if (value === null) return '—';
		const n = Number(value);
		if (!Number.isFinite(n)) return value;
		return new Intl.NumberFormat('de-DE', { maximumFractionDigits: 4 }).format(n);
	}

	/** Prices as an amount with two decimals; without a price „—". */
	function money(value: string | null): string {
		if (value === null) return '—';
		const n = Number(value);
		if (!Number.isFinite(n)) return value;
		return new Intl.NumberFormat('de-DE', {
			minimumFractionDigits: 2,
			maximumFractionDigits: 2
		}).format(n);
	}

	/** A click on the row opens the article in the window (like the address list). */
	function openArticle(a: ArticleList) {
		openArticleEdit(a.id);
	}

	/**
	 * The search mask passes its criteria as query parameters; the same way
	 * carries reload, back button and the chosen article group.
	 */
	let lastQuery: string | null = null;
	$effect(() => {
		const query = page.url.search;
		if (query === lastQuery) return;
		lastQuery = query;
		criteria = articleSearchFromParams(page.url.searchParams);
		// `0` deliberately means „ohne Warengruppe" — no real group has the key 0.
		const groupParam = page.url.searchParams.get('group');
		const group = groupParam === null ? Number.NaN : Number(groupParam);
		groupFilter = Number.isInteger(group) && group >= 0 ? group : null;
		const active = page.url.searchParams.get('active');
		activeFilter = active === 'active' || active === 'inactive' ? active : 'all';
		if (page.url.searchParams.get('search') === '1') searchOpen = true;
		void load();
	});

	$effect(() => {
		if (!groups.length) void loadGroups();
	});
</script>

{#if !auth.hasRight(RIGHT.mayReadArticles)}
	<AccessDenied pageName="Artikelliste" right={RIGHT.mayReadArticles} rightLabel="Artikel lesen" />
{:else}
<h1>Artikel</h1>

<div class="page">
	<aside class="groups" aria-label="Warengruppen">
		<button
			type="button"
			class="entry"
			class:active={groupFilter === null && activeFilter === 'all'}
			aria-pressed={groupFilter === null && activeFilter === 'all'}
			onclick={() => {
				groupFilter = null;
				selectActive('all');
			}}>Alle Artikel zeigen</button
		>
		<button
			type="button"
			class="entry"
			class:active={activeFilter === 'active'}
			aria-pressed={activeFilter === 'active'}
			onclick={() => selectActive('active')}>Nur aktive Artikel zeigen</button
		>
		<button
			type="button"
			class="entry"
			class:active={activeFilter === 'inactive'}
			aria-pressed={activeFilter === 'inactive'}
			onclick={() => selectActive('inactive')}>Nur inaktive Artikel zeigen</button
		>

		<hr />

		<button
			type="button"
			class="entry"
			class:active={groupFilter === 0}
			aria-pressed={groupFilter === 0}
			title="Artikel, die in keiner Warengruppe stehen"
			onclick={() => selectGroup(0)}>Ohne Warengruppe</button
		>

		<hr />

		<button type="button" class="entry" onclick={expandAll}>Alles aufklappen</button>
		<button type="button" class="entry" onclick={collapseAll}>Alles zuklappen</button>

		<hr />

		{#snippet branch(nodes: ArticleGroup[], depth: number)}
			<ul class="tree" class:sublevel={depth > 0}>
				{#each nodes as node (node.id)}
					{@const hatKinder = groupsWithChildren.has(node.id)}
					<li>
						<div class="row">
							<button
								type="button"
								class="entry group"
								class:active={groupFilter === node.id}
								class:path={groupFilter !== null && istVorfahr(node.id, groupFilter)}
								aria-pressed={groupFilter === node.id}
								title={node.name ?? `Gruppe ${node.id}`}
								onclick={() => selectGroup(node.id)}
							>
								{node.name ?? `Gruppe ${node.id}`}
							</button>
							{#if hatKinder}
								<button
									type="button"
									class="arrow"
									aria-expanded={!istZugeklappt(node.id)}
									aria-label={istZugeklappt(node.id) ? 'aufklappen' : 'zuklappen'}
									title={istZugeklappt(node.id) ? 'aufklappen' : 'zuklappen'}
									onclick={() => toggle(node.id)}
								>
									{istZugeklappt(node.id) ? '▸' : '▾'}
								</button>
							{/if}
						</div>
						{#if hatKinder && !istZugeklappt(node.id)}
							{@render branch(children(node.id), depth + 1)}
						{/if}
					</li>
				{/each}
			</ul>
		{/snippet}

		{#if groups.length}
			{@render branch(children(null), 0)}
		{:else}
			<p class="empty">Keine Warengruppen angelegt.</p>
		{/if}
	</aside>

	<section>
		<h2>Bestand ({total})</h2>
		<div class="toolbar">
			<input
				placeholder="Schnellsuche (Name, Matchcode, Hersteller, Inventar, EAN, Art.-Nr.)"
				bind:value={quickSearch}
				onkeydown={(e) => {
					if (e.key !== 'Enter') return;
					quick();
				}}
			/>
			<button onclick={quick} disabled={loading}>{loading ? 'Lade …' : 'Schnellsuche'}</button>
			<button class="search" onclick={() => (searchOpen = true)}>Suche …</button>
		</div>

		{#if criteria}
			<div class="criteria">
				<span class="badge">Suchmaske:</span>
				{#each criteriaList() as part, i (i)}
					<span class="chip">{part}</span>
				{/each}
				<button type="button" class="reset" onclick={reset}>Suche zurücksetzen</button>
			</div>
		{/if}

		<table>
			<thead>
				<tr>
					<th>Art.-Nr.</th>
					<th>Inv.-Nr.</th>
					{#if pricingEnabled}
						<th class="amount">Bestand Miete</th>
						<th class="amount">Bestand Verkauf</th>
					{/if}
					<th>Name</th>
					{#if pricingEnabled}
						<th class="amount">Preis Miete</th>
						<th class="amount">Preis Verkauf</th>
					{/if}
				</tr>
			</thead>
			<tbody>
				{#each items as a (a.id)}
					<tr
						class:inactive={!a.active}
						tabindex="0"
						title="Artikel öffnen"
						onclick={() => openArticle(a)}
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								e.preventDefault();
								openArticle(a);
							}
						}}
					>
						<td class="number">{a.article_no}</td>
						<td class="number">{a.inventory_no ?? '—'}</td>
						{#if pricingEnabled}
							<td class="amount">{quantity(a.rental_stock)}</td>
							<td class="amount">{quantity(a.sale_stock)}</td>
						{/if}
						<td>{a.name ?? '—'}</td>
						{#if pricingEnabled}
							<td class="amount">{money(a.rental_price)}</td>
							<td class="amount">{money(a.sale_price)}</td>
						{/if}
					</tr>
				{:else}
					<tr>
						<td colspan={columns}>
							{criteria
								? 'Kein Artikel passt zu den Kriterien.'
								: groupFilter === 0
									? 'Alle Artikel stehen in einer Warengruppe.'
									: groupFilter !== null
										? 'Diese Warengruppe enthält keine Artikel.'
										: 'Keine Artikel vorhanden.'}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</section>
</div>

{#if error}
	<p class="bad">{error}</p>
{/if}

{#if searchOpen}
	<ArticleSearch
		criteria={criteria ?? { article_type: 'active' }}
		onsearch={applySearch}
		onclose={closeSearch}
	/>
{/if}
{/if}

<style>
	.bad {
		color: var(--bad);
	}
	h2 {
		font-size: 1.05rem;
		margin: 0 0 0.6rem;
	}

	/* Two columns as in the original: article groups on the left, list on the right. */
	.page {
		display: grid;
		grid-template-columns: 14rem minmax(0, 1fr);
		gap: 1rem;
		margin-top: 1rem;
		align-items: start;
	}
	@media (max-width: 900px) {
		.page {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	.groups {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 4px;
		padding: 0.35rem;
		font-size: 0.85rem;
	}
	.groups hr {
		border: none;
		border-top: 1px solid var(--border);
		margin: 0.35rem 0;
	}
	.entry {
		display: block;
		width: 100%;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		padding: 0.2rem 0.25rem;
		border-radius: 3px;
		cursor: pointer;
	}
	.entry:hover {
		background: var(--surface-3);
	}
	.entry.active {
		background: var(--accent);
		color: #fff;
	}
	/* Parent group of the chosen subgroup: only hinted, so the choice stays unique. */
	.entry.path {
		background: var(--surface-3);
		font-weight: 600;
	}
	.tree {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	/* Main groups flush with the column, subgroups slightly indented. */
	.tree.sublevel {
		padding-left: 0.9rem;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.05rem;
	}
	.row .group {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.arrow {
		flex: 0 0 auto;
		width: 1.1rem;
		padding: 0;
		border: none;
		background: none;
		color: var(--muted);
		cursor: pointer;
		font-size: 0.7rem;
		line-height: 1;
		text-align: center;
	}
	.arrow:hover {
		color: var(--text);
	}
	.empty {
		color: var(--muted-2);
		font-size: 0.8rem;
		margin: 0.3rem 0.35rem;
	}

	.toolbar {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	input {
		flex: 1;
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
	}
	button {
		padding: 0.4rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
	.toolbar button.search {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.criteria {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.35rem;
		margin-bottom: 0.75rem;
		font-size: 0.8rem;
	}
	.badge {
		color: var(--muted);
	}
	.chip {
		background: var(--surface-3);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 0.1rem 0.5rem;
	}
	.reset {
		border: none;
		background: none;
		color: var(--accent);
		cursor: pointer;
		padding: 0;
		font-size: 0.8rem;
		text-decoration: underline;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		background: var(--surface);
	}
	th,
	td {
		text-align: left;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.9rem;
	}
	th.amount,
	td.amount {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	td.number {
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	/* Whole row clickable, same hover as in the address list. */
	tbody tr {
		cursor: pointer;
	}
	tbody tr:hover > td,
	tbody tr:focus-visible > td {
		background: var(--surface-3);
	}
	tbody tr:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}
	tr.inactive > td {
		color: var(--muted-2);
	}
</style>
