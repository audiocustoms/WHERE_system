/**
 * The master-data sheets of the module behind the button `Import / Export`.
 *
 * Three tables leave the module and come back unchanged, so master data written in one
 * installation can be read in the next without a hand on the file: the lists of the
 * employee mask, the employees themselves and the shifts of the duty planning. The
 * module reads them back in `crates/where-module-people/src/masterdata.rs`; head names
 * and values here have to stay in step with that reader.
 */
import type { Employee, PeopleListEntry, Shift } from '$core/api';
import { tableCsv } from './roster-export';
import { save, type ExchangeSheet } from './roster-exchange';

/** Head of the list sheet: which list, then the name of the entry. */
export const LISTS_HEAD = ['Liste', 'Name', 'Abteilung', 'Vorarbeiter'];

/** Head of the employee sheet — the columns of the mask. */
export const STAFF_HEAD = [
	'Personalnummer',
	'Anrede',
	'Titel',
	'Vorname',
	'Nachname',
	'Geburtsdatum',
	'Geburtsort',
	'Nationalität',
	'Familienstand',
	'Straße',
	'PLZ',
	'Ort',
	'Land',
	'E-Mail',
	'Telefon',
	'Mobil',
	'Eintritt',
	'Austritt',
	'Beschäftigung',
	'Wochenstunden',
	'Arbeitstage',
	'Urlaubstage',
	'Position',
	'Abteilung',
	'Unterabteilung',
	'Standort',
	'Kostenstelle',
	'Vorgesetzter',
	'Sozialversicherungsnummer',
	'Steuer-ID',
	'Notiz',
	'Aktiv',
	'Azubi',
	'Automatisch disponierbar',
	'Qualifikationen'
];

/** Head of the shift sheet. */
export const SHIFTS_HEAD = [
	'Kürzel',
	'Name',
	'Beginn',
	'Ende',
	'Pause',
	'Band',
	'Nur Azubis',
	'Farbe',
	'Aktiv',
	'Ausgeschlossene Positionen'
];

/** A switch as the sheet writes it; the reader takes `ja`/`nein`. */
function yesNo(value: boolean): string {
	return value ? 'ja' : 'nein';
}

/** An empty text stays empty, so a round trip keeps the record unchanged. */
function label(value: string | null | undefined): string {
	return (value ?? '').trim();
}

/** Hours per week with a decimal comma, as a German Excel reads them. */
function hours(value: string | null): string {
	return value === null ? '' : value.replace('.', ',');
}

/** The band of a shift as the sheet names it. */
function band(value: Shift['band']): string {
	if (value === 'early') return 'früh';
	if (value === 'late') return 'spät';
	return value;
}

/**
 * All four lists as one sheet: the departments, their sub-departments, the positions
 * and the locations, each in its maintained order. A sub-department and a location
 * carry the department they belong to in the `Abteilung` column; only a location
 * carries the foreman.
 */
export function listRows(
	departments: PeopleListEntry[],
	subdepartments: PeopleListEntry[],
	positions: PeopleListEntry[],
	locations: PeopleListEntry[]
): ExchangeSheet {
	const rows: string[][] = [];
	for (const entry of departments) rows.push(['Abteilungen', entry.name, '', '']);
	for (const entry of subdepartments) {
		rows.push(['Unterabteilungen', entry.name, label(entry.department), '']);
	}
	for (const entry of positions) rows.push(['Positionen', entry.name, '', '']);
	for (const entry of locations) {
		rows.push([
			'Standorte',
			entry.name,
			label(entry.department),
			yesNo(entry.needs_foreman ?? true)
		]);
	}
	return { head: LISTS_HEAD, rows, notes: [] };
}

/**
 * The employee master as one sheet: one row per person, active or not. The personnel
 * number is the key the import finds a person again by; without one the import falls
 * back to the name, so a row is never lost here.
 */
export function staffRows(employees: Employee[]): ExchangeSheet {
	const rows = employees.map((person) => [
		label(person.personnel_no),
		label(person.salutation),
		label(person.title),
		person.first_name ?? '',
		person.last_name ?? '',
		label(person.birth_date),
		label(person.birth_place),
		label(person.nationality),
		label(person.marital_status),
		label(person.street),
		label(person.postal_code),
		label(person.city),
		label(person.country),
		label(person.email),
		label(person.phone),
		label(person.mobile),
		label(person.entry_date),
		label(person.exit_date),
		label(person.employment_type),
		hours(person.weekly_hours),
		person.work_days === null ? '' : String(person.work_days),
		person.vacation_days === null ? '' : String(person.vacation_days),
		label(person.position),
		label(person.department),
		label(person.subdepartment),
		label(person.location),
		label(person.cost_center),
		label(person.manager),
		label(person.social_security_no),
		label(person.tax_id),
		label(person.notes),
		yesNo(person.active),
		yesNo(person.is_apprentice),
		yesNo(person.auto_plan),
		person.qualifications.join(', ')
	]);
	return { head: STAFF_HEAD, rows, notes: [] };
}

/**
 * The shifts of the duty planning as one sheet, in the order the page keeps them. The
 * short name is the key; a shift without one is found by its name.
 */
export function shiftRows(shifts: Shift[]): ExchangeSheet {
	const rows = shifts.map((entry) => [
		label(entry.short_name),
		entry.name,
		entry.starts_at,
		entry.ends_at,
		String(entry.break_minutes),
		band(entry.band),
		yesNo(entry.apprentice_only),
		label(entry.color),
		yesNo(entry.active),
		entry.excluded_positions.join(', ')
	]);
	return { head: SHIFTS_HEAD, rows, notes: [] };
}

/** Writes a sheet as the file the module reads back. */
export function saveSheet(name: string, sheet: ExchangeSheet): void {
	save(`${name}.csv`, tableCsv(sheet.head, sheet.rows), 'text/csv;charset=utf-8');
}
