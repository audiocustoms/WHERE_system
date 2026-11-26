<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type ArticleFile } from './api';

	interface Props {
		articleId: number;
		/** Without write permission the storage stays readable, but without upload/link/delete. */
		schreibbar?: boolean;
	}

	let { articleId, schreibbar = true }: Props = $props();

	let open = $state(false);
	let entries = $state<ArticleFile[]>([]);
	let allVersions = $state(false);
	let error = $state<string | null>(null);
	let laeuft = $state(false);
	let folder = $state('');
	let linkName = $state('');
	let linkTarget = $state('');

	async function load() {
		laeuft = true;
		error = null;
		try {
			entries = allVersions
				? await api.article.files.history(articleId)
				: await api.article.files.list(articleId);
		} catch (e) {
			error = errorText(e);
		} finally {
			laeuft = false;
		}
	}

	async function toggle() {
		open = !open;
		if (open) await load();
	}

	async function upload(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		error = null;
		try {
			await api.article.files.upload(articleId, file, folder);
			input.value = '';
			await load();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function linkHinzufuegen(event: SubmitEvent) {
		event.preventDefault();
		if (!linkName.trim() || !linkTarget.trim()) return;
		error = null;
		try {
			await api.article.files.link(articleId, {
				folder,
				name: linkName.trim(),
				target: linkTarget.trim()
			});
			linkName = '';
			linkTarget = '';
			await load();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	async function remove(id: number) {
		if (!confirm(t('Eintrag aus der Dateiablage entfernen?'))) return;
		try {
			await api.article.files.remove(articleId, id);
			await load();
		} catch (e) {
			error = errorText(e);
		}
	}

	function size(bytes: number | null): string {
		if (bytes === null) return '—';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	onMount(() => {
		// Determine the count for the button on first display.
		void load();
	});
</script>

<div class="storage">
	<button type="button" class="button" onclick={toggle}>
		{t('Dateiablage ({count})', { count: entries.filter((e) => e.is_current).length })}
	</button>

	{#if open}
		<div class="panel">
			<div class="tools">
				{#if schreibbar}
					<label class="field">
						<span>{t('Ordner')}</span>
						<input bind:value={folder} placeholder={t('(Wurzel)')} />
					</label>
					<label class="field">
						<span>{t('Datei hinzufügen')}</span>
						<input type="file" onchange={upload} />
					</label>
				{/if}
				<label class="check">
					<input type="checkbox" bind:checked={allVersions} onchange={load} />
					{t('alle Versionen')}
				</label>
				<button type="button" class="button" onclick={load} disabled={laeuft}>{t('Aktualisieren')}</button>
			</div>

			{#if schreibbar}
				<form class="link" onsubmit={linkHinzufuegen}>
					<input bind:value={linkName} placeholder={t('Name des Links')} />
					<input bind:value={linkTarget} placeholder={t('https://…')} />
					<button type="submit" class="button">{t('Link hinzufügen')}</button>
				</form>
			{/if}

			{#if error}
				<p class="bad">{error}</p>
			{/if}

			<table>
				<thead>
					<tr><th>{t('Ordner')}</th><th>{t('Name')}</th><th>{t('Größe')}</th><th>{t('Version')}</th><th>{t('Hochgeladen')}</th><th></th></tr>
				</thead>
				<tbody>
					{#each entries as e (e.id)}
						<tr class:old={!e.is_current}>
							<td>{e.folder || '/'}</td>
							<td>
								{#if e.is_link}
									<a href={e.link_target ?? '#'} target="_blank" rel="noreferrer">{e.name}</a>
								{:else}
									<a href={api.article.files.contentUrl(articleId, e.id)}>{e.name}</a>
								{/if}
							</td>
							<td>{size(e.size)}</td>
							<td>{e.version}</td>
							<td>{e.uploaded_at ? new Date(e.uploaded_at).toLocaleString('de-DE') : '—'}</td>
							<td>
								{#if schreibbar}
									<button type="button" class="delete" onclick={() => remove(e.id)}>{t('entfernen')}</button>
								{/if}
							</td>
						</tr>
					{:else}
						<tr><td colspan="6" class="empty">{t('Noch keine Dateien oder Links.')}</td></tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<style>
	.storage {
		position: relative;
	}
	.button {
		padding: 0.45rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.85rem;
	}
	.button:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.panel {
		position: absolute;
		bottom: 2.6rem;
		left: 0;
		z-index: 20;
		width: min(56rem, 90vw);
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		box-shadow: 0 8px 24px rgba(31, 35, 40, 0.15);
		padding: 0.75rem;
	}
	.tools {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
		flex-wrap: wrap;
		margin-bottom: 0.5rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.75rem;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.8rem;
	}
	input {
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font-family: inherit;
		font-size: 0.8rem;
	}
	input[type='file'] {
		border: none;
		padding: 0;
	}
	.link {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.6rem;
	}
	.link input {
		flex: 1;
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		text-align: left;
		padding: 0.3rem 0.45rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.8rem;
	}
	th {
		color: var(--muted);
		font-weight: 600;
	}
	tr.old {
		color: var(--muted-2);
	}
	.delete {
		border: none;
		background: none;
		color: var(--bad);
		cursor: pointer;
		padding: 0;
		font-size: 0.8rem;
	}
	.empty {
		color: var(--muted-2);
	}
	.bad {
		color: var(--bad);
		font-size: 0.8rem;
	}
</style>
