<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '../escape.svelte';
	import SearchWindow from './SearchWindow.svelte';
	import { api, errorText, type AddressSearchInput, type SetOperator } from '../api';

	interface Props {
		onclose: () => void;
		onsearch: (criteria: AddressSearchInput) => void;
	}

	let { onclose, onsearch }: Props = $props();

	/** Choice list „Typ" — in the original „Alle Adressen" as default. */
	const TYPES: { value: NonNullable<AddressSearchInput['address_type']>; text: string }[] = [
		{ value: 'all', text: t('Alle Adressen') },
		{ value: 'customers', text: t('Kunden') },
		{ value: 'suppliers', text: t('Lieferanten') }
	];

	const OPERATORS: { value: SetOperator; text: string }[] = [
		{ value: 'contains', text: t('enthält') },
		{ value: 'contains_any', text: t('oder enthält') },
		{ value: 'contains_all', text: t('und enthält') },
		{ value: 'contains_none', text: t('enthält nicht') }
	];

	/** Right-hand choice of the „Vorgang" bar („Suchtext enthält…"). */
	const MODI: { value: NonNullable<AddressSearchInput['name_match']>; text: string }[] = [
		{ value: 'contains', text: t('Suchtext enthält…') },
		{ value: 'starts_with', text: t('Suchtext beginnt mit…') },
		{ value: 'equals', text: t('Suchtext ist genau…') }
	];

	let typ = $state<NonNullable<AddressSearchInput['address_type']>>('all');
	let name = $state('');
	let group = $state('');
	let customerNo = $state('0');
	let debitor = $state('');
	let address = $state('');
	let phone = $state('');
	let email = $state('');
	let includeContacts = $state(false);
	let indexOp = $state<SetOperator>('contains_any');
	let index = $state<string[]>([]);
	let includeContactsIndex = $state(false);
	let modus = $state<NonNullable<AddressSearchInput['name_match']>>('contains');
	let sucheditor = $state(false);
	let indexChoice = $state(false);
	let indexValues = $state<string[] | null>(null);
	let error = $state<string | null>(null);

	function count(value: string): number | null {
		const n = Number(value.trim());
		return Number.isFinite(n) && n > 0 ? n : null;
	}

	function value(value: string): string | undefined {
		const s = value.trim();
		return s === '' ? undefined : s;
	}

	function criteria(): AddressSearchInput {
		return {
			address_type: typ,
			name: value(name),
			name_match: modus,
			include_contacts: includeContacts,
			group: value(group),
			customer_no: count(customerNo),
			debtor_no: value(debitor),
			address_text: value(address),
			phone: value(phone),
			email: value(email),
			search_index: index,
			search_index_op: indexOp,
			include_contact_index: includeContactsIndex
		};
	}

	function search() {
		error = null;
		onsearch(criteria());
	}

	async function openIndex() {
		indexChoice = true;
		if (indexValues !== null) return;
		try {
			indexValues = await api.masterData.addressIndex();
		} catch (e) {
			indexValues = [];
			error = errorText(e);
		}
	}

	function toggleIndex(value: string) {
		index = index.includes(value) ? index.filter((v) => v !== value) : [...index, value];
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>

<SearchWindow
	title={t('Adresssuche')}
	width="26rem"
	bandLabel={t('Vorgang')}
	bandText={t('Adressliste')}
	modeOptions={MODI}
	modeValue={modus}
	onmodechange={(value) => (modus = value as NonNullable<AddressSearchInput['name_match']>)}
	{onclose}
>
	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">{t('Typ')}</span>
			<select bind:value={typ} aria-label={t('Typ')}>
				{#each TYPES as t (t.value)}
					<option value={t.value}>{t.text}</option>
				{/each}
			</select>
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Name')}</span>
			<input bind:value={name} onkeydown={(e) => e.key === 'Enter' && search()} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Adressgruppe')}</span>
			<input bind:value={group} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Knd. / Debitor-Nr.')}</span>
			<input bind:value={customerNo} inputmode="numeric" />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Anschrift')}</span>
			<input bind:value={address} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('Telefon')}</span>
			<input bind:value={phone} />
		</div>
		<div class="sw-row">
			<span class="sw-label">{t('eMail')}</span>
			<input bind:value={email} />
		</div>
		<div class="sw-row indented">
			<label class="sw-check">
				<input type="checkbox" bind:checked={includeContacts} />
				{t('inklusive Ansprechpartner')}
			</label>
		</div>
	</div>

	<hr class="sw-divider" />

	<div class="sw-form">
		<div class="sw-row">
			<span class="sw-label">{t('Suchindex')}</span>
			<select class="operator" bind:value={indexOp} aria-label={t('Verknüpfung Suchindex')}>
				{#each OPERATORS as o (o.value)}
					<option value={o.value}>{o.text}</option>
				{/each}
			</select>
			<button class="sw-button" type="button" onclick={openIndex}>{t('Index wählen')}</button>
		</div>
		{#if index.length}
			<div class="sw-row indented">
				<span class="value">{index.join(', ')}</span>
				<button class="sw-extra" type="button" title={t('Auswahl löschen')} onclick={() => (index = [])}>
					×
				</button>
			</div>
		{/if}
		<div class="sw-row indented">
			<label class="sw-check">
				<input type="checkbox" bind:checked={includeContactsIndex} />
				{t('inklusive Ansprechpartner')}
			</label>
		</div>
	</div>

	{#if error}
		<p class="sw-error">{error}</p>
	{/if}
	{#if sucheditor}
		<p class="sw-note">
			{t('Gespeicherte Suchen für Adressen gibt es in WHERE noch nicht — der Sucheditor folgt mit dem Adressstamm.')}
		</p>
	{/if}

	{#if indexChoice}
		<div class="sw-backdrop">
			<div class="sw-window" role="dialog" aria-modal="true" aria-label={t('Index wählen')} style="width: min(22rem, 100%)">
				<div class="sw-titlebar">
					<span class="sw-icon" aria-hidden="true"></span>
					<span class="sw-title">{t('Index wählen')}</span>
					<button class="sw-close" type="button" title={t('Schließen')} onclick={() => (indexChoice = false)}>
						×
					</button>
				</div>
				<div class="sw-content">
					{#if indexValues === null}
						<p class="sw-small">{t('Lade Index …')}</p>
					{:else if indexValues.length === 0}
						<p class="sw-small">{t('Für Adressen sind noch keine Suchindex-Werte gepflegt.')}</p>
					{:else}
						<div class="sw-form">
							{#each indexValues as value (value)}
								<label class="sw-check">
									<input
										type="checkbox"
										checked={index.includes(value)}
										onchange={() => toggleIndex(value)}
									/>
									{value}
								</label>
							{/each}
						</div>
					{/if}
				</div>
				<div class="sw-footer">
					<button class="sw-button sw-primary" type="button" onclick={() => (indexChoice = false)}>
						{t('Übernehmen')}
					</button>
				</div>
			</div>
		</div>
	{/if}

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>{t('Abbrechen')}</button>
		<button class="sw-button" type="button" onclick={() => (sucheditor = !sucheditor)}>{t('Sucheditor')}</button>
		<button class="sw-button sw-primary" type="button" onclick={search}>{t('Suchen')}</button>
	{/snippet}
</SearchWindow>
