<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { auth } from '../../lib/auth.svelte';
	import { RIGHT } from '../../lib/rights';
	import AccessDenied from '../../lib/AccessDenied.svelte';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<!--
	Every page behind `Admin` depends on the right `administrative-einstellungen.zugriff`.
	Without that right the menu does not appear in the taskbar at all; whoever opens the
	address directly sees the notice here instead of the settings.
-->
{#if auth.isAdmin}
	{@render children()}
{:else if !auth.restoring}
	<AccessDenied
		pageName={t('Einstellungen')}
		right={RIGHT.admin}
		rightLabel={t('administrative Einstellungen')}
	/>
{/if}
