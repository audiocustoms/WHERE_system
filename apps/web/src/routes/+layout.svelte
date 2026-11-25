<script lang="ts">
	import { onMount } from 'svelte';
	import favicon from '#lib/assets/favicon.svg';
	import TopMenu from '#lib/TopMenu.svelte';
	import SearchCenter from '#lib/search/SearchCenter.svelte';
	import AddressDialog from '#lib/addresses/AddressDialog.svelte';
	import ArticleDialog from '#lib/articles/ArticleDialog.svelte';
	import ImportDialog from '#lib/import/ImportDialog.svelte';
	import ConfirmDialog from '#lib/ConfirmDialog.svelte';
	import LoginMask from '#lib/LoginMask.svelte';
	import PasswordChangeDialog from '#lib/PasswordChangeDialog.svelte';
	import { auth } from '../lib/auth.svelte';
	import { modules } from '../lib/modules.svelte';
	import { searchOpen } from '../lib/search/searchOverlay.svelte';
	import { addressDialogOpen } from '../lib/addresses/addressDialogStore.svelte';
	import { articleDialogOpen } from '../lib/articles/articleDialogStore.svelte';
	import { importOpen } from '../lib/import/importStore.svelte';
	import { installWindowChrome } from '../lib/windowChrome';
	import { startEscape } from '../lib/escape.svelte';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	// `Esc` closes the topmost open window — one listener for all of them.
	startEscape();

	onMount(() => {
		auth.restore();
		// The product name is not in the register before sign-in yet.
		void modules.loadInstallationName();
		// Dialogs as under Windows: move and resize —
		// one listener for all masks.
		return installWindowChrome();
	});

	// The module register depends on the session: load it once after signing in
	// (and after a tenant change). Without a session it stays empty.
	$effect(() => {
		if (auth.user) void modules.load();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>{modules.productName}</title>
</svelte:head>

{#if auth.user}
	<div class="app">
		<TopMenu />

		<div class="shell">
			<main>
				<!-- While the first-installation password applies, no page loads:
				     the API blocks all work anyway. -->
				{#if auth.mustChangePassword}
					<p class="first-login">Bitte zuerst das Kennwort ändern.</p>
				{:else}
					{@render children()}
				{/if}
			</main>
		</div>
	</div>

	{#if !auth.mustChangePassword && searchOpen()}
		<SearchCenter />
	{/if}

	{#if !auth.mustChangePassword && addressDialogOpen()}
		<AddressDialog />
	{/if}

	{#if !auth.mustChangePassword && articleDialogOpen()}
		<ArticleDialog />
	{/if}

	{#if !auth.mustChangePassword && importOpen()}
		<ImportDialog />
	{/if}

	<!-- First installation: work cannot continue without a separate password. -->
	{#if auth.mustChangePassword}
		<PasswordChangeDialog />
	{/if}

	<ConfirmDialog />
{:else if !auth.restoring}
	<LoginMask />
{/if}

<style>
	.first-login {
		margin: 3rem auto;
		text-align: center;
		font-size: 0.9rem;
		color: var(--muted);
	}

	/* Colour palette. `app.html` sets data-theme before rendering. */
	:global(:root) {
		color-scheme: light;
		--bg: #f6f7f9;
		--surface: #ffffff;
		--surface-2: #f6f8fa;
		--surface-3: #eaeef2;
		--text: #1f2328;
		--muted: #57606a;
		--muted-2: #8b949e;
		--border: #aab3bd;
		--accent: #0969da;
		--good: #1a7f37;
		--bad: #cf222e;
		--highlight: #ddf4ff;
		--notice-bg: #fff8e6;
		--notice-border: #f0d99b;
		--menu-band-1: #f6f7f8;
		--menu-band-2: #e8eaee;
		--menu-band-border: #a7afb8;
		--menu-band-hover: #dde2e9;
		--menu-surface: #f2f2f2;
		--menu-border: #a6a6a6;
		--menu-separator: #b5b9be;
		--menu-hover: #dcf0ff;
		--menu-open: #cce8ff;
		--menu-shortcut: #6b7480;
		--menu-arrow: #8a929c;
	}

	:global(:root[data-theme='dark']) {
		color-scheme: dark;
		--bg: #14171c;
		--surface: #1c2027;
		--surface-2: #232830;
		--surface-3: #2b313a;
		--text: #e6e9ee;
		--muted: #a2acb9;
		--muted-2: #828c99;
		--border: #586374;
		--accent: #6cb6ff;
		--good: #56d364;
		--bad: #ff7b72;
		--highlight: #1f3a52;
		--notice-bg: #2b2717;
		--notice-border: #5c4c1f;
		--menu-band-1: #232830;
		--menu-band-2: #1b1f26;
		--menu-band-border: #4a5563;
		--menu-band-hover: #2b313a;
		--menu-surface: #1f242c;
		--menu-border: #5b6577;
		--menu-separator: #4a5563;
		--menu-hover: #2b3540;
		--menu-open: #2c4a63;
		--menu-shortcut: #98a2ae;
		--menu-arrow: #828c99;
	}

	:global(body) {
		margin: 0;
		font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
		color: var(--text);
		background: var(--bg);
	}

	/* Uniform native controls in both designs. */
	:global(input:not([type='checkbox']):not([type='radio'])),
	:global(select),
	:global(textarea) {
		background: var(--surface);
		color: var(--text);
	}
	:global(input::placeholder),
	:global(textarea::placeholder) {
		color: var(--muted-2);
	}
	:global(input:disabled),
	:global(select:disabled),
	:global(textarea:disabled) {
		background: var(--surface-2);
		color: var(--muted);
	}
	:global(input[type='checkbox']),
	:global(input[type='radio']) {
		accent-color: var(--accent);
	}
	:global(input[type='file']) {
		color: var(--muted);
	}
	:global(input[type='file']::file-selector-button) {
		margin-right: 0.5rem;
		padding: 0.25rem 0.6rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-2);
		color: var(--text);
		font-family: inherit;
		font-size: 0.82rem;
		cursor: pointer;
	}
	/* Header on top, page column on the left, content on the right. */
	.app {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		grid-template-rows: auto minmax(0, 1fr);
		grid-template-areas:
			'page-head page-head'
			'seite inhalt';
		min-height: 100vh;
	}
	.shell {
		grid-area: content;
		justify-self: center;
		width: 100%;
		max-width: 72rem;
		padding: 1.5rem;
		box-sizing: border-box;
	}
	/* A page that carries a wide board uses the whole window. */
	.shell:global(:has(> main > .wide)) {
		max-width: none;
	}
	main {
		min-height: 60vh;
	}
/* Heading of every page: same size, same spacing, same height —
   no matter whether it sits directly in `main` or in a `.page-head` with tools. */
:global(main h1) {
	margin: 0;
	font-size: 1.5rem;
	font-weight: 700;
	line-height: 1.9rem;
	letter-spacing: 0.01em;
}
:global(main > h1) {
	margin-bottom: 1rem;
}
:global(main > header.page-head) {
	margin-bottom: 1rem;
}
</style>
