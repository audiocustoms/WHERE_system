/**
 * The exchange sheets of the board behind the button `Import / Export`.
 *
 * Two tables leave the board and come back unchanged — the disposition and the
 * plan of the month. Both are written as they are read by the module
 * (`crates/where-module-people/src/exchange.rs`), so a file written on one machine
 * imports on the next without a hand on it.
 */
import type { DispoEntry, ScheduleEntry, Employee, PeopleListEntry, Shift } from '$core/api';
import { t } from '$core/i18n.svelte';
import { toIso, toText, monthRange } from './roster-report';

/** Head of the disposition sheet, as the module reads it. */
export const DISPO_HEAD = ['Standort', 'Datum', 'Beginn', 'Titel', 'Kennzeichen', 'Ignoriert'];

/** Head of the plan sheet, as the module reads it. */
export const PLAN_HEAD = [
	'Personalnummer',
	'Name',
	'Datum',
	'Art',
	'Schicht',
	'Beginn',
	'Ende',
	'Pause',
	'Notiz'
];

/** One sheet of the exchange: its head, its rows and what could not go in. */
export interface ExchangeSheet {
	head: string[];
	rows: string[][];
	/** What stayed out of the sheet, named as the window shows it. */
	notes: string[];
}

/** `Nachname, Vorname` as the board writes it. */
function name(employee: Employee): string {
	return `${employee.last_name ?? ''}, ${employee.first_name ?? ''}`.replace(/^, /, '');
}

/** The short name of a shift, its full name when it has none. */
function shiftName(shift: Shift): string {
	return (shift.short_name ?? shift.name).trim();
}

/** Every day of the month as `YYYY-MM-DD`, in order. */
function daysOfMonth(year: number, month: number): string[] {
	const period = monthRange(year, month);
	const days: string[] = [];
	const end = new Date(`${period.to}T00:00:00`);
	for (let current = new Date(`${period.from}T00:00:00`); current <= end; current.setDate(current.getDate() + 1)) {
		days.push(toIso(current));
	}
	return days;
}

/**
 * The disposition of the month as its sheet: one row per event, in the order of the
 * locations of the master data, then by day and by position inside the day.
 */
export function dispoRows(
	year: number,
	month: number,
	dispo: DispoEntry[],
	locations: PeopleListEntry[]
): ExchangeSheet {
	const period = monthRange(year, month);
	const places = new Map<number, string>();
	for (const location of locations) places.set(location.id, location.name);
	const order = new Map<number, number>();
	locations.forEach((location, index) => order.set(location.id, index));

	const inside = dispo.filter((entry) => entry.day >= period.from && entry.day <= period.to);
	const sorted = [...inside].sort((left, right) => {
		const leftOrder = order.get(left.location_id) ?? Number.MAX_SAFE_INTEGER;
		const rightOrder = order.get(right.location_id) ?? Number.MAX_SAFE_INTEGER;
		if (leftOrder !== rightOrder) return leftOrder - rightOrder;
		if (left.location_id !== right.location_id) return left.location_id - right.location_id;
		if (left.day !== right.day) return left.day < right.day ? -1 : 1;
		return left.sort_order - right.sort_order;
	});

	const rows = sorted.map((entry) => [
		places.get(entry.location_id) ?? `Standort ${entry.location_id}`,
		toText(entry.day),
		entry.starts_at ?? '',
		entry.title,
		entry.flag ?? '',
		entry.ignored ? 'ja' : ''
	]);
	return { head: DISPO_HEAD, rows, notes: [] };
}

/**
 * The plan of the month as its sheet: one row per cell, employee by employee in the
 * order of the board and day by day. Only filled cells are written — the import
 * replaces the month, so an empty day needs no row.
 */
export function planRows(
	year: number,
	month: number,
	cells: Record<string, ScheduleEntry>,
	employees: Employee[],
	shifts: Shift[]
): ExchangeSheet {
	const days = daysOfMonth(year, month);
	const byId = new Map<number, Shift>();
	for (const entry of shifts) byId.set(entry.id, entry);

	const rows: string[][] = [];
	const notes: string[] = [];
	for (const employee of employees) {
		if (employee.id === null) continue;
		const number = (employee.personnel_no ?? '').trim();
		const own: string[][] = [];
		for (const day of days) {
			const entry = cells[`${employee.id}|${day}`];
			if (entry === undefined) continue;
			const found = entry.shift_id === null ? undefined : byId.get(entry.shift_id);
			let kind = '';
			let shift = '';
			let start = '';
			let end = '';
			let breakMinutes = '';
			if (entry.kind === 'shift') {
				if (found === undefined) continue;
				kind = 'Schicht';
				shift = shiftName(found);
			} else if (entry.kind === 'free') {
				kind = 'Zeitfenster';
				start = entry.starts_at ?? '';
				end = entry.ends_at ?? '';
				breakMinutes = entry.break_minutes === null ? '' : String(entry.break_minutes);
			} else if (entry.kind === 'sick') {
				kind = 'Krank';
			} else {
				kind = 'Urlaub';
			}
			own.push([
				number,
				name(employee),
				toText(day),
				kind,
				shift,
				start,
				end,
				breakMinutes,
				entry.note ?? ''
			]);
		}
		if (own.length === 0) continue;
		if (number === '') {
			notes.push(t('{name} hat keine Personalnummer und fehlt im Export.', { name: name(employee) }));
			continue;
		}
		rows.push(...own);
	}
	return { head: PLAN_HEAD, rows, notes };
}

/** Hands a sheet to the browser as a download. */
export function save(filename: string, data: string | Uint8Array, type: string): void {
	const blob = new Blob([data as BlobPart], { type });
	const url = URL.createObjectURL(blob);
	const anchor = document.createElement('a');
	anchor.href = url;
	anchor.download = filename;
	anchor.click();
	URL.revokeObjectURL(url);
}
