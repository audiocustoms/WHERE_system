<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '$core/escape.svelte';
	import { onMount } from 'svelte';
	import '$core/search/search-window.css';

	interface Props {
		/** Text of the chosen cell, e.g. `F - 08:00–15:30`. */
		cell: string;
		/** First day of the series as `YYYY-MM-DD`. */
		from: string;
		/** Days in a row the planning rules allow (`max_consecutive_days`). */
		maxDays?: number | null;
		/** The number of days was confirmed. */
		onapply: (days: number) => void;
		/** The window was closed without a number. */
		onclose: () => void;
	}

	let { cell, from, maxDays = null, onapply, onclose }: Props = $props();

	/** Longest series the server writes at once. */
	const MAX_DAYS = 366;

	let days = $state(1);
	let field: HTMLInputElement | null = $state(null);

	function dayOf(iso: string): Date {
		const [year, month, day] = iso.split('-').map(Number);
		return new Date(year, month - 1, day);
	}

	function longDate(date: Date): string {
		return date.toLocaleDateString('de-DE', {
			weekday: 'short',
			day: '2-digit',
			month: '2-digit',
			year: 'numeric'
		});
	}

	/** Days really written: whole numbers between 1 and the limit. */
	const count = $derived(
		Math.min(MAX_DAYS, Math.max(1, Math.trunc(Number.isFinite(days) ? days : 1)))
	);

	const lastDay = $derived.by(() => {
		const start = dayOf(from);
		return new Date(start.getFullYear(), start.getMonth(), start.getDate() + count - 1);
	});

	const span = $derived(`${longDate(dayOf(from))} – ${longDate(lastDay)}`);

	function apply() {
		onapply(count);
	}

	// `Esc` closes the window like `Abbrechen`; `Enter` applies.
	$effect(() => escapeWindows(() => onclose()));

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			apply();
		}
	}

	onMount(() => {
		field?.focus();
		field?.select();
	});
</script>

<svelte:window onkeydowncapture={onKeydown} />

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label={t('Schicht auf mehrere Tage')}
		style="width: min(22rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Tage in Folge')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<div class="content">
			<p class="shift">{cell}</p>
			<label class="field">
				<span class="label">{t('Wie viele Tage in Folge?')}</span>
				<input
					bind:this={field}
					type="number"
					min="1"
					max={MAX_DAYS}
					step="1"
					value={days}
					oninput={(event) => (days = event.currentTarget.valueAsNumber)}
				/>
			</label>
			<p class="span">{span}</p>
			<p class="note">
				{count === 1
					? t('1 = nur dieser Tag.')
					: t('{n} Tage, der gewählte Tag zählt mit.', { n: count })}
				{#if maxDays !== null}
					{maxDays === 1
						? t('Regeln: höchstens 1 Tag in Folge — weitere Tage bleiben frei.')
						: t('Regeln: höchstens {n} Tage in Folge — weitere Tage bleiben frei.', {
								n: maxDays
							})}
				{/if}
			</p>
		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={onclose}>{t('Abbrechen')}</button>
			<button class="sw-button" type="button" onclick={apply}>{t('Übernehmen')}</button>
		</div>
	</div>
</div>

<style>
	.content {
		padding: 0.7rem 0.85rem 0.85rem;
	}
	.shift {
		margin: 0 0 0.6rem;
		padding: 0.3rem 0.55rem;
		border-radius: 4px;
		background: color-mix(in srgb, var(--accent) 10%, transparent);
		color: var(--accent);
		font-size: 0.82rem;
		font-weight: 600;
	}
	.field {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6rem;
	}
	.label {
		font-size: 0.85rem;
	}
	.field input {
		width: 5rem;
		padding: 0.3rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: inherit;
		font: inherit;
		font-variant-numeric: tabular-nums;
		text-align: right;
	}
	.field input:focus {
		border-color: var(--accent);
		outline: none;
	}
	.span {
		margin: 0.6rem 0 0;
		font-size: 0.85rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.note {
		margin: 0.15rem 0 0;
		color: var(--muted);
		font-size: 0.76rem;
	}
</style>
