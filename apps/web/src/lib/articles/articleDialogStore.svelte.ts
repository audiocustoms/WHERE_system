import type { Article } from '../api';

/**
 * State of the article window. `Artikel +` and a click on a row of the
 * article list open the same mask — as with the addresses. The record is loaded
 * in the window, so the list does not have to supply anything.
 */
export type ArticleDialog = { mode: 'create' } | { mode: 'edit'; id: number };

let dialog = $state<ArticleDialog | null>(null);

export const articleDialog = () => dialog;

export const articleDialogOpen = () => dialog !== null;

export function openArticleCreate() {
	dialog = { mode: 'create' };
}

export function openArticleEdit(id: number) {
	dialog = { mode: 'edit', id };
}

/** For callers that already hold a record. */
export function openArticleEditWith(article: Article) {
	if (article.id === null) openArticleCreate();
	else openArticleEdit(article.id);
}

export function closeArticleDialog() {
	dialog = null;
}
