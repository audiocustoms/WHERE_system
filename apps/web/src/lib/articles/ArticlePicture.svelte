<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type ArticleImage } from '../api';
	import { escapeWindows } from '../escape.svelte';

	interface Props {
		articleId: number | null;
		/** Without write right the picture stays visible, the buttons are missing. */
		schreibbar?: boolean;
	}

	let { articleId, schreibbar = true }: Props = $props();

	const PLACEHOLDER = '/article_placeholder.gif';

	let picture = $state<ArticleImage>({ file: null, placeholder: false });
	let pictureUrl = $state<string | null>(null);
	let gross = $state(false);
	let error = $state<string | null>(null);
	let laeuft = $state(false);

	/** The object URL of the loaded photo; old ones are released. */
	function setzeUrl(createNew: string | null) {
		if (pictureUrl && pictureUrl !== createNew) URL.revokeObjectURL(pictureUrl);
		pictureUrl = createNew;
	}

	async function load() {
		if (articleId === null) {
			picture = { file: null, placeholder: false };
			setzeUrl(null);
			return;
		}
		try {
			picture = await api.article.image.get(articleId);
			if (picture.file) {
				setzeUrl(URL.createObjectURL(await api.article.image.content(articleId, picture.file.id)));
			} else {
				setzeUrl(null);
			}
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function upload(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file || articleId === null) return;
		error = null;
		laeuft = true;
		try {
			picture = await api.article.image.upload(articleId, file);
			if (picture.file) {
				setzeUrl(URL.createObjectURL(await api.article.image.content(articleId, picture.file.id)));
			}
			input.value = '';
		} catch (e) {
			error = errorText(e, true);
		} finally {
			laeuft = false;
		}
	}

	async function placeholder(an: boolean) {
		if (articleId === null) return;
		error = null;
		try {
			picture = await api.article.image.placeholder(articleId, an);
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function remove() {
		if (articleId === null) return;
		error = null;
		try {
			picture = await api.article.image.remove(articleId);
			setzeUrl(null);
		} catch (e) {
			error = errorText(e, true);
		}
	}

	/** What the preview shows: `null` = no picture chosen yet. */
	const source = $derived(picture.placeholder ? PLACEHOLDER : pictureUrl);

	onMount(load);

	function close() {
		gross = false;
	}

	/** `Esc` closes only the large picture — the mask itself stays open. */
	$effect(() => {
		if (!gross) return;
		return escapeWindows(() => close());
	});
</script>

<figure class="article-picture">
	{#if articleId === null}
		<div class="frame empty">{t('Bild erst nach dem Speichern')}</div>
	{:else if source}
		<button
			type="button"
			class="frame"
			title={t('Bild in voller Größe anzeigen')}
			onclick={() => (gross = true)}
		>
			<img src={source} alt={t('Artikelbild')} loading="lazy" />
			{#if picture.placeholder}<span class="marker">{t('Platzhalter')}</span>{/if}
		</button>
	{:else}
		<div class="frame empty">{t('Kein Bild')}</div>
	{/if}

	{#if schreibbar && articleId !== null}
		<figcaption class="tools">
			<label class="tool-button">
				{t('Bild wählen')}
				<input type="file" accept="image/*" onchange={upload} disabled={laeuft} />
			</label>
			<button
				type="button"
				class="tool-button"
				class:active={picture.placeholder}
				title={t('Animierten Platzhalter statt eines Fotos zeigen')}
				onclick={() => placeholder(!picture.placeholder)}
			>
				{t('Platzhalter')}
			</button>
			{#if picture.file}
				<button type="button" class="tool-button" onclick={remove}>{t('entfernen')}</button>
			{/if}
		</figcaption>
	{/if}

	{#if error}
		<p class="error">{error}</p>
	{/if}
</figure>

{#if gross && source}
	<div class="backdrop" role="presentation">
		<div class="fullsize" role="dialog" tabindex="-1" aria-modal="true" aria-label={t('Artikelbild')}>
			<button type="button" class="close" title={t('Schließen')} onclick={close}>×</button>
			<img src={source} alt={t('Artikelbild in voller Größe')} />
		</div>
	</div>
{/if}

<style>
	.article-picture {
		margin: 0 0 0.75rem;
	}
	.frame {
		display: block;
		width: 100%;
		box-sizing: border-box;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-2);
		overflow: hidden;
		cursor: zoom-in;
		position: relative;
	}
	.frame img {
		display: block;
		width: 100%;
		height: auto;
		max-height: 11rem;
		object-fit: contain;
	}
	.frame.empty {
		padding: 2rem 0.75rem;
		min-height: 6rem;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.78rem;
		color: var(--muted-2);
		cursor: default;
	}
	.marker {
		position: absolute;
		left: 0.35rem;
		bottom: 0.35rem;
		padding: 0 0.3rem;
		font-size: 0.66rem;
		color: var(--muted);
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 3px;
	}
	.tools {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		margin-top: 0.3rem;
	}
	.tool-button {
		position: relative;
		padding: 0.2rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;
	}
	.tool-button.active {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.tool-button input[type='file'] {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}
	.error {
		margin: 0.3rem 0 0;
		font-size: 0.75rem;
		color: var(--bad);
	}

	/* Full size above the mask. */
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 210;
		display: grid;
		place-items: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 72%);
	}
	.fullsize {
		position: relative;
		max-width: min(90vw, 1100px);
		max-height: 90vh;
	}
	.fullsize img {
		display: block;
		max-width: 100%;
		max-height: 86vh;
		background: var(--surface);
		border-radius: 4px;
	}
	.close {
		position: absolute;
		top: -0.6rem;
		right: -0.6rem;
		width: 2rem;
		height: 2rem;
		border: 1px solid var(--border);
		border-radius: 50%;
		background: var(--surface);
		color: var(--text);
		font-size: 1.1rem;
		line-height: 1;
		cursor: pointer;
	}
</style>
