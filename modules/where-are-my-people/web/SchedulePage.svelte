<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		ApiError,
		api,
		errorText,
		type DispoDayProposal,
		type DispoEntry,
		type ScheduleAudit,
		type ScheduleDemandDay,
		type ScheduleDemandEntry,
		type ScheduleEntry,
		type ScheduleKind,
		type ScheduleViolation,
		type Employee,
		type HolidayImportReport,
		type PeopleListEntry,
		type PlanningRules,
		type SchoolHoliday,
		type Shift
	} from '$core/api';
	import { TIME_WINDOW_COLOR } from './roster-report';
	import ScheduleDemandDialog from './ScheduleDemandDialog.svelte';
	import ScheduleDispoDayDialog from './ScheduleDispoDayDialog.svelte';
	import ScheduleExchangeDialog from './ScheduleExchangeDialog.svelte';
	import ScheduleFreeTimeDialog from './ScheduleFreeTimeDialog.svelte';
	import ScheduleGenerateDialog from './ScheduleGenerateDialog.svelte';
	import ScheduleHideLocationsDialog from './ScheduleHideLocationsDialog.svelte';
	import ScheduleJumpDialog from './ScheduleJumpDialog.svelte';
	import ScheduleDepartmentDialog from './ScheduleDepartmentDialog.svelte';
	import ScheduleLocationDialog from './ScheduleLocationDialog.svelte';
	import SchedulePrintDialog from './SchedulePrintDialog.svelte';
	import ScheduleSeriesDialog from './ScheduleSeriesDialog.svelte';
	import { auth } from '$core/auth.svelte';
	import { escapeWindows } from '$core/escape.svelte';
	import { RIGHT } from '$core/rights';
	import AccessDenied from '$core/AccessDenied.svelte';

	interface BoardDay {
		/** `YYYY-MM-DD`. */
		iso: string;
		/** `Mo`, `Di`, … (German weekday abbreviations). */
		weekday: string;
		/** Day of the month without a leading zero. */
		day: string;
		sunday: boolean;
	}

	const WEEKDAYS = ['So', 'Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa'];

	function toIso(date: Date): string {
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${date.getFullYear()}-${month}-${day}`;
	}

	function plusDays(date: Date, days: number): Date {
		return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
	}

	/** `count` days in a row from `iso`, the first day included. */
	function followingDays(iso: string, count: number): string[] {
		const [year, month, day] = iso.split('-').map(Number);
		const start = new Date(year, month - 1, day);
		return Array.from({ length: count }, (_, index) => toIso(plusDays(start, index)));
	}

	/** First of the month of `date`. */
	function monthStart(date: Date): Date {
		return new Date(date.getFullYear(), date.getMonth(), 1);
	}

	/** What one cell holds. */
	interface Cell {
		kind: ScheduleKind;
		shift_id: number | null;
		/** Only a free time has its own clocks; `HH:MM`. */
		starts_at?: string | null;
		ends_at?: string | null;
		break_minutes?: number | null;
		/** Free text of the cell, shown under the shift. */
		note?: string | null;
	}

	/** A time window typed into a cell by hand. */
	interface FreeTime {
		starts_at: string;
		ends_at: string;
		break_minutes: number;
	}

	/** Position value of a demand row that does not care which position covers it. */
	const OPEN_POSITION = '*';

	/** Cell: `employeeId|date` → entry of the schedule. */
	function rowKey(employeeId: number, day: string): string {
		return `${employeeId}|${day}`;
	}

	/** A schedule-plan entry with the fields the hours need. */
	interface YearEntry {
		employee_id: number;
		day: string;
		kind: ScheduleKind;
		shift_id: number | null;
		starts_at: string | null;
		ends_at: string | null;
		break_minutes: number | null;
	}

	/** Option value of a cell: `''`, `sick`, `vacation`, `free` or `shift:<id>`. */
	function selectionValue(cell: Cell | undefined): string {
		if (cell === undefined) return '';
		if (cell.kind === 'shift') return `shift:${cell.shift_id}`;
		return cell.kind;
	}

	/** Text of a planned cell: `F - 08:00–15:30`, `08:00–14:00` for a free time. */
	function cellText(cell: Cell | undefined): string {
		if (cell === undefined) return '—';
		if (cell.kind === 'sick') return t('Krank (K)');
		if (cell.kind === 'vacation') return t('Urlaub (U)');
		if (cell.kind === 'free') {
			if (!cell.starts_at || !cell.ends_at) return t('Frei');
			return `${cell.starts_at}–${cell.ends_at}`;
		}
		const shift = shifts.find((entry) => entry.id === cell.shift_id);
		if (shift === undefined) return '—';
		// A free day marks the day without times.
		if (shift.band === 'free') return shift.short_name ?? shift.name;
		return `${shift.short_name ?? shift.name} - ${shift.starts_at}–${shift.ends_at}`;
	}

	/** `Marcus Denk` — the whole name above the list of the menu. */
	function fullName(employee: Employee): string {
		return `${employee.first_name ?? ''} ${employee.last_name ?? ''}`.trim();
	}

	/** `Esc` closes the open cell menu — like every other window. */
	$effect(() => {
		if (menu === null) return;
		return escapeWindows(() => (menu = null));
	});

	/** Opens the shift menu below the clicked cell. */
	function openMenu(
		event: MouseEvent,
		employee: Employee,
		day: BoardDay,
		key: string,
		cell: Cell | undefined
	) {
		const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
		const width = Math.max(box.width, 13 * 16);
		menu = {
			key,
			employee,
			day,
			value: selectionValue(cell),
			width,
			x: Math.max(8, Math.min(box.left, window.innerWidth - width - 8)),
			y: Math.min(box.bottom + 4, window.innerHeight - 21 * 16)
		};
	}

	/** Chooses one option of the open menu. A shift and a free time first ask for the days in a row. */
	function choose(value: string) {
		if (menu === null) return;
		const { employee, day, key } = menu;
		menu = null;
		if (value.startsWith('shift:')) {
			series = {
				employee,
				day,
				value,
				label: cellText(selectionCell(value) ?? undefined),
				time: null
			};
			return;
		}
		if (value === 'free') {
			const cell = entries[key];
			free = {
				employee,
				day,
				from: cell?.starts_at ?? null,
				to: cell?.ends_at ?? null,
				breakMinutes: cell?.break_minutes ?? null
			};
			return;
		}
		void set(employee, day, value);
	}

	/** Parses an option value; `null` clears the cell. A free time needs its clocks. */
	function selectionCell(value: string): Cell | null {
		if (value === '') return null;
		if (value === 'sick' || value === 'vacation') return { kind: value, shift_id: null };
		if (value.startsWith('shift:')) {
			const id = Number(value.slice('shift:'.length));
			return Number.isFinite(id) ? { kind: 'shift', shift_id: id } : null;
		}
		return null;
	}

	/** Position name without case and surrounding spaces, to match the master data. */
	function positionKey(position: string | null | undefined): string {
		return (position ?? '').trim().toLowerCase();
	}

	let start = $state(monthStart(new Date()));
	/** The „Springe zu" window with the calendar is open. */
	let jumpOpen = $state(false);
	/** The „Dienstplan generieren" window is open. */
	let generateOpen = $state(false);
	/** The print window; it opens after the rule check of the month. */
	let printOpen = $state(false);
	/** The „Import / Export" window is open. */
	let exchangeOpen = $state(false);
	/** What the rule check found, as long as the window names it. */
	let audit = $state<ScheduleAudit | null>(null);
	let checking = $state(false);

	let employees = $state<Employee[]>([]);
	let shifts = $state<Shift[]>([]);
	let entries = $state<Record<string, Cell>>({});
	/** Schedule entries from January of the shown year — for the hours of the year. */
	let yearEntries = $state<YearEntry[]>([]);
	/** Hours a vacation or sick day counts. */
	let absence = $state<{ sick: number; vacation: number }>({ sick: 8, vacation: 8 });
	/** From this day the over-/under-hours count (`null` = 1 January). */
	let hoursFrom = $state<string | null>(null);
	/** Staffing demand per day, from the day dialog. */
	let demand = $state<ScheduleDemandDay[]>([]);
	/** Demand the disposition of the days asks for, and the unclear events. */
	let proposal = $state<DispoDayProposal[]>([]);
	/** Cells of the shown month that break a rule of the planning. */
	let violations = $state<ScheduleViolation[]>([]);
	/** Days of those cells — the board marks them with `!`. */
	const conflictDays = $derived(new Set(violations.map((violation) => violation.day)));
	/** Day whose demand dialog is open. */
	let demandDay = $state<BoardDay | null>(null);
	/** Locations of the disposition, in their maintained order. */
	let locations = $state<PeopleListEntry[]>([]);
	/** The locations the board leaves out — a view of the work, it changes nothing else. */
	let hiddenLocations = $state<Set<number>>(new Set());
	/** The „Ausblenden" window is open. */
	let hideOpen = $state(false);
	/** The locations the board shows: the ones `Ausblenden` left in. */
	const visibleLocations = $derived(
		locations.filter((location) => !hiddenLocations.has(location.id))
	);
	/** `Zeige Abteilung`: the department the board shows (administration only), empty = all. */
	let showDepartment = $state('');
	/** The „Zeige Abteilung" window is open. */
	let departmentOpen = $state(false);
	/** The departments offered by `Zeige Abteilung`. */
	let departmentList = $state<PeopleListEntry[]>([]);
	/**
	 * The shifts the legend shows: the chosen department and the shifts that belong to
	 * no department (they are open to every dispatcher).
	 */
	const legendShifts = $derived.by(() => {
		const selected = showDepartment.trim().toLowerCase();
		if (selected === '') return shifts;
		return shifts.filter((shift) => {
			const department = (shift.department ?? '').trim().toLowerCase();
			return department === '' || department === selected;
		});
	});
	/** The disposition entries of the shown period. */
	let dispoEntries = $state<DispoEntry[]>([]);
	/** Whether the rows of the disposition stay at the top while the shifts scroll. */
	let dispoSticky = $state(true);
	const DISPO_STICKY_KEY = 'where-people-dispo-sticky';
	/** Open disposition window: the location and the day it shows. */
	let dispoDay = $state<{ location: PeopleListEntry; day: BoardDay } | null>(null);
	/** Location whose settings window is open. */
	let locationSettings = $state<PeopleListEntry | null>(null);
	/** The scrolling board; a day jump moves it. */
	let grid = $state<HTMLDivElement | null>(null);
	/** Height the board takes: from its top edge to the bottom of the window. */
	let gridHeight = $state(0);
	/** Height of the legend above the board — the board takes what it leaves. */
	let legendHeight = $state(0);
	/** Open shift menu: the cell it belongs to, its value and where it opened. */
	let menu = $state<{
		key: string;
		employee: Employee;
		day: BoardDay;
		value: string;
		x: number;
		y: number;
		width: number;
	} | null>(null);
	/** Open "days in a row" window: the cell a chosen shift or free time is written onto. */
	let series = $state<{
		employee: Employee;
		day: BoardDay;
		value: string;
		label: string;
		/** The clocks of a free time; `null` for a shift. */
		time: FreeTime | null;
	} | null>(null);
	/** Open "free Eingabe" window: the cell whose own clocks are typed. */
	let free = $state<{
		employee: Employee;
		day: BoardDay;
		from: string | null;
		to: string | null;
		breakMinutes: number | null;
	} | null>(null);
	/** Positions in the order maintained under master data. */
	let positionNames = $state<string[]>([]);
	/** Rules of the schedulening (rest period, days in a row, …). */
	let rules = $state<PlanningRules | null>(null);
	/** Note about a series the rules cut short, or a finished import. */
	let notice = $state<string | null>(null);
	/** Warnings of the last PeM import, shown under its success notice. */
	/** Colors for sick, vacation, holidays and school holidays from the settings. */
	let colors = $state<{ sick: string; vacation: string; holiday: string; school: string }>({
		sick: '#B91C1C',
		vacation: '#0369A1',
		holiday: '#B45309',
		school: '#2563EB'
	});
	/** Public holidays of the shown period, `YYYY-MM-DD` → name. */
	let holidays = $state<Record<string, string>>({});
	/** School holidays of the shown period. */
	let schoolHolidays = $state<SchoolHoliday[]>([]);
	let load = $state(false);
	let error = $state<string | null>(null);
	/** Cells that are currently being written. */
	let saving = $state<string[]>([]);
	/** Position name (normalised) → place in the maintained order of the lists. */
	let positions = $state<Map<string, number>>(new Map());
	/** Department name (normalised) → the tint its employees carry in the board. */
	let departmentColors = $state<Map<string, string>>(new Map());
	/** Sub-department name (normalised) → the tint that overrides the department's. */
	let subdepartmentColors = $state<Map<string, string>>(new Map());
	/** Location name (normalised) → the tint of its disposition row. */
	let locationColors = $state<Map<string, string>>(new Map());
	/** Employee currently dragged by its name cell. */
	let dragged = $state<number | null>(null);
	/** Row the dragged employee would land in front of or behind. */
	let dropTarget = $state<{ id: number; after: boolean } | null>(null);

	/** The board belongs to `Personal-Disposition`; the API checks the same rights. */
	const mayRead = $derived(auth.isAdmin || auth.hasRight(RIGHT.mayReadDispo));
	const mayEdit = $derived(auth.isAdmin || auth.hasRight(RIGHT.mayEditDispo));

	/** Employees ordered by the position order maintained under master data. */
	function compareRows(a: Employee, b: Employee): number {
		const rankA = positions.get(positionKey(a.position)) ?? Number.MAX_SAFE_INTEGER;
		const rankB = positions.get(positionKey(b.position)) ?? Number.MAX_SAFE_INTEGER;
		if (rankA !== rankB) return rankA - rankB;
		const posA = a.position ?? '';
		const posB = b.position ?? '';
		if (rankA === Number.MAX_SAFE_INTEGER) {
			const byPosition = posA.localeCompare(posB, 'de');
			if (byPosition !== 0) return byPosition;
		}
		return a.last_name.localeCompare(b.last_name, 'de');
	}

	/** Rows the way the board shows them: arranged by hand first, the rest by position. */
	function sorted(list: Employee[]): Employee[] {
		return [...list].sort((left, right) => {
			const leftOrder = left.board_order ?? Number.MAX_SAFE_INTEGER;
			const rightOrder = right.board_order ?? Number.MAX_SAFE_INTEGER;
			if (leftOrder !== rightOrder) return leftOrder - rightOrder;
			return compareRows(left, right);
		});
	}

	/** Normalised department name — the master data compares the same way. */
	function departmentKey(name: string | null | undefined): string {
		return (name ?? '').trim().toLowerCase();
	}

	/** The tint of the department the employee belongs to, `null` when there is none. */
	function departmentColor(employee: Employee): string | null {
		if (!employee.department) return null;
		return departmentColors.get(departmentKey(employee.department)) ?? null;
	}

	/** The tint of the sub-department the employee works in, the department's otherwise,
	 * `null` when neither carries a colour. */
	function staffColor(employee: Employee): string | null {
		const own = subdepartmentColors.get(departmentKey(employee.subdepartment));
		if (own !== undefined) return own;
		return departmentColor(employee);
	}

	/** The tint of a location, `null` when there is none. */
	function locationColor(location: PeopleListEntry): string | null {
		return locationColors.get(departmentKey(location.name)) ?? null;
	}

	/** A row takes the dragged employee; the pointer decides in front of or behind. */
	function dragStart(event: DragEvent, employee: Employee) {
		if (!mayEdit || employee.id === null) return;
		dragged = employee.id;
		if (event.dataTransfer) {
			event.dataTransfer.effectAllowed = 'move';
			event.dataTransfer.setData('text/plain', String(employee.id));
		}
	}

	function dragOver(event: DragEvent, employee: Employee) {
		if (dragged === null || employee.id === null || dragged === employee.id) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
		const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
		dropTarget = { id: employee.id, after: event.clientY > box.top + box.height / 2 };
	}

	async function drop(event: DragEvent, employee: Employee) {
		event.preventDefault();
		const source = dragged;
		const target = dropTarget;
		dragged = null;
		dropTarget = null;
		if (source === null || target === null || employee.id === null || source === target.id) return;
		const list = [...employees];
		const from = list.findIndex((entry) => entry.id === source);
		if (from < 0) return;
		const [moved] = list.splice(from, 1);
		const byRoom = list.findIndex((entry) => entry.id === target.id);
		if (byRoom < 0) return;
		list.splice(target.after ? byRoom + 1 : byRoom, 0, moved);
		employees = list;
		const ids = list
			.map((entry) => entry.id)
			.filter((id): id is number => id !== null);
		try {
			await api.people.boardOrder(ids);
		} catch (problem) {
			error = errorText(problem);
			await loadBoard();
		}
	}

	function dragEnd() {
		dragged = null;
		dropTarget = null;
	}

	/** Rows of the board; `start` marks the first row of a new position block. */
	const rows = $derived.by(() => {
		let previous = '';
		return employees.map((employee, index) => {
			const group = positionKey(employee.position);
			// The first row opens the board — it carries the separator line as well.
			const start = index === 0 || group !== previous;
			previous = group;
			return { employee, start };
		});
	});

	const days = $derived.by(() => {
		const count = new Date(start.getFullYear(), start.getMonth() + 1, 0).getDate();
		const list: BoardDay[] = [];
		for (let i = 0; i < count; i += 1) {
			const date = plusDays(start, i);
			list.push({
				iso: toIso(date),
				weekday: WEEKDAYS[date.getDay()],
				day: String(date.getDate()),
				sunday: date.getDay() === 0
			});
		}
		return list;
	});

	const period = $derived(
		start.toLocaleDateString('de-DE', { month: 'long', year: 'numeric' })
	);

	/** The month the day row belongs to. */
	const daysOfMonth = $derived(new Date(start.getFullYear(), start.getMonth(), 1));

	/** The days of that month, as `1 … 28/29/30/31`. */
	const monthDays = $derived.by(() => {
		const count = new Date(daysOfMonth.getFullYear(), daysOfMonth.getMonth() + 1, 0).getDate();
		return Array.from({ length: count }, (_, index) => index + 1);
	});

	/** `true` when that day of the month is a Sunday. */
	function isSunday(day: number): boolean {
		return new Date(daysOfMonth.getFullYear(), daysOfMonth.getMonth(), day).getDay() === 0;
	}

	/** Jumps the board to the picked day — to its month. */
	function jumpTo(date: Date) {
		jumpOpen = false;
		start = monthStart(date);
		void loadBoard();
	}

	/** Width of one day column, as the table is laid out. */
	const DAY_WIDTH = 200;

	/** Scrolls the board so that the day sits directly next to the names. */
	async function jumpToDay(date: Date) {
		const iso = toIso(date);
		start = monthStart(date);
		await loadBoard();
		const index = days.findIndex((day) => day.iso === iso);
		if (index >= 0 && grid !== null) grid.scrollLeft = index * DAY_WIDTH;
	}

	/** Who works a shift or a free time on a given day — sick and vacation do not count. */
	const staffing = $derived.by(() => {
		const counter: Record<string, number> = {};
		for (const day of days) {
			counter[day.iso] = employees.filter((employee) => {
				const kind = employee.id === null ? undefined : entries[rowKey(employee.id, day.iso)]?.kind;
				return kind === 'shift' || kind === 'free';
			}).length;
		}
		return counter;
	});

	/** Staffing demand per day: `YYYY-MM-DD` → shift and needed employees. */
	const demandMap = $derived(
		new Map(demand.map((entry) => [entry.day, entry.entries]))
	);

	/** The demand the disposition asks for per day. */
	const proposalMap = $derived(
		new Map(proposal.map((entry) => [entry.day, entry]))
	);

	/** `true` when the disposition of that day could not be read into shifts. */
	function unclearOn(iso: string): boolean {
		return proposalMap.get(iso)?.marked ?? false;
	}

	/** `true` when a cell of that day breaks a rule of the planning. */
	function conflictOn(iso: string): boolean {
		return conflictDays.has(iso);
	}

	/** The broken rules of one day: who breaks which. */
	function conflictsOn(iso: string): { name: string; reason: string }[] {
		return violations
			.filter((violation) => violation.day === iso)
			.map((violation) => ({
				name: nameOf(violation.employee_id),
				reason: reasonText(violation.reason)
			}));
	}

	/** The disposition entries per `locationId|day`. */
	const dispoCard = $derived.by(() => {
		const card = new Map<string, DispoEntry[]>();
		for (const entry of dispoEntries) {
			const key = `${entry.location_id}|${entry.day}`;
			const list = card.get(key);
			if (list) list.push(entry);
			else card.set(key, [entry]);
		}
		return card;
	});

	/** The events of one location on one day. */
	function eventsOf(locationId: number, iso: string): DispoEntry[] {
		return dispoCard.get(`${locationId}|${iso}`) ?? [];
	}

	/** The text a location cell shows, one line per event. */
	function eventText(entry: DispoEntry): string {
		return entry.starts_at ? `${entry.starts_at} ${entry.title}` : entry.title;
	}

	/** How many employees the day needs in total. */
	function demandTotal(iso: string): number {
		return (demandMap.get(iso) ?? []).reduce((sizeTotal, entry) => sizeTotal + entry.needed, 0);
	}

	/** Employees already planned per `shiftId|position` on that day; `*` counts the shift. */
	function plannedOn(iso: string): Record<string, number> {
		const counter: Record<string, number> = {};
		const perShift = new Map<number, string[]>();
		for (const employee of employees) {
			if (employee.id === null) continue;
			const cell = entries[rowKey(employee.id, iso)];
			if (cell === undefined || cell.kind !== 'shift' || cell.shift_id === null) continue;
			const position = positionKey(employee.position);
			const key = `${cell.shift_id}|${position}`;
			counter[key] = (counter[key] ?? 0) + 1;
			perShift.set(cell.shift_id, [...(perShift.get(cell.shift_id) ?? []), position]);
		}
		// An open row counts everyone the specific rows of that shift do not claim.
		for (const [shiftId, positions] of perShift) {
			const specific = new Set(
				(demandMap.get(iso) ?? [])
					.filter((entry) => entry.shift_id === shiftId && !isOpenDemand(entry))
					.map((entry) => positionKey(entry.position))
			);
			counter[`${shiftId}|${OPEN_POSITION}`] = positions.filter(
				(position) => !specific.has(position)
			).length;
		}
		return counter;
	}

	/** Who would fill an open row: the employee with the fewest hours, not planned that day. */
	function proposalOn(iso: string): string | null {
		const free = employees.filter(
			(employee) => employee.id !== null && entries[rowKey(employee.id, iso)] === undefined
		);
		if (free.length === 0) return null;
		const selected = free
			.map((employee) => ({
				name: name(employee),
				actual: figures.get(employee.id as number)?.actual ?? 0
			}))
			.sort((left, right) => left.actual - right.actual || left.name.localeCompare(right.name, 'de'))[0];
		return `${selected.name} · ${hoursText(selected.actual)} h`;
	}

	/** A demand row whose position does not matter. */
	function isOpenDemand(entry: ScheduleDemandEntry): boolean {
		return entry.position.trim() === OPEN_POSITION;
	}

	/** Days between two `YYYY-MM-DD` days. */
	function daysBetween(from: string, to: string): number {
		return Math.round(
			(new Date(`${to}T00:00:00`).getTime() - new Date(`${from}T00:00:00`).getTime()) / 86400000
		);
	}

	/**
	 * The shift an employee stays in: the band of the block their last worked
	 * cell belongs to. A block starts with the first worked day of a band and
	 * holds `shift_rotation_days` days — the next cell after that window starts a
	 * new one, even in the same band. A free day and an `intermediate` shift are no
	 * band and do not end a block.
	 */
	function rotationShift(employeeId: number, iso: string): number | null {
		const days = Math.max(1, rules?.shift_rotation_days ?? 14);
		const cells: { day: string; band: string }[] = [];
		for (const [key, cell] of Object.entries(entries)) {
			const [id, day] = key.split('|');
			if (Number(id) !== employeeId) continue;
			if (day > iso) continue;
			if (cell.kind !== 'shift' || cell.shift_id === null) continue;
			const band = shifts.find((shift) => shift.id === cell.shift_id)?.band;
			if (band === undefined || band === 'free' || band === 'intermediate') continue;
			cells.push({ day, band });
		}
		if (cells.length === 0) return null;
		cells.sort((left, right) => (left.day < right.day ? -1 : 1));
		let band: string | null = null;
		let start: string | null = null;
		for (const cell of cells) {
			const inBlock = start === null ? null : daysBetween(start, cell.day);
			if (band !== cell.band || inBlock === null || inBlock >= days) {
				start = cell.day;
				band = cell.band;
			}
		}
		if (band === null || start === null) return null;
		if (daysBetween(start, iso) >= days) return null;
		const target = shifts
			.filter((shift) => shift.active && shift.band === band && !shift.apprentice_only)
			.sort((left, right) => left.sort_order - right.sort_order)[0];
		return target?.id ?? null;
	}

	/** Whether the shift does not have the position of that employee. */
	function excludedFor(shift: Shift, employee: Employee): boolean {		const position = positionKey(employee.position);
		if (position === '') return false;
		return (shift.excluded_positions ?? []).some(
			(excluded) => positionKey(excluded) === position
		);
	}

	/** Whether the shift carries the weekend marker (`WE`) in its name. */
	function isWeekendShift(shift: Shift): boolean {
		return [shift.short_name ?? '', shift.name].some((label) =>
			label.split(/[^A-Za-z0-9]+/).some((part) => part.toUpperCase() === 'WE')
		);
	}

	/**
	 * Why a shift cannot be given to that employee on that day, or `null`.
	 *
	 * Nobody but an apprentice works an apprentice shift; on Saturday and Sunday the
	 * regular late shift is not worked, only the weekend one. An apprentice may take a
	 * regular shift — the print check warns when nobody shares it.
	 */
	function blockReason(shift: Shift, employee: Employee, iso: string): string | null {
		if (excludedFor(shift, employee)) return t('Position in dieser Schicht ausgeschlossen');
		// A free day is no work: it is open to everybody, every day.
		if (shift.band === 'free') return null;
		if (shift.apprentice_only && !employee.is_apprentice) return t('nur für Auszubildende');
		const weekday = new Date(`${iso}T00:00:00`).getDay();
		if ((weekday === 0 || weekday === 6) && shift.band === 'late' && !isWeekendShift(shift)) {
			return t('am Wochenende nur S-WE');
		}
		return null;
	}

	/** The holiday name of a day, as long as the settings show them. */
	function holidayName(iso: string): string | undefined {
		if (rules?.show_holidays === false) return undefined;
		return holidays[iso];
	}

	/** The school-holiday period that covers `iso`, if any and if it is shown. */
	function schoolHolidayAt(iso: string): SchoolHoliday | null {
		if (rules?.show_school_holidays === false) return null;
		return schoolHolidays.find((period) => period.starts_on <= iso && iso <= period.ends_on) ?? null;
	}

	/** First day of a school-holiday stretch within the shown period. */
	function schoolHolidayStarts(index: number): boolean {
		const day = schoolHolidayAt(days[index].iso);
		if (day === null) return false;
		if (index === 0) return true;
		return schoolHolidayAt(days[index - 1].iso)?.name !== day.name;
	}

	/** Reads the demand again after the day dialog changed it. */
	async function loadDemand() {
		const end = days.length > 0 ? days[days.length - 1].iso : toIso(start);
		try {
			demand = await api.people.schedule.demand.range(toIso(start), end);
			proposal = await api.people.schedule.demand.dispoDemand(
				toIso(start),
				end,
				showDepartment.trim() === '' ? undefined : showDepartment.trim()
			);
		} catch (e) {
			error = errorText(e);
		}
	}

	async function loadBoard() {
		load = true;
		error = null;
		notice = null;
		menu = null;
		const end = days.length > 0 ? days[days.length - 1].iso : toIso(start);
		const onlyDepartment = showDepartment.trim() === '' ? undefined : showDepartment.trim();
		try {
			const year = start.getFullYear();
			const [
				people,
				shiftList,
				scheduleEntries,
				settings,
				indexes,
				departments,
				loadedEntries,
				demandDays,
				calendar,
				places,
				dispo,
				proposals,
				auditOfMonth,
				subdepartments
			] = await Promise.all([
				api.people.list(),
				api.people.shifts.list(),
				api.people.schedule.range(toIso(start), end),
				api.people.planning.get(),
				api.people.lists.list('positions'),
				api.people.lists.list('departments'),
				api.people.schedule.range(`${year}-01-01`, end),
				api.people.schedule.demand.range(toIso(start), end),
				api.people.holidays.range(toIso(start), end),
				api.people.schedule.dispo.locations(onlyDepartment),
				api.people.schedule.dispo.range(toIso(start), end, onlyDepartment),
				api.people.schedule.demand.dispoDemand(toIso(start), end, onlyDepartment),
				api.people.schedule.audit(toIso(start), end),
				api.people.lists.list('subdepartments')
			]);
			colors = {
				sick: settings.sick_color,
				vacation: settings.vacation_color,
				holiday: settings.holiday_color,
				school: settings.school_holiday_color
			};
			holidays = Object.fromEntries(calendar.holidays.map((day) => [day.day, day.name]));
			schoolHolidays = calendar.school_holidays;
			absence = {
				sick: Number(settings.sick_day_hours),
				vacation: Number(settings.vacation_day_hours)
			};
			hoursFrom = settings.balance_from;
			rules = settings;
			positions = new Map(indexes.map((entry, index) => [positionKey(entry.name), index]));
			positionNames = indexes.map((entry) => entry.name);
			departmentList = departments;
			departmentColors = new Map(
				departments
					.filter((entry) => (entry.color ?? '').trim() !== '')
					.map((entry) => [departmentKey(entry.name), entry.color as string])
			);
			subdepartmentColors = new Map(
				subdepartments
					.filter((entry) => (entry.color ?? '').trim() !== '')
					.map((entry) => [departmentKey(entry.name), entry.color as string])
			);
			locationColors = new Map(
				places
					.filter((entry) => (entry.color ?? '').trim() !== '')
					.map((entry) => [departmentKey(entry.name), entry.color as string])
			);
			employees = sorted(
				people.filter(
					(employee) =>
						employee.active &&
						(onlyDepartment === undefined ||
							(employee.department ?? '').trim().toLowerCase() === onlyDepartment.toLowerCase())
				)
			);
			demand = demandDays;
			proposal = proposals;
			violations = auditOfMonth.violations;
			locations = places;
			dispoEntries = dispo;
			shifts = shiftList;
			yearEntries = loadedEntries.map(
				({ employee_id, day, kind, shift_id, starts_at, ends_at, break_minutes }) => ({
					employee_id,
					day,
					kind,
					shift_id,
					starts_at,
					ends_at,
					break_minutes
				})
			);
			const card: Record<string, Cell> = {};
			for (const entry of scheduleEntries) {
				card[rowKey(entry.employee_id, entry.day)] = {
					kind: entry.kind,
					shift_id: entry.shift_id,
					starts_at: entry.starts_at,
					ends_at: entry.ends_at,
					break_minutes: entry.break_minutes,
					note: entry.note
				};
			}
			entries = card;
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	onMount(() => {
		try {
			const stored = localStorage.getItem(DISPO_STICKY_KEY);
			if (stored !== null) dispoSticky = stored !== '0';
		} catch {
			// Blocked storage keeps the board on its default for this visit.
		}
		if (mayRead) void loadBoard();
		measureGrid();
		window.addEventListener('resize', measureGrid);
		return () => window.removeEventListener('resize', measureGrid);
	});

	/** Switches the disposition between `Sticky` and `Scroll` and remembers the choice. */
	function setDispoSticky(value: boolean) {
		dispoSticky = value;
		try {
			localStorage.setItem(DISPO_STICKY_KEY, value ? '1' : '0');
		} catch {
			// See `onMount`: without storage the choice lasts for this visit only.
		}
	}

	/**
	 * The board is the only part of the window that scrolls: it reaches from its top
	 * edge down to the padding of the shell, so the heading, the dates, the legend and
	 * the disposition above it stay where they are.
	 */
	function measureGrid() {
		if (grid === null) return;
		const top = grid.getBoundingClientRect().top;
		// Below the board only the padding of the shell is left; two pixels stay free so
		// the page itself never needs a scrollbar next to the one of the shifts.
		const shell = grid.closest('.shell');
		const bottom = shell === null ? 24 : parseFloat(getComputedStyle(shell).paddingBottom) || 0;
		gridHeight = Math.max(260, Math.floor(window.innerHeight - top - bottom - 2));
	}

	// Everything above the board decides its height — the legend grows with the
	// shifts (and shrinks with them, its height is bound), the month strip with the
	// month, a message with its text.
	$effect(() => {
		void shifts.length;
		void days.length;
		void visibleLocations.length;
		void legendHeight;
		void error;
		void notice;
		void load;
		const frame = requestAnimationFrame(measureGrid);
		return () => cancelAnimationFrame(frame);
	});

	/** How long a message under the dates stays before it clears itself. */
	const NOTICE_SECONDS = 10;

	// A message under the dates is read once: it disappears after a few seconds, so the
	// board keeps its full height.
	$effect(() => {
		if (error === null && notice === null) return;
		const timer = setTimeout(() => {
			error = null;
			notice = null;
		}, NOTICE_SECONDS * 1000);
		return () => clearTimeout(timer);
	});

	/** Refreshes the conflict markers of the day row after a cell was written. */
	async function refreshAudit() {
		const end = days.length > 0 ? days[days.length - 1].iso : toIso(start);
		try {
			const result = await api.people.schedule.audit(toIso(start), end);
			violations = result.violations;
		} catch {
			// The marker is a hint; the written plan stands either way.
		}
	}

	function page(direction: -1 | 1) {
		start = new Date(start.getFullYear(), start.getMonth() + direction, 1);
		void loadBoard();
	}

	function today() {
		start = monthStart(new Date());
		void loadBoard();
	}

	/**
	 * Before the print window opens: do the rules and the law hold for the plan of
	 * the month? A broken rule is shown first — the window is the next thing only
	 * when the plan is clean or the planner wants to print anyway.
	 */
	async function checkPrint() {
		error = '';
		checking = true;
		try {
			const from = toIso(new Date(start.getFullYear(), start.getMonth(), 1));
			const to = toIso(new Date(start.getFullYear(), start.getMonth() + 1, 0));
			const result = await api.people.schedule.audit(from, to);
			if (result.violations.length > 0) {
				audit = result;
			} else {
				printOpen = true;
			}
		} catch (e) {
			error = errorText(e);
		} finally {
			checking = false;
		}
	}

	/** The name of an employee for the list of the rule check. */
	function nameOf(employeeId: number): string {
		const found = employees.find((employee) => employee.id === employeeId);
		return found === undefined ? `#${employeeId}` : name(found);
	}

	async function set(employee: Employee, day: BoardDay, value: string, time: FreeTime | null = null) {
		if (employee.id === null) return;
		const key = rowKey(employee.id, day.iso);
		const previous = entries[key];
		const cell: Cell | null =
			value === 'free'
				? time === null
					? null
					: { kind: 'free', shift_id: null, ...time }
				: selectionCell(value);
		const next = { ...entries };
		if (cell === null) delete next[key];
		else next[key] = { ...cell, note: previous?.note ?? null };
		entries = next;
		saving = [key];
		error = null;
		try {
			await api.people.schedule.set({
				employee_id: employee.id,
				day: day.iso,
				kind: cell?.kind ?? null,
				shift_id: cell?.shift_id ?? null,
				starts_at: cell?.starts_at ?? null,
				ends_at: cell?.ends_at ?? null,
				break_minutes: cell?.break_minutes ?? null,
				// The note belongs to the cell and stays when the shift changes.
				note: previous?.note ?? null
			});
			const remaining = yearEntries.filter(
				(entry) => !(entry.employee_id === employee.id && entry.day === day.iso)
			);
			if (cell === null) {
				yearEntries = remaining;
			} else {
				yearEntries = [
					...remaining,
					{
						employee_id: employee.id,
						day: day.iso,
						kind: cell.kind,
						shift_id: cell.shift_id,
						starts_at: cell.starts_at ?? null,
						ends_at: cell.ends_at ?? null,
						break_minutes: cell.break_minutes ?? null
					}
				];
			}
			void refreshAudit();
		} catch (e) {
			// A single cell is refused when the shift does not take the employee, when
			// the position is excluded or when the day is a holiday that may not be
			// worked. A lone apprentice is written and warned about at the print check.
			const label = errorText(e);
			error =
				e instanceof ApiError && e.status === 409
					? label.includes('apprentice')
						? t('Auszubildende/r darf nicht allein in einer Schicht sein.')
						: label.includes('holiday')
							? t('Feiertagsarbeit ist nicht erlaubt.')
							: label
					: label;
			const back = { ...entries };
			if (previous === undefined) delete back[key];
			else back[key] = previous;
			entries = back;
		} finally {
			saving = [];
		}
	}

	/** Writes the free text under the shift; the cell itself stays as it is. */
	async function setNote(employee: Employee, day: BoardDay, value: string) {
		if (employee.id === null) return;
		const key = rowKey(employee.id, day.iso);
		const previous = entries[key];
		if (previous === undefined) return;
		const label = value.trim();
		const note = label === '' ? null : label;
		if ((previous.note ?? null) === note) return;
		entries = { ...entries, [key]: { ...previous, note } };
		saving = [...saving, key];
		error = null;
		try {
			await api.people.schedule.set({
				employee_id: employee.id,
				day: day.iso,
				kind: previous.kind,
				shift_id: previous.shift_id ?? null,
				starts_at: previous.starts_at ?? null,
				ends_at: previous.ends_at ?? null,
				break_minutes: previous.break_minutes ?? null,
				note
			});
		} catch (e) {
			error = refusalText(errorText(e));
			entries = { ...entries, [key]: previous };
		} finally {
			saving = saving.filter((entry) => entry !== key);
		}
	}

	/**
	 * Wording of the refusals a single cell answers with. The API names them in one
	 * sentence of its own; the board shows them in the language of the interface.
	 */
	function refusalText(label: string): string {
		if (label.includes('Only apprentices may be planned into this shift')) {
			return t('Diese Schicht ist nur für Auszubildende');
		}
		if (label.startsWith('The position ')) {
			return t('Die Position ist in dieser Schicht ausgeschlossen');
		}
		if (label.includes('Work on a public holiday')) {
			return t('Feiertagsarbeit ist nicht erlaubt');
		}
		if (label.includes('The late shift is not worked on Saturday or Sunday')) {
			return t('Die Spätschicht wird am Wochenende nicht gearbeitet');
		}
		return label;
	}

	/** Text for the rule that stopped a series. */
	function reasonText(reason: string): string {
		switch (reason) {
			case 'sunday':
				return t('Sonntagsarbeit ist nicht erlaubt');
			case 'absence':
				return t('der Tag ist als Krank oder Urlaub geplant');
			case 'holiday':
				return t('Feiertagsarbeit ist nicht erlaubt');
			case 'apprentice_alone':
				return t('Auszubildende/r darf nicht allein in einer Schicht sein');
			case 'max_consecutive_days':
				return t('höchstens {n} Tage in Folge', { n: rules?.max_consecutive_days ?? 6 });
			case 'min_rest_hours':
				return t('weniger als {n} h Ruhezeit zum Nachbartag', {
					n: rules?.min_rest_hours ?? 11
				});
			case 'compensation_rest':
				return t('weniger als 24 + {n} h um den Ersatzruhetag nach der Sonntagsarbeit', {
					n: rules?.min_rest_hours ?? 11
				});
			case 'max_daily_hours':
				return t('die Schicht ist länger als {n} h', { n: rules?.max_daily_hours ?? 10 });
			case 'max_weekly_hours':
				return t('die Wochenarbeitszeit von {n} h ist erreicht', {
					n: rules?.max_weekly_hours ?? 48
				});
			case 'min_days_off_per_week':
				return t('mindestens {n} freie Tage pro Woche', {
					n: rules?.min_days_off_per_week ?? 1
				});
			case 'excluded_position':
				return t('die Position ist in dieser Schicht ausgeschlossen');
			case 'weekend':
				return t('am Wochenende ist nur die Wochenend-Spätschicht erlaubt');
			case 'apprentice_only':
				return t('die Schicht ist nur für Auszubildende');
			default:
				return t('die Einstellungen lassen den Tag nicht zu');
		}
	}

	/** `Mo., 16.11.2026`. */
	function dayLong(iso: string): string {
		const [year, month, day] = iso.split('-').map(Number);
		return new Date(year, month - 1, day).toLocaleDateString('de-DE', {
			weekday: 'short',
			day: '2-digit',
			month: '2-digit',
			year: 'numeric'
		});
	}

	/** Writes the chosen cell onto `count` days in a row, the first day included. */
	async function setSeries(
		employee: Employee,
		day: BoardDay,
		value: string,
		count: number,
		time: FreeTime | null = null
	) {
		const id = employee.id;
		if (id === null) return;
		if (count <= 1) {
			await set(employee, day, value, time);
			return;
		}
		const cell: Cell | null =
			value === 'free'
				? time === null
					? null
					: { kind: 'free', shift_id: null, ...time }
				: selectionCell(value);
		if (cell === null) return;
		const days = followingDays(day.iso, count);
		const previousEntries = entries;
		const previousYear = yearEntries;
		const next = { ...entries };
		for (const iso of days) next[rowKey(id, iso)] = cell;
		entries = next;
		saving = days.map((iso) => rowKey(id, iso));
		error = null;
		notice = null;
		try {
			const result = await api.people.schedule.series({
				employee_id: id,
				day: day.iso,
				days: count,
				kind: cell.kind,
				shift_id: cell.shift_id,
				starts_at: cell.starts_at ?? null,
				ends_at: cell.ends_at ?? null,
				break_minutes: cell.break_minutes ?? null
			});
			const written = result.entries;
			const affected = new Set(written.map((entry) => entry.day));
			// Days the rules kept free go back to what they were.
			const back = { ...previousEntries };
			for (const entry of written) {
				back[rowKey(id, entry.day)] = {
					kind: entry.kind,
					shift_id: entry.shift_id,
					starts_at: entry.starts_at,
					ends_at: entry.ends_at,
					break_minutes: entry.break_minutes
				};
			}
			entries = back;
			const remaining = previousYear.filter(
				(entry) => !(entry.employee_id === id && affected.has(entry.day))
			);
			yearEntries = [
				...remaining,
				...written.map((entry) => ({
					employee_id: entry.employee_id,
					day: entry.day,
					kind: entry.kind,
					shift_id: entry.shift_id,
					starts_at: entry.starts_at,
					ends_at: entry.ends_at,
					break_minutes: entry.break_minutes
				}))
			];
			if (result.blocked) {
				const target = written.length;
				notice =
					target === 0
						? t('Nichts eingetragen — {day}: {reason}.', {
								day: dayLong(result.blocked.day),
								reason: reasonText(result.blocked.reason)
							})
						: t('{written} von {requested} Tagen eingetragen — {day}: {reason}.', {
								written: target,
								requested: result.requested,
								day: dayLong(result.blocked.day),
								reason: reasonText(result.blocked.reason)
							});
			}
			void refreshAudit();
		} catch (e) {
			error = refusalText(errorText(e));
			entries = previousEntries;
			yearEntries = previousYear;
		} finally {
			saving = [];
		}
	}

	/** `Dr. A. Lovelace` as in the employee list. */
	function name(employee: Employee): string {
		const title = employee.title ? `${employee.title} ` : '';
		const initial = employee.first_name.trim().charAt(0);
		const firstName = initial ? `${initial.toUpperCase()}. ` : '';
		return `${title}${firstName}${employee.last_name}`;
	}

	const color = (shiftId: number) => shifts.find((shift) => shift.id === shiftId)?.color ?? null;

	/** Adds a row the board types itself: a disposition row with a free-text name. */
	async function addRow() {
		error = null;
		try {
			await api.people.schedule.rows.create();
			await loadBoard();
		} catch (e) {
			error = errorText(e);
		}
	}

	/** Renames a typed row; the board keeps the name the server accepted. */
	async function renameRow(location: PeopleListEntry, value: string) {
		const name = value.trim();
		if (name === '' || name === location.name) return;
		error = null;
		const previous = locations;
		locations = locations.map((entry) =>
			entry.id === location.id ? { ...entry, name } : entry
		);
		try {
			await api.people.schedule.rows.rename(location.id, name);
		} catch (e) {
			error = errorText(e);
			locations = previous;
		}
	}

	/** Removes a typed row with its events; a house of the master data cannot go. */
	async function removeRow(location: PeopleListEntry) {
		error = null;
		const previous = locations;
		locations = locations.filter((entry) => entry.id !== location.id);
		try {
			await api.people.schedule.rows.remove(location.id);
			await loadBoard();
		} catch (e) {
			error = errorText(e);
			locations = previous;
		}
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

	/** Times of a shift as the legend shows them; a free day has none. */
	function shiftTime(shift: Shift): string {
		if (shift.band === 'free') return t('ganzer Tag');
		return `${shift.starts_at}–${shift.ends_at}`;
	}

	/** A free day of the catalogue is a marker, not work. */
	function isFreeDay(shiftId: number | null): boolean {
		const shift = shiftId === null ? undefined : shifts.find((entry) => entry.id === shiftId);
		return shift?.band === 'free';
	}

	/** Hours a shift lasts, the break already taken off. A free day counts none. */
	function shiftHours(shiftId: number | null): number {
		const shift = shiftId === null ? undefined : shifts.find((entry) => entry.id === shiftId);
		if (shift === undefined) return 0;
		if (shift.band === 'free') return 0;
		return windowHours(shift.starts_at, shift.ends_at, shift.break_minutes);
	}

	/** Hours one schedule entry counts. */
	function entryHours(entry: YearEntry): number {
		if (entry.kind === 'shift') return shiftHours(entry.shift_id);
		if (entry.kind === 'sick') return absence.sick;
		if (entry.kind === 'free') {
			return windowHours(entry.starts_at, entry.ends_at, entry.break_minutes);
		}
		return absence.vacation;
	}

	/** Working days of a month for a contract with `perWeek` days a week. */
	function workingDays(year: number, month: number, perWeek: number): number {
		const daysInMonth = new Date(year, month + 1, 0).getDate();
		let count = 0;
		for (let day = 1; day <= daysInMonth; day += 1) {
			const weekday = new Date(year, month, day).getDay();
			// Sunday only with a seven-day week, Saturday from six days on.
			if (weekday === 0 && perWeek < 7) continue;
			if (weekday === 6 && perWeek < 6) continue;
			count += 1;
		}
		return count;
	}

	/** `7,50` — hours always with two decimals; `—` when the value is unknown. */
	function hoursText(value: number | null | undefined, withSign = false): string {
		if (value === null || value === undefined) return '—';
		const label = Math.abs(value).toLocaleString('de-DE', {
			minimumFractionDigits: 2,
			maximumFractionDigits: 2
		});
		if (!withSign) return value < 0 ? `−${label}` : label;
		if (value > 0) return `+${label}`;
		if (value < 0) return `−${label}`;
		return label;
	}

	/**
	 * `Soll` and `Ist` of the shown month per employee plus the balance of the year up
	 * to that month. `Soll` comes from the contract (weekly hours over the working
	 * days of the month), `Ist` from the schedule — shifts with their break, vacation
	 * and sick days with the hours set in the settings.
	 */
	const figures = $derived.by(() => {
		const year = start.getFullYear();
		const month = start.getMonth();
		const actualPerMonth = new Map<string, number>();
		for (const entry of yearEntries) {
			const key = `${entry.employee_id}|${Number(entry.day.slice(5, 7)) - 1}`;
			actualPerMonth.set(key, (actualPerMonth.get(key) ?? 0) + entryHours(entry));
		}
		// The balance starts at the date of the settings — before that nothing counts.
		let startMonth = 0;
		if (hoursFrom !== null) {
			const [fromYear, fromMonth] = hoursFrom.split('-').map(Number);
			startMonth = fromYear < year ? 0 : fromYear > year ? 12 : fromMonth - 1;
		}
		const card = new Map<number, { target: number | null; actual: number; balance: number | null }>();
		for (const employee of employees) {
			if (employee.id === null) continue;
			const perWeek = employee.work_days ?? 5;
			const withoutContract = employee.weekly_hours === null;
			const hoursPerDay = withoutContract ? 0 : Number(employee.weekly_hours) / perWeek;
			let balance = 0;
			for (let current = startMonth; current <= month; current += 1) {
				const targetMonth = hoursPerDay * workingDays(year, current, perWeek);
				const actualMonth = actualPerMonth.get(`${employee.id}|${current}`) ?? 0;
				balance += actualMonth - targetMonth;
			}
			// `Soll` and `Ist` always describe the shown month, even when the balance
			// starts later and shows nothing yet.
			const target = hoursPerDay * workingDays(year, month, perWeek);
			const actual = actualPerMonth.get(`${employee.id}|${month}`) ?? 0;
			card.set(employee.id, {
				target: withoutContract ? null : target,
				actual,
				balance: withoutContract ? null : balance
			});
		}
		return card;
	});

	/** Cell color: shift color, otherwise the color of the absence or the free time. */
	function cellColor(cell: Cell | undefined): string | null {
		if (cell === undefined) return null;
		if (cell.kind === 'sick') return colors.sick;
		if (cell.kind === 'vacation') return colors.vacation;
		if (cell.kind === 'free') return TIME_WINDOW_COLOR;
		return cell.shift_id === null ? null : color(cell.shift_id);
	}
</script>

<svelte:head><title>{t('Dienstplan — WHERE')}</title></svelte:head>

{#if !mayRead}
	<AccessDenied
		pageName={t('Dienstplan')}
		right={RIGHT.mayReadDispo}
		rightLabel={t('Personaldispo lesen')}
	/>
{:else}
	<div class="wide">
		<h1>{t('Dienstplan')}</h1>

		<div class="tools">
			<div class="page">
				<button type="button" title={t('zurück')} aria-label={t('einen Zeitraum zurück')} onclick={() => page(-1)}>
					◀
				</button>
				<button type="button" onclick={today}>{t('Heute')}</button>
				<button type="button" title={t('weiter')} aria-label={t('einen Zeitraum weiter')} onclick={() => page(1)}>
					▶
				</button>
			</div>
			<span class="period">{period}</span>
			<button type="button" class="jump" onclick={() => (jumpOpen = true)}>{t('Springe zu')}</button>
			{#if auth.isAdmin}
				<button type="button" class="jump" onclick={() => (departmentOpen = true)}>
					{t('Zeige Abteilung')}{showDepartment ? ` · ${showDepartment}` : ''}
				</button>
			{/if}
			{#if mayEdit}
				<button type="button" class="generate" onclick={() => (generateOpen = true)}>
					{t('Dienstplan generieren')}
				</button>
			{/if}
			<button type="button" class="jump" onclick={() => (exchangeOpen = true)}>
				{t('Import / Export')}
			</button>
			<button type="button" class="print" onclick={() => void checkPrint()} disabled={checking}>
				{checking ? t('Prüfe …') : t('Drucken')}
			</button>
			{#if load}<span class="loads">{t('Lade …')}</span>{/if}
		</div>

		<nav class="month-days" aria-label={t('Tag des Monats')}>
			{#each monthDays as day (day)}
				<button
					type="button"
					class:sunday={isSunday(day)}
					onclick={() => void jumpToDay(new Date(daysOfMonth.getFullYear(), daysOfMonth.getMonth(), day))}
					>{day}</button
				>
			{/each}
		</nav>

		{#if error}<p class="bad">{error}</p>{/if}
		{#if notice}
			<div class="seriesNotice">
				<p>{notice}</p>
			</div>
		{/if}

		{#if shifts.length === 0}
			<p class="empty">
				{t('Noch keine Schicht angelegt — lege sie unter {where} an.', {
					where: t('Stammdaten → Personal → Dienstplan')
				})}
			</p>
		{:else}
			<div class="legend" bind:clientHeight={legendHeight}>
				<div class="shifts">
					{#each legendShifts as shift (shift.id)}
						<div class="entry" class:inactive={!shift.active}>
							<span class="marker" style={`--marker:${shift.color ?? '#94a3b8'}`}></span>
							<span class="text">
								<span class="name">
									{#if shift.short_name}<span class="shortName">{shift.short_name}</span>{/if}
									{shift.name}
								</span>
								<span class="time">
									{shiftTime(shift)} · {hoursText(shiftHours(shift.id))} {t('h')}{shift.active
										? ''
										: ` · ${t('inaktiv')}`}
								</span>
							</span>
						</div>
					{/each}
				</div>
				<div class="absences">
					<div class="entry">
						<span class="marker" style={`--marker:${colors.sick}`}></span>
						<span class="text">
							<span class="name"><span class="shortName">{t('K')}</span>{t('Krank')}</span>
						</span>
					</div>
					<div class="entry">
						<span class="marker" style={`--marker:${colors.vacation}`}></span>
						<span class="text">
							<span class="name"><span class="shortName">{t('U')}</span>{t('Urlaub')}</span>
						</span>
					</div>
					<div class="entry">
						<span class="marker" style={`--marker:${TIME_WINDOW_COLOR}`}></span>
						<span class="text">
							<span class="name">{t('freie Zeit')}</span>
						</span>
					</div>
					{#if rules?.show_holidays !== false}
						<div class="entry">
							<span class="marker" style={`--marker:${colors.holiday}`}></span>
							<span class="text">
								<span class="name">{t('Feiertag')}</span>
							</span>
						</div>
					{/if}
					{#if rules?.show_school_holidays !== false}
						<div class="entry">
							<span class="marker" style={`--marker:${colors.school}`}></span>
							<span class="text">
								<span class="name">{t('Ferien')}</span>
							</span>
						</div>
					{/if}
				</div>
			</div>

			<div
				class="grid"
				bind:this={grid}
				style={gridHeight > 0 ? `height:${gridHeight}px` : ''}
			>
				{#snippet dispoRows()}
					{#each visibleLocations as location (location.id)}
						{@const color = locationColor(location)}
						<tr class="dispo" class:locationColor={color !== null} style={color ? `--location:${color}` : ''}>
							<td class="employees">
								<span class="rowHead">
									{#if location.is_text}
										{#if mayEdit}
											<input
												class="rowText"
												type="text"
												maxlength="100"
												placeholder={t('Bezeichnung')}
												title={t('Bezeichnung der Zeile')}
												aria-label={t('Bezeichnung der Zeile')}
												value={location.name}
												onchange={(event) =>
													renameRow(location, event.currentTarget.value)}
											/>
										{:else}
											<span class="rowText">{location.name}</span>
										{/if}
										<button
											type="button"
											class="rowSettings"
											title={t('Einstellungen der Zeile')}
											aria-label={t('Einstellungen der Zeile')}
											onclick={() => (locationSettings = location)}>⚙</button
										>
										{#if mayEdit}
											<button
												type="button"
												class="rowRemove"
												title={t('Zeile entfernen')}
												aria-label={t('Zeile entfernen')}
												onclick={() => void removeRow(location)}>−</button
											>
										{/if}
									{:else}
										<button
											type="button"
											class="locationName"
											title={t('Einstellungen des Standorts')}
											onclick={() => (locationSettings = location)}
											>{location.name}</button
										>
									{/if}
								</span>
							</td>
							{#each days as day (day.iso)}
								{@const events = eventsOf(location.id, day.iso)}
								<td
									class:sunday={day.sunday}
									class:holiday={holidayName(day.iso) !== undefined}
									class:set={events.length > 0}
								>
									{#if events.length > 0}
										<button
											type="button"
											class="dispoCell"
											title={events.map(eventText).join('\n')}
											aria-label={t('{location} am {day}: {events}', {
												location: location.name,
												day: `${t(day.weekday)} ${day.day}.`,
												events: events.map(eventText).join(', ')
											})}
											onclick={() => (dispoDay = { location, day })}
										>
											{#each events as entry (entry.id)}
												<span class="event">
													{#if entry.starts_at}<span class="time"
															>{entry.starts_at}</span
														>{/if}{entry.title}
												</span>
											{/each}
										</button>
									{:else if mayEdit}
										<button
											type="button"
											class="dispoCell empty"
											title={t('Termin eintragen')}
											aria-label={t('{location} am {day}: Termin eintragen', {
												location: location.name,
												day: `${t(day.weekday)} ${day.day}.`
											})}
											onclick={() => (dispoDay = { location, day })}>+</button
										>
									{:else}
										<span class="event">—</span>
									{/if}
								</td>
							{/each}
						</tr>
					{/each}
				{/snippet}

				<table
					style={`width: 100%; min-width: calc(250px + ${days.length * 140}px); --holiday: ${colors.holiday}; --schoolHolidays: ${colors.school};`}
				>
					<thead>
						<tr>
							<th class="employees">
								<span class="headGrid">
									<span class="dispoMode" role="group" aria-label={t('Dispo der Standorte')}>
										<button
											type="button"
											class:active={dispoSticky}
											aria-pressed={dispoSticky}
											title={t('Dispo beim Scrollen top festhalten')}
											onclick={() => setDispoSticky(true)}>{t('Sticky')}</button
										>
										<button
											type="button"
											class:active={!dispoSticky}
											aria-pressed={!dispoSticky}
											title={t('Dispo mit den Schichten scrollen lassen')}
											onclick={() => setDispoSticky(false)}>{t('Scroll')}</button
										>
									</span>
									{#if mayEdit}
										<button
											type="button"
											class="rowNew"
											title={t('Eigene Zeile hinzufügen')}
											aria-label={t('Eigene Zeile hinzufügen')}
											onclick={addRow}>+</button
										>
									{/if}
									<span class="dispoMode" role="group" aria-label={t('Standorte der Ansicht')}>
										<button
											type="button"
											class:active={hiddenLocations.size > 0}
											title={t('Standorte in der Ansicht ausblenden')}
											onclick={() => (hideOpen = true)}
											>{t('Ausblenden')}{hiddenLocations.size > 0 ? ` · ${hiddenLocations.size}` : ''}</button
										>
									</span>
								</span>
							</th>
							{#each days as day, index (day.iso)}
								{@const needed = demandTotal(day.iso)}
								{@const holiday = holidayName(day.iso)}
								{@const schoolHolidayPeriod = schoolHolidayAt(day.iso)}
								<th
									class:sunday={day.sunday}
									class:holiday={holiday !== undefined}
									class:schoolHolidays={schoolHolidayPeriod !== null}
									title={[holiday, schoolHolidayPeriod?.name].filter(Boolean).join(' · ')}
								>
									<button
										type="button"
										class="dateButton"
										title={t('Personalbedarf für {day} bearbeiten', {
											day: `${t(day.weekday)} ${day.day}.`
										})}
										aria-label={t('Personalbedarf für {day} bearbeiten', {
											day: `${t(day.weekday)} ${day.day}.`
										})}
										onclick={() => (demandDay = day)}
									>
										<span class="weekday">{t(day.weekday)}</span>
										<span class="date">{day.day}.</span>
										{#if holiday !== undefined}
											<span class="holidayName">{holiday}</span>
										{/if}
										{#if schoolHolidayPeriod !== null && schoolHolidayStarts(index)}
											<span class="schoolHolidayName">{schoolHolidayPeriod.name}</span>
										{/if}
										{#if needed > 0}
											<span
												class="demand"
												class:missing={(staffing[day.iso] ?? 0) < needed}
												title={needed === 1
													? t('1 Mitarbeiter benötigt')
													: t('{n} Mitarbeiter benötigt', { n: needed })}
											>
												{needed}
											</span>
										{/if}
										{#if unclearOn(day.iso)}
											<span class="demand unclear" title={t('Dispo nicht eindeutig — Datum öffnen')}
												>?</span
											>
										{/if}
										{#if conflictOn(day.iso)}
											<span class="conflict" title={t('Regelverstoß — Datum öffnen')}>!</span>
										{/if}
									</button>
								</th>
							{/each}
						</tr>
						{#if dispoSticky}{@render dispoRows()}{/if}
					</thead>
					<tbody>
						{#if !dispoSticky}{@render dispoRows()}{/if}
						{#each rows as row (row.employee.id)}
							{@const employee = row.employee}
							{@const amount = employee.id === null ? undefined : figures.get(employee.id)}
							{@const tone = staffColor(employee)}
							<tr
								class="person"
								class:groupStart={row.start}
								class:dragged={dragged !== null && dragged === employee.id}
								class:drop={dropTarget?.id === employee.id && dropTarget.after === false}
								class:dropBelow={dropTarget?.id === employee.id && dropTarget.after === true}
							>
								<td
									class="employees"
									class:drag={mayEdit && employee.id !== null}
									style={tone ? `--employeeColor:${tone}` : ''}
									draggable={mayEdit && employee.id !== null}
									ondragstart={(event) => dragStart(event, employee)}
									ondragover={(event) => dragOver(event, employee)}
									ondrop={(event) => void drop(event, employee)}
									ondragend={dragEnd}
								>
									<span class="figures">
										<span class="head empty"></span>
										<span class="head" title={t('Sollzeit des Monats')}>{t('Soll')}</span>
										<span class="head" title={t('Ist-Zeit laut Dienstplan')}>{t('Ist')}</span>
										<span class="head" title={t('Über- oder Minusstunden seit Beginn des Stundenkontos')}>+/−</span>

										<span class="who" title={name(employee)}>{name(employee)}</span>
										<span class="amount">{hoursText(amount?.target)}</span>
										<span class="amount">{hoursText(amount?.actual)}</span>
										<span
											class="amount balance"
											class:plus={(amount?.balance ?? 0) > 0.004}
											class:minus={(amount?.balance ?? 0) < -0.004}
											>{hoursText(amount?.balance, true)}</span
										>

										<span class="position" title={employee.position ?? ''}>{employee.position ?? ''}</span>
									</span>
								</td>
								{#each days as day (day.iso)}
									{@const key = employee.id === null ? '' : rowKey(employee.id, day.iso)}
									{@const cell = entries[key]}
									{@const tone = cellColor(cell)}
									{@const free = cell?.kind === 'shift' && isFreeDay(cell.shift_id ?? null)}
									<td
										class:sunday={day.sunday}
										class:holiday={holidayName(day.iso) !== undefined}
										class:set={cell !== undefined}
									>
										{#if mayEdit}
											<button
												type="button"
												class="cell"
												class:open={menu?.key === key}
												class:free-day={free}
												style={tone ? `--cellColor:${tone}` : ''}
												title={t('{name} am {day}', {
													name: name(employee),
													day: `${t(day.weekday)} ${day.day}.`
												})}
												aria-haspopup="listbox"
												aria-expanded={menu?.key === key}
												aria-label={t('{name} am {day}: {cell}', {
													name: name(employee),
													day: `${t(day.weekday)} ${day.day}.`,
													cell: cellText(cell)
												})}
												disabled={saving.includes(key)}
												onclick={(event) => openMenu(event, employee, day, key, cell)}
											>
												{cellText(cell)}
											</button>
										{:else}
											<span
												class="cell still"
												class:free-day={free}
												style={tone ? `--cellColor:${tone}` : ''}
											>
												{cellText(cell)}
											</span>
										{/if}
										{#if cell !== undefined}
											{#if mayEdit}
												<input
													class="cellNote"
													type="text"
													maxlength="200"
													title={cell.note ?? ''}
													aria-label={t('Notiz zu {name} am {day}', {
														name: name(employee),
														day: `${t(day.weekday)} ${day.day}.`
													})}
													value={cell.note ?? ''}
													disabled={saving.includes(key)}
													onchange={(event) =>
														void setNote(employee, day, event.currentTarget.value)}
												/>
											{:else if cell.note}
												<span class="cellNote still" title={cell.note}>{cell.note}</span>
											{/if}
										{/if}
									</td>
								{/each}
							</tr>
						{:else}
							<tr>
								<td colspan={days.length + 1}>
									{load ? t('Lade …') : t('Noch keine aktiven Mitarbeiter erfasst.')}
								</td>
							</tr>
						{/each}
					</tbody>
					<tfoot>
						<tr>
							<td class="employees">{t('Eingeteilt')}</td>
							{#each days as day (day.iso)}
								{@const needed = demandTotal(day.iso)}
								<td
									class:sunday={day.sunday}
									class:holiday={holidayName(day.iso) !== undefined}
									class:missing={needed > 0 && (staffing[day.iso] ?? 0) < needed}
								>
									{staffing[day.iso] ?? 0}{#if needed > 0}<span class="of">/{needed}</span>{/if}
								</td>
							{/each}
						</tr>
					</tfoot>
				</table>
			</div>
		{/if}
	</div>

	{#if menu}
		<div class="menu-backdrop" role="presentation" onclick={() => (menu = null)}></div>
		<div
			class="menu"
			role="listbox"
			aria-label={t('Schicht wählen')}
			style={`left:${menu.x}px; top:${menu.y}px; min-width:${menu.width}px`}
		>
			<div class="menu-head">
				<span class="day">{dayLong(menu.day.iso)}</span>
				<span class="who">{fullName(menu.employee)}</span>
			</div>
			<button
				type="button"
				class="menu-entry"
				class:selected={menu.value === ''}
				role="option"
				aria-selected={menu.value === ''}
				onclick={() => choose('')}
			>
				<span class="marker empty" aria-hidden="true"></span>
				<span class="text"><span class="name">{t('nicht eingeteilt')}</span></span>
			</button>
			{#each shifts as shift (shift.id)}
				{@const selectedShift = menu.value === `shift:${shift.id}`}
				{@const reason = blockReason(shift, menu.employee, menu.day.iso)}
				{@const stays =
					menu.employee.id !== null && rotationShift(menu.employee.id, menu.day.iso) === shift.id}
				<button
					type="button"
					class="menu-entry"
					class:selected={selectedShift}
					class:inactive={!shift.active || reason !== null}
					role="option"
					aria-selected={selectedShift}
					disabled={reason !== null}
					title={reason ?? undefined}
					onclick={() => choose(`shift:${shift.id}`)}
				>
					<span class="marker" style={`--marker:${shift.color ?? '#94a3b8'}`} aria-hidden="true"></span>
					<span class="text">
						<span class="name">
							{shift.short_name ? `${shift.short_name} · ${shift.name}` : shift.name}{shift.active
								? ''
								: ` · ${t('inaktiv')}`}{reason !== null ? ` · ${reason}` : ''}{stays
								? ` · ${t('bleibt ({n} Tage)', { n: rules?.shift_rotation_days ?? 14 })}`
								: ''}
						</span>
						<span class="time">
							{shift.starts_at}–{shift.ends_at} · {hoursText(shiftHours(shift.id))} {t('h')}
						</span>
					</span>
				</button>
			{/each}
			<button
				type="button"
				class="menu-entry"
				class:selected={menu.value === 'free'}
				role="option"
				aria-selected={menu.value === 'free'}
				onclick={() => choose('free')}
			>
				<span class="marker" style={`--marker:${TIME_WINDOW_COLOR}`} aria-hidden="true"></span>
				<span class="text">
					<span class="name">{t('Freie Eingabe …')}</span>
					<span class="time">{t('Beginn und Ende selbst eintragen')}</span>
				</span>
			</button>
			<button
				type="button"
				class="menu-entry"
				class:selected={menu.value === 'sick'}
				role="option"
				aria-selected={menu.value === 'sick'}
				onclick={() => choose('sick')}
			>
				<span class="marker" style={`--marker:${colors.sick}`} aria-hidden="true"></span>
				<span class="text"><span class="name">{t('Krank (K)')}</span></span>
			</button>
			<button
				type="button"
				class="menu-entry"
				class:selected={menu.value === 'vacation'}
				role="option"
				aria-selected={menu.value === 'vacation'}
				onclick={() => choose('vacation')}
			>
				<span class="marker" style={`--marker:${colors.vacation}`} aria-hidden="true"></span>
				<span class="text"><span class="name">{t('Urlaub (U)')}</span></span>
			</button>
		</div>
	{/if}

	{#if jumpOpen}
		<ScheduleJumpDialog
			month={start}
			period={days.map((day) => day.iso)}
			onpick={jumpTo}
			onclose={() => (jumpOpen = false)}
		/>
	{/if}

	{#if departmentOpen}
		<ScheduleDepartmentDialog
			selected={showDepartment}
			departments={departmentList}
			onpick={(name) => {
				showDepartment = name;
				departmentOpen = false;
				void loadBoard();
			}}
			onclose={() => (departmentOpen = false)}
		/>
	{/if}

	{#if hideOpen}
		<ScheduleHideLocationsDialog
			locations={locations}
			hidden={hiddenLocations}
			ontoggle={(id, hidden) => {
				const next = new Set(hiddenLocations);
				if (hidden) next.add(id);
				else next.delete(id);
				hiddenLocations = next;
			}}
			onclose={() => (hideOpen = false)}
		/>
	{/if}

	{#if generateOpen}
		<ScheduleGenerateDialog
			from={days.length > 0 ? days[0].iso : toIso(start)}
			to={days.length > 0 ? days[days.length - 1].iso : toIso(start)}
			ongenerated={() => void loadBoard()}
			onclose={() => (generateOpen = false)}
		/>
	{/if}

	{#if printOpen}
		<SchedulePrintDialog
			from={days.length > 0 ? days[0].iso : toIso(start)}
			to={days.length > 0 ? days[days.length - 1].iso : toIso(start)}
			{employees}
			{shifts}
			colors={colors}
			absence={absence}
			departmentColors={departmentColors}
			subdepartmentColors={subdepartmentColors}
			onclose={() => (printOpen = false)}
		/>
	{/if}

	{#if exchangeOpen}
		<ScheduleExchangeDialog
			from={days.length > 0 ? days[0].iso : toIso(start)}
			{employees}
			{shifts}
			colors={colors}
			absence={absence}
			departmentColors={departmentColors}
			subdepartmentColors={subdepartmentColors}
			onimported={() => void loadBoard()}
			onclose={() => (exchangeOpen = false)}
		/>
	{/if}

	{#if audit !== null}
		<div class="backdrop" role="presentation" onclick={() => (audit = null)}></div>
		<div class="audit" role="dialog" aria-modal="true" aria-label={t('Regelprüfung')}>
			<header>
				<h2>{t('Regelprüfung')}</h2>
				<button type="button" aria-label={t('Schließen')} onclick={() => (audit = null)}>×</button>
			</header>
			<p class="finding">
				{t('{found} von {checked} Zellen verletzen eine Regel.', {
					found: audit.violations.length,
					checked: audit.checked
				})}
			</p>
			<ul>
				{#each audit.violations.slice(0, 200) as violation (`${violation.employee_id}|${violation.day}|${violation.reason}`)}
					<li>
						<span class="day">{dayLong(violation.day)}</span>
						<span class="who">{nameOf(violation.employee_id)}</span>
						<span class="reason">{reasonText(violation.reason)}</span>
					</li>
				{/each}
			</ul>
			<footer>
				<button type="button" onclick={() => (audit = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="continue"
					onclick={() => {
						audit = null;
						printOpen = true;
					}}>{t('Trotzdem drucken')}</button
				>
			</footer>
		</div>
	{/if}

	{#if locationSettings}
		<ScheduleLocationDialog
			location={locationSettings}
			positions={positionNames}
			mayEdit={auth.isAdmin}
			onclose={() => (locationSettings = null)}
			onsaved={() => {
				locationSettings = null;
				void loadBoard();
			}}
		/>
	{/if}

	{#if dispoDay}
		<ScheduleDispoDayDialog
			location={dispoDay.location}
			day={dispoDay.day}
			entries={eventsOf(dispoDay.location.id, dispoDay.day.iso)}
			mayEdit={mayEdit}
			onclose={(changed) => {
				dispoDay = null;
				if (changed) void loadBoard();
			}}
		/>
	{/if}

	{#if free}
		<ScheduleFreeTimeDialog
			day={free.day.iso}
			from={free.from}
			to={free.to}
			breakMinutes={free.breakMinutes}
			onapply={(time) => {
				if (free === null) return;
				const { employee, day } = free;
				free = null;
				// The clocks are typed once; the days in a row follow like after a shift.
				series = {
					employee,
					day,
					value: 'free',
					label: cellText({ kind: 'free', shift_id: null, ...time }),
					time
				};
			}}
			onclose={() => (free = null)}
		/>
	{/if}

	{#if series}
		<ScheduleSeriesDialog
			cell={series.label}
			from={series.day.iso}
			maxDays={rules?.max_consecutive_days ?? null}
			onapply={(count) => {
				if (series === null) return;
				const { employee, day, value, time } = series;
				series = null;
				void setSeries(employee, day, value, count, time);
			}}
			onclose={() => (series = null)}
		/>
	{/if}

	{#if demandDay}
		<ScheduleDemandDialog
			day={demandDay}
			positions={positionNames}
			shifts={shifts}
			demand={demandMap.get(demandDay.iso) ?? []}
			planned={plannedOn(demandDay.iso)}
			proposal={proposalOn(demandDay.iso)}
			dispo={proposalMap.get(demandDay.iso) ?? null}
			violations={conflictsOn(demandDay.iso)}
			mayEdit={mayEdit}
			onclose={(changed) => {
				demandDay = null;
				if (changed) void loadDemand();
			}}
			onrefresh={() => void loadBoard()}
		/>
	{/if}
{/if}

<style>
	.tools {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin: 1rem 0 0.75rem;
		flex-wrap: wrap;
	}
	.jump {
		color: var(--accent);
	}
	.jump:hover {
		border-color: var(--accent);
	}
	/* `Zeige Abteilung`: the view filter of the board. */
	/* The one action that writes a whole period sits on the right. */
	.generate {
		margin-left: auto;
		border-color: #3f9a4e;
		background: #3f9a4e;
		color: #fff;
		font-weight: 600;
	}
	.generate:hover {
		background: #358644;
		border-color: #358644;
	}
	/* Printing follows the generation and asks for its own window. */
	.print {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.print:hover {
		border-color: var(--accent);
	}
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(15, 23, 42, 0.55);
		z-index: 40;
	}
	.audit {
		position: fixed;
		top: 8vh;
		left: 50%;
		transform: translateX(-50%);
		width: min(720px, 92vw);
		max-height: 78vh;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 1rem 1.2rem;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 10px;
		box-shadow: 0 18px 50px rgba(15, 23, 42, 0.35);
		z-index: 41;
	}
	.audit header {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}
	.audit h2 {
		margin: 0;
		font-size: 1.1rem;
	}
	.audit header button {
		margin-left: auto;
	}
	.audit .finding {
		margin: 0;
		font-weight: 600;
	}
	.audit ul {
		margin: 0;
		padding: 0;
		list-style: none;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.audit li {
		display: grid;
		grid-template-columns: 12rem 1fr;
		gap: 0.2rem 0.8rem;
		padding: 0.2rem 0;
		border-bottom: 1px dashed var(--border);
		font-size: 0.9rem;
	}
	.audit li .day {
		font-variant-numeric: tabular-nums;
	}
	.audit li .reason {
		grid-column: 2;
		color: #b45309;
	}
	.audit footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.6rem;
	}
	.audit footer .continue {
		border-color: #b45309;
		color: #b45309;
		font-weight: 600;
	}
	.page {
		display: flex;
		gap: 0.3rem;
	}
	button {
		padding: 0.35rem 0.8rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
	.period {
		font-weight: 600;
	}
	.loads {
		color: var(--muted);
		font-size: 0.85rem;
	}
	/* One button per day of the shown month, across the whole width. */
	.month-days {
		display: flex;
		gap: 0.15rem;
		margin: 0 0 0.75rem;
	}
	.month-days button {
		flex: 1 1 0;
		min-width: 0;
		padding: 0.16rem 0;
		border: 1px solid var(--menu-band-border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--muted);
		font: inherit;
		font-size: 0.7rem;
		font-variant-numeric: tabular-nums;
		cursor: pointer;
	}
	.month-days button.sunday {
		background: color-mix(in srgb, var(--accent) 14%, var(--surface));
		color: var(--accent);
		font-weight: 600;
	}
	.month-days button:hover {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 22%, var(--surface));
		color: var(--accent);
	}
	/* One line per entry — short name, name, time — so the legend stays flat and the
	   board keeps the height of the window. */
	.legend {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.2rem 1rem;
		margin-bottom: 0.45rem;
		padding: 0.3rem 0.55rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
	}
	.shifts {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
		gap: 0.1rem 1rem;
	}
	.absences {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, auto));
		align-items: center;
		gap: 0.1rem 0.8rem;
		padding-left: 0.9rem;
		border-left: 1px solid var(--surface-3);
	}
	.entry {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		min-width: 0;
		line-height: 1.15;
	}
	.marker {
		flex: none;
		align-self: center;
		height: 0.95rem;
		width: 0.22rem;
		border-radius: 2px;
		background: var(--marker, #94a3b8);
	}
	.text {
		display: flex;
		flex: 1 1 auto;
		align-items: baseline;
		gap: 0.3rem;
		min-width: 0;
	}
	.name {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.76rem;
	}
	.shortName {
		display: inline-block;
		margin-right: 0.25rem;
		padding: 0.02rem 0.22rem;
		border-radius: 3px;
		background: var(--surface-3);
		color: var(--muted);
		font-size: 0.62rem;
		font-weight: 600;
	}
	.time {
		flex: none;
		color: var(--muted-2);
		font-size: 0.7rem;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.entry.inactive .name {
		color: var(--muted-2);
	}
	.grid {
		overflow: auto;
		box-sizing: border-box;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
	}
	table {
		border-collapse: collapse;
		table-layout: fixed;
	}
	th,
	td {
		border-bottom: 1px solid var(--surface-3);
		padding: 0.25rem 0.3rem;
		font-size: 0.85rem;
		text-align: center;
		white-space: nowrap;
	}
	/* The row of dates stays on top while the shifts scroll under it. The rows of the
	   disposition follow it or scroll with the body, depending on the switch in the
	   head. The stick sits on the head: a cell cannot leave its row, a head can leave
	   the table body — and the row of a body itself sticks in no browser but Chrome. */
	.grid thead {
		position: sticky;
		top: 0;
		z-index: 3;
	}
	.grid thead th {
		background: var(--surface);
		/* A collapsed border belongs to the table and would stay behind when the row
		   sticks; the line walks with the cell as a shadow. */
		border-bottom: none;
	}
	.grid thead th:not(.schoolHolidays) {
		box-shadow: inset 0 -1px 0 var(--surface-3);
	}
	/* The corner above the name column stays in front of everything. */
	.grid thead th.employees {
		z-index: 4;
		box-shadow: inset -1px 0 0 var(--surface-3), inset 0 -1px 0 var(--surface-3);
	}
	/* The disposition sticks with the dates; it sits in the head, so it needs no stick
	   of its own. */
	th.employees,
	td.employees {
		position: sticky;
		left: 0;
		z-index: 1;
		background: var(--surface);
		text-align: left;
		padding: 0.35rem 0.45rem;
		box-sizing: border-box;
		width: 250px;
		min-width: 250px;
		max-width: 250px;
		/* Collapsed borders belong to the table and would scroll with the day
		   cells; an inset shadow stays on the fixed column. */
		border-right: none;
		box-shadow: inset -1px 0 0 var(--surface-3);
	}
	th.employees {
		color: var(--muted);
		font-weight: 600;
	}
	/* Name with the hours of the month and the balance of the year. */
	.figures {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 2.9rem 2.9rem 3.7rem;
		column-gap: 0.25rem;
		align-items: baseline;
		font-variant-numeric: tabular-nums;
	}
	.figures .head {
		color: var(--muted-2);
		font-size: 0.62rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		text-align: right;
		white-space: nowrap;
	}
	.figures .head.empty {
		text-align: left;
	}
	.figures .who,
	.figures .position {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.figures .who {
		font-size: 0.8rem;
	}
	.figures .amount {
		color: var(--muted);
		font-size: 0.74rem;
		text-align: right;
		white-space: nowrap;
	}
	.figures .balance.plus {
		color: var(--good);
	}
	.figures .balance.minus {
		color: var(--bad);
	}
	.figures .position {
		grid-column: 1 / -1;
		color: var(--muted-2);
		font-size: 0.72rem;
	}
	/* Visible break between the position blocks of the board. */
	tbody tr.groupStart > td {
		border-top: 2px solid var(--border);
		padding-top: 0.5rem;
	}
	/* The sub-department tints the name column of its employees, the department when it has
	   none; without a colour the mix stays the surface, so the sticky cell keeps its opaque
	   background. */
	tbody tr.person > td.employees {
		background: color-mix(in srgb, var(--employeeColor, var(--surface)) 30%, var(--surface));
	}
	tbody tr.person > td.employees.drag {
		cursor: grab;
	}
	/* The dragged row waits where it was until it is dropped elsewhere. */
	tbody tr.dragged > td {
		opacity: 0.5;
	}
	tbody tr.drop > td {
		box-shadow: inset 0 2px 0 var(--accent);
	}
	tbody tr.dropBelow > td {
		box-shadow: inset 0 -2px 0 var(--accent);
	}
	/* Row of free-text notes under the dates. */
	/* The name cell of the date row carries the switch of the disposition, the button
	   that adds a note row and the `Ausblenden` button. Both lines share one grid
	   column, so `Ausblenden` is exactly as wide as `Sticky/Scroll` above it. */
	th.employees > .headGrid {
		display: grid;
		grid-template-columns: max-content 1fr;
		align-items: center;
		column-gap: 0.35rem;
		row-gap: 0.2rem;
	}
	th.employees > .headGrid > .dispoMode {
		grid-column: 1;
	}
	th.employees > .headGrid > .rowNew {
		grid-column: 2;
		justify-self: end;
	}
	.dispoMode {
		display: inline-flex;
		border: 1px solid var(--border);
		border-radius: 4px;
		overflow: hidden;
	}
	.dispoMode button {
		flex: 1 1 auto;
		padding: 0.05rem 0.3rem;
		border: none;
		background: var(--surface);
		color: var(--muted);
		font: inherit;
		font-size: 0.62rem;
		line-height: 1.4;
		cursor: pointer;
	}
	.dispoMode button + button {
		border-left: 1px solid var(--border);
	}
	.dispoMode button.active {
		background: color-mix(in srgb, var(--accent) 18%, var(--surface));
		color: var(--accent);
		font-weight: 600;
	}
	.dispoMode button:hover {
		color: var(--accent);
	}
	.rowNew,
	.rowRemove {
		width: 1.3rem;
		height: 1.3rem;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--muted);
		font-size: 0.9rem;
		line-height: 1;
		cursor: pointer;
	}
	.rowNew:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.rowRemove:hover {
		border-color: var(--bad);
		color: var(--bad);
	}
	/* The settings of a typed row; the name of a house opens them by itself. */
	.rowSettings {
		flex: none;
		width: 1.3rem;
		height: 1.3rem;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--muted);
		font-size: 0.85rem;
		line-height: 1;
		cursor: pointer;
	}
	.rowSettings:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.rowHead {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	.rowText {
		flex: 1 1 auto;
		min-width: 0;
		padding: 0.15rem 0.25rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: 0.78rem;
	}
	.rowText::placeholder {
		color: var(--muted-2);
	}
	/* The name of a location opens its settings. */
	.locationName {
		flex: 1 1 auto;
		min-width: 0;
		padding: 0.15rem 0.25rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: 0.78rem;
		font-weight: 600;
		text-align: left;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: pointer;
	}
	.locationName:hover,
	.locationName:focus-visible {
		border-color: var(--border);
		background: var(--surface);
		color: var(--accent);
	}
	input.rowText:hover,
	input.rowText:focus {
		border-color: var(--border);
		background: var(--surface);
	}
	/* Rows of the disposition: one per location, its events per day. */
	/* The disposition sits in the head (sticky) or in the body (scroll) — the look is
	   the same in both places. */
	tr.dispo > td {
		padding: 0.2rem 0.25rem;
		background: color-mix(in srgb, var(--accent) 4%, var(--surface));
		/* A sticky row would leave its line behind; the shadow travels with the cell. */
		border-bottom: none;
		box-shadow: inset 0 -1px 0 var(--surface-3);
	}
	tr.dispo > td.employees {
		padding: 0.2rem 0.6rem;
		background: color-mix(in srgb, var(--accent) 4%, var(--surface));
		box-shadow: inset -1px 0 0 var(--surface-3), inset 0 -1px 0 var(--surface-3);
	}
	/* A location with a colour takes it into its disposition row. */
	tr.dispo.locationColor > td.employees {
		background: color-mix(in srgb, var(--location) 18%, var(--surface));
	}
	.dispoCell {
		display: flex;
		flex-direction: column;
		gap: 0.05rem;
		width: 100%;
		padding: 0.15rem 0.25rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: 0.72rem;
		line-height: 1.25;
		text-align: left;
		cursor: pointer;
	}
	.dispoCell:hover,
	.dispoCell:focus {
		border-color: var(--accent);
		background: var(--surface);
	}
	.dispoCell.empty {
		align-items: center;
		color: var(--muted-2);
	}
	.event {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.event .time {
		color: var(--accent);
		font-variant-numeric: tabular-nums;
		margin-right: 0.2rem;
	}
	td.set .event {
		font-weight: 600;
	}
	th .weekday {
		display: block;
		color: var(--muted);
		font-weight: 600;
	}
	th .date {
		display: block;
	}
	/* The date opens the staffing demand of that day. */
	th .dateButton {
		position: relative;
		display: block;
		width: 100%;
		padding: 0.15rem 0.2rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	th .dateButton:hover {
		border-color: var(--border);
		background: var(--surface);
	}
	/* Shortage of the day, in the top right corner of the date. */
	th .demand {
		position: absolute;
		top: 0.05rem;
		right: 0.2rem;
		padding: 0 0.32rem;
		border-radius: 999px;
		background: color-mix(in srgb, var(--good) 18%, transparent);
		color: var(--good);
		font-size: 0.68rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
		line-height: 1.3;
	}
	th .demand.missing,
	td.missing {
		background: color-mix(in srgb, var(--bad) 16%, transparent);
		color: var(--bad);
	}
	th .demand.unclear {
		right: 1.5rem;
		background: color-mix(in srgb, var(--muted) 20%, transparent);
		color: var(--muted);
	}
	/* A rule of the planning is broken on that day — the other corner of the date. */
	th .conflict {
		position: absolute;
		top: 0.05rem;
		left: 0.25rem;
		padding: 0 0.32rem;
		border-radius: 999px;
		background: color-mix(in srgb, var(--bad) 18%, transparent);
		color: var(--bad);
		font-size: 0.68rem;
		font-weight: 700;
		line-height: 1.3;
	}
	td .of {
		color: var(--muted);
		font-size: 0.72rem;
	}
	th.sunday,
	td.sunday {
		background: color-mix(in srgb, var(--accent) 8%, var(--surface));
	}
	/* A public holiday tints the whole column and names the day under the date. */
	th.holiday,
	td.holiday {
		background: color-mix(in srgb, var(--holiday) 16%, var(--surface));
	}
	th .holidayName {
		display: block;
		color: var(--holiday);
		font-size: 0.68rem;
		font-weight: 700;
		line-height: 1.15;
	}
	/* A school-holiday stretch gets a bar under the date row and its name at the start. */
	th.schoolHolidays {
		box-shadow: inset 0 -3px 0 color-mix(in srgb, var(--schoolHolidays) 60%, transparent);
	}
	th .schoolHolidayName {
		display: block;
		color: var(--schoolHolidays);
		font-size: 0.66rem;
		font-weight: 600;
		line-height: 1.15;
	}
	/* The cell shows `Kürzel - von–bis` and opens the menu of all shifts. */
	.cell {
		display: block;
		width: 100%;
		min-width: 7.5rem;
		padding: 0.2rem 0.25rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: inherit;
		font: inherit;
		font-size: 0.8rem;
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.cell.still {
		cursor: default;
	}
	.cell:disabled {
		color: var(--muted);
	}
	.cell.open {
		border-color: var(--accent);
	}
	td.set .cell {
		border-color: var(--cellColor, var(--border));
		box-shadow: inset 3px 0 0 var(--cellColor, transparent);
		font-weight: 600;
	}
	/* A free day of the catalogue marks the day without counting as work. */
	.cell.free-day {
		background: repeating-linear-gradient(
			45deg,
			color-mix(in srgb, var(--muted-2) 14%, transparent) 0 4px,
			transparent 4px 8px
		);
		font-weight: 500;
		font-style: italic;
		color: var(--muted);
	}
	/* Under the shift the free text of the cell — same width as the shift above it. */
	.cellNote {
		display: block;
		box-sizing: border-box;
		width: 100%;
		min-width: 0;
		margin-top: 0.1rem;
		padding: 0.05rem 0.25rem;
		border: 1px solid transparent;
		border-radius: 4px;
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: 0.72rem;
		text-align: left;
		text-overflow: ellipsis;
	}
	.cellNote.still {
		cursor: default;
		overflow: hidden;
	}
	input.cellNote:hover,
	input.cellNote:focus {
		border-color: var(--border);
		background: var(--surface);
	}
	.menu-backdrop {
		position: fixed;
		inset: 0;
		z-index: 40;
	}
	.menu {
		position: fixed;
		z-index: 41;
		max-height: 21rem;
		overflow: auto;
		padding: 0.2rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
		box-shadow: 0 10px 28px rgb(0 0 0 / 18%);
	}
	/* Day and employee above the list, so a click on the wrong row is caught at once. */
	.menu-head {
		position: sticky;
		top: -0.2rem;
		z-index: 1;
		display: flex;
		flex-direction: column;
		gap: 0.05rem;
		margin: -0.2rem -0.2rem 0.2rem;
		padding: 0.3rem 0.45rem 0.35rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}
	.menu-head .day {
		font-size: 0.72rem;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}
	.menu-head .who {
		font-size: 0.85rem;
		font-weight: 700;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.menu-entry {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		width: 100%;
		padding: 0.25rem 0.35rem;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.menu-entry:hover,
	.menu-entry:focus-visible {
		background: var(--surface-3);
	}
	.menu-entry.selected {
		background: color-mix(in srgb, var(--accent) 14%, transparent);
	}
	.menu-entry.inactive .name {
		color: var(--muted-2);
	}
	.menu .marker.empty {
		background: var(--border);
	}
	tfoot td {
		font-weight: 600;
		color: var(--muted);
	}
	.empty,
	.bad,
	.seriesNotice {
		margin: 0.8rem 0 0;
		font-size: 0.85rem;
	}
	.bad {
		color: var(--bad);
	}
	/* A series that the planning rules cut short, or a finished import. */
	.seriesNotice {
		padding: 0.45rem 0.7rem;
		border: 1px solid var(--notice-border);
		border-radius: 6px;
		background: var(--notice-bg);
	}
	.seriesNotice p {
		margin: 0;
	}
</style>
