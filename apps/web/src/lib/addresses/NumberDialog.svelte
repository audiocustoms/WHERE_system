<script lang="ts">
	import { escapeWindows } from '../escape.svelte';
	import { onMount } from 'svelte';
	import { api, errorText } from '../api';
	import { askCancel } from '../confirm.svelte';
	import './address-window.css';

	interface Props {
		/** „Kunde"/„Interessent" → customer number, „Lieferant" → supplier number. */
		role: 'customer' | 'supplier';
		/** Number already set (when opened again). */
		number?: number | null;
		/** Debtor/creditor number already set. */
		secondary?: string | null;
		onconfirm: (number: number, secondary: string | null) => void;
		oncancel: () => void;
	}

	let { role, number = null, secondary = null, onconfirm, oncancel }: Props = $props();

	// The role is fixed on opening; the dialog is not switched.
	// svelte-ignore state_referenced_locally
	const isCustomer = role === 'customer';
	const title = isCustomer ? 'Kundennummer vergeben' : 'Lieferantennummer vergeben';
	const numberLabel = isCustomer ? 'Kunden-Nr' : 'Lief-Nr';
	const secondaryLabel = isCustomer ? 'Debitor-Nr' : 'Kreditor-Nr';

	// On opening deliberately only the initial value — afterwards the field leads.
	// svelte-ignore state_referenced_locally
	let entered = $state(number ? String(number) : '');
	// svelte-ignore state_referenced_locally
	let secondaryText = $state(secondary ?? '');
	let error = $state<string | null>(null);
	let busy = $state(false);

	/** Proposal from the number series; does not consume the number yet. */
	onMount(async () => {
		if (entered) return;
		try {
			const series = await api.masterData.numberSeriesEntry(isCustomer ? 'customer' : 'supplier');
			entered = series.next_value !== null ? String(series.next_value) : '';
		} catch (e) {
			error = errorText(e);
		}
	});

	async function secondProposal() {
		busy = true;
		error = null;
		try {
			const series = await api.masterData.numberSeriesEntry(isCustomer ? 'debtor' : 'creditor');
			if (series.next_value !== null) secondaryText = String(series.next_value);
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = false;
		}
	}

	function apply() {
		const value = Number(entered.trim());
		if (!Number.isInteger(value) || value <= 0) {
			error = `Bitte eine gültige ${numberLabel} eingeben.`;
			return;
		}
		onconfirm(value, secondaryText.trim() === '' ? null : secondaryText.trim());
	}

	/** The number assignment asks on cancel as well. */
	async function cancel() {
		if (await askCancel('Die Nummernvergabe')) oncancel();
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void cancel()));
</script>

<div class="sw-backdrop" role="presentation">
	<div class="sw-window adr-window" role="dialog" aria-modal="true" aria-label={title} style="width: min(24rem, 100%)">
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{title}</span>
			<button class="sw-close" type="button" title="Schließen" onclick={cancel}>×</button>
		</div>

		<div class="sw-content">
			<p class="sw-small">
				Erst mit der Nummer wird die Adresse eindeutig — alle weiteren Felder sind danach
				freigegeben.
			</p>
			<div class="adr-row">
				<span class="adr-label">{numberLabel}</span>
				<input bind:value={entered} inputmode="numeric" aria-label={numberLabel} />
			</div>
			<div class="adr-row">
				<span class="adr-label">{secondaryLabel}</span>
				<input bind:value={secondaryText} inputmode="numeric" aria-label={secondaryLabel} />
				<button
					class="adr-extra"
					type="button"
					title="{secondaryLabel} aus dem Nummernkreis"
					onclick={secondProposal}
					disabled={busy}>+</button
				>
			</div>

			{#if error}
				<p class="sw-error">{error}</p>
			{/if}
		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={cancel}>Abbrechen</button>
			<button class="sw-button sw-primary" type="button" onclick={apply}>Übernehmen</button>
		</div>
	</div>
</div>
