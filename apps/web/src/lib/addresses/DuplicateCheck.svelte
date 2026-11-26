<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '../escape.svelte';
	import { api, errorText, type Address } from '../api';
	import { askCancel } from '../confirm.svelte';
	import './address-window.css';

	interface Props {
		/** Closes the window without creating anything. */
		onexit: () => void;
		/** Opens the address mask — also when the search returned hits. */
		oncontinue: (term: string) => void;
	}

	let { onexit, oncontinue }: Props = $props();

	let term = $state('');
	/** `null` = not searched yet. */
	let hits = $state<Address[] | null>(null);
	let searching = $state(false);
	let error = $state<string | null>(null);
	let input = $state<HTMLInputElement | null>(null);

	async function check() {
		const value = term.trim();
		if (!value) {
			error = t('Bitte einen markanten Begriff der neuen Adresse eingeben.');
			return;
		}
		searching = true;
		error = null;
		try {
			const page = await api.addresses.duplicates(value, 25);
			hits = page.items;
		} catch (e) {
			error = errorText(e);
		} finally {
			searching = false;
		}
	}

	function place(address: Address): string {
		return [address.postal_code ?? null, address.city].filter(Boolean).join(' ') || '—';
	}

	function kind(address: Address): string {
		return (
			[
				address.is_customer ? t('Kunde') : null,
				address.is_interested ? t('Interessent') : null,
				address.is_supplier ? t('Lieferant') : null
			]
				.filter(Boolean)
				.join(' / ') || '—'
		);
	}

	/** Closing the creation asks as well (`Abbrechen`). */
	async function cancel() {
		if (await askCancel(t('Die Adressanlage'))) onexit();
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void cancel()));
</script>

<div class="sw-backdrop" role="presentation">
	<div class="sw-window adr-window" role="dialog" aria-modal="true" aria-label={t('Dublettenverhinderer')} style="width: min(38rem, 100%)">
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Dublettenverhinderer')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={cancel}>×</button>
		</div>

		<div class="adr-search-band">
			<p>{t('Bitte einen markanten Begriff der neuen Adresse eingeben und Return drücken')}</p>
			<div class="adr-search-row">
				<input
					bind:this={input}
					bind:value={term}
					placeholder={t('Suchbegriff')}
					aria-label={t('Suchbegriff')}
					onkeydown={(e) => e.key === 'Enter' && check()}
				/>
				<button type="button" title={t('Suchen')} onclick={check} disabled={searching}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
						<circle cx="10.5" cy="10.5" r="6.5" />
						<line x1="15.5" y1="15.5" x2="21" y2="21" />
					</svg>
				</button>
			</div>
		</div>

		{#if error}
			<p class="adr-message bad">{error}</p>
		{:else if hits !== null && hits.length === 0}
			<p class="adr-message good">{t('Keine relevanten Treffer in der Datenbank gefunden')}</p>
		{:else if hits !== null}
			<div class="adr-hits">
				<table>
					<thead>
						<tr>
							<th>{t('Nr.')}</th>
							<th>{t('Name')}</th>
							<th>{t('Ort')}</th>
							<th>{t('Rolle')}</th>
							<th>{t('Telefon')}</th>
						</tr>
					</thead>
					<tbody>
						{#each hits as address (address.id)}
							<tr>
								<td>{address.customer_no ?? address.debtor ?? '—'}</td>
								<td>{address.name1 ?? `#${address.id}`}</td>
								<td>{place(address)}</td>
								<td>{kind(address)}</td>
								<td>{address.phone ?? '—'}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="adr-message">
				{t('{count} mögliche Treffer — bitte prüfen, ob die Adresse schon vorhanden ist. Sonst führt „Weiter mit der Adressanlage“ in die neue Maske.', { count: hits.length })}
			</p>
		{/if}

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={cancel}>{t('Abbrechen')}</button>
			{#if hits !== null}
				<button
					class="sw-button sw-primary adr-continue"
					type="button"
					title={hits.length > 0 ? t('Trotz der aufgelisteten Treffer eine neue Adresse anlegen') : undefined}
					onclick={() => oncontinue(term.trim())}
				>
					{t('Weiter mit der Adressanlage')}
				</button>
			{/if}
		</div>
	</div>
</div>
