<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '../escape.svelte';
	import type { Snippet } from 'svelte';
	import './search-window.css';

	export interface BandOption {
		value: string;
		text: string;
	}

	interface Props {
		/** Window title of the title bar. */
		title: string;
		/** Width of the window as a CSS length. */
		width?: string;
		/** Label of the dark bar; without it the bar is left out. */
		bandLabel?: string;
		/** Fixed text of the bar when no choice is offered. */
		bandText?: string;
		bandOptions?: BandOption[];
		bandValue?: string;
		onbandchange?: (value: string) => void;
		/** Right-hand choice of the bar („Suchtext enthält…"). */
		modeOptions?: BandOption[];
		modeValue?: string;
		onmodechange?: (value: string) => void;
		children: Snippet;
		/** Button row (Abbrechen / Sucheditor / Suchen); `cancel` closes without asking. */
		actions: Snippet<[() => void]>;
		onclose: () => void;
	}

	let {
		title,
		width = '33rem',
		bandLabel,
		bandText,
		bandOptions,
		bandValue = '',
		onbandchange,
		modeOptions,
		modeValue = '',
		onmodechange,
		children,
		actions,
		onclose
	}: Props = $props();

	/**
	 * Closing does **not** ask: a search window holds only criteria, no
	 * input that could be lost. The `Abbrechen` button of the mask runs
	 * through the same path (`actions(cancel)`).
	 */
	function cancel() {
		onclose();
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => cancel()));
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		style="width: min({width}, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{title}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={cancel}>×</button>
		</div>

		{#if bandLabel}
			<div class="sw-band">
				<span class="sw-band-marker">{bandLabel}</span>
				{#if bandOptions}
					<select
						class="sw-band-select"
						value={bandValue}
						onchange={(e) => onbandchange?.(e.currentTarget.value)}
						aria-label={t('Vorgang')}
					>
						{#each bandOptions as option (option.value)}
							<option value={option.value}>{option.text}</option>
						{/each}
					</select>
				{:else}
					<span class="sw-band-text">{bandText}</span>
				{/if}
				{#if modeOptions}
					<select
						class="sw-band-mode"
						value={modeValue}
						onchange={(e) => onmodechange?.(e.currentTarget.value)}
						aria-label={t('Suchtext')}
					>
						{#each modeOptions as option (option.value)}
							<option value={option.value}>{option.text}</option>
						{/each}
					</select>
				{/if}
			</div>
		{/if}

		<div class="sw-content">
			{@render children()}
		</div>

		<div class="sw-footer">
			{@render actions(cancel)}
		</div>
	</div>
</div>
