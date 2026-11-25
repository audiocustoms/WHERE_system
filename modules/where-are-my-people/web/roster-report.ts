/**
 * The sheet behind the print button and the exchange of the board: one month of
 * the plan as the printer, the CSV, the XLSX and the PDF show it.
 *
 * The dialog of the printer and the window of the exchange share this, so both
 * show the same month with the same rows, hours and columns.
 */
import { api, type DispoEntry, type DutyEntry, type Employee, type PeopleListEntry, type Shift } from '$core/api';
import type { Report, SheetDay, DispoEvent, DispoRow, StaffRow } from './roster-export';

/** Colour of a cell with a time window of its own — the same on the board and on paper. */
export const TIME_WINDOW_COLOR = '#7C3AED';

const WEEKDAYS = ['So', 'Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa'];

/** Months as the sheet writes them. */
export const MONTHS = [
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

/** Which part of the month the sheet carries. */
export type ReportPart = 'all' | 'dispo' | 'staff';

/** The month of the board, from its first to its last day. */
export interface Month {
	from: string;
	to: string;
}

export function toDate(iso: string): Date {
	const [year, month, day] = iso.split('-').map(Number);
	return new Date(year, month - 1, day);
}

export function toIso(date: Date): string {
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${date.getFullYear()}-${month}-${day}`;
}

export function toText(iso: string): string {
	const [year, month, day] = iso.split('-');
	return `${day}.${month}.${year}`;
}

/** The month that starts on `date`. */
export function monthOf(date: Date): { year: number; month: number } {
	return { year: date.getFullYear(), month: date.getMonth() + 1 };
}

/** The first and the last day of a month. */
export function monthRange(year: number, month: number): Month {
	const anchor = new Date(year, month - 1, 1);
	return { from: toIso(anchor), to: toIso(new Date(year, month, 0)) };
}

/** Everything the sheet needs about one month, straight from the API. */
export interface MonthData {
	cells: Record<string, DutyEntry>;
	dispo: DispoEntry[];
	locations: PeopleListEntry[];
	holidays: Record<string, string>;
}

export async function loadMonth(period: Month): Promise<MonthData> {
	const [entries, calendar, disposition, list] = await Promise.all([
		api.people.duty.range(period.from, period.to),
		api.people.holidays.range(period.from, period.to),
		api.people.duty.dispo.range(period.from, period.to),
		api.people.lists.list('locations')
	]);
	const cells: Record<string, DutyEntry> = {};
	for (const entry of entries) cells[`${entry.employee_id}|${entry.day}`] = entry;
	return {
		cells,
		dispo: disposition,
		locations: list,
		holidays: Object.fromEntries(calendar.holidays.map((day) => [day.day, day.name]))
	};
}

/** One row of the staff with everything the sheet prints beside the days. */
export interface SheetRow extends StaffRow {
	colors: string[];
	times: string[];
	notes: string[];
}

export interface ReportSources {
	period: Month;
	data: MonthData;
	employees: Employee[];
	shifts: Shift[];
	colors: { sick: string; vacation: string };
	/** Hours one day of vacation and one day of sickness count. */
	absence: { sick: number; vacation: number };
	paper: { width: number; height: number };
	/** Tints of the master data, normalised name → `#RRGGBB`; the sub-department
	 * wins over the department. */
	staffColors?: { subdepartment: Map<string, string>; department: Map<string, string> };
	/** Which part of the month the sheet carries. */
	part?: ReportPart;
}

/** Hours a time window lasts, the break already taken off. */
function windowHours(
	startsAt: string | null | undefined,
	endsAt: string | null | undefined,
	breakMinutes: number | null | undefined
): number {
	if (!startsAt || !endsAt) return 0;
	const [startHour, startMinute] = startsAt.split(':').map(Number);
	const [endHour, endMinute] = endsAt.split(':').map(Number);
	let minutes = endHour * 60 + endMinute - (startHour * 60 + startMinute);
	if (minutes <= 0) minutes += 24 * 60;
	return Math.max(0, (minutes - (breakMinutes ?? 0)) / 60);
}

/** Working days of the month for a contract with `perWeek` days a week. */
function workdays(year: number, month: number, perWeek: number): number {
	const daysInMonth = new Date(year, month + 1, 0).getDate();
	let count = 0;
	for (let day = 1; day <= daysInMonth; day += 1) {
		const weekday = new Date(year, month, day).getDay();
		if (weekday === 0 && perWeek < 7) continue;
		if (weekday === 6 && perWeek < 6) continue;
		count += 1;
	}
	return count;
}

/** `Nachname, Vorname` as the sheet prints it. */
export function personName(employee: Employee): string {
	return `${employee.last_name ?? ''}, ${employee.first_name ?? ''}`.replace(/^, /, '');
}

/** The month as a printed sheet. */
export function buildReport(sources: ReportSources): Report {
	const part = sources.part ?? 'all';
	const { period, data, employees, shifts, colors, absence } = sources;
	const anchor = toDate(period.from);

	const days: SheetDay[] = [];
	const end = toDate(period.to);
	for (let current = toDate(period.from); current <= end; current.setDate(current.getDate() + 1)) {
		const iso = toIso(current);
		days.push({
			iso,
			weekday: WEEKDAYS[current.getDay()],
			day: String(current.getDate()).padStart(2, '0'),
			sunday: current.getDay() === 0,
			holiday: data.holidays[iso] ?? null
		});
	}

	function shift(id: number | null | undefined): Shift | undefined {
		return id === null || id === undefined
			? undefined
			: shifts.find((entry) => entry.id === id);
	}

	function entryHours(entry: DutyEntry | undefined): number {
		if (entry === undefined) return 0;
		if (entry.kind === 'sick') return absence.sick;
		if (entry.kind === 'vacation') return absence.vacation;
		if (entry.kind === 'free') {
			return windowHours(entry.starts_at, entry.ends_at, entry.break_minutes);
		}
		const found = shift(entry.shift_id);
		if (found === undefined || found.band === 'free') return 0;
		return windowHours(found.starts_at, found.ends_at, found.break_minutes);
	}

	function cellText(entry: DutyEntry | undefined): string {
		if (entry === undefined) return '';
		if (entry.kind === 'sick') return 'Krank';
		if (entry.kind === 'vacation') return 'Urlaub';
		// A free time window has no shift: the sheet prints the hours themselves.
		if (entry.kind === 'free') {
			if (!entry.starts_at || !entry.ends_at) return 'frei';
			return `${entry.starts_at}–${entry.ends_at}`;
		}
		const found = shift(entry.shift_id);
		if (found === undefined) return '';
		return found.short_name ?? found.name;
	}

	function cellColor(entry: DutyEntry | undefined): string {
		if (entry === undefined) return '';
		if (entry.kind === 'sick') return colors.sick;
		if (entry.kind === 'vacation') return colors.vacation;
		if (entry.kind === 'free') return TIME_WINDOW_COLOR;
		const found = shift(entry.shift_id);
		if (found === undefined || found.band === 'free') return '';
		return found.color ?? '#64748b';
	}

	function cellTime(entry: DutyEntry | undefined): string {
		const found = shift(entry?.shift_id);
		if (found === undefined || found.band === 'free') return '';
		return `${found.starts_at}–${found.ends_at}`;
	}

	/** The tint of a row: the sub-department colour, the department's when it has none. */
	function rowColor(employee: Employee): string {
		const key = (name: string | null | undefined) => (name ?? '').trim().toLowerCase();
		const own = sources.staffColors?.subdepartment.get(key(employee.subdepartment));
		if (own !== undefined && own !== '') return own;
		return sources.staffColors?.department.get(key(employee.department)) ?? '';
	}

	const staff: SheetRow[] =
		part === 'dispo'
			? []
			: employees
					.filter((employee) => employee.id !== null)
					.map((employee) => {
						let actual = 0;
						const labels: string[] = [];
						const colorsPerDay: string[] = [];
						const times: string[] = [];
						const notes: string[] = [];
						for (const day of days) {
							const entry = data.cells[`${employee.id}|${day.iso}`];
							actual += entryHours(entry);
							labels.push(cellText(entry));
							colorsPerDay.push(cellColor(entry));
							times.push(cellTime(entry));
							notes.push(entry?.note ?? '');
						}
						const perWeek = employee.work_days ?? 5;
						const target =
							employee.weekly_hours === null
								? null
								: (Number(employee.weekly_hours) / perWeek) *
									workdays(anchor.getFullYear(), anchor.getMonth(), perWeek);
						return {
							name: personName(employee),
							department: employee.department ?? '',
							subdepartment: employee.subdepartment ?? '',
							position: employee.position ?? '',
							color: rowColor(employee),
							target,
							actual: Math.round(actual * 100) / 100,
							difference: target === null ? null : Math.round((actual - target) * 100) / 100,
							days: labels,
							colors: colorsPerDay,
							times,
							notes
						};
					});

	const dispo: DispoRow[] = [];
	if (part !== 'staff') {
		const perLocation = new Map<number, Map<string, DispoEvent[]>>();
		for (const entry of data.dispo) {
			const perDay = perLocation.get(entry.location_id) ?? new Map<string, DispoEvent[]>();
			const list = perDay.get(entry.day) ?? [];
			list.push({ time: entry.starts_at ?? '', title: entry.title });
			perDay.set(entry.day, list);
			perLocation.set(entry.location_id, perDay);
		}
		const known = data.locations.filter((location) => perLocation.has(location.id));
		const otherLocations = [...perLocation.keys()]
			.filter((id) => !data.locations.some((location) => location.id === id))
			.map((id) => ({ id, name: `Standort ${id}` }) as PeopleListEntry);
		for (const location of [...known, ...otherLocations]) {
			dispo.push({
				location: location.name,
				days: days.map((day) => perLocation.get(location.id)?.get(day.iso) ?? [])
			});
		}
	}

	const legend = shifts
		.filter((entry) => entry.active)
		.map(
			(entry) =>
				`${entry.short_name ?? entry.name} ${
					entry.band === 'free' ? 'frei' : `${entry.starts_at}–${entry.ends_at}`
				}`
		)
		.join(' · ');

	const title = `Dienstplan ${MONTHS[anchor.getMonth()]} ${anchor.getFullYear()}${
		part === 'dispo' ? ' — Dispo' : part === 'staff' ? ' — Dienste' : ''
	}`;

	return {
		title,
		period: `${toText(period.from)} – ${toText(period.to)}`,
		asOf: new Date().toLocaleDateString('de-DE'),
		days,
		dispo,
		staff,
		legend,
		paper: sources.paper,
		file: `${part === 'dispo' ? 'dispo' : part === 'staff' ? 'dienste' : 'dienstplan'}-${period.from.slice(0, 7)}`
	};
}
