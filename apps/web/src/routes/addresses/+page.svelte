<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		api,
		errorText,
		type Address,
		type AddressSearchInput,
		type AddressSort,
		type AddressState
	} from '../../lib/api';
	import { addressSearchFromParams, describeAddressSearch } from '../../lib/search/addressSearchQuery';
	import { openAddressEdit } from '../../lib/addresses/addressDialogStore.svelte';
	import { auth } from '../../lib/auth.svelte';
	import { RIGHT } from '../../lib/rights';
	import AccessDenied from '../../lib/AccessDenied.svelte';

	let items = $state<Address[]>([]);
	let total = $state(0);
	let search = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);

	/** „Bestand" above the list: active, inactive or everything. */
	let bestandFilter = $state<AddressState>('active');
	let sort = $state<AddressSort>('name');
	let dir = $state<'asc' | 'desc'>('asc');

	/** Criteria from the search mask (`/addresses?type=…`); `null` = unfiltered. */
	let filter = $state<AddressSearchInput | null>(null);

	async function load() {
		loading = true;
		error = null;
		try {
			const result = filter
				? await api.addresses.search({ ...filter, limit: 100 })
				: await api.addresses.list({
						limit: 100,
						q: search || undefined,
						active: bestandFilter,
						sort,
						dir
					});
			items = result.items;
			total = result.total;
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	/** Quick search: clears a filter of the search mask. */
	function quickSearch() {
		if (filter) {
			filter = null;
			search = '';
			void goto('/addresses', { replace: true, reset: false });
		}
		void load();
	}

	const filterText = $derived(filter ? describeAddressSearch(filter) : []);

	/** Clears the criteria of the search mask and loads the full list again. */
	function resetFilter() {
		filter = null;
		search = '';
		void goto('/addresses', { replace: true, reset: false });
		void load();
	}

	/** A click on a column head sorts by it, a second click flips the direction. */
	function sortBy(column: AddressSort) {
		if (sort === column) dir = dir === 'asc' ? 'desc' : 'asc';
		else {
			sort = column;
			dir = 'asc';
		}
		void load();
	}

	function chooseState(next: AddressState) {
		bestandFilter = next;
		void load();
	}

	const columns: { key: AddressSort; title: string }[] = [
		{ key: 'type', title: 'Typ' },
		{ key: 'number', title: 'Kundennummer' },
		{ key: 'name', title: 'Name' },
		{ key: 'country', title: 'Land' },
		{ key: 'postal_code', title: 'PLZ' },
		{ key: 'city', title: 'Ort' }
	];

	const bestand: { key: AddressState; title: string }[] = [
		{ key: 'active', title: 'active' },
		{ key: 'inactive', title: 'inaktiv' },
		{ key: 'all', title: 'alle' }
	];

	/** Codes the address is filed under (order as in the role column). */
	function typeCodes(a: Address): string {
		return (
			[
				a.is_customer ? 'K' : null,
				a.is_interested ? 'I' : null,
				a.is_supplier ? 'L' : null,
				a.is_location ? 'Loc' : null,
				a.is_vip ? 'VIP' : null
			]
				.filter(Boolean)
				.join(' · ') || '—'
		);
	}

	/** Customer number; suppliers without one show their supplier number. */
	function number(a: Address): string {
		return String(a.customer_no ?? a.supplier_no ?? '—');
	}

	/** Communication stacked as in the original: phone, mobile, e-mail. */
	function communication(a: Address): string[] {
		return [a.phone, a.mobile, a.email].filter((value): value is string => !!value);
	}

	onMount(() => {
		// The search mask passes its criteria as query parameters.
		filter = addressSearchFromParams(page.url.searchParams);
		void load();
	});
</script>

{#if !auth.hasRight(RIGHT.mayReadAddresses)}
	<AccessDenied pageName="Adressen" right={RIGHT.mayReadAddresses} rightLabel="Adresse lesen" />
{:else}
<h1>Adressen</h1>

<section>
	<h2>Bestand ({total})</h2>
	{#if filterText.length}
		<p class="filter">
			Suchmaske: {filterText.join(' · ')}
			<button class="reset" onclick={resetFilter}>Filter zurücksetzen</button>
		</p>
	{/if}
	<div class="toolbar">
		<input
			placeholder="Suche (Name, Kundennr.)"
			bind:value={search}
			onkeydown={(e) => e.key === 'Enter' && quickSearch()}
		/>
		<button onclick={quickSearch} disabled={loading}>{loading ? 'Lade …' : 'Suchen'}</button>
		<span class="stock" role="group" aria-label="Bestand">
			{#each bestand as option (option.key)}
				<button
					class="choice"
					class:active={bestandFilter === option.key}
					aria-pressed={bestandFilter === option.key}
					disabled={loading || !!filter}
					title={filter ? 'Die Suchmaske bestimmt den Bestand — Filter zurücksetzen' : undefined}
					onclick={() => chooseState(option.key)}>{option.title}</button
				>
			{/each}
		</span>
	</div>
	<table>
		<thead>
			<tr>
				{#each columns as column (column.key)}
					<th>
						<button
							class="sort"
							disabled={!!filter}
							title={filter ? 'Die Suchmaske sortiert nach Name — Filter zurücksetzen' : 'Nach {spalte.titel} sortieren'}
							onclick={() => sortBy(column.key)}
						>
							{column.title}
							<span class="arrow" aria-hidden="true"
								>{sort === column.key ? (dir === 'asc' ? '▲' : '▼') : ''}</span
							>
						</button>
					</th>
				{/each}
				<th class="communication">Kommunikation</th>
			</tr>
		</thead>
		<tbody>
			{#each items as a (a.id)}
				<tr
					class:inactive={!a.active}
					tabindex="0"
					title="Adresse öffnen"
					onclick={() => openAddressEdit(a)}
					onkeydown={(e) => {
						if (e.key === 'Enter' || e.key === ' ') {
							e.preventDefault();
							openAddressEdit(a);
						}
					}}
				>
					<td class="type">{typeCodes(a)}</td>
					<td class="number">{number(a)}</td>
					<td>{a.name1 ?? '—'}</td>
					<td>{a.country ?? '—'}</td>
					<td class="postal">{a.postal_code ?? '—'}</td>
					<td>{a.city ?? '—'}</td>
					<td class="communication">
						{#each communication(a) as line, index (index)}
							<span>{line}</span>
						{:else}
							—
						{/each}
					</td>
				</tr>
			{:else}
				<tr class="empty"><td colspan="7">Keine Datensätze.</td></tr>
			{/each}
		</tbody>
	</table>
</section>

{#if error}
	<p class="bad">{error}</p>
{/if}
{/if}

<style>
	.bad {
		color: var(--bad);
	}
	h2 {
		font-size: 1.05rem;
		margin-top: 1rem;
	}
	input {
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font-size: 0.85rem;
	}
	button {
		padding: 0.4rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
	.toolbar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	/* „Bestand": active / inactive / all as a connected button row. */
	.stock {
		display: flex;
		margin-left: auto;
	}
	.stock .choice {
		padding: 0.35rem 0.8rem;
		border-radius: 0;
		font-size: 0.8rem;
	}
	.stock .choice:first-child {
		border-radius: 4px 0 0 4px;
	}
	.stock .choice:last-child {
		border-radius: 0 4px 4px 0;
	}
	.stock .choice + .choice {
		border-left: none;
	}
	.stock .choice.active {
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}
	.filter {
		margin: 0 0 0.6rem;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.reset {
		margin-left: 0.4rem;
		padding: 0;
		border: none;
		background: none;
		color: var(--accent);
		font-size: 0.8rem;
		cursor: pointer;
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
	th {
		padding: 0;
	}
	/* The column head is a button so the whole cell sorts. */
	.sort {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		width: 100%;
		padding: 0.4rem 0.6rem;
		border: none;
		border-radius: 0;
		background: none;
		color: inherit;
		font-family: inherit;
		font-size: 0.9rem;
		font-weight: 600;
	}
	.sort:hover {
		background: var(--surface-3);
	}
	/* Without a stock and sort choice the column head stays plain text. */
	.sort:disabled,
	.stock .choice:disabled {
		cursor: default;
		opacity: 0.6;
	}
	.sort:disabled:hover {
		background: none;
	}
	.arrow {
		color: var(--accent);
		font-size: 0.7rem;
	}
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
	.type,
	.number,
	.postal {
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.communication span {
		display: block;
	}
	tr.empty {
		cursor: default;
	}
</style>
