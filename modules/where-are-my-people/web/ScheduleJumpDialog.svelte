<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '$core/escape.svelte';
	import '$core/search/search-window.css';

	interface Props {
		/** Month the calendar opens on. */
		month: Date;
		/** Days the board currently shows; they are marked in the calendar. */
		period: string[];
		/** A day was picked. */
		onpick: (date: Date) => void;
		/** The window was closed without a day. */
		onclose: () => void;
	}

	let { month, period, onpick, onclose }: Props = $props();

	interface CalendarDay {
		/** `YYYY-MM-DD`. */
		iso: string;
		/** Day of the month. */
		day: number;
		/** Belongs to a neighbouring month. */
		outside: boolean;
		/** Today. */
		today: boolean;
		/** Lies in the period the board shows. */
		inPeriod: boolean;
		sunday: boolean;
	}

	const WEEKDAYS = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

	function toIso(date: Date): string {
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${date.getFullYear()}-${month}-${day}`;
	}

	function plusDays(date: Date, days: number): Date {
		return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
	}

	/** Monday of the week of `date`. */
	function weekStart(date: Date): Date {
		return new Date(
			date.getFullYear(),
			date.getMonth(),
			date.getDate() - ((date.getDay() + 6) % 7)
		);
	}

	/** Months the calendar pages away from the month it opened on. */
	let offset = $state(0);

	/** The month the calendar currently shows. */
	const shown = $derived(new Date(month.getFullYear(), month.getMonth() + offset, 1));

	const days = $derived.by(() => {
		const start = weekStart(shown);
		const todayIso = toIso(new Date());
		const marked = new Set(period);
		const list: CalendarDay[] = [];
		for (let index = 0; index < 42; index += 1) {
			const date = plusDays(start, index);
			const iso = toIso(date);
			list.push({
				iso,
				day: date.getDate(),
				outside: date.getMonth() !== shown.getMonth(),
				today: iso === todayIso,
				inPeriod: marked.has(iso),
				sunday: date.getDay() === 0
			});
		}
		return list;
	});

	const title = $derived(shown.toLocaleDateString('de-DE', { month: 'long', year: 'numeric' }));

	function pageBy(direction: -1 | 1) {
		offset += direction;
	}

	function pick(iso: string) {
		const [year, month, day] = iso.split('-').map(Number);
		onpick(new Date(year, month - 1, day));
	}


	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>


<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window calendar-window"
		role="dialog"
		aria-modal="true"
		aria-label={t('Zu einem Tag springen')}
		style="width: min(22rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Springe zu')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<div class="content">
			<div class="head">
				<button
					type="button"
					class="arrow"
					title={t('einen Monat zurück')}
					aria-label={t('einen Monat zurück')}
					onclick={() => pageBy(-1)}>‹</button
				>
				<span class="title">{title}</span>
				<button
					type="button"
					class="arrow"
					title={t('einen Monat weiter')}
					aria-label={t('einen Monat weiter')}
					onclick={() => pageBy(1)}>›</button
				>
			</div>

			<div class="grid">
				{#each WEEKDAYS as weekday (weekday)}
					<span class="weekday">{t(weekday)}</span>
				{/each}
				{#each days as day (day.iso)}
					{@const [year, month, number] = day.iso.split('-').map(Number)}
					<button
						type="button"
						class="day"
						class:outside={day.outside}
						class:sunday={day.sunday}
						class:today={day.today}
						class:period={day.inPeriod}
						aria-current={day.inPeriod ? 'true' : undefined}
						aria-label={new Date(year, month - 1, number).toLocaleDateString('de-DE', {
							weekday: 'long',
							day: 'numeric',
							month: 'long',
							year: 'numeric'
						})}
						onclick={() => pick(day.iso)}>{day.day}</button
					>
				{/each}
			</div>
		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={onclose}>{t('Schließen')}</button>
		</div>
	</div>
</div>

<style>
	.calendar-window {
		max-height: min(40rem, 90vh);
	}
	.content {
		padding: 0.6rem 0.75rem 0.75rem;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		margin-bottom: 0.4rem;
	}
	.title {
		flex: 1 1 auto;
		text-align: center;
		font-size: 0.88rem;
		font-weight: 600;
	}
	.arrow {
		padding: 0 0.45rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--muted);
		font-size: 1rem;
		line-height: 1.5;
		cursor: pointer;
	}
	.arrow:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(7, minmax(0, 1fr));
		gap: 2px;
	}
	.weekday {
		padding-bottom: 0.2rem;
		color: var(--muted-2);
		font-size: 0.68rem;
		font-weight: 600;
		text-align: center;
	}
	.day {
		padding: 0.3rem 0;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: 0.82rem;
		font-variant-numeric: tabular-nums;
		line-height: 1.35;
		cursor: pointer;
	}
	.day:hover {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 10%, transparent);
	}
	.day.outside {
		color: var(--muted-2);
	}
	.day.sunday {
		color: var(--muted);
	}
	/* Days the board currently shows. */
	.day.period {
		background: color-mix(in srgb, var(--accent) 16%, transparent);
		font-weight: 600;
	}
	.day.today {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 700;
	}
</style>
