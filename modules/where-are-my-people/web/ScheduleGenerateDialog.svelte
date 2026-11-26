<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '$core/escape.svelte';
	import { api, errorText, type ScheduleGenerateReport } from '$core/api';
	import '$core/search/search-window.css';

	/**
	 * „Dienstplan generieren" — the window behind the green button of the board.
	 *
	 * It asks for the period — a free range, one calendar week or one month — and
	 * writes the plan from the stored demand, the disposition, the employees and
	 * the planning rules. The report names what was written and what stayed open.
	 */
	interface Props {
		/** First day the board shows; the free range opens on it. */
		from: string;
		/** Last day the board shows; the free range closes on it. */
		to: string;
		onclose: () => void;
		/** The plan was written; the board reads it again. */
		ongenerated: () => void;
	}

	let { from, to, onclose, ongenerated }: Props = $props();

	type Mode = 'range' | 'week' | 'month';

	const MODES: { value: Mode; label: string }[] = [
		{ value: 'range', label: 'Von – bis' },
		{ value: 'week', label: 'Woche' },
		{ value: 'month', label: 'Monat' }
	];

	const MONTHS = [
		'Januar',
		'Februar',
		'März',
		'April',
		'Mai',
		'Juni',
		'Juli',
		'August',
		'September',
		'Oktober',
		'November',
		'Dezember'
	];

	/** Longest period one call may write. */
	const MAX_DAYS = 92;

	function toIso(date: Date): string {
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${date.getFullYear()}-${month}-${day}`;
	}

	function toDate(iso: string): Date {
		const [year, month, day] = iso.split('-').map(Number);
		return new Date(year, month - 1, day);
	}

	function plusDays(date: Date, days: number): Date {
		return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
	}

	/** Monday of the ISO week of `date`. */
	function monday(date: Date): Date {
		return plusDays(date, -((date.getDay() + 6) % 7));
	}

	/** Monday of the first ISO week of the year. */
	function firstMonday(year: number): Date {
		const fourth = new Date(year, 0, 4);
		return monday(fourth);
	}

	function isoWeek(date: Date): { year: number; week: number } {
		const year = plusDays(monday(date), 3).getFullYear();
		const week =
			Math.round((monday(date).getTime() - firstMonday(year).getTime()) / (7 * 86400000)) + 1;
		return { year, week };
	}

	/** 52 or 53 — the last ISO week of the year. */
	function weeksInYear(year: number): number {
		return Math.round(
			(firstMonday(year + 1).getTime() - firstMonday(year).getTime()) / (7 * 86400000)
		);
	}

	function asGerman(iso: string): string {
		return toDate(iso).toLocaleDateString('de-DE', {
			day: '2-digit',
			month: '2-digit',
			year: 'numeric'
		});
	}

	/** First day the board shows; the window opens there. */
	function firstDay(): string {
		return from;
	}

	/** Last day the board shows. */
	function lastDay(): string {
		return to;
	}

	const start = toDate(firstDay());
	const startWeek = isoWeek(start);

	let mode = $state<Mode>('range');
	let fromChoice = $state(firstDay());
	let toChoice = $state(lastDay());
	let weekYear = $state(String(startWeek.year));
	let weekNumber = $state(String(startWeek.week));
	let monthYear = $state(String(start.getFullYear()));
	let monthNumber = $state(String(start.getMonth() + 1));
	/** Replace the shifts that are already planned. */
	let overwrite = $state(false);

	let busy = $state(false);
	let error = $state<string | null>(null);
	let report = $state<ScheduleGenerateReport | null>(null);

	const years = $derived(
		Array.from({ length: 7 }, (_, index) => String(start.getFullYear() - 2 + index))
	);
	const weeks = $derived(
		Array.from({ length: weeksInYear(Number(weekYear)) }, (_, index) => String(index + 1))
	);

	/** The period the chosen mode describes. */
	const period = $derived.by(() => {
		if (mode === 'week') {
			const chosenMonday = plusDays(
				firstMonday(Number(weekYear)),
				(Number(weekNumber) - 1) * 7
			);
			return { from: toIso(chosenMonday), to: toIso(plusDays(chosenMonday, 6)) };
		}
		if (mode === 'month') {
			const year = Number(monthYear);
			const month = Number(monthNumber);
			return {
				from: toIso(new Date(year, month - 1, 1)),
				to: toIso(new Date(year, month, 0))
			};
		}
		return { from: fromChoice, to: toChoice };
	});

	const days = $derived(
		period.from && period.to
			? Math.round(
					(toDate(period.to).getTime() - toDate(period.from).getTime()) / 86400000
				) + 1
			: 0
	);

	/** What keeps the window from writing. */
	const obstacle = $derived.by(() => {
		if (!period.from || !period.to) return t('Bitte einen Zeitraum wählen.');
		if (days <= 0) return t('Das Ende liegt vor dem Beginn.');
		if (days > MAX_DAYS) return t('Höchstens {n} Tage auf einmal.', { n: MAX_DAYS });
		return null;
	});

	async function generate() {
		if (obstacle !== null) return;
		busy = true;
		error = null;
		try {
			report = await api.people.schedule.generate({
				from: period.from,
				to: period.to,
				overwrite: overwrite
			});
			ongenerated();
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = false;
		}
	}

	/** The gaps by day, so the list stays short. */
	const openDays = $derived.by(() => {
		if (!report) return [];
		const perDay = new Map<string, number>();
		for (const gap of report.gaps) {
			perDay.set(gap.day, (perDay.get(gap.day) ?? 0) + gap.missing);
		}
		return [...perDay.entries()].map(([day, missing]) => ({ day, missing }));
	});

	/** What the run wrote, as one line of numbers and words. */
	const summary = $derived.by(() => {
		if (!report) return '';
		const parts = [
			`${report.assigned} ${t('Zellen geschrieben')}`,
			`${report.kept} ${t('unverändert')}`
		];
		if (report.replaced) parts.push(`${report.replaced} ${t('ersetzt')}`);
		if (report.extra) parts.push(`${report.extra} ${t('Extraschichten')}`);
		if (report.free_days) parts.push(`${report.free_days} ${t('freie Tage')}`);
		return parts.join(' · ');
	});

	/** Missing places over all days. */
	const openTotal = $derived(openDays.reduce((sum, day) => sum + day.missing, 0));


	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>


<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window generate-window"
		role="dialog"
		aria-modal="true"
		aria-label={t('Dienstplan generieren')}
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Dienstplan generieren')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<div class="sw-content">
			<div class="sw-form">
				<span class="sw-heading">{t('Zeitraum')}</span>

				<div class="modes" role="group" aria-label={t('Zeitraum')}>
					{#each MODES as entry (entry.value)}
						<button
							type="button"
							class:active={mode === entry.value}
							aria-pressed={mode === entry.value}
							onclick={() => (mode = entry.value)}>{t(entry.label)}</button
						>
					{/each}
				</div>

				{#if mode === 'range'}
					<div class="fields">
						<label class="label" for="generate-from">{t('Von')}</label>
						<input
							id="generate-from"
							class="date"
							type="date"
							bind:value={fromChoice}
						/>
						<label class="label" for="generate-to">{t('Bis')}</label>
						<input
							id="generate-to"
							class="date"
							type="date"
							bind:value={toChoice}
						/>
					</div>
				{:else if mode === 'week'}
					<div class="fields">
						<label class="label" for="generate-week">{t('Kalenderwoche')}</label>
						<select id="generate-week" class="week" bind:value={weekNumber}>
							{#each weeks as number (number)}
								<option value={number}>{t('KW {n}', { n: number })}</option>
							{/each}
						</select>
						<label class="label" for="generate-week-year">{t('Jahr')}</label>
						<select id="generate-week-year" class="year" bind:value={weekYear}>
							{#each years as year (year)}
								<option value={year}>{year}</option>
							{/each}
						</select>
					</div>
				{:else}
					<div class="fields">
						<label class="label" for="generate-month">{t('Monat')}</label>
						<select id="generate-month" class="month" bind:value={monthNumber}>
							{#each MONTHS as name, index (name)}
								<option value={String(index + 1)}>{t(name)}</option>
							{/each}
						</select>
						<label class="label" for="generate-month-year">{t('Jahr')}</label>
						<select id="generate-month-year" class="year" bind:value={monthYear}>
							{#each years as year (year)}
								<option value={year}>{year}</option>
							{/each}
						</select>
					</div>
				{/if}

				<p class="period">
					{asGerman(period.from)} – {asGerman(period.to)}
					{#if days > 0}· {days === 1 ? t('1 Tag') : t('{n} Tage', { n: days })}{/if}
				</p>

				<span class="sw-heading">{t('Optionen')}</span>

				<label class="sw-check">
					<input type="checkbox" bind:checked={overwrite} />
					{t('Bereits geplante Schichten ersetzen')}
				</label>

				{#if report}
					<hr class="sw-divider" />
					<span class="sw-heading">{t('Ergebnis')}</span>
					<p class="sw-note">{summary}</p>
					<p class="sw-small">{t('{n} Tage geprüft.', { n: report.days })}</p>
					{#if report.assigned === 0 && report.kept === 0 && report.gaps.length === 0}
						<p class="sw-small">{t('Kein Bedarf in diesem Zeitraum — nichts zu tun.')}</p>
					{:else if openDays.length === 0}
						<p class="sw-small">{t('Alle Zeilen des Bedarfs sind besetzt.')}</p>
					{:else}
						<p class="sw-small">
							{t('Offen geblieben: {count} Stellen an {n} Tagen.', {
								count: openTotal,
								n: openDays.length
							})}
						</p>
						<ul class="open">
							{#each openDays as day (day.day)}
								<li>
									<span>{asGerman(day.day)}</span>
									<span class="sw-small">{t('{n} offen', { n: day.missing })}</span>
								</li>
							{/each}
						</ul>
					{/if}
					{#if report.unclear_days.length > 0}
						<p class="sw-small">
							{t('Ohne erkennbare Schicht in der Dispo:')} {report.unclear_days
								.map(asGerman)
								.join(', ')}
						</p>
					{/if}
				{/if}

				{#if obstacle && !report}
					<p class="sw-small">{obstacle}</p>
				{/if}
				{#if error}
					<p class="sw-error">{error}</p>
				{/if}
			</div>
		</div>

		<div class="sw-footer">
			{#if report}
				<button class="sw-button sw-primary" type="button" onclick={onclose}>{t('Fertig')}</button>
			{:else}
				<button class="sw-button" type="button" onclick={onclose}>{t('Abbrechen')}</button>
				<button
					class="sw-button sw-primary"
					type="button"
					disabled={obstacle !== null || busy}
					title={obstacle ?? t('Dienstplan schreiben')}
					onclick={generate}>{busy ? t('Schreibe …') : t('Generieren')}</button
				>
			{/if}
		</div>
	</div>
</div>

<style>
	.generate-window {
		width: min(34rem, 100%);
	}
	.modes {
		display: flex;
		gap: 0.3rem;
		margin: 0.3rem 0 0.35rem;
	}
	.modes button {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: inherit;
		font: inherit;
		font-size: 0.85rem;
		cursor: pointer;
	}
	.modes button.active {
		border-color: var(--accent);
		background: var(--accent);
		color: #fff;
	}
	/* Label and field of one period, wide enough for a month name. */
	.fields {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin-bottom: 0.15rem;
	}
	.label {
		flex: 0 0 auto;
	}
	.fields input,
	.fields select {
		padding: 0.15rem 0.35rem;
		border: 1px solid var(--menu-band-border);
		border-radius: 2px;
		background: var(--surface);
		color: var(--label);
		font: inherit;
		font-size: 0.82rem;
	}
	.fields input.date {
		flex: 1 1 auto;
		min-width: 9.5rem;
	}
	.fields select.month {
		flex: 1 1 auto;
		min-width: 9.5rem;
	}
	.fields select.week {
		flex: 1 1 auto;
		min-width: 8rem;
	}
	.fields select.year {
		flex: 0 0 6.5rem;
	}
	/* The resolved period, right under the fields. */
	.period {
		margin: 0 0 0.15rem;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}
	.open {
		max-height: 9rem;
		margin: 0.3rem 0 0;
		padding: 0;
		overflow-y: auto;
		list-style: none;
		font-size: 0.8rem;
	}
	.open li {
		display: flex;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.1rem 0;
		font-variant-numeric: tabular-nums;
	}
</style>
