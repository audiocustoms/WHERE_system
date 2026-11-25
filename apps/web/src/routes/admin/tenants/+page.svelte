<script lang="ts">
	import { onMount } from 'svelte';
	import { api, errorText, type Address, type Tenant, type TenantReference } from '../../../lib/api';
	import '../../../lib/search/search-window.css';

	/** The two tabs from `img/Admin Menü/Mandanten`. */
	type Tab = 'einstellungen' | 'adresse';

	const TABS: { key: Tab; label: string }[] = [
		{ key: 'einstellungen', label: 'Einstellungen' },
		{ key: 'adresse', label: 'Adresse' }
	];

	/** The four options of the master data sync. */
	const SYNC_OPTIONS = [
		{ key: 'mandanten.sync.preise', label: 'Preise synchronisieren' },
		{ key: 'mandanten.sync.mwst', label: 'MwSt-Zuordnung synchronisieren' },
		{ key: 'mandanten.sync.fibu', label: 'FiBu-Kontierung synchronisieren' },
		{ key: 'mandanten.sync.lagerplatz', label: 'Anhänger Lagerplatz' }
	];

	/** The seven separations that apply to all tenants. */
	const SEPARATOR_OPTIONS = [
		{ key: 'mandanten.trennen.kassenbuecher', label: 'Kassenbücher trennen' },
		{ key: 'mandanten.trennen.dokumentvorlagen', label: 'Dokumentvorlagen trennen' },
		{ key: 'mandanten.trennen.zahlungskonditionen', label: 'Zahlungskonditionen trennen' },
		{ key: 'mandanten.trennen.standardtexte', label: 'Standardtexte trennen' },
		{ key: 'mandanten.trennen.staffeln', label: 'Zeitfaktoren- und Mengenstaffeln trennen' },
		{ key: 'mandanten.trennen.emailvorlagen', label: 'E-Mailvorlagen trennen' },
		{
			key: 'mandanten.trennen.stammdaten',
			label: 'Möglichkeit die Sichtbarkeit der Stammdaten zu separieren'
		}
	];

	const OPTIONS = [...SYNC_OPTIONS, ...SEPARATOR_OPTIONS];

	/** The form is the tenant being edited; `id === null` means „neu". */
	interface Form {
		id: number | null;
		short_name: string;
		name: string;
		tenant_id: string;
		sync_id: string;
		parent: string;
		web_identifier: string;
		currency: string;
		print_language: string;
		hide_actions: boolean;
		color: string;
		color_on: boolean;
		address_extra: string;
		street: string;
		postal_code: string;
		city: string;
		country_iso3: string;
		state: string;
		phone: string;
		fax: string;
		email: string;
		email_accounting: string;
		website: string;
		vat_id: string;
		tax_number: string;
		register_entry: string;
		register_place: string;
		linked_address: number | null;
		linked_address_name: string;
	}

	/** The dialog behind `⚙`: details that neither of the two tabs shows. */
	interface Details {
		id: number;
		name: string;
		parent: string;
		sync_id: string;
		web_identifier: string;
		currency: string;
	}

	let tenants = $state<Tenant[]>([]);
	let reference = $state<TenantReference>({ countries: [], print_languages: [] });
	let tab = $state<Tab>('einstellungen');
	let form = $state<Form | null>(null);
	let options = $state<Record<string, boolean>>({});
	let saved = $state<string>('');
	let optionsSaved = $state<string>('{}');
	let message = $state<string>('');
	let load = $state(true);
	let error = $state<string | null>(null);

	/** Open dialogs: „Weitere Angaben" and the address picker. */
	let details = $state<Details | null>(null);
	let addressSearch = $state<{ q: string; matches: Address[] } | null>(null);

	onMount(loadAll);

	const changed = $derived(
		!!form &&
			(JSON.stringify(form) !== saved || JSON.stringify(options) !== optionsSaved)
	);

	async function loadAll(onlyId?: number | null) {
		load = true;
		error = null;
		try {
			tenants = await api.tenants.list();
			if (!reference.countries.length) reference = await api.tenants.reference();
			const loaded = await api.prefs.load('mandanten.');
			options = Object.fromEntries(
				OPTIONS.map((entry) => [entry.key, loaded.values[entry.key] === 'true'])
			);
			optionsSaved = JSON.stringify(options);
			const target = onlyId === undefined ? form?.id : onlyId;
			if (target != null) await choose(target);
			else if (target === null) form = emptyForm();
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	function emptyForm(): Form {
		return {
			id: null,
			short_name: '',
			name: '',
			tenant_id: '0',
			sync_id: '0',
			parent: '',
			web_identifier: '',
			currency: '',
			print_language: 'Deutsch [deu]',
			hide_actions: false,
			color: '0',
			color_on: false,
			address_extra: '',
			street: '',
			postal_code: '',
			city: '',
			country_iso3: '',
			state: '',
			phone: '',
			fax: '',
			email: '',
			email_accounting: '',
			website: '',
			vat_id: '',
			tax_number: '',
			register_entry: '',
			register_place: '',
			linked_address: null,
			linked_address_name: ''
		};
	}

	function ausMandant(mandant: Tenant): Form {
		return {
			id: mandant.id,
			short_name: mandant.short_name ?? '',
			name: mandant.name,
			tenant_id: String(mandant.tenant_id),
			sync_id: String(mandant.sync_id),
			parent: mandant.parent == null ? '' : String(mandant.parent),
			web_identifier: mandant.web_identifier ?? '',
			currency: mandant.currency ?? '',
			print_language: mandant.print_language ?? 'Deutsch [deu]',
			hide_actions: mandant.hide_actions,
			color: String(mandant.color),
			color_on: mandant.color_on,
			address_extra: mandant.address_extra ?? '',
			street: mandant.street ?? '',
			postal_code: mandant.postal_code ?? '',
			city: mandant.city ?? '',
			country_iso3: mandant.country_iso3 ?? '',
			state: mandant.state ?? '',
			phone: mandant.phone ?? '',
			fax: mandant.fax ?? '',
			email: mandant.email ?? '',
			email_accounting: mandant.email_accounting ?? '',
			website: mandant.website ?? '',
			vat_id: mandant.vat_id ?? '',
			tax_number: mandant.tax_number ?? '',
			register_entry: mandant.register_entry ?? '',
			register_place: mandant.register_place ?? '',
			linked_address: mandant.linked_address,
			linked_address_name: mandant.linked_address_name ?? ''
		};
	}

	/** The long country name next to the code, as the mask shows it. */
	function landName(code: string): string {
		const searched = code.trim().toUpperCase();
		if (!searched) return '';
		const matches = reference.countries.find((entry) => entry.iso3 === searched);
		return matches ? matches.name : searched;
	}

	async function choose(id: number) {
		error = null;
		try {
			const mandant = await api.tenants.get(id);
			form = ausMandant(mandant);
			saved = JSON.stringify(form);
			message = '';
		} catch (e) {
			error = errorText(e);
		}
	}

	function createNew() {
		form = emptyForm();
		saved = JSON.stringify({ ...form, name: '' });
		message = '';
		tab = 'einstellungen';
	}

	function numbers(text: string): number | null {
		const value = text.trim().replace(',', '.');
		if (value === '') return null;
		const count = Number(value);
		return Number.isNaN(count) ? null : count;
	}

	function input() {
		const current = form!;
		return {
			name: current.name,
			short_name: current.short_name.trim() || null,
			tenant_id: numbers(current.tenant_id),
			sync_id: numbers(current.sync_id),
			parent: numbers(current.parent),
			web_identifier: current.web_identifier.trim() || null,
			currency: current.currency.trim() || null,
			currency_id: null,
			print_language: current.print_language.trim() || null,
			print_language_id: null,
			hide_actions: current.hide_actions,
			color: numbers(current.color),
			color_on: current.color_on,
			address_extra: current.address_extra.trim() || null,
			street: current.street.trim() || null,
			postal_code: current.postal_code.trim() || null,
			city: current.city.trim() || null,
			country_iso3: current.country_iso3.trim() || null,
			country: null,
			state: current.state.trim() || null,
			phone: current.phone.trim() || null,
			fax: current.fax.trim() || null,
			email: current.email.trim() || null,
			email_accounting: current.email_accounting.trim() || null,
			website: current.website.trim() || null,
			vat_id: current.vat_id.trim() || null,
			tax_number: current.tax_number.trim() || null,
			register_entry: current.register_entry.trim() || null,
			register_place: current.register_place.trim() || null,
			linked_address: current.linked_address
		};
	}

	async function commit() {
		if (!form) return;
		error = null;
		message = '';
		try {
			const mandant =
				form.id === null
					? await api.tenants.create(input())
					: await api.tenants.update(form.id, input());
			const previous = JSON.parse(optionsSaved) as Record<string, boolean>;
			for (const entry of OPTIONS) {
				const war = previous[entry.key] === true;
				const ist = options[entry.key] === true;
				if (war !== ist) await api.prefs.save(entry.key, ist ? 'true' : null);
			}
			await loadAll(mandant.id);
			message = 'Gespeichert.';
		} catch (e) {
			error = errorText(e);
		}
	}

	async function cancel() {
		await loadAll();
		message = '';
	}

	/** `⚙`: details of the table that neither of the two tabs shows. */
	function detailsOeffnen() {
		if (!form?.id) return;
		details = {
			id: form.id,
			name: form.name,
			parent: form.parent,
			sync_id: form.sync_id,
			web_identifier: form.web_identifier,
			currency: form.currency
		};
	}

	async function saveDetails() {
		if (!details || !form) return;
		form.parent = details.parent;
		form.sync_id = details.sync_id;
		form.web_identifier = details.web_identifier;
		form.currency = details.currency;
		details = null;
		await commit();
	}

	/** „Verlinkte Adresse": the address search as a small picker. */
	async function loadAddresses(q: string) {
		addressSearch = { q, matches: addressSearch?.matches ?? [] };
		try {
			const page = await api.addresses.list({ q, limit: 20 });
			if (addressSearch) addressSearch = { q, matches: page.items };
		} catch (e) {
			error = errorText(e);
		}
	}

	function setAddress(address: Address) {
		if (!form) return;
		form.linked_address = address.id;
		form.linked_address_name = address.name1 ?? `Adresse ${address.id}`;
		addressSearch = null;
	}

	function mayDeleteAddress() {
		if (!form) return;
		form.linked_address = null;
		form.linked_address_name = '';
		addressSearch = null;
	}
</script>

<svelte:head><title>Einstellungen Mandanten — WHERE</title></svelte:head>

<h1>Einstellungen Mandanten</h1>

<div class="window">
	<div class="left">
		<div class="list">
			<table>
				<thead>
					<tr>
						<th class="abbr">Abk.</th>
						<th>Firmename</th>
						<th class="center">Stammdaten-Sync</th>
					</tr>
				</thead>
				<tbody>
					{#each tenants as mandant (mandant.id)}
						<tr class:marked={form?.id === mandant.id} onclick={() => choose(mandant.id)}>
							<td class="abbr">{mandant.short_name ?? ''}</td>
							<td>{mandant.name}</td>
							<td class="center">
								<input
									type="checkbox"
									checked={mandant.sync_id > 0}
									disabled
									title="Stammdaten-Abgleich eingerichtet (SyncID)"
								/>
							</td>
						</tr>
					{:else}
						<tr><td colspan="3">{load ? 'Lade …' : 'Noch kein Mandant.'}</td></tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="list-head">
			<button type="button" class="square" title="Neuer Mandant" onclick={createNew}>+</button>
			<button
				type="button"
				class="square"
				title="Weitere Angaben zum Mandanten"
				disabled={form?.id == null}
				onclick={detailsOeffnen}>⚙</button
			>
		</div>

		<section class="group">
			<h2>Optionen für Stammdaten-Sync</h2>
			{#each SYNC_OPTIONS as entry (entry.key)}
				<label class="check">
					<input
						type="checkbox"
						checked={options[entry.key] ?? false}
						onchange={(e) => (options[entry.key] = e.currentTarget.checked)}
					/>
					{entry.label}
				</label>
			{/each}
		</section>

		<section class="group">
			<h2>Optionen (für alle Mandanten)</h2>
			{#each SEPARATOR_OPTIONS as entry (entry.key)}
				<label class="check">
					<input
						type="checkbox"
						checked={options[entry.key] ?? false}
						onchange={(e) => (options[entry.key] = e.currentTarget.checked)}
					/>
					{entry.label}
				</label>
			{/each}
		</section>
	</div>

	<div class="right">
		<div class="tabs" role="tablist" aria-label="Mandant">
			{#each TABS as entry (entry.key)}
				<button
					type="button"
					role="tab"
					aria-selected={tab === entry.key}
					class:active={tab === entry.key}
					onclick={() => (tab = entry.key)}>{entry.label}</button
				>
			{/each}
		</div>

		{#if form}
			<div class="sheet">
				{#if tab === 'einstellungen'}
					<div class="row">
						<span class="caption">Firmenname</span>
						<input
							class="text"
							maxlength="80"
							value={form.name}
							oninput={(e) => (form!.name = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Abkürzung</span>
						<input
							class="text short"
							maxlength="20"
							value={form.short_name}
							oninput={(e) => (form!.short_name = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Ident.-Nr.</span>
						<input
							class="numeric"
							type="number"
							min="0"
							value={form.tenant_id}
							oninput={(e) => (form!.tenant_id = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Land</span>
						<input
							class="text short"
							maxlength="3"
							list="mandanten-laender"
							placeholder="DEU"
							value={form.country_iso3}
							oninput={(e) => (form!.country_iso3 = e.currentTarget.value.toUpperCase())}
						/>
						<span class="computed">{landName(form.country_iso3)}</span>
					</div>
					<div class="row">
						<span class="caption">Drucksprache</span>
						<select
							class="text"
							value={form.print_language}
							onchange={(e) => (form!.print_language = e.currentTarget.value)}
						>
							{#each reference.print_languages as sprache (sprache.id)}
								<option value={sprache.label}>{sprache.label}</option>
							{/each}
							{#if !reference.print_languages.some((s) => s.label === form!.print_language)}
								<option value={form.print_language}>{form.print_language}</option>
							{/if}
						</select>
					</div>
					<div class="row">
						<span class="caption">Toolbarfarbe</span>
						<label class="check">
							<input
								type="checkbox"
								checked={form.color_on}
								onchange={(e) => (form!.color_on = e.currentTarget.checked)}
							/>
							Aktiv
						</label>
					</div>
					<div class="row">
						<span class="caption">Informationen über den Artikeleinsatz zeigen</span>
						<label class="check">
							<input
								type="checkbox"
								checked={form.hide_actions}
								onchange={(e) => (form!.hide_actions = e.currentTarget.checked)}
							/>
							Vor Anderen verstecken
						</label>
					</div>
					<div class="row">
						<span class="caption">Verlinkte Adresse</span>
						<div class="field-with-button">
							<button
								type="button"
								class="square"
								title="Adresse wählen"
								onclick={() => loadAddresses('')}>+</button
							>
							{#if form.linked_address != null}
								<button
									type="button"
									class="square"
									title="Verlinkung entfernen"
									onclick={mayDeleteAddress}>×</button
								>
							{/if}
							<span class="value">{form.linked_address_name || 'Keine'}</span>
						</div>
					</div>
				{:else}
					<div class="row">
						<span class="caption">Adresszusatz</span>
						<input
							class="text"
							value={form.address_extra}
							oninput={(e) => (form!.address_extra = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Straße / Postfach</span>
						<input
							class="text"
							value={form.street}
							oninput={(e) => (form!.street = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">PLZ</span>
						<input
							class="text short"
							maxlength="10"
							value={form.postal_code}
							oninput={(e) => (form!.postal_code = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Ort</span>
						<input
							class="text"
							value={form.city}
							oninput={(e) => (form!.city = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Land</span>
						<input
							class="text short"
							maxlength="3"
							list="mandanten-laender"
							placeholder="DEU"
							value={form.country_iso3}
							oninput={(e) => (form!.country_iso3 = e.currentTarget.value.toUpperCase())}
						/>
						<span class="computed">{landName(form.country_iso3)}</span>
					</div>
					<div class="row">
						<span class="caption">Telefon</span>
						<input
							class="text"
							value={form.phone}
							oninput={(e) => (form!.phone = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Fax</span>
						<input class="text" value={form.fax} oninput={(e) => (form!.fax = e.currentTarget.value)} />
					</div>
					<div class="row">
						<span class="caption">eMail</span>
						<input
							class="text"
							value={form.email}
							oninput={(e) => (form!.email = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">eMail (Buchhaltung)</span>
						<input
							class="text"
							value={form.email_accounting}
							oninput={(e) => (form!.email_accounting = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Webseite</span>
						<input
							class="text"
							value={form.website}
							oninput={(e) => (form!.website = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">UST-Ident-Nr.</span>
						<input
							class="text"
							value={form.vat_id}
							oninput={(e) => (form!.vat_id = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Steuernummer</span>
						<input
							class="text"
							value={form.tax_number}
							oninput={(e) => (form!.tax_number = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Register-Eintrag</span>
						<input
							class="text"
							value={form.register_entry}
							oninput={(e) => (form!.register_entry = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="caption">Register-Sitz</span>
						<input
							class="text"
							value={form.register_place}
							oninput={(e) => (form!.register_place = e.currentTarget.value)}
						/>
					</div>
				{/if}

				<datalist id="mandanten-laender">
					{#each reference.countries as land (land.iso3)}
						<option value={land.iso3}>{land.name}</option>
					{/each}
				</datalist>

				<div class="sheet-foot">
					<button
						type="button"
						class="btn primary"
						onclick={commit}
						disabled={!form.name.trim() || !changed}>Änderungen speichern</button
					>
				</div>
			</div>
		{:else}
			<div class="sheet empty">
				<p class="small">Links einen Mandanten wählen oder mit `+` einen neuen anlegen.</p>
			</div>
		{/if}
	</div>
</div>

<footer class="foot">
	<span class="small">
		Die Liste zeigt `Abk.`, Firmenname und ob ein Stammdaten-Abgleich eingerichtet ist.
	</span>
	<span class="spacer"></span>
	<button type="button" class="btn" onclick={cancel} disabled={!form || !changed}>
		Abbrechen
	</button>
	<button
		type="button"
		class="btn primary"
		onclick={commit}
		disabled={!form?.name.trim() || !changed}>Sichern</button
	>
</footer>

{#if error}
	<p class="error">{error}</p>
{/if}
{#if message}
	<p class="message">{message}</p>
{/if}

{#if details}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label="Weitere Angaben"
			style="width: min(30rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">Weitere Angaben — {details.name}</span>
				<button
					class="sw-close"
					type="button"
					title="Schließen"
					onclick={() => (details = null)}>×</button
				>
			</div>
			<div class="sw-content">
				<label class="field">
					Übergeordneter Mandant
					<select
						class="text"
						value={details.parent}
						onchange={(e) => (details!.parent = e.currentTarget.value)}
					>
						<option value="">— keiner —</option>
						{#each tenants.filter((entry) => entry.id !== details!.id) as entry (entry.id)}
							<option value={String(entry.id)}>{entry.name}</option>
						{/each}
					</select>
				</label>
				<label class="field">
					Stammdaten-Sync (ID)
					<input
						class="text"
						type="number"
						min="0"
						value={details.sync_id}
						oninput={(e) => (details!.sync_id = e.currentTarget.value)}
					/>
				</label>
				<label class="field">
					Web-Kennung
					<input
						class="text"
						maxlength="64"
						value={details.web_identifier}
						oninput={(e) => (details!.web_identifier = e.currentTarget.value)}
					/>
				</label>
				<label class="field">
					Währung
					<input
						class="text short"
						maxlength="20"
						placeholder="EUR"
						value={details.currency}
						oninput={(e) => (details!.currency = e.currentTarget.value.toUpperCase())}
					/>
				</label>
			</div>
			<div class="sw-footer">
				<button type="button" class="sw-button" onclick={() => (details = null)}>Abbrechen</button>
				<button type="button" class="sw-button primary" onclick={saveDetails}>Übernehmen</button>
			</div>
		</div>
	</div>
{/if}

{#if addressSearch}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label="Adresse wählen"
			style="width: min(34rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">Verlinkte Adresse</span>
				<button
					class="sw-close"
					type="button"
					title="Schließen"
					onclick={() => (addressSearch = null)}>×</button
				>
			</div>
			<div class="sw-content">
				<label class="field">
					Suche
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="text"
						autofocus
						placeholder="Name, Nummer, Ort"
						value={addressSearch.q}
						oninput={(e) => loadAddresses(e.currentTarget.value)}
					/>
				</label>
				<div class="matches">
					{#each addressSearch.matches as address (address.id)}
						<button type="button" class="match-row" onclick={() => setAddress(address)}>
							<span class="number">{address.customer_no ?? ''}</span>
							<span>{address.name1 ?? `Adresse ${address.id}`}</span>
							<span class="small">{address.city ?? ''}</span>
						</button>
					{:else}
						<p class="small">Keine Adresse gefunden.</p>
					{/each}
				</div>
			</div>
			<div class="sw-footer">
				<button type="button" class="sw-button" onclick={() => (addressSearch = null)}>Abbrechen</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.error {
		margin: 0 0 1rem;
		padding: 0.5rem 0.7rem;
		border: 1px solid var(--bad);
		border-radius: 4px;
		color: var(--bad);
		background: color-mix(in srgb, var(--bad) 8%, var(--surface));
	}
	.message {
		margin: 0.5rem 0 0;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.window {
		display: grid;
		grid-template-columns: 26rem minmax(24rem, 30rem);
		gap: 0.9rem;
		align-items: start;
	}
	.left {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.list {
		max-height: 15rem;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.82rem;
	}
	thead th {
		position: sticky;
		top: 0;
		padding: 0.35rem 0.45rem;
		text-align: left;
		font-weight: 600;
		color: var(--accent);
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
	}
	tbody td {
		padding: 0.25rem 0.45rem;
		border-bottom: 1px solid var(--surface-3);
	}
	tbody tr {
		cursor: pointer;
	}
	tbody tr:nth-child(even) {
		background: var(--surface-2);
	}
	tbody tr:hover {
		background: var(--highlight);
	}
	tbody tr.marked {
		background: var(--highlight);
		outline: 1px solid var(--accent);
		outline-offset: -1px;
	}
	th.abbr,
	td.abbr {
		width: 4.5rem;
	}
	th.center,
	td.center {
		text-align: center;
		width: 7.5rem;
	}
	.list-head {
		display: flex;
		gap: 0.25rem;
		justify-content: flex-end;
	}
	.square {
		width: 1.9rem;
		height: 1.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.9rem;
		line-height: 1;
		cursor: pointer;
	}
	.square:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		margin-top: 0.4rem;
	}
	.group h2 {
		margin: 0 0 0.2rem;
		font-size: 0.92rem;
		color: var(--accent);
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.82rem;
	}
	.tabs {
		display: flex;
		gap: 0.25rem;
	}
	.tabs button {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-bottom: none;
		border-radius: 3px 3px 0 0;
		background: var(--surface-2);
		color: var(--muted);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
	}
	.tabs button.active {
		background: var(--surface);
		color: var(--accent);
		font-weight: 600;
	}
	.sheet {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-height: 30rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: 0 4px 4px 4px;
		background: var(--surface);
	}
	.sheet.empty {
		justify-content: center;
		align-items: center;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 1.9rem;
	}
	.caption {
		width: 12rem;
		flex-shrink: 0;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.text {
		flex: 1;
		min-width: 0;
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
	}
	.text.short {
		flex: 0 0 6rem;
	}
	.computed {
		flex: 1;
		font-size: 0.82rem;
		color: var(--muted);
		letter-spacing: 0.02em;
	}
	.numeric {
		width: 6rem;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		text-align: right;
	}
	.field-with-button {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		flex: 1;
		min-width: 0;
	}
	.value {
		font-size: 0.84rem;
	}
	.sheet-foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: auto;
		padding-top: 0.6rem;
	}
	.sheet-foot .btn {
		margin-left: auto;
	}
	.foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 57rem;
		margin-top: 0.9rem;
		padding-top: 0.6rem;
		border-top: 1px solid var(--border);
	}
	.spacer {
		flex: 1;
	}
	.btn {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
		text-decoration: none;
	}
	.btn:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.btn.primary {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.btn.primary:disabled {
		border-color: var(--border);
		color: var(--muted-2);
		font-weight: 400;
	}
	.small {
		font-size: 0.78rem;
		color: var(--muted-2);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 0.6rem;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.matches {
		display: flex;
		flex-direction: column;
		max-height: 18rem;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: 4px;
	}
	.match-row {
		display: flex;
		gap: 0.5rem;
		align-items: baseline;
		padding: 0.25rem 0.45rem;
		border: none;
		border-bottom: 1px solid var(--surface-3);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
		text-align: left;
		cursor: pointer;
	}
	.match-row:hover {
		background: var(--highlight);
	}
	.match-row .number {
		width: 4rem;
		flex-shrink: 0;
		color: var(--muted-2);
	}
</style>
