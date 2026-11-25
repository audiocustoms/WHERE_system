<script lang="ts">
	import { escapeWindows } from '$core/escape.svelte';
	import { onMount } from 'svelte';
	import '$core/search/search-window.css';

	interface Props {
		/** Day the free time is written to, as `YYYY-MM-DD`. */
		day: string;
		/** Start of the window as `HH:MM`. */
		from?: string | null;
		/** End of the window as `HH:MM`. */
		to?: string | null;
		/** Break of the window in minutes. */
		breakMinutes?: number | null;
		/** The window was confirmed. */
		onapply: (time: { starts_at: string; ends_at: string; break_minutes: number }) => void;
		/** The window was closed without a time. */
		onclose: () => void;
	}

	let { day, from = null, to = null, breakMinutes = null, onapply, onclose }: Props = $props();

	/** Longest break the server accepts. */
	const MAX_BREAK = 720;

	/** Values the window opens with; it always belongs to one cell. */
	function defaults() {
		return {
			start: from ?? '08:00',
			end: to ?? '14:00',
			clampedBreak: breakMinutes === null ? '0' : String(breakMinutes)
		};
	}

	const initial = defaults();

	let start = $state(initial.start);
	let end = $state(initial.end);
	let breakText = $state(initial.clampedBreak);
	let field: HTMLInputElement | null = $state(null);

	/** `HH:MM` in 24-hour format. */
	function isTime(label: string): boolean {
		return /^([01][0-9]|2[0-3]):[0-5][0-9]$/.test(label);
	}

	/**
	 * 24-hour time: `6`, `630`, `06.30` or `6:3` become `HH:MM`.
	 * Anything that cannot be parsed stays as typed.
	 */
	function timeText(label: string): string {
		const raw = label.trim().replace(/[.,;\s]+/g, ':');
		const parts = raw.split(':').filter((part) => part !== '');
		let hours: number;
		let minutes: number;
		if (parts.length === 1) {
			const digits = parts[0].replace(/\D/g, '');
			if (digits === '') return label;
			if (digits.length === 3) {
				hours = Number(digits.slice(0, 1));
				minutes = Number(digits.slice(1));
			} else if (digits.length === 4) {
				hours = Number(digits.slice(0, 2));
				minutes = Number(digits.slice(2));
			} else if (digits.length <= 2) {
				hours = Number(digits);
				minutes = 0;
			} else {
				return label;
			}
		} else if (parts.length === 2) {
			hours = Number(parts[0]);
			minutes = Number(parts[1]);
		} else {
			return label;
		}
		if (!Number.isInteger(hours) || !Number.isInteger(minutes)) return label;
		if (hours < 0 || hours > 23 || minutes < 0 || minutes > 59) return label;
		return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`;
	}

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

	/** Minutes between start and end; an end before the start runs over midnight. */
	const minutes = $derived.by(() => {
		if (!isTime(start) || !isTime(end)) return 0;
		const [startHour, startMinute] = start.split(':').map(Number);
		const [endHour, endMinute] = end.split(':').map(Number);
		let total = endHour * 60 + endMinute - (startHour * 60 + startMinute);
		if (total <= 0) total += 24 * 60;
		return total;
	});

	const clampedBreak = $derived.by(() => {
		const value = Number(breakText);
		if (!Number.isFinite(value)) return 0;
		return Math.min(MAX_BREAK, Math.max(0, Math.trunc(value)));
	});

	const hours = $derived(Math.max(0, (minutes - clampedBreak) / 60));

	/** Why the window cannot be written, or `null` when it can. */
	const failure = $derived.by(() => {
		if (!isTime(start)) return 'Der Beginn ist keine Uhrzeit.';
		if (!isTime(end)) return 'Das Ende ist keine Uhrzeit.';
		if (start === end) return 'Beginn und Ende sind gleich.';
		if (clampedBreak >= minutes) return 'Die Pause ist länger als die freie Zeit.';
		return null;
	});

	function apply() {
		if (failure !== null) return;
		onapply({ starts_at: start, ends_at: end, break_minutes: clampedBreak });
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
		aria-label="Freie Eingabe"
		style="width: min(22rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">Freie Eingabe</span>
			<button class="sw-close" type="button" title="Schließen" onclick={onclose}>×</button>
		</div>

		<div class="content">
			<p class="day">{longDate(dayOf(day))}</p>
			<label class="field">
				<span class="label">Von (24 h)</span>
				<input
					bind:this={field}
					type="text"
					inputmode="numeric"
					autocomplete="off"
					maxlength="5"
					placeholder="HH:MM"
					title="Uhrzeit im 24-Stunden-Format, z. B. 08:00"
					bind:value={start}
					onblur={(event) => (start = timeText(event.currentTarget.value))}
				/>
			</label>
			<label class="field">
				<span class="label">Bis (24 h)</span>
				<input
					type="text"
					inputmode="numeric"
					autocomplete="off"
					maxlength="5"
					placeholder="HH:MM"
					title="Uhrzeit im 24-Stunden-Format, z. B. 14:30"
					bind:value={end}
					onblur={(event) => (end = timeText(event.currentTarget.value))}
				/>
			</label>
			<label class="field">
				<span class="label">Pause (Min.)</span>
				<input
					type="number"
					inputmode="numeric"
					min="0"
					max={MAX_BREAK}
					step="1"
					value={breakText}
					oninput={(event) => (breakText = event.currentTarget.value)}
				/>
			</label>
			{#if failure === null}
				<p class="duration">
					{start}–{end} · {hours.toLocaleString('de-DE', {
						minimumFractionDigits: 2,
						maximumFractionDigits: 2
					})} h
				</p>
			{:else}
				<p class="error">{failure}</p>
			{/if}

		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={onclose}>Abbrechen</button>
			<button class="sw-button" type="button" onclick={apply} disabled={failure !== null}>
				Übernehmen
			</button>
		</div>
	</div>
</div>

<style>
	.content {
		padding: 0.7rem 0.85rem 0.85rem;
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
	}
	.day {
		margin: 0 0 0.15rem;
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
	.duration {
		margin: 0.35rem 0 0;
		font-size: 0.85rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.error {
		margin: 0.35rem 0 0;
		color: var(--danger, #b91c1c);
		font-size: 0.8rem;
	}
</style>
