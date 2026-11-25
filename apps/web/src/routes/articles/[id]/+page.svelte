<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { openArticleCreate, openArticleEdit } from '../../../lib/articles/articleDialogStore.svelte';
	import { auth } from '../../../lib/auth.svelte';
	import { RIGHT } from '../../../lib/rights';
	import AccessDenied from '../../../lib/AccessDenied.svelte';

	/**
	 * The article lives in the window (`ArticleDialog` in the layout). This route stays for
	 * bookmarks and links: it opens the window and switches the address to the
	 * list, so a reload does not tear open a window again. The navigation
	 * runs via `history` so the window state is not lost with the page.
	 */
	const idParam = $derived(page.params.id ?? 'new');
	const istNeu = $derived(idParam === 'new');

	onMount(() => {
		if (istNeu) openArticleCreate();
		else {
			const id = Number(idParam);
			if (Number.isInteger(id) && id > 0) openArticleEdit(id);
		}
		history.replaceState(history.state, '', `/articles${location.search}`);
	});
</script>

<svelte:head>
	<title>Artikel — WHERE</title>
</svelte:head>

{#if !auth.hasRight(RIGHT.mayReadArticles)}
	<AccessDenied pageName="Artikel" right={RIGHT.mayReadArticles} rightLabel="Artikel lesen" />
{:else if istNeu && !auth.hasRight(RIGHT.mayCreateArticle)}
	<AccessDenied
		pageName="Neuer Artikel"
		right={RIGHT.mayCreateArticle}
		rightLabel="Artikel anlegen"
	/>
{/if}
