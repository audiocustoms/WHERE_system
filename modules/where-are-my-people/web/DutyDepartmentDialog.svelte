<script lang="ts">
	import type { PeopleListEntry } from '$core/api';
	import { escapeWindows } from '$core/escape.svelte';
	import '$core/search/search-window.css';

	interface Props {
		/** The department the board shows; empty = every one. */
		selected: string;
		/** The departments of the master data. */
		departments: PeopleListEntry[];
		/** A department was picked; empty means `alle`. */
		onpick: (name: string) => void;
		/** The window was closed without a choice. */
		onclose: () => void;
	}

	let { selected, departments, onpick, onclose }: Props = $props();

	// `Esc` closes the window like `Schließen`.
	$effect(() => escapeWindows(() => onclose()));
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label="Zeige Abteilung"
		style="width: min(18rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">Zeige Abteilung</span>
			<button class="sw-close" type="button" title="Schließen" onclick={onclose}>×</button>
		</div>

		<ul class="choices">
			<li>
				<button
					type="button"
					class="entry"
					class:selected={selected === ''}
					aria-current={selected === '' ? 'true' : undefined}
					onclick={() => onpick('')}>alle</button
				>
			</li>
			{#each departments as entry (entry.id)}
				<li>
					<button
						type="button"
						class="entry"
						class:selected={selected === entry.name}
						aria-current={selected === entry.name ? 'true' : undefined}
						onclick={() => onpick(entry.name)}
					>
						{#if entry.color}
							<span class="tint" style={`background:${entry.color}`} aria-hidden="true"></span>
						{/if}
						{entry.name}
					</button>
				</li>
			{/each}
		</ul>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={onclose}>Schließen</button>
		</div>
	</div>
</div>

<style>
	.choices {
		list-style: none;
		margin: 0;
		padding: 0.35rem;
		max-height: min(24rem, 60vh);
		overflow: auto;
	}
	.entry {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		width: 100%;
		padding: 0.35rem 0.5rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: 0.9rem;
		text-align: left;
		cursor: pointer;
	}
	.entry:hover {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 10%, transparent);
	}
	.entry.selected {
		background: color-mix(in srgb, var(--accent) 16%, transparent);
		font-weight: 600;
	}
	.tint {
		width: 0.6rem;
		height: 0.6rem;
		border-radius: 2px;
		flex: none;
	}
</style>
