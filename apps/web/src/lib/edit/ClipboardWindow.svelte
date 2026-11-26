<script lang="ts">
	import { t } from '$core/i18n.svelte';
	import { escapeWindows } from '../escape.svelte';
	import { clipboardOpen, hideClipboard } from './clipboard.svelte';
	import { readClipboard, runEditorCommand } from './editorCommands';
	import { showStatus } from '../statusMessage.svelte';
	import '../search/search-window.css';

	const open = $derived(clipboardOpen());
	let content = $state<string | null>(null);
	let denied = $state(false);
	let loading = $state(false);

	/** Liest die Zwischenablage neu — beim Öffnen und auf Wunsch. */
	async function read() {
		loading = true;
		denied = false;
		const text = await readClipboard();
		loading = false;
		if (text === null) {
			denied = true;
			content = null;
			return;
		}
		content = text;
	}

	$effect(() => {
		if (open) void read();
	});

	$effect(() => {
		if (!open) return;
		return escapeWindows(hideClipboard);
	});

	async function insert() {
		if (!content) return;
		const ok = await runEditorCommand('paste');
		hideClipboard();
		if (!ok) showStatus(t('Kein Eingabefeld aktiv.'));
	}
</script>

{#if open}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={t('Zwischenablage')}
			style="width: min(34rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{t('Zwischenablage')}</span>
				<button class="sw-close" type="button" title={t('Schließen')} onclick={hideClipboard}>
					×
				</button>
			</div>

			<div class="sw-content">
				{#if denied}
					<p class="sw-error">{t('Der Browser gibt den Inhalt der Zwischenablage nicht heraus.')}</p>
				{:else if loading}
					<p class="sw-small">{t('Lade …')}</p>
				{:else if content}
					<textarea class="text" readonly rows="12">{content}</textarea>
					<p class="sw-small">{content.length} Zeichen</p>
				{:else}
					<p class="sw-small">{t('Die Zwischenablage ist leer.')}</p>
				{/if}
			</div>

			<div class="sw-footer">
				<button class="sw-button" type="button" onclick={read} disabled={loading}>
					{t('Aktualisieren')}
				</button>
				<button class="sw-button sw-primary" type="button" onclick={insert} disabled={!content}>
					{t('Einsetzen')}
				</button>
				<button class="sw-button" type="button" onclick={hideClipboard}>{t('Schließen')}</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.text {
		width: 100%;
		box-sizing: border-box;
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
		resize: vertical;
	}
</style>
