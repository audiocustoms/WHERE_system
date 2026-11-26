<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { api, errorText } from '$core/api';
	import ResetMonthDialog from './ResetMonthDialog.svelte';

	/**
	 * Settings of `WHERE - are my people?` behind `Admin → Module`: the resets that
	 * empty one month of the module's data. Every reset first asks for the month.
	 */
	type Action = 'shifts' | 'dispo';

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

	/** The month the window opens on; it keeps the last confirmed one. */
	let month = $state(new Date(new Date().getFullYear(), new Date().getMonth(), 1));
	/** The reset the month window asks for. */
	let pending = $state<Action | null>(null);
	let busy = $state(false);
	let notice = $state<string | null>(null);
	let error = $state<string | null>(null);

	const title = $derived(
		pending === 'dispo' ? t('Dispo zurücksetzen') : t('Schichten zurücksetzen')
	);

	function monthText(date: Date): string {
		return `${t(MONTHS[date.getMonth()])} ${date.getFullYear()}`;
	}

	function ask(action: Action) {
		notice = null;
		error = null;
		pending = action;
	}

	async function reset(action: Action, target: Date) {
		pending = null;
		busy = true;
		error = null;
		notice = null;
		try {
			const report =
				action === 'shifts'
					? await api.people.schedule.resetShifts(target.getMonth() + 1, target.getFullYear())
					: await api.people.schedule.dispo.reset(target.getMonth() + 1, target.getFullYear());
			const single = action === 'shifts' ? t('Schicht') : t('Termin');
			const many = action === 'shifts' ? t('Schichten') : t('Termine');
			const monthName = monthText(target);
			notice =
				report.cleared === 0
					? t('Keine {what} im {month}.', { what: many, month: monthName })
					: report.cleared === 1
						? t('1 {what} im {month} entfernt.', { what: single, month: monthName })
						: t('{n} {what} im {month} entfernt.', {
								n: report.cleared,
								what: many,
								month: monthName
							});
			month = target;
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>{t('WHERE - are my people? — WHERE')}</title></svelte:head>

<h1>{t('WHERE - are my people?')}</h1>

<div class="options">
	<button type="button" class="option" disabled={busy} onclick={() => ask('shifts')}>
		{t('Schichten zurücksetzen')}
	</button>
	<button type="button" class="option" disabled={busy} onclick={() => ask('dispo')}>
		{t('Dispo zurücksetzen')}
	</button>
</div>

{#if notice}<p class="good">{notice}</p>{/if}
{#if error}<p class="bad">{error}</p>{/if}

{#if pending !== null}
	<ResetMonthDialog
		title={title}
		{month}
		onconfirm={(target) => {
			const action = pending;
			if (action !== null) void reset(action, target);
		}}
		onclose={() => (pending = null)}
	/>
{/if}

<style>
	.options {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}
	.option {
		padding: 0.45rem 0.85rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
		color: inherit;
		font: inherit;
		font-size: 0.9rem;
		cursor: pointer;
	}
	.option:hover:not(:disabled) {
		border-color: var(--accent);
		color: var(--accent);
	}
	.option:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.good {
		color: var(--good);
		font-size: 0.88rem;
	}
	.bad {
		color: var(--bad);
		font-size: 0.88rem;
	}
</style>
