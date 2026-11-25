<script lang="ts">
	import { goto } from '$app/navigation';
	import type { AddressSearchInput, ArticleSearchInput } from '../api';
	import SearchAreaDialog from './SearchAreaDialog.svelte';
	import AddressSearch from './AddressSearch.svelte';
	import ArticleSearch from '../ArticleSearch.svelte';
	import OrderSearch from '$modules/where-does-my-stuff-go/web/search/OrderSearch.svelte';
	import PackageSlipSearch from '$modules/where-does-my-stuff-go/web/search/PackageSlipSearch.svelte';
	import InvoiceSearch from '$modules/where-is-my-money/web/search/InvoiceSearch.svelte';
	import IncomingInvoiceSearch from '$modules/where-is-my-money/web/search/IncomingInvoiceSearch.svelte';
	import DocumentSearch from './DocumentSearch.svelte';
	import { modules } from '../modules.svelte';
	import { closeSearch, searchStart } from './searchOverlay.svelte';
	import { addressSearchParams } from './addressSearchQuery';
	import { articleSearchParams, isArticleSearchEmpty } from './articleSearchQuery';
	import type { SearchArea } from './areas';
	import { auth } from '../auth.svelte';
	import { RIGHT } from '../rights';

	const start = searchStart();

	/** Whether the window opens with the area choice or directly with a mask. */
	const fromChooser = start === 'bereich';

	/** Mounted while the search window is open — every call starts fresh. */
	let phase = $state<'bereich' | 'maske'>(fromChooser ? 'bereich' : 'maske');
	let area = $state<SearchArea>(
		start === 'articles' && auth.hasRight(RIGHT.mayReadArticles) ? 'articles' : 'addresses'
	);

	function openArea(id: SearchArea) {
		area = id;
		phase = 'maske';
	}

	/** Addresses: criteria as query parameters, the address page shows the hits. */
	function showAddresses(criteria: AddressSearchInput) {
		const query = addressSearchParams(criteria);
		closeSearch();
		void goto(`/addresses${query}`);
	}

	/**
	 * Articles: without criteria `Suchen` leads to the article list, with criteria
	 * to the hit list (`/articles?…`).
	 */
	function showArticles(criteria: ArticleSearchInput) {
		const query = isArticleSearchEmpty(criteria) ? '' : articleSearchParams(criteria);
		closeSearch();
		void goto(`/articles${query}`);
	}

	/** Abbrechen returns to the area choice — or closes the window. */
	function closeMask() {
		if (fromChooser) phase = 'bereich';
		else closeSearch();
	}

	/** Escape does the same as Abbrechen — without asking, like closing. */
	function onKeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape') return;
		event.preventDefault();
		if (phase === 'maske' && fromChooser) phase = 'bereich';
		else closeSearch();
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if phase === 'bereich'}
	<SearchAreaDialog onclose={closeSearch} onsearch={openArea} />
{:else if area === 'addresses'}
	<AddressSearch onclose={closeMask} onsearch={showAddresses} />
{:else if area === 'articles'}
	<ArticleSearch criteria={{ article_type: 'active' }} onclose={closeMask} onsearch={showArticles} />
{:else if area === 'orders' && modules.isInstalled('go')}
	<OrderSearch onclose={closeMask} />
{:else if area === 'package-slips' && modules.isInstalled('go')}
	<PackageSlipSearch onclose={closeMask} />
{:else if area === 'invoices' && modules.isInstalled('money')}
	<InvoiceSearch onclose={closeMask} />
{:else if area === 'incoming-invoices' && modules.isInstalled('money')}
	<IncomingInvoiceSearch onclose={closeMask} />
{:else if area === 'documents'}
	<DocumentSearch onclose={closeMask} />
{/if}
