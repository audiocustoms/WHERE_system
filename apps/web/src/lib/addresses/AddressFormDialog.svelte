<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { escapeWindows } from '../escape.svelte';
	import { onMount } from 'svelte';
	import { api, errorText, type AddressInput } from '../api';
	import { askCancel } from '../confirm.svelte';
	import { modules } from '../modules.svelte';
	import NumberDialog from './NumberDialog.svelte';
	import './address-window.css';

	interface Props {
		initial: AddressInput;
		/** Set = edit an existing address instead of creating one. */
		id?: number | null;
		/** Without write right all fields stay locked and `Sichern` is missing. */
		schreibbar?: boolean;
		/** Without delete right the button `Löschen` is missing. */
		loeschbar?: boolean;
		onsaved: () => void;
		oncancel: () => void;
	}

	let {
		initial,
		id = null,
		schreibbar = true,
		loeschbar = true,
		onsaved,
		oncancel
	}: Props = $props();

	type Tab = 'base' | 'customer' | 'supplier' | 'location' | 'sales' | 'secrets';

	let tab = $state<Tab>('base');
	// The mask starts with the passed-in values and then drives them itself.
	// svelte-ignore state_referenced_locally
	let form = $state<AddressInput>({ ...initial });
	let saving = $state(false);
	let error = $state<string | null>(null);

	/** Open number dialog: role and whether it takes the role back on cancel. */
	let dialog = $state<{ role: 'customer' | 'supplier'; revert: boolean } | null>(null);

	const roleTabLabel = $derived(
		form.is_customer
			? t('Kunde')
			: form.is_interested
				? t('Interessent')
				: t('Interessent/Kunde')
	);

	/** Without a number the mask stays locked — only the roles are usable.
	    Without write right it is locked entirely (the fields stay readable). */
	const locked = $derived(
		!schreibbar ||
			((form.is_customer || form.is_interested) && !form.customer_no) ||
			(form.is_supplier && !form.supplier_no)
	);

	const lockHint = $derived(
		!schreibbar
			? t('Für diese Adresse fehlt das Recht zum Anlegen bzw. Bearbeiten — die Maske ist nur zum Lesen offen.')
			: locked
				? !form.customer_no && (form.is_customer || form.is_interested)
					? t('Bitte zuerst die Kundennummer vergeben — danach sind alle Felder freigegeben.')
					: t('Bitte zuerst die Lieferantennummer vergeben — danach sind alle Felder freigegeben.')
				: null
	);

	const stamp = $derived(
		id
			? t('Bestand') +
					(form.customer_no || form.supplier_no
						? `\n${t('Nummer {no}', { no: String(form.customer_no ?? form.supplier_no) })}`
						: '')
			: form.customer_no || form.supplier_no
				? t('Neu angelegt') +
					`\n${t('Nummer {no}', { no: String(form.customer_no ?? form.supplier_no) })}`
				: t('Neu angelegt')
	);

	/**
	 * The role tabs only become accessible once the marker is set —
	 * as in the original. `Vertriebstätigkeiten` and `Geheimnisse` hang on no role.
	 */
	const freeTabs = $derived<Record<Tab, boolean>>({
		base: true,
		customer: form.is_customer || form.is_interested,
		supplier: form.is_supplier,
		location: form.is_location,
		sales: true,
		secrets: true
	});

	const tabNote: Record<Tab, string> = {
		base: '',
		customer: t('Zuerst „Interessent“ oder „Kunde“ markieren'),
		supplier: t('Zuerst „Lieferant“ markieren'),
		location: t('Zuerst „Location“ markieren'),
		sales: '',
		secrets: ''
	};

	/** If the role drops away while its tab is open, it goes back to the base data. */
	$effect(() => {
		if (!freeTabs[tab]) tab = 'base';
	});

	/**
	 * Sets a role. Interested and customer exclude each other, everything else
	 * can be combined freely. Without a number the number dialog opens.
	 */
	function setRole(role: 'interested' | 'customer' | 'supplier' | 'location' | 'employee' | 'vip', checked: boolean) {
		error = null;
		if (role === 'interested') {
			form.is_interested = checked;
			if (checked) {
				form.is_customer = false;
				if (!form.customer_no) dialog = { role: 'customer', revert: true };
			}
			return;
		}
		if (role === 'customer') {
			form.is_customer = checked;
			if (checked) {
				form.is_interested = false;
				if (!form.customer_no) dialog = { role: 'customer', revert: true };
			}
			return;
		}
		if (role === 'supplier') {
			form.is_supplier = checked;
			if (checked && !form.supplier_no) dialog = { role: 'supplier', revert: true };
			return;
		}
		if (role === 'location') form.is_location = checked;
		else if (role === 'employee') form.is_employee = checked;
		else form.is_vip = checked;
	}

	function confirmNumber(number: number, secondary: string | null) {
		const open = dialog;
		dialog = null;
		if (open?.role === 'supplier') {
			form.supplier_no = number;
			form.creditor = secondary;
		} else {
			form.customer_no = number;
			form.debtor = secondary;
		}
	}

	function cancelNumber() {
		const open = dialog;
		dialog = null;
		if (!open?.revert) return;
		// Without a number the role does not exist — the check goes back.
		if (open.role === 'supplier') form.is_supplier = false;
		else if (form.is_interested) form.is_interested = false;
		else form.is_customer = false;
	}

	/** Opens the number dialog again (button „+" next to the number). */
	function changeNumber(role: 'customer' | 'supplier') {
		dialog = { role, revert: false };
	}

	async function save() {
		saving = true;
		error = null;
		try {
			if (id) await api.addresses.replace(id, form);
			else await api.addresses.create(form);
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** Deleting exists only for an existing record. */
	async function remove() {
		if (!id) return;
		if (!confirm(t('Adresse {name} wirklich löschen?', { name: form.name1 ?? id }))) return;
		saving = true;
		error = null;
		try {
			await api.addresses.remove(id);
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** `Abbrechen` and closing the mask ask first. */
	async function cancel() {
		if (await askCancel(t(id ? 'Das Bearbeiten der Adresse' : 'Die Adressanlage'))) oncancel();
	}

	/**
	 * Salutations from the settings list (`img/Admin Menü` → Adressen → Anreden). The
	 * default stays so the mask is usable even without a maintained list.
	 */
	let salutations = $state<string[]>(['Herr', 'Frau', 'Divers', 'Firma']);
	onMount(async () => {
		try {
			const list = await api.settings.list('anreden');
			const values = list.rows
				.map((row) => row.values.Salutation?.trim())
				.filter((value): value is string => !!value);
			if (values.length > 0) salutations = values;
		} catch {
			// Without a list the defaults stay.
		}
	});

	const TITLE = ['', 'Dr.', 'Prof.', 'Prof. Dr.', 'Dipl.-Ing.'];

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void cancel()));
</script>

<div class="sw-backdrop" role="presentation">
	<div class="sw-window adr-window" role="dialog" aria-modal="true" aria-label={t('Adresse')} style="width: min(66rem, 100%)">
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Adresse:')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={cancel}>×</button>
		</div>

		<div class="adr-band">
			<h2>{t('Adresse')}</h2>
			<span class="adr-stamp">{stamp}</span>
		</div>

		<nav class="adr-tabs" aria-label={t('Register')}>
			<button class:active={tab === 'base'} type="button" onclick={() => (tab = 'base')}>{t('Basisdaten')}</button>
			<button
				class:active={tab === 'customer'}
				type="button"
				disabled={!freeTabs.customer}
				title={freeTabs.customer ? undefined : tabNote.customer}
				onclick={() => (tab = 'customer')}>{roleTabLabel}</button
			>
			<button
				class:active={tab === 'supplier'}
				type="button"
				disabled={!freeTabs.supplier}
				title={freeTabs.supplier ? undefined : tabNote.supplier}
				onclick={() => (tab = 'supplier')}>{t('Lieferant')}</button
			>
			<button
				class:active={tab === 'location'}
				type="button"
				disabled={!freeTabs.location}
				title={freeTabs.location ? undefined : tabNote.location}
				onclick={() => (tab = 'location')}>{t('Location')}</button
			>
			<button class:active={tab === 'sales'} type="button" onclick={() => (tab = 'sales')}>
				{t('Vertriebstätigkeiten')}
			</button>
			<button class:active={tab === 'secrets'} type="button" onclick={() => (tab = 'secrets')}>
				{t('Geheimnisse')}
			</button>
		</nav>

		{#if lockHint}
			<p class="sw-note">{lockHint}</p>
		{/if}

		<div class="adr-content">
			{#if tab === 'base'}
				<div class="adr-row narrow">
					<label class="adr-check">
						<input type="checkbox" bind:checked={form.active} />
						{t('Aktiv')}
					</label>
					<span class="adr-checkrow" style="padding-left: 0">
						<label class="adr-check">
							<input type="radio" name="art" checked={!form.is_person} onchange={() => (form.is_person = false)} />
							{t('Firma')}
						</label>
						<label class="adr-check">
							<input type="radio" name="art" checked={form.is_person} onchange={() => (form.is_person = true)} />
							{t('Einzelperson')}
						</label>
					</span>
				</div>

				<div class="adr-columns base">
					<div>
						{#if form.is_person}
							<div class="adr-row">
								<span class="adr-label">{t('Anrede')}</span>
								<select bind:value={form.salutation} disabled={locked} aria-label={t('Anrede')}>
									{#each ['', ...salutations] as a (a)}
										<option value={a === '' ? null : a}>{a === '' ? t('Keine Angabe') : a}</option>
									{/each}
								</select>
								<select bind:value={form.title} disabled={locked} aria-label={t('Titel')}>
									{#each TITLE as t (t)}
										<option value={t === '' ? null : t}>{t === '' ? '—' : t}</option>
									{/each}
								</select>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Vorname')}</span>
								<input bind:value={form.name2} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Nachname')}</span>
								<input bind:value={form.name1} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Spitzname')}</span>
								<input bind:value={form.name3} disabled={locked} />
								<input type="date" bind:value={form.birthday} disabled={locked} title={t('Geburtstag')} />
							</div>
						{:else}
							<div class="adr-row">
								<span class="adr-label">{t('Firma')}</span>
								<input bind:value={form.name1} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Zusatz 1')}</span>
								<input bind:value={form.name2} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Zusatz 2')}</span>
								<input bind:value={form.name3} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Adressgruppe')}</span>
								<input bind:value={form.group} disabled={locked} placeholder={t('z. B. Theater')} />
								<span></span>
							</div>
						{/if}

						<fieldset class="adr-group">
							<legend>{t('Anschrift')}</legend>
							<div class="adr-row">
								<span class="adr-label">{t('Adresszusatz')}</span>
								<input bind:value={form.address_addition} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Straße')}</span>
								<input bind:value={form.street} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('PLZ / Ort')}</span>
								<input bind:value={form.postal_code} disabled={locked} style="max-width: 6rem" />
								<input bind:value={form.city} disabled={locked} />
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Bundesland')}</span>
								<input bind:value={form.state} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Land')}</span>
								<input bind:value={form.country} disabled={locked} placeholder={t('DEU')} />
								<span></span>
							</div>
						</fieldset>

						<fieldset class="adr-group">
							<legend>{t('Kommunikation')}</legend>
							<div class="adr-row">
								<span class="adr-label">{t('Telefon')}</span>
								<input bind:value={form.phone} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Fax')}</span>
								<input bind:value={form.fax} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Mobiltelefon')}</span>
								<input bind:value={form.mobile} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('eMail')}</span>
								<input type="email" bind:value={form.email} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Webseite')}</span>
								<input bind:value={form.website} disabled={locked} />
								<span></span>
							</div>
						</fieldset>
					</div>

					<div class="adr-roles">
						<label class="adr-check">
							<input
								type="checkbox"
								checked={form.is_interested}
								onchange={(e) => setRole('interested', e.currentTarget.checked)}
							/>
							{t('Interessent')}
						</label>
						<label class="adr-check">
							<input
								type="checkbox"
								checked={form.is_customer}
								onchange={(e) => setRole('customer', e.currentTarget.checked)}
							/>
							{t('Kunde')}
						</label>
						<label class="adr-check">
							<input
								type="checkbox"
								checked={form.is_supplier}
								onchange={(e) => setRole('supplier', e.currentTarget.checked)}
							/>
							{t('Lieferant')}
						</label>
						<label class="adr-check">
							<input
								type="checkbox"
								checked={form.is_location}
								onchange={(e) => setRole('location', e.currentTarget.checked)}
							/>
							{t('Location')}
						</label>
						<label class="adr-check">
							<input
								type="checkbox"
								checked={form.is_employee}
								onchange={(e) => setRole('employee', e.currentTarget.checked)}
							/>
							{t('Mitarbeiter')}
						</label>
						<label class="adr-check">
							<input
								type="checkbox"
								checked={form.is_vip}
								onchange={(e) => setRole('vip', e.currentTarget.checked)}
							/>
							{t('V.I.P.')}
						</label>
					</div>

					<div>
						<fieldset class="adr-group">
							<legend>{t('Ansprechpartner')}</legend>
							<p class="sw-small">
								{t('Die Ansprechpartner liegen in einer eigenen Tabelle (`"AdressAP"`) — die Liste folgt mit dem nächsten Schritt.')}
							</p>
						</fieldset>
					</div>
				</div>
			{:else if tab === 'customer'}
				<div class="adr-columns">
					<div>
						<div class="adr-row">
							<span class="adr-label">{t('Kunden-Nr')}</span>
							<input
								value={form.customer_no ?? ''}
								readonly
								aria-label={t('Kunden-Nr')}
							/>
							<button
								class="adr-extra"
								type="button"
								title={t('Kundennummer vergeben')}
								onclick={() => changeNumber('customer')}>+</button
							>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('Debitor-Nr')}</span>
							<input bind:value={form.debtor} disabled={locked} />
							<span></span>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('UST-Ident-Nr.')}</span>
							<input bind:value={form.vat_id} disabled={locked} />
							<span></span>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('EORI-Nr.')}</span>
							<input bind:value={form.eori_no} disabled={locked} />
							<span></span>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('Steuernummer')}</span>
							<input bind:value={form.tax_number} disabled={locked} />
							<span></span>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('Register-Eintrag')}</span>
							<input bind:value={form.register_entry} disabled={locked} />
							<span></span>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('Register-Sitz')}</span>
							<input bind:value={form.register_seat} disabled={locked} />
							<span></span>
						</div>
					</div>

					<div>
						{#if modules.isInstalled('go')}
							<div class="adr-row">
								<span class="adr-label">{t('Rabatt Miete *')}</span>
								<input type="number" step="0.01" bind:value={form.discount_rental} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Rabatt Verkauf *')}</span>
								<input type="number" step="0.01" bind:value={form.discount_sale} disabled={locked} />
								<span></span>
							</div>
						{/if}
						<div class="adr-row">
							<span class="adr-label">{t('Kreditlimit')}</span>
							<input type="number" step="0.01" bind:value={form.credit_limit} disabled={locked} />
							<span></span>
						</div>
						{#if modules.isInstalled('go')}
							<div class="adr-row">
								<span class="adr-label">{t('Auftrags-/Liefersperre')}</span>
								<input
									type="checkbox"
									checked={form.auftragssperre}
									disabled={locked}
									onchange={(e) => (form.auftragssperre = e.currentTarget.checked)}
									aria-label={t('Auftragssperre')}
								/>
								<span></span>
							</div>
						{/if}

						{#if modules.isInstalled('money')}
						<fieldset class="adr-group">
							<legend>{t('Bankverbindung')}</legend>
							<div class="adr-row">
								<span class="adr-label">{t('Bank')}</span>
								<input bind:value={form.bank} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Konto')}</span>
								<input bind:value={form.bank_account} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('BLZ')}</span>
								<input bind:value={form.bank_code} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('IBAN')}</span>
								<input bind:value={form.iban} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('BIC')}</span>
								<input bind:value={form.bic} disabled={locked} />
								<span></span>
							</div>
						</fieldset>
						{/if}
					</div>
				</div>
			{:else if tab === 'supplier'}
				<div class="adr-columns">
					<div>
						<div class="adr-row">
							<span class="adr-label">{t('Lief-Nr')}</span>
							<input value={form.supplier_no ?? ''} readonly aria-label={t('Lief-Nr')} />
							<button
								class="adr-extra"
								type="button"
								title={t('Lieferantennummer vergeben')}
								onclick={() => changeNumber('supplier')}>+</button
							>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('Kreditor-Nr')}</span>
							<input bind:value={form.creditor} disabled={locked} />
							<span></span>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('UST-Ident-Nr.')}</span>
							<input bind:value={form.vat_id} disabled={locked} />
							<span></span>
						</div>
						<div class="adr-row">
							<span class="adr-label">{t('Steuernummer')}</span>
							<input bind:value={form.tax_number} disabled={locked} />
							<span></span>
						</div>
					</div>
					<div>
						<div class="adr-row">
							<span class="adr-label">{t('Umsatz')}</span>
							<input type="number" step="0.01" bind:value={form.supplier_revenue} disabled={locked} />
							<span></span>
						</div>
						{#if modules.isInstalled('go')}
							<div class="adr-row">
								<span class="adr-label">{t('Rabatt Anmietung *')}</span>
								<input
									type="number"
									step="0.01"
									bind:value={form.supplier_discount_rental}
									disabled={locked}
								/>
								<span></span>
							</div>
						{/if}
						<div class="adr-row">
							<span class="adr-label">{t('Rabatt Einkauf *')}</span>
							<input
								type="number"
								step="0.01"
								bind:value={form.supplier_discount_purchase}
								disabled={locked}
							/>
							<span></span>
						</div>

						{#if modules.isInstalled('money')}
						<fieldset class="adr-group">
							<legend>{t('Bankverbindung')}</legend>
							<div class="adr-row">
								<span class="adr-label">{t('Bank')}</span>
								<input bind:value={form.supplier_bank} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('Konto')}</span>
								<input bind:value={form.supplier_bank_account} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('BLZ')}</span>
								<input bind:value={form.supplier_bank_code} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('IBAN')}</span>
								<input bind:value={form.supplier_iban} disabled={locked} />
								<span></span>
							</div>
							<div class="adr-row">
								<span class="adr-label">{t('BIC')}</span>
								<input bind:value={form.supplier_bic} disabled={locked} />
								<span></span>
							</div>
						</fieldset>
						{/if}
						<p class="sw-small">{t('* mandantenabhängige Angabe. Währung und Zahlungskondition folgen.')}</p>
					</div>
				</div>
			{:else if tab === 'location'}
				<p class="sw-note">
					{t('Die Registerkarte „Location" (Objekt, Räume, Flächen, Stromanschlüsse) folgt mit dem Location-Stamm.')}
				</p>
			{:else if tab === 'sales'}
				<p class="sw-note">
					{t('Die Registerkarte „Vertriebstätigkeiten" (Kontakthistorie, Angebote, Aufträge, Rechnungen) folgt mit dem Vertriebsmodul.')}
				</p>
			{:else}
				<p class="sw-note">
					{t('Die Registerkarte „Geheimnisse" (interne Notizen und Zugriffsrechte) folgt mit der Benutzerverwaltung.')}
				</p>
			{/if}

			{#if error}
				<p class="sw-error">{error}</p>
			{/if}
		</div>

		<div class="sw-footer">
			{#if id && loeschbar}
				<button class="sw-button adr-delete" type="button" onclick={remove} disabled={saving}>{t('Löschen')}</button>
			{/if}
			<button class="sw-button" type="button" onclick={cancel}
				>{schreibbar ? t('Abbrechen') : t('Schließen')}</button
			>
			{#if schreibbar}
				<button class="sw-button sw-primary" type="button" onclick={save} disabled={saving || locked}>
					{saving ? t('Sichern …') : t('Sichern')}
				</button>
			{/if}
		</div>
	</div>

	{#if dialog}
		<NumberDialog
			role={dialog.role}
			number={dialog.role === 'supplier' ? form.supplier_no : form.customer_no}
			secondary={dialog.role === 'supplier' ? form.creditor : form.debtor}
			onconfirm={confirmNumber}
			oncancel={cancelNumber}
		/>
	{/if}
</div>
