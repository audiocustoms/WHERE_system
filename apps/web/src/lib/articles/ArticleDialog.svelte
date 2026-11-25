<script lang="ts">
	import { escapeWindows } from '../escape.svelte';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import ArticleForm from '../ArticleForm.svelte';
	import FileStorage from '../FileStorage.svelte';
	import { askCancel } from '../confirm.svelte';
	import { api, errorText, type Article, type ArticleGroup, type Language } from '../api';
	import { auth } from '../auth.svelte';
	import { RIGHT } from '../rights';
	import { articleDialog, closeArticleDialog } from './articleDialogStore.svelte';
	import '../search/search-window.css';
	import './article-window.css';

	const dialog = articleDialog();
	const id = dialog?.mode === 'edit' ? dialog.id : null;

	let article = $state<Article | null>(null);
	let article_groups = $state<ArticleGroup[]>([]);
	let languages = $state<Language[]>([]);
	let busy = $state(false);
	let error = $state<string | null>(null);

	const schreibbar = $derived(
		id === null ? auth.hasRight(RIGHT.mayCreateArticle) : auth.hasRight(RIGHT.mayEditArticle)
	);

	/** Creating and reading are two rights: the mask opens only with its own. */
	const maySee = $derived(
		auth.hasRight(RIGHT.mayReadArticles) && (id !== null || auth.hasRight(RIGHT.mayCreateArticle))
	);

	onMount(() => {
		if (maySee) void load();
	});

	async function load() {
		error = null;
		try {
			if (!article_groups.length) article_groups = await api.masterData.articleGroups();
			if (!languages.length) languages = await api.masterData.languages();
			article = id === null ? await api.article.new() : await api.article.get(id);
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function save(updated: Article) {
		busy = true;
		error = null;
		try {
			await (updated.id === null
				? api.article.create(updated)
				: api.article.replace(updated.id, updated));
			close();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			busy = false;
		}
	}

	/** Closing without saving asks first — as in all masks. */
	async function cancel() {
		if (await askCancel(id === null ? 'Das Anlegen des Artikels' : 'Das Bearbeiten des Artikels')) {
			close();
		}
	}

	/** After saving: window closed, the list behind shows the record. */
	function close() {
		closeArticleDialog();
		if (location.pathname.startsWith('/articles')) location.reload();
		else void goto('/articles');
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void cancel()));
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window art-window"
		role="dialog"
		aria-modal="true"
		aria-label="Artikel"
		style="width: min(76rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">Artikel:</span>
			<button class="sw-close" type="button" title="Schließen" onclick={cancel}>×</button>
		</div>

		<div class="art-content">
			{#if !maySee}
				<p class="art-message bad">
					Für diese Aktion fehlt das Recht
					{id === null ? '„Artikel anlegen“' : '„Artikel lesen“'}.
				</p>
			{:else if error && !article}
				<p class="art-message bad">{error}</p>
			{:else if !article}
				<p class="art-message">Lade Artikel …</p>
			{:else}
				{#if error}
					<p class="art-message bad">{error}</p>
				{/if}
				{#key `${article.id ?? 'new'}`}
					<ArticleForm
						article={article}
						{article_groups}
						{languages}
						{busy}
						{schreibbar}
						imFenster
						onsave={save}
						oncancel={close}
					>
						{#snippet actions()}
							{#if article && article.id !== null}
								<FileStorage articleId={article.id} schreibbar={schreibbar} />
							{/if}
						{/snippet}
					</ArticleForm>
				{/key}
			{/if}
		</div>
	</div>
</div>
