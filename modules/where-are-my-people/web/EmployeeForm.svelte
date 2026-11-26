<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type Employee,
		type EmployeeInput,
		type PeopleListEntry
	} from '$core/api';
	import { auth } from '$core/auth.svelte';
	import { askConfirm } from '$core/confirm.svelte';
	import { escapeWindows } from '$core/escape.svelte';
	import { RIGHT } from '$core/rights';
	import '$core/search/search-window.css';
	import './people-window.css';

	interface Props {
		/** `null` creates a new employee. */
		employee: Employee | null;
		onclose: () => void;
		onsaved: (employee: Employee) => void;
		/** After deleting in the footer. */
		onremoved: () => void;
	}

	let { employee, onclose, onsaved, onremoved }: Props = $props();

	type Tab = 'person' | 'employment' | 'qualifications';

	/** The form works with text; the record is built only when saving. */
	interface Draft {
		personnel_no: string;
		salutation: string;
		title: string;
		first_name: string;
		last_name: string;
		birth_date: string;
		birth_place: string;
		nationality: string;
		marital_status: string;
		street: string;
		postal_code: string;
		city: string;
		country: string;
		email: string;
		phone: string;
		mobile: string;
		entry_date: string;
		exit_date: string;
		employment_type: string;
		weekly_hours: string;
		work_days: string;
		vacation_days: string;
		position: string;
		department: string;
		subdepartment: string;
		location: string;
		cost_center: string;
		manager: string;
		social_security_no: string;
		tax_id: string;
		notes: string;
		active: boolean;
		is_apprentice: boolean;
		auto_plan: boolean;
		qualifications: string[];
	}

	const SALUTATIONS = ['Herr', 'Frau', 'Divers'];
	const MARITAL_STATUS = [
		'ledig',
		'verheiratet',
		'geschieden',
		'verwitwet',
		'eingetragene Partnerschaft'
	];
		const EMPLOYMENT_TYPES = [
		'Vollzeit',
		'Teilzeit',
		'Geringfügig beschäftigt',
		'Ausbildung',
		'Praktikum',
		'Werkvertrag'
	];

	const PERSONAL_DATA_RIGHT = 'personal.persoenliche-daten-sehen';

	function label(value: string | null | undefined): string {
		return value ?? '';
	}

	function asDate(value: string | null | undefined): string {
		return value ? value.slice(0, 10) : '';
	}

	function asNumber(value: string | null | undefined): string {
		if (!value) return '';
		const n = Number(value);
		return Number.isFinite(n) ? String(n) : value;
	}

		function draftOf(source: Employee | null): Draft {
		return {
			personnel_no: label(source?.personnel_no),
			salutation: label(source?.salutation),
			title: label(source?.title),
			first_name: label(source?.first_name),
			last_name: label(source?.last_name),
			birth_date: asDate(source?.birth_date ?? null),
			birth_place: label(source?.birth_place),
			nationality: label(source?.nationality),
			marital_status: label(source?.marital_status),
			street: label(source?.street),
			postal_code: label(source?.postal_code),
			city: label(source?.city),
			country: label(source?.country ?? 'Deutschland'),
			email: label(source?.email),
			phone: label(source?.phone),
			mobile: label(source?.mobile),
			entry_date: asDate(source?.entry_date ?? null),
			exit_date: asDate(source?.exit_date ?? null),
			employment_type: label(source?.employment_type),
			weekly_hours: asNumber(source?.weekly_hours ?? null),
			work_days: source?.work_days == null ? '' : String(source.work_days),
			vacation_days: source?.vacation_days == null ? '' : String(source.vacation_days),
			position: label(source?.position),
			department: label(source?.department),
			subdepartment: label(source?.subdepartment),
			location: label(source?.location),
			cost_center: label(source?.cost_center),
			manager: label(source?.manager),
			social_security_no: label(source?.social_security_no),
			tax_id: label(source?.tax_id),
			notes: label(source?.notes),
			active: source?.active ?? true,
			is_apprentice: source?.is_apprentice ?? false,
			auto_plan: source?.auto_plan ?? true,
			qualifications: [...(source?.qualifications ?? [])]
		};
	}

	// svelte-ignore state_referenced_locally
		let e = $state<Draft>(draftOf(employee));
	let tab = $state<Tab>('person');
	let saving = $state(false);
	let error = $state<string | null>(null);

	/** Birth data, address and social insurance only with the right. */
	const maySeePersonal = $derived(auth.hasRight(RIGHT.maySeePersonal));
	const isNew = $derived(employee === null);
	/** Delete appears in the footer only for an existing record and with the right. */
	const deletable = $derived(!isNew && auth.hasRight(RIGHT.mayDeletePersonal));

	/** Suggestion lists from `Stammdaten → Personal → Abteilungen und Positionen`. */
	let departments = $state<PeopleListEntry[]>([]);
	let subDepartments = $state<PeopleListEntry[]>([]);
	let positions = $state<PeopleListEntry[]>([]);
	let locations = $state<PeopleListEntry[]>([]);

	// `Esc` closes the mask like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));

	onMount(async () => {
		try {
			[departments, subDepartments, positions, locations] = await Promise.all([
				api.people.lists.list('departments'),
				api.people.lists.list('subdepartments'),
				api.people.lists.list('positions'),
				api.people.lists.list('locations')
			]);
			// Rows the board types itself are no houses: nobody works "in" them.
			locations = locations.filter((entry) => entry.is_text !== true);
		} catch {
			// Without the lists, department, position and location stay empty selects.
		}
	});

	/** The sub-departments that belong to the chosen department. */
	const matchingSubDepartments = $derived.by(() => {
		const department = e.department.trim().toLowerCase();
		if (department === '') return subDepartments;
		return subDepartments.filter(
			(entry) => (entry.department ?? '').trim().toLowerCase() === department
		);
	});

	// New employees are offered the next number of the number series;
	// if the field stays empty, the API assigns the number when saving.
	onMount(async () => {
		if (!isNew) return;
		try {
			const { personnel_no } = await api.people.nextNumber();
			if (e.personnel_no.trim() === '') e.personnel_no = personnel_no;
		} catch {
			// Without a number series the field stays empty and is filled when saving.
		}
	});

	function toText(value: string): string | null {
		const trimmed = value.trim();
		return trimmed === '' ? null : trimmed;
	}

	function toDate(value: string): string | null {
		return value === '' ? null : value;
	}

	function toNumber(value: string): string | null {
		const trimmed = value.trim().replace(',', '.');
		if (trimmed === '') return null;
		const n = Number(trimmed);
		return Number.isFinite(n) ? String(n) : null;
	}

	function toInput(): EmployeeInput {
		return {
			personnel_no: toText(e.personnel_no),
			salutation: toText(e.salutation),
			title: toText(e.title),
			first_name: e.first_name.trim(),
			last_name: e.last_name.trim(),
			birth_date: toDate(e.birth_date),
			birth_place: toText(e.birth_place),
			nationality: toText(e.nationality),
			marital_status: toText(e.marital_status),
			street: toText(e.street),
			postal_code: toText(e.postal_code),
			city: toText(e.city),
			country: toText(e.country),
			email: toText(e.email),
			phone: toText(e.phone),
			mobile: toText(e.mobile),
			entry_date: toDate(e.entry_date),
			exit_date: toDate(e.exit_date),
			employment_type: toText(e.employment_type),
			weekly_hours: toNumber(e.weekly_hours),
			work_days: e.work_days === '' ? null : Number(e.work_days),
			vacation_days: e.vacation_days === '' ? null : Number(e.vacation_days),
			position: toText(e.position),
			department: toText(e.department),
			subdepartment: toText(e.subdepartment),
			location: toText(e.location),
			cost_center: toText(e.cost_center),
			manager: toText(e.manager),
			social_security_no: toText(e.social_security_no),
			tax_id: toText(e.tax_id),
			notes: toText(e.notes),
			active: e.active,
			is_apprentice: e.is_apprentice,
			auto_plan: e.auto_plan,
			qualifications: e.qualifications.map((q) => q.trim()).filter((q) => q !== '')
		};
	}

	async function save() {
		if (e.first_name.trim() === '' || e.last_name.trim() === '') {
			tab = 'person';
			error = t('Vor- und Nachname sind Pflichtfelder.');
			return;
		}
		saving = true;
		error = null;
		try {
						const body = toInput();
			const saved =
				employee?.id == null
					? await api.people.create(body)
					: await api.people.update(employee.id, body);
			onsaved(saved);
		} catch (err) {
			error = errorText(err);
		} finally {
			saving = false;
		}
	}

	function addQualification() {
		e.qualifications = [...e.qualifications, ''];
	}

	function removeQualification(index: number) {
		e.qualifications = e.qualifications.filter((_, i) => i !== index);
	}

	async function remove() {
		if (employee?.id == null) return;
		if (
			!(await askConfirm({
				message: t('Mitarbeiter „{name}“ entfernen?', {
					name: `${e.first_name} ${e.last_name}`
				}),
				confirmLabel: t('Entfernen')
			}))
		)
			return;
		saving = true;
		error = null;
		try {
			await api.people.remove(employee.id);
			onremoved();
		} catch (err) {
			error = errorText(err);
		} finally {
			saving = false;
		}
	}

	const stamp = $derived(
		employee?.changed_at
			? `${t('geändert')}\n${new Date(employee.changed_at).toLocaleString('de-DE')}`
			: t('neu')
	);
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window pep-window"
		role="dialog"
		aria-modal="true"
		aria-label={t('Mitarbeiter')}
		style="width: min(62rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{t('Mitarbeiter:')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={onclose}>×</button>
		</div>

		<div class="pep-band">
			<h2>
								{#if isNew}{t('Neuer Mitarbeiter')}{:else}{e.first_name} {e.last_name}{/if}
			</h2>
			<span class="pep-stamp">{stamp}</span>
		</div>

		<nav class="pep-register" aria-label={t('Register')}>
			<button 	class:active={tab === 'person'} type="button" 	onclick={() => (tab = 'person')}>
				{t('Person')}
			</button>
			<button
					class:active={tab === 'employment'}
				type="button"
					onclick={() => (tab = 'employment')}>{t('Beschäftigung')}</button
			>
			<button
					class:active={tab === 'qualifications'}
				type="button"
					onclick={() => (tab = 'qualifications')}>{t('Qualifikationen')}</button
			>
		</nav>

		{#if error}<p class="pep-error">{error}</p>{/if}

		<form
			class="pep-content"
			onsubmit={(event) => {
				event.preventDefault();
									save();
			}}
		>
				{#if tab === 'person'}
				<fieldset class="pep-group">
					<legend>{t('Name')}</legend>
					<div class="pep-columns">
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('Pers.-Nr.')}</span>
								<input
									bind:value={e.personnel_no}
									maxlength="30"
									placeholder={t('wird automatisch vergeben')}
									title={t('Leer lassen: der Nummernkreis vergibt die nächste Personalnummer (ab 1001).')}
								/>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Anrede')}</span>
								<select bind:value={e.salutation}>
									<option value="">—</option>
									{#each SALUTATIONS as salutation (salutation)}
										<option value={salutation}>{t(salutation)}</option>
									{/each}
								</select>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Titel')}</span>
								<input bind:value={e.title} maxlength="50" />
							</div>
							<div class="pep-row required">
								<span class="pep-label">{t('Vorname')}</span>
								<input bind:value={e.first_name} required maxlength="100" />
							</div>
							<div class="pep-row required">
								<span class="pep-label">{t('Nachname')}</span>
								<input bind:value={e.last_name} required maxlength="100" />
							</div>
						</div>
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('Geburtsdatum')}</span>
								<input
									type="date"
									bind:value={e.birth_date}
										disabled={!maySeePersonal}
										title={maySeePersonal
										? undefined
										: t('Dafür fehlt das Recht „Persönliche Daten sehen“.')}
								/>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Geburtsort')}</span>
								<input
									bind:value={e.birth_place}
									maxlength="100"
										disabled={!maySeePersonal}
								/>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Nationalität')}</span>
								<input
									bind:value={e.nationality}
									maxlength="60"
										disabled={!maySeePersonal}
								/>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Familienstand')}</span>
								<select bind:value={e.marital_status} 	disabled={!maySeePersonal}>
									<option value="">—</option>
									{#each MARITAL_STATUS as status (status)}
										<option value={status}>{t(status)}</option>
									{/each}
								</select>
							</div>
						</div>
					</div>
				</fieldset>

				<fieldset class="pep-group">
					<legend>{t('Anschrift und Kontakt')}</legend>
					<div class="pep-columns">
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('Straße')}</span>
								<input bind:value={e.street} maxlength="150" 	disabled={!maySeePersonal} />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('PLZ')}</span>
								<input bind:value={e.postal_code} maxlength="20" 	disabled={!maySeePersonal} />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Ort')}</span>
								<input bind:value={e.city} maxlength="100" 	disabled={!maySeePersonal} />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Land')}</span>
								<input bind:value={e.country} maxlength="60" 	disabled={!maySeePersonal} />
							</div>
						</div>
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('Telefon')}</span>
								<input bind:value={e.phone} maxlength="50" />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Mobil')}</span>
								<input bind:value={e.mobile} maxlength="50" />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('eMail')}</span>
								<input type="email" bind:value={e.email} maxlength="150" />
							</div>
						</div>
					</div>
				</fieldset>
			{/if}

				{#if tab === 'employment'}
				<fieldset class="pep-group">
					<legend>{t('Anstellung')}</legend>
					<div class="pep-columns">
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('Eintritt')}</span>
								<input type="date" bind:value={e.entry_date} />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Austritt')}</span>
								<input type="date" bind:value={e.exit_date} />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Beschäftigungsart')}</span>
								<select bind:value={e.employment_type}>
									<option value="">—</option>
										{#each EMPLOYMENT_TYPES as type (type)}
												<option value={type}>{t(type)}</option>
									{/each}
								</select>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Wochenstunden')}</span>
								<input bind:value={e.weekly_hours} inputmode="decimal" placeholder="40" />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Wochenarbeitstage')}</span>
								<input
									type="number"
									inputmode="numeric"
									min="1"
									max="7"
									step="1"
									placeholder="5"
									value={e.work_days}
									oninput={(event) => (e.work_days = event.currentTarget.value)}
								/>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Urlaubstage')}</span>
								<input
									type="number"
									inputmode="numeric"
									min="0"
									max="366"
									step="1"
									placeholder="30"
									value={e.vacation_days}
									oninput={(event) => (e.vacation_days = event.currentTarget.value)}
								/>
							</div>
						</div>
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('Abteilung')}</span>
								<select bind:value={e.department}>
									<option value="">—</option>
										{#each departments as entry (entry.id)}
											<option value={entry.name}>{entry.name}</option>
									{/each}
									{#if e.department && 	!departments.some((entry) => entry.name === e.department)}
										<option value={e.department}>{e.department}</option>
									{/if}
								</select>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Unterabteilung')}</span>
								<select bind:value={e.subdepartment}>
									<option value="">—</option>
										{#each matchingSubDepartments as entry (entry.id)}
											<option value={entry.name}>{entry.name}</option>
									{/each}
									{#if e.subdepartment && 	!matchingSubDepartments.some((entry) => entry.name === e.subdepartment)}
										<option value={e.subdepartment}>{e.subdepartment}</option>
									{/if}
								</select>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Position')}</span>
								<select bind:value={e.position}>
									<option value="">—</option>
										{#each positions as entry (entry.id)}
											<option value={entry.name}>{entry.name}</option>
									{/each}
									{#if e.position && 	!positions.some((entry) => entry.name === e.position)}
										<option value={e.position}>{e.position}</option>
									{/if}
								</select>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Standort')}</span>
								<select bind:value={e.location}>
									<option value="">—</option>
										{#each locations as entry (entry.id)}
											<option value={entry.name}>{entry.name}</option>
									{/each}
									{#if e.location && 	!locations.some((entry) => entry.name === e.location)}
										<option value={e.location}>{e.location}</option>
									{/if}
								</select>
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Kostenstelle')}</span>
								<input bind:value={e.cost_center} maxlength="50" />
							</div>
							<div class="pep-row">
								<span class="pep-label">{t('Vorgesetzter')}</span>
								<input bind:value={e.manager} maxlength="100" />
							</div>
						</div>
					</div>
					<div class="pep-row">
						<span class="pep-label">{t('Mitarbeiterart')}</span>
						<label class="pep-check">
							<input type="checkbox" bind:checked={e.is_apprentice} />
							{t('Auszubildende/r')}
						</label>
					</div>
					<div class="pep-row">
						<span class="pep-label">{t('Status')}</span>
						<label class="pep-check">
							<input type="checkbox" bind:checked={e.active} />
							{t('aktiv')}
						</label>
					</div>
					<div class="pep-row">
						<span class="pep-label">{t('Disposition')}</span>
						<label class="pep-check">
							<input
								type="checkbox"
								checked={!e.auto_plan}
								onchange={(event) => (e.auto_plan = !event.currentTarget.checked)}
							/>
							{t('nicht automatisch disponierbar')}
						</label>
					</div>
				</fieldset>

				<fieldset class="pep-group">
					<legend>{t('Sozialversicherung')}</legend>
					<div class="pep-columns">
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('SV-Nummer')}</span>
								<input
									bind:value={e.social_security_no}
									maxlength="30"
										disabled={!maySeePersonal}
								/>
							</div>
						</div>
						<div>
							<div class="pep-row">
								<span class="pep-label">{t('Steuer-ID')}</span>
								<input bind:value={e.tax_id} maxlength="30" 	disabled={!maySeePersonal} />
							</div>
						</div>
					</div>
				</fieldset>

				<fieldset class="pep-group">
					<legend>{t('Notizen')}</legend>
					<div class="pep-row">
						<span class="pep-label">{t('Bemerkung')}</span>
						<textarea bind:value={e.notes} rows="4"></textarea>
					</div>
				</fieldset>
			{/if}

				{#if tab === 'qualifications'}
				<fieldset class="pep-group">
					<legend>{t('Qualifikationen')}</legend>
					<div class="pep-quals">
						{#each e.qualifications as _, index (index)}
							<div class="pep-qual">
								<input
									bind:value={e.qualifications[index]}
									maxlength="100"
									placeholder={t('z. B. Staplerschein')}
								/>
								<button
									type="button"
									title={t('Qualifikation entfernen')}
										onclick={() => removeQualification(index)}>×</button
								>
							</div>
						{:else}
							<p class="pep-qual-empty">{t('Noch keine Qualifikation eingetragen.')}</p>
						{/each}
					</div>
					<div class="pep-qual-footer">
						<button class="pep-add" type="button" 	onclick={addQualification}>
							{t('Qualifikation +')}
						</button>
					</div>
				</fieldset>
			{/if}
		</form>

		<div class="sw-footer">
						{#if deletable}
				<button
					class="sw-button pep-footer-delete"
					type="button"
						onclick={remove}
					disabled={saving}>{t('Löschen')}</button
				>
			{/if}
			<button class="sw-button" type="button" onclick={onclose}>{t('Abbrechen')}</button>
			<button class="sw-button sw-primary" type="button" 	onclick={save} disabled={saving}>
				{saving ? t('Sichern …') : t('Sichern')}
			</button>
		</div>
	</div>
</div>
