<script lang="ts">
	import { api, errorText, type Device, type InspectionReport } from '../api';
	import { escapeWindows } from '../escape.svelte';

	interface Props {
		articleId: number;
		device: Device;
		/** Without write right the list stays readable, upload and remove are missing. */
		schreibbar?: boolean;
	}

	let { articleId, device, schreibbar = true }: Props = $props();

	let reports = $state<InspectionReport[]>([]);
	let error = $state<string | null>(null);
	let hint = $state<string | null>(null);
	let laeuft = $state(false);

	/** The currently open PDF: object URL and title. */
	let view = $state<{ url: string; title: string } | null>(null);

	/** Serial and inventory no. are the anchor of the report. */
	const binding = $derived(
		[device.serial_no ? `SN ${device.serial_no}` : 'ohne Serien-Nr.', device.inventory_no ? `Inv ${device.inventory_no}` : null]
			.filter(Boolean)
			.join(' · ')
	);

	async function load() {
		try {
			reports = await api.article.reports.ofDevice(articleId, device.id);
		} catch (e) {
			error = errorText(e, true);
		}
	}

	// Another device shows other reports.
	$effect(() => {
		void device.id;
		void articleId;
		reports = [];
		error = null;
		view = null;
		void load();
	});

	async function upload(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		error = null;
		hint = null;
		laeuft = true;
		try {
			const createNew = await api.article.reports.upload(articleId, device.id, file);
			hint = `„${createNew.title}" hinterlegt.`;
			input.value = '';
			await load();
		} catch (e) {
			error = errorText(e, true);
		} finally {
			laeuft = false;
		}
	}

	async function oeffnen(report: InspectionReport) {
		error = null;
		try {
			const blob = await api.article.reports.content(articleId, device.id, report.id);
			if (view) URL.revokeObjectURL(view.url);
			view = { url: URL.createObjectURL(blob), title: report.title };
		} catch (e) {
			error = errorText(e, true);
		}
	}

	function close() {
		if (view) URL.revokeObjectURL(view.url);
		view = null;
	}

	async function remove(report: InspectionReport) {
		if (!confirm(`Prüfprotokoll „${report.title}" entfernen?`)) return;
		error = null;
		try {
			await api.article.reports.remove(articleId, device.id, report.id);
			await load();
		} catch (e) {
			error = errorText(e, true);
		}
	}

	function date(value: string | null): string {
		return value ? new Date(value).toLocaleDateString('de-DE') : '—';
	}

	function size(bytes: number | null): string {
		if (bytes === null) return '—';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	/** `Esc` closes the report view, not the mask below it. */
	$effect(() => {
		if (!view) return;
		return escapeWindows(() => close());
	});
</script>

<div class="reports">
	<p class="binding">{binding}</p>

	{#if schreibbar}
		<label class="tool-button">
			PDF hinzufügen
			<input type="file" accept="application/pdf,.pdf" onchange={upload} disabled={laeuft} />
		</label>
	{/if}

	{#if error}
		<p class="bad">{error}</p>
	{/if}
	{#if hint}
		<p class="good">{hint}</p>
	{/if}

	<table>
		<thead>
			<tr><th>Protokoll</th><th class="number">Größe</th><th>Datum</th><th></th></tr>
		</thead>
		<tbody>
			{#each reports as p (p.id)}
				<tr>
					<td><button type="button" class="title" onclick={() => oeffnen(p)}>{p.title}</button></td>
					<td class="number">{size(p.size)}</td>
					<td>{date(p.uploaded_at)}</td>
					<td class="action">
						{#if schreibbar}
							<button type="button" class="remove" title="Entfernen" onclick={() => remove(p)}
								>entfernen</button
							>
						{/if}
					</td>
				</tr>
			{:else}
				<tr><td colspan="4" class="empty">Noch kein Prüfprotokoll hinterlegt.</td></tr>
			{/each}
		</tbody>
	</table>
</div>

{#if view}
	<div class="backdrop" role="presentation">
		<div class="viewer" role="dialog" tabindex="-1" aria-modal="true" aria-label={view.title}>
			<header>
				<span class="title-text">{view.title}</span>
				<a class="tool-button" href={view.url} download={`${view.title}.pdf`}>herunterladen</a>
				<button type="button" class="close" title="Schließen" onclick={close}>×</button>
			</header>
			<iframe src={view.url} title={view.title}></iframe>
		</div>
	</div>
{/if}

<style>
	.reports {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.binding {
		margin: 0;
		font-size: 0.78rem;
		color: var(--muted);
	}
	.tool-button {
		position: relative;
		align-self: flex-start;
		padding: 0.25rem 0.6rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.78rem;
		cursor: pointer;
	}
	.tool-button input[type='file'] {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.78rem;
	}
	th,
	td {
		text-align: left;
		padding: 0.25rem 0.35rem;
		border-bottom: 1px solid var(--surface-3);
	}
	th {
		color: var(--muted);
		font-weight: 600;
	}
	td.number,
	th.number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	td.action {
		text-align: right;
		white-space: nowrap;
	}
	.title {
		border: 0;
		background: none;
		padding: 0;
		color: var(--accent);
		font: inherit;
		font-size: 0.78rem;
		text-align: left;
		cursor: pointer;
		text-decoration: underline;
	}
	.remove {
		border: 0;
		background: none;
		padding: 0;
		color: var(--bad);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;
	}
	.empty {
		color: var(--muted-2);
	}
	.bad {
		margin: 0;
		font-size: 0.78rem;
		color: var(--bad);
	}
	.good {
		margin: 0;
		font-size: 0.78rem;
		color: var(--good);
	}

	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 215;
		display: grid;
		place-items: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 65%);
	}
	.viewer {
		display: flex;
		flex-direction: column;
		width: min(64rem, 92vw);
		height: min(52rem, 90vh);
		background: var(--surface);
		border-radius: 6px;
		overflow: hidden;
	}
	.viewer header {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.5rem 0.7rem;
		border-bottom: 1px solid var(--border);
	}
	.title-text {
		flex: 1;
		font-size: 0.9rem;
		font-weight: 600;
	}
	.close {
		width: 1.9rem;
		height: 1.9rem;
		border: 1px solid var(--border);
		border-radius: 50%;
		background: var(--surface-2);
		color: var(--text);
		font-size: 1rem;
		line-height: 1;
		cursor: pointer;
	}
	iframe {
		flex: 1;
		width: 100%;
		border: 0;
		background: var(--surface-2);
	}
</style>
