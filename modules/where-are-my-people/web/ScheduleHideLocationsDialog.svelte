<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import type { PeopleListEntry } from '$core/api';
	import { escapeWindows } from '$core/escape.svelte';
	import '$core/search/search-window.css';

	interface Props {
		/** The locations of the disposition the board shows. */
		locations: PeopleListEntry[];
		/** The locations the board leaves out. */
		hidden: Set<number>;
		/** A location was switched: `hidden = true` hides it from the board. */
		ontoggle: (id: number, hidden: boolean) => void;
		onclose: () => void;
	}

	let { locations, hidden, ontoggle, onclose }: Props = $props();

	// `Esc` closes the window like `Schließen`.
	$effect(() => escapeWindows(() => onclose()));
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label={t('Ausblenden')}
		style="width: min(20rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Ausblenden')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<ul class="choices">
			{#each locations as location (location.id)}
				<li>
										<label class="entry">
						<input
							type="checkbox"
							checked={!hidden.has(location.id)}
							onchange={(event) => ontoggle(location.id, !event.currentTarget.checked)}
						/>
						{#if location.color}
							<span class="tint" style={`background:${location.color}`} aria-hidden="true"></span>
						{/if}
						{location.name}
					</label>
				</li>
			{/each}
		</ul>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={onclose}>{t('Schließen')}</button>
		</div>
	</div>
</div>

<style>
	.choices {
		list-style: none;
		margin: 0;
		padding: 0.35rem;
		/* The window caps its own height; the list scrolls only when it has to. */
		min-height: 0;
		overflow-y: auto;
	}
	.entry {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		box-sizing: border-box;
		width: 100%;
		padding: 0.3rem 0.5rem;
		border-radius: 4px;
		font-size: 0.9rem;
		cursor: pointer;
	}
	.entry:hover {
		background: color-mix(in srgb, var(--accent) 10%, transparent);
	}
	.entry input {
		margin: 0;
		accent-color: var(--accent);
	}
	.tint {
		width: 0.6rem;
		height: 0.6rem;
		border-radius: 2px;
		flex: none;
	}
</style>
