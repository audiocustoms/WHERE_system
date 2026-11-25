<script lang="ts">
	import { onMount } from 'svelte';
	import { api, errorText, type BankAccount, type BankAccountInput } from '$core/api';
	import { askConfirm } from '$core/confirm.svelte';
	import '$core/search/search-window.css';

	/** The three tabs from `img/Admin Menü/Bankverbindungen`. */
	type Tab = 'general' | 'sepa' | 'switzerland';

	const TABS: { key: Tab; label: string }[] = [
		{ key: 'general', label: 'Allgemein' },
		{ key: 'sepa', label: 'SEPA' },
		{ key: 'switzerland', label: 'Schweiz' }
	];

	/** The form is the edited record; `id === null` means "new". */
	interface Form {
		id: number | null;
		label: string;
		holder: string;
		iban: string;
		bic: string;
		institute: string;
		account_number: string;
		blz: string;
		bank_address: string;
		standard: boolean;
		fibu_account: string;
		sepa_creditor_id: string;
		sepa_direct_debit: boolean;
		sepa_lead_first: string;
		sepa_lead_follow: string;
		sepa_submit_time: string;
		ch_street: string;
		ch_house_number: string;
		ch_postal_code: string;
		ch_city: string;
		ch_country: string;
		ch_holder_address: string;
	}

	let accounts = $state<BankAccount[]>([]);
	let tab = $state<Tab>('general');
	let form = $state<Form | null>(null);
	let saved = $state<string>('');
	let load = $state(true);
	let error = $state<string | null>(null);

	onMount(loadAll);

	async function loadAll(onlyId?: number | null) {
		load = true;
		error = null;
		try {
			accounts = await api.bankAccounts.list();
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
			label: '',
			holder: '',
			iban: '',
			bic: '',
			institute: '',
			account_number: '',
			blz: '',
			bank_address: '',
			standard: false,
			fibu_account: '',
			sepa_creditor_id: '',
			sepa_direct_debit: false,
			sepa_lead_first: '',
			sepa_lead_follow: '',
			sepa_submit_time: '',
			ch_street: '',
			ch_house_number: '',
			ch_postal_code: '',
			ch_city: '',
			ch_country: '',
			ch_holder_address: ''
		};
	}

	function fromAccount(account: BankAccount): Form {
		return {
			id: account.id,
			label: account.label,
			holder: account.holder ?? '',
			iban: account.iban ?? '',
			bic: account.bic ?? '',
			institute: account.institute ?? '',
			account_number: account.account_number ?? '',
			blz: account.blz ?? '',
			bank_address: account.bank_address ?? '',
			standard: account.standard,
			fibu_account: account.fibu_account ?? '',
			sepa_creditor_id: account.sepa_creditor_id ?? '',
			sepa_direct_debit: account.sepa_direct_debit,
			sepa_lead_first: account.sepa_lead_first?.toString() ?? '',
			sepa_lead_follow: account.sepa_lead_follow?.toString() ?? '',
			sepa_submit_time: (account.sepa_submit_time ?? '').slice(0, 5),
			ch_street: account.ch_street ?? '',
			ch_house_number: account.ch_house_number ?? '',
			ch_postal_code: account.ch_postal_code ?? '',
			ch_city: account.ch_city ?? '',
			ch_country: account.ch_country ?? '',
			ch_holder_address: account.ch_holder_address ?? ''
		};
	}

	async function choose(id: number) {
		error = null;
		try {
			const account = await api.bankAccounts.get(id);
			form = fromAccount(account);
			saved = JSON.stringify(form);
		} catch (e) {
			error = errorText(e);
		}
	}

	function createNew() {
		form = emptyForm();
		saved = JSON.stringify(form);
		tab = 'general';
	}

	const changed = $derived(form !== null && JSON.stringify(form) !== saved);

	async function commit() {
		if (!form) return;
		error = null;
		const numbers = (text: string) => (text.trim() === '' ? null : Number(text));
		const body: BankAccountInput = {
			label: form.label,
			holder: form.holder.trim() || null,
			iban: form.iban.trim() || null,
			bic: form.bic.trim() || null,
			institute: form.institute.trim() || null,
			account_number: form.account_number.trim() || null,
			blz: form.blz.trim() || null,
			bank_address: form.bank_address.trim() || null,
			standard: form.standard,
			fibu_account: form.fibu_account.trim() || null,
			sepa_creditor_id: form.sepa_creditor_id.trim() || null,
			sepa_direct_debit: form.sepa_direct_debit,
			sepa_lead_first: numbers(form.sepa_lead_first),
			sepa_lead_follow: numbers(form.sepa_lead_follow),
			sepa_submit_time: form.sepa_submit_time.trim() || null,
			ch_street: form.ch_street.trim() || null,
			ch_house_number: form.ch_house_number.trim() || null,
			ch_postal_code: form.ch_postal_code.trim() || null,
			ch_city: form.ch_city.trim() || null,
			ch_country: form.ch_country.trim() || null,
			ch_holder_address: form.ch_holder_address.trim() || null
		};
		try {
			const account =
				form.id === null
					? await api.bankAccounts.create(body)
					: await api.bankAccounts.update(form.id, body);
			await loadAll(account.id);
		} catch (e) {
			error = errorText(e);
		}
	}

	async function cancel() {
		if (changed && !(await askConfirm({
			title: 'Abbrechen',
			message: 'Das Bearbeiten der Bankverbindung wirklich abbrechen? Nicht gesicherte Eingaben gehen verloren.',
			confirmLabel: 'Ja, cancel',
			cancelLabel: 'Weiter bearbeiten'
		})))
			return;
		if (form?.id != null) await choose(form.id);
		else createNew();
	}

	async function remove() {
		if (form?.id == null) return;
		const account = accounts.find((entry) => entry.id === form!.id);
		const ok = await askConfirm({
			title: 'Löschen',
			message: `Bankverbindung „${account?.label ?? form.label}“ löschen?`,
			confirmLabel: 'Löschen',
			cancelLabel: 'Behalten'
		});
		if (!ok) return;
		error = null;
		try {
			await api.bankAccounts.remove(form.id);
			form = null;
			saved = '';
			await loadAll(null);
		} catch (e) {
			error = errorText(e);
		}
	}

	/** `Std` in the list: toggles the default connection. */
	async function toggleStandard(account: BankAccount) {
		error = null;
		try {
			await api.bankAccounts.setStandard(account.id, !account.standard);
			await loadAll();
		} catch (e) {
			error = errorText(e);
		}
	}

	/** "Convert account & bank code into IBAN & BIC". */
	async function convert() {
		if (!form) return;
		error = null;
		try {
			const result = await api.bankAccounts.convert({
				blz: form.blz.trim(),
				account_number: form.account_number.trim()
			});
			form = {
				...form,
				iban: result.iban,
				bic: result.bic ?? form.bic,
				institute: result.institute ?? form.institute,
				bank_address: result.bank_address ?? form.bank_address
			};
		} catch (e) {
			error = errorText(e);
		}
	}
</script>

<svelte:head><title>Bankverbindungen — WHERE</title></svelte:head>

<h1>Bankverbindungen</h1>

<div class="window">
	<div class="left">
		<div class="list-head">
			<button type="button" class="square" title="Neue Bankverbindung" onclick={createNew}>+</button>
			<button
				type="button"
				class="square"
				title="Löschen"
				disabled={form?.id == null}
				onclick={remove}>−</button
			>
		</div>
		<div class="list">
			<table>
				<thead>
					<tr><th class="center">Std</th><th class="center">EB</th><th>Bankverbindungen</th></tr>
				</thead>
				<tbody>
					{#each accounts as account (account.id)}
						<tr
							class:selected={form?.id === account.id}
							onclick={() => choose(account.id)}
						>
							<td class="center">
								<input
									type="checkbox"
									checked={account.standard}
									title="Standard-Bankverbindung"
									onclick={(e) => e.stopPropagation()}
									onchange={() => toggleStandard(account)}
								/>
							</td>
							<td class="center">
								<input
									type="checkbox"
									checked={account.electronic_banking}
									disabled
									title="Kontenmappe für Electronic Banking hinterlegt"
								/>
							</td>
							<td>{account.label}</td>
						</tr>
					{:else}
						<tr><td colspan="3">{load ? 'Lade …' : 'Noch keine Bankverbindung.'}</td></tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>

	<div class="right">
		<div class="tab" role="tablist" aria-label="Bankverbindung">
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
				{#if tab === 'general'}
					<div class="row">
						<span class="label">Bezeichnung</span>
						<input
							class="text"
							maxlength="255"
							value={form.label}
							oninput={(e) => (form!.label = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">Kontoinhaber</span>
						<input
							class="text"
							value={form.holder}
							oninput={(e) => (form!.holder = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">IBAN (QR-IBAN)</span>
						<input
							class="text"
							maxlength="34"
							value={form.iban}
							oninput={(e) => (form!.iban = e.currentTarget.value.toUpperCase())}
						/>
					</div>
					<div class="row">
						<span class="label">BIC</span>
						<input
							class="text calculated"
							maxlength="11"
							value={form.bic}
							oninput={(e) => (form!.bic = e.currentTarget.value.toUpperCase())}
						/>
					</div>
					<div class="row">
						<span class="label">Bankname</span>
						<input
							class="text"
							value={form.institute}
							oninput={(e) => (form!.institute = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">Kontonummer</span>
						<input
							class="text"
							maxlength="20"
							value={form.account_number}
							oninput={(e) => (form!.account_number = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">BLZ</span>
						<div class="field-with-button">
							<input
								class="text"
								maxlength="20"
								value={form.blz}
								oninput={(e) => (form!.blz = e.currentTarget.value)}
							/>
							<button type="button" class="button" onclick={convert}>
								Kto &amp; BLZ in IBAN &amp; BIC convert
							</button>
						</div>
					</div>
					<div class="row top">
						<span class="label">Anschrift der Bank</span>
						<textarea
							class="text"
							rows="4"
							value={form.bank_address}
							oninput={(e) => (form!.bank_address = e.currentTarget.value)}
						></textarea>
					</div>
					<div class="row">
						<span class="label">Standard Bankverbindung</span>
						<label class="check">
							<input
								type="checkbox"
								checked={form.standard}
								onchange={(e) => (form!.standard = e.currentTarget.checked)}
							/>
							Ja
						</label>
					</div>
					<div class="row">
						<span class="label">FIBU-Konto</span>
						<input
							class="text short"
							maxlength="10"
							value={form.fibu_account}
							oninput={(e) => (form!.fibu_account = e.currentTarget.value)}
						/>
					</div>
				{:else if tab === 'sepa'}
					<div class="row">
						<span class="label">Gläubiger-ID</span>
						<input
							class="text"
							maxlength="255"
							value={form.sepa_creditor_id}
							oninput={(e) => (form!.sepa_creditor_id = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">Sammellastschrift</span>
						<label class="check">
							<input
								type="checkbox"
								checked={form.sepa_direct_debit}
								onchange={(e) => (form!.sepa_direct_debit = e.currentTarget.checked)}
							/>
							Ja
						</label>
					</div>
					<div class="row">
						<span class="label">Späteste Vorlaufzeit (Werktage) bei Erstlastschriften</span>
						<input
							class="number"
							type="number"
							min="0"
							value={form.sepa_lead_first}
							oninput={(e) => (form!.sepa_lead_first = e.currentTarget.value)}
						/>
						<span class="unit">Tage</span>
					</div>
					<div class="row">
						<span class="label">Späteste Vorlaufzeit (Werktage) bei Folgelastschriften</span>
						<input
							class="number"
							type="number"
							min="0"
							value={form.sepa_lead_follow}
							oninput={(e) => (form!.sepa_lead_follow = e.currentTarget.value)}
						/>
						<span class="unit">Tage</span>
					</div>
					<div class="row">
						<span class="label">Späteste Abgabezeit an Werktagen</span>
						<input
							class="time"
							type="time"
							value={form.sepa_submit_time}
							oninput={(e) => (form!.sepa_submit_time = e.currentTarget.value)}
						/>
					</div>
				{:else}
					<h2>Anschrift Kontoinhaber</h2>
					<div class="row">
						<span class="label">Straße</span>
						<input
							class="text"
							value={form.ch_street}
							oninput={(e) => (form!.ch_street = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">Hausnummer</span>
						<input
							class="text short"
							value={form.ch_house_number}
							oninput={(e) => (form!.ch_house_number = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">PLZ</span>
						<input
							class="text short"
							value={form.ch_postal_code}
							oninput={(e) => (form!.ch_postal_code = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">Ort</span>
						<input
							class="text"
							value={form.ch_city}
							oninput={(e) => (form!.ch_city = e.currentTarget.value)}
						/>
					</div>
					<div class="row">
						<span class="label">Land</span>
						<div class="field-with-button">
							<input
								class="text short"
								maxlength="2"
								placeholder="CH"
								value={form.ch_country}
								oninput={(e) => (form!.ch_country = e.currentTarget.value.toUpperCase())}
							/>
							<span class="unit">(CH oder LI)</span>
						</div>
					</div>
					<div class="row top">
						<span class="label">Anschrift gesamt</span>
						<textarea
							class="text"
							rows="3"
							value={form.ch_holder_address}
							oninput={(e) => (form!.ch_holder_address = e.currentTarget.value)}
						></textarea>
					</div>
				{/if}

				<div class="sheet-foot">
					<button
						type="button"
						class="button primary"
						onclick={commit}
						disabled={!form.label.trim()}>{form.id === null ? 'Anlegen' : 'Sichern'}</button
					>
				</div>
			</div>
		{:else}
			<div class="sheet empty">
				<p class="small">Links eine Bankverbindung wählen oder mit `+` eine neue anlegen.</p>
			</div>
		{/if}
	</div>
</div>

<footer class="footer">
	<span class="small">Die Liste zeigt `Std` (Standard) und `EB` (Electronic Banking).</span>
	<span class="spacer"></span>
	<button type="button" class="button" onclick={cancel} disabled={!form || !changed}>
		Abbrechen
	</button>
	<a class="button" href="/">Schließen</a>
</footer>

{#if error}
	<p class="error">{error}</p>
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
	.window {
		display: grid;
		grid-template-columns: 22rem minmax(24rem, 34rem);
		gap: 0.9rem;
		align-items: start;
	}
	.left {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
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
	.list {
		max-height: 26rem;
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
	tbody tr.selected {
		background: var(--highlight);
		outline: 1px solid var(--accent);
		outline-offset: -1px;
	}
	th.center,
	td.center {
		text-align: center;
		width: 2.4rem;
	}
	.tab {
		display: flex;
		gap: 0.25rem;
	}
	.tab button {
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
	.tab button.active {
		background: var(--surface);
		color: var(--accent);
		font-weight: 600;
	}
	.sheet {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-height: 26rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: 0 4px 4px 4px;
		background: var(--surface);
	}
	.sheet.empty {
		justify-content: center;
		align-items: center;
	}
	.sheet h2 {
		margin: 0 0 0.4rem;
		font-size: 0.95rem;
		color: var(--accent);
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 1.9rem;
	}
	.row.top {
		align-items: flex-start;
	}
	.label {
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
	textarea.text {
		resize: vertical;
	}
	.text.short {
		flex: 0 0 6rem;
	}
	.text.calculated {
		background: color-mix(in srgb, var(--highlight) 60%, var(--surface));
	}
	.field-with-button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		flex: 1;
		min-width: 0;
	}
	.field-with-button .text:not(.short) {
		flex: 0 0 9rem;
	}
	.number {
		width: 4.5rem;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		text-align: right;
	}
	.time {
		width: 6rem;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
	}
	.unit {
		font-size: 0.78rem;
		color: var(--muted-2);
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.84rem;
	}
	.sheet-foot {
		display: flex;
		justify-content: flex-end;
		margin-top: auto;
		padding-top: 0.6rem;
	}
	.footer {
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
	.button {
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
	.button:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.button.primary {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.button.primary:disabled {
		border-color: var(--border);
		color: var(--muted-2);
		font-weight: 400;
	}
	.small {
		font-size: 0.78rem;
		color: var(--muted-2);
	}
</style>
