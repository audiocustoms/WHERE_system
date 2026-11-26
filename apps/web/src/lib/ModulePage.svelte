<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import type { Snippet } from 'svelte';
	import type { ModuleId } from './api';
	import { modules } from './modules.svelte';

	/**
	 * Frame for a page that belongs to a module. If the module is not installed, the
	 * page explains that and leads to the module administration instead of showing
	 * the mask with empty data.
	 */
	interface Props {
		id: ModuleId;
		product: string;
		children: Snippet;
	}

	let { id, product, children }: Props = $props();
</script>

{#if modules.isInstalled(id)}
	{@render children()}
{:else}
	<header class="page-head">
		<h1>{product}</h1>
	</header>
	<div class="notice">
		<strong>{t('Dieses Modul ist nicht installiert.')}</strong>
		<p>
			{t('Die Seite gehört zum Modul {name}. Installieren Sie es in der', { name: product })}
			<a href="/admin/modules">{t('Modulverwaltung')}</a>.
		</p>
	</div>
{/if}

<style>
	.notice {
		margin-top: 1rem;
		padding: 1rem;
		border: 1px solid var(--notice-border);
		background: var(--notice-bg);
		border-radius: 6px;
	}
	.notice p {
		margin: 0.4rem 0 0;
		color: var(--muted);
	}
</style>
