<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '$core/escape.svelte';
	import '$core/search/search-window.css';

	/**
	 * Asks for the month a reset applies to. The window opens on the month of the
	 * board and hands back the first day of the chosen month.
	 */
	interface Props {
		/** Heading of the window — the option that asked for it. */
		title: string;
		/** Month the window opens on. */
		month: Date;
		/** The month was confirmed. */
		onconfirm: (month: Date) => void;
		/** The window was closed without a month. */
		onclose: () => void;
	}

	let { title, month, onconfirm, onclose }: Props = $props();

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

	/** Years the select offers around the one the window opened on. */
	const YEARS = Array.from({ length: 11 }, (_, index) => month.getFullYear() - 5 + index);

	/** Months and years the selects offer; both open on the month handed in. */
	function initialMonth(): string {
		return String(month.getMonth() + 1);
	}

	function initialYear(): string {
		return String(month.getFullYear());
	}

	let monthChoice = $state(initialMonth());
	let yearChoice = $state(initialYear());

	function confirm() {
		onconfirm(new Date(Number(yearChoice), Number(monthChoice) - 1, 1));
	}


	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>


<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		style="width: min(24rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{title}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<div class="sw-content">
			<div class="sw-form">
				<div class="row">
					<label for="reset-month">{t('Monat')}</label>
					<select id="reset-month" class="month" bind:value={monthChoice}>
						{#each MONTHS as name, index (name)}
							<option value={String(index + 1)}>{t(name)}</option>
						{/each}
					</select>
					<label for="reset-year">{t('Jahr')}</label>
					<select id="reset-year" class="year" bind:value={yearChoice}>
						{#each YEARS as year (year)}
							<option value={String(year)}>{year}</option>
						{/each}
					</select>
				</div>
			</div>
		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={onclose}>{t('Abbrechen')}</button>
			<button class="sw-button sw-primary" type="button" onclick={confirm}>{title}</button>
		</div>
	</div>
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	.row label {
		color: var(--label);
		font-size: 0.82rem;
		line-height: 1.2;
	}
	.row select {
		padding: 0.12rem 0.3rem;
		border: 1px solid var(--menu-band-border);
		border-radius: 2px;
		background: var(--surface);
		color: var(--label);
		font-family: inherit;
		font-size: 0.78rem;
	}
	.row select.month {
		flex: 1 1 auto;
		min-width: 0;
	}
	.row select.year {
		flex: 0 0 5rem;
	}
</style>
