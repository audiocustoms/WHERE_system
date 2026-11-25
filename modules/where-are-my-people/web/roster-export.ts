/**
 * The duty plan as a file: CSV, XLSX and PDF.
 *
 * The preview shows the same sheets the PDF carries, so both are built from one
 * report — the days of the month, the rows of the disposition per location and the
 * rows of the staff. A sheet never carries more than one week, so a month becomes
 * three to five pages that start on the first day of that month.
 *
 * Nothing here asks the network and nothing renders the board: the CSV and XLSX are
 * plain text and a zip, the PDF is written by hand with the standard fonts, so no
 * library has to travel with the module.
 */

export interface SheetDay {
	iso: string;
	/** `Mo`, `Di`, … */
	weekday: string;
	/** Day of the month, `01` to `31`. */
	day: string;
	sunday: boolean;
	holiday: string | null;
}

/** One entry of the disposition: the time of the event and what it is. */
export interface DispoEvent {
	time: string;
	title: string;
}

/** A line of a cell of the disposition — the time only before the first line of an event. */
export interface DispoLine {
	time: string;
	title: string;
	/** `true` when the line continues the title of the event above it. */
	wrapped?: boolean;
}

/** One row of the disposition: the events of a location, per day. */
export interface DispoRow {
	location: string;
	/** One list of entries per day, in the order of the days of the report. */
	days: DispoEvent[][];
}

/** One row of the staff. */
export interface StaffRow {
	name: string;
	department: string;
	subdepartment: string;
	position: string;
	/** The tint of the row: the sub-department's colour, the department's when it has none. */
	color: string;
	target: number | null;
	actual: number;
	difference: number | null;
	/** The cell of each day, in the order of the days of the report. */
	days: string[];
	/** The colour of each cell, empty when it carries none. */
	colors: string[];
	/** The window of the shift in each cell, empty for absences and free days. */
	times: string[];
	/** The free text of each cell, empty when it carries none. */
	notes: string[];
}

/**
 * The line under the name of an employee: the position, then the department and
 * the sub-department beneath it — the same on the print, in the XLSX and in the PDF.
 */
export function staffLineText(
	row: Pick<StaffRow, 'position' | 'department' | 'subdepartment'>
): string {
	return [row.position, row.department, row.subdepartment]
		.map((part) => (part ?? '').trim())
		.filter((part) => part !== '')
		.join(' · ');
}

/**
 * Size of that line against the font of the row. Three parts have to fit into the
 * first column, so it runs a little smaller than the name above it; the print reads
 * the same number through the CSS variable `--line2`.
 */
export const SECOND_LINE_FACTOR = 0.7;

export interface Report {
	title: string;
	period: string;
	asOf: string;
	/** Every day of the period, in order. */
	days: SheetDay[];
	/** The locations of the disposition, in the order of the master data. */
	dispo: DispoRow[];
	staff: StaffRow[];
	legend: string;
	/** Paper in millimetres. */
	paper: { width: number; height: number };
	/** File name without extension. */
	file: string;
}

const WEEKDAYS = ['So', 'Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa'];

export function toDate(iso: string): Date {
	const [year, month, day] = iso.split('-').map(Number);
	return new Date(year, month - 1, day);
}

/** `7,50` — hours always with two decimals. */
export function hoursText(value: number | null, withSign = false): string {
	if (value === null) return '';
	const label = Math.abs(value).toLocaleString('de-DE', {
		minimumFractionDigits: 2,
		maximumFractionDigits: 2
	});
	if (!withSign) return value < 0 ? `-${label}` : label;
	if (value > 0) return `+${label}`;
	if (value < 0) return `-${label}`;
	return label;
}

/** `01.11.` — the head of a day column and the cells of the exports. */
export function shortDay(day: SheetDay): string {
	return `${day.day}.${day.iso.slice(5, 7)}.`;
}

/**
 * The days of the period cut into weeks that begin on Monday: the first sheet starts
 * on the first day of the month and ends with the Sunday of that week, the following
 * sheets run Monday to Sunday.
 */
export function weeks(days: SheetDay[]): SheetDay[][] {
	const sheets: SheetDay[][] = [];
	let current: SheetDay[] = [];
	for (const day of days) {
		current.push(day);
		if (current.length === 7 || day.sunday) {
			sheets.push(current);
			current = [];
		}
	}
	if (current.length > 0) sheets.push(current);
	return sheets;
}

/** How many sheets the report becomes — the preview and the export agree. */
export function pageCount(report: Report): number {
	return weeks(report.days).length;
}

/** Points per millimetre: the paper is measured in millimetres, the PDF in points. */
const MM = 2.834645669;

/** The paper the sheet is printed on and exported to. */
export const PAPERS = [
	{ value: 'a3-quer', label: 'DIN A3 quer', css: 'A3 landscape', width: 420, height: 297 },
	{ value: 'a3-hoch', label: 'DIN A3 hoch', css: 'A3 portrait', width: 297, height: 420 },
	{ value: 'a4-quer', label: 'DIN A4 quer', css: 'A4 landscape', width: 297, height: 210 },
	{ value: 'a4-hoch', label: 'DIN A4 hoch', css: 'A4 portrait', width: 210, height: 297 }
];

/** One paper of `PAPIERE`. */
export type Paper = (typeof PAPERS)[number];

/** A line of an event: the time in front of the title. */
const DISPO_LINE = 1.25;
/** The room above and below the lines of a row of the disposition, in millimetres. */
const DISPO_MARGIN = 1.6;
/** How far a wrapped line steps in, so it reads as part of the event above it. */
const DISPO_INDENT = 3.5;

/**
 * Where one sheet puts its blocks, in millimetres.
 *
 * The preview and the PDF take the same numbers, so a printed page and an exported
 * one break their lines at the same place.
 */
export interface SheetMetrics {
	margin: number;
	head: number;
	foot: number;
	/** The empty row between the disposition and the staff. */
	gap: number;
	/** Height of a row of dates — taller where a holiday writes underneath. */
	dayHead: number;
	/** Width of the first column: the location of a row or the name of a person. */
	first: number;
	/** Width of one day column. */
	day: number;
	/** Height of the whole block of the disposition. */
	dispo: number;
	/** Height of every row of the disposition, in the order of the locations. */
	dispoRows: number[];
	/** The lines of every cell: location, then day of the sheet, then line. */
	dispoCells: DispoLine[][][];
	/** Font of the disposition in points. */
	dispoFont: number;
	/** Height of a row of staff. */
	row: number;
	/** Font of the staff in points. */
	font: number;
}

/**
 * The space one sheet of the report offers.
 *
 * Every event of a day keeps its own line, so a row of the disposition is as tall as
 * the busiest day of that location — a day with five events shows all five instead of
 * one line that eats the rest. The disposition starts with a small font of its own and
 * shrinks further where the calendar is full, but it never takes the room the staff
 * needs for its rows: the sheet holds the plan, not the calendar.
 */
export function sheetMetrics(report: Report, page: SheetDay[]): SheetMetrics {
	const margin = 10;
	const head = 11;
	const foot = 6;
	const gap = 3.5;
	const dayHead = page.some((day) => day.holiday !== null) ? 8.4 : 6.5;
	const innerWidth = report.paper.width - 2 * margin;
	const first = Math.min(innerWidth * 0.34, 86);
	const dayWidth = (innerWidth - first) / Math.max(1, page.length);
	const room = report.paper.height - 2 * margin - head - foot - dayHead * 2 - gap;

	const indexes = page.map((entry) => report.days.findIndex((day) => day.iso === entry.iso));

	/** The lines of one cell: one event per line block, its title wrapped to the column. */
	const cellRows = (entries: DispoEvent[], size: number): DispoLine[] => {
		const rows: DispoLine[] = [];
		for (const entry of entries) {
			const timeWidth = entry.time === '' ? 0 : textWidth(entry.time, size) + 2.8;
			const [first, ...wrapped] = wrapText(
				entry.title,
				Math.max(24, dayWidth * MM - 3 - timeWidth),
				Math.max(24, dayWidth * MM - 3 - DISPO_INDENT * MM),
				size
			);
			rows.push({ time: entry.time, title: first });
			for (const remaining of wrapped) rows.push({ time: '', title: remaining, wrapped: true });
		}
		return rows;
	};

	const buildRows = (size: number) => {
		const cells = report.dispo.map((row) =>
			indexes.map((offset) => cellRows(row.days[offset] ?? [], size))
		);
		const heights = cells.map(
			(days) =>
				DISPO_MARGIN +
				Math.max(1, ...days.map((rows) => rows.length)) * ((size * DISPO_LINE) / MM)
		);
		return { cells, heights };
	};

	// The staff keeps the room of its smallest rows; the disposition may use what is
	// left of the paper, but never more than 40 % of it — a full calendar must not
	// push the plan off the sheet.
	const staff = Math.max(1, report.staff.length);
	const limit = Math.max(0, Math.min(room * 0.4, room - staff * 4.2));

	let dispoFont = 9;
	let built = buildRows(dispoFont);
	let dispo = built.heights.reduce((total, value) => total + value, 0);
	while (dispoFont > 4.6 && dispo > limit) {
		dispoFont = Math.round((dispoFont - 0.2) * 10) / 10;
		built = buildRows(dispoFont);
		dispo = built.heights.reduce((total, value) => total + value, 0);
	}

	let dispoRows = built.heights;
	if (dispo > limit && limit > 0) {
		const factor = limit / dispo;
		dispoRows = dispoRows.map((value) => value * factor);
		dispo = limit;
	}

	const staffSpace = room - dispo;
	const row = Math.max(4.2, Math.min(9.2, staffSpace / staff));
	// A row carries two lines: the name with the hours, and the position above the
	// department. 0.8 millimetres of row per point of font is what both need.
	const font = Math.max(4.6, Math.min(11, (row - 0.2) / 0.8));

	return {
		margin,
		head,
		foot,
		gap,
		dayHead,
		first,
		day: dayWidth,
		dispo,
		dispoRows,
		dispoCells: built.cells,
		dispoFont,
		row,
		font
	};
}

/** Wraps a text to a column: two lines carry the longest of the titles. */
function wrapText(
	value: string,
	firstWidth: number,
	followWidth: number,
	size: number,
	maxRows = 2
): string[] {
	const words = value.split(' ').filter((part) => part !== '');
	if (words.length === 0) return [''];
	const rows: string[] = [];
	let current = '';
	for (const word of words) {
		const attempt = current === '' ? word : `${current} ${word}`;
		if (current === '' || textWidth(attempt, size) <= (rows.length === 0 ? firstWidth : followWidth)) {
			current = attempt;
			continue;
		}
		if (rows.length + 1 >= maxRows) {
			current = attempt;
			continue;
		}
		rows.push(current);
		current = word;
	}
	rows.push(current);
	return rows;
}

// ---------------------------------------------------------------- the pages

/** One run of text in a cell — the print sets the time of an event in bold. */
interface Run {
	label: string;
	bold?: boolean;
	/** Font size in points. */
	size?: number;
	/** Colour as `#rrggbb`. */
	color?: string;
}

/** One cell of a page, as the print draws it. */
interface Cell {
	/** The text in runs; `\n` breaks the line inside the cell. */
	runs?: Run[];
	/** The value as a number — the hours of a row. */
	amount?: number;
	/** `true` when the number carries a sign, as `+5,00` does. */
	sign?: boolean;
	/** The fill of the cell as `#rrggbb`. */
	fill?: string;
	right?: boolean;
	top?: boolean;
	/** `false` keeps the cell on one line, so a long text runs over its neighbours. */
	wrap?: boolean;
	/** `cell` frames the cell thinly, `block` puts a thick line on top, `none` draws nothing. */
	frame?: 'cell' | 'block' | 'none';
}

interface Row {
	/** Height of the row in millimetres, as on paper. */
	height: number;
	cells: (Cell | null)[];
}

/** One printed page: the head, the disposition, the empty row, the staff, the footer. */
export interface Sheet {
	metrics: SheetMetrics;
	rows: Row[];
}

/** The columns in front of the days: the name, then `Soll`, `Ist` and `+/-`. */
const LEAD_IN = 4;
/** The three columns of the hours inside the first block of the sheet, in millimetres. */
const HOUR_WIDTH = [13, 13, 12];

const HEAD_HEX = '#eceff3';
const SUNDAY_HEX = '#f4f4f5';
const GRAY_HEX = '#6b7280';
const RED_HEX = '#8a1c1c';
const BLOCK_HEX = '#333333';
const FRAME_HEX = '#8a8a8a';

/** A shift colour as the sheet fills it: a fifth of it on white. */
function cellTint(color: string, share = 0.2): string {
	const found = /^#?([0-9a-f]{6})$/i.exec(color.trim());
	if (found === null) return HEAD_HEX;
	const amount = Number.parseInt(found[1], 16);
	const parts = [(amount >> 16) & 0xff, (amount >> 8) & 0xff, amount & 0xff];
	return `#${parts
		.map((part) =>
			Math.round(255 - (255 - part) * share)
				.toString(16)
				.padStart(2, '0')
		)
		.join('')}`;
}

/**
 * The printed pages of the month, each as the grid of the sheet: the head with the
 * month, the week and the day of the print, the row of the dates, the disposition of
 * every location, an empty row, the row of the dates with `Soll` | `Ist` | `+/-`, the
 * staff and the footer. CSV and XLSX both write these pages, so a file shows what the
 * printer shows — same rows, same columns, same line breaks.
 */
export function sheets(report: Report): Sheet[] {
	const pages = weeks(report.days);
	return pages.map((page, number) => {
		const metrics = sheetMetrics(report, page);
		const columns = LEAD_IN + page.length;

		const hours = (label: string): Cell => ({
			runs: [{ label, bold: true, size: metrics.font * 0.95 }],
			right: true,
			fill: HEAD_HEX,
			frame: 'cell'
		});
		const date = (day: SheetDay): Cell => {
			const runs: Run[] = [
				{ label: `${day.weekday} ${shortDay(day)}`, bold: true, size: metrics.font * 0.95 }
			];
			if (day.holiday !== null) {
				runs.push({ label: `\n${day.holiday}`, size: metrics.font * 0.78, color: RED_HEX });
			}
			return { runs, fill: day.sunday ? SUNDAY_HEX : HEAD_HEX, frame: 'cell' };
		};

		// The head of the page: the month, the week and the day it was printed.
		const head: (Cell | null)[] = Array.from({ length: columns }, () => ({ frame: 'none' }));
		head[0] = {
			runs: [{ label: report.title, bold: true, size: metrics.font * 1.9 }],
			wrap: false,
			frame: 'none'
		};
		const period = `${shortDay(page[0])} – ${shortDay(page[page.length - 1])}`;
		const asOf = `Stand ${report.asOf}`;
		if (columns - 1 > LEAD_IN) {
			head[LEAD_IN] = {
				runs: [{ label: period, size: metrics.font }],
				wrap: false,
				frame: 'none'
			};
			head[columns - 1] = {
				runs: [{ label: asOf, size: metrics.font * 0.9, color: GRAY_HEX }],
				right: true,
				wrap: false,
				frame: 'none'
			};
		} else {
			head[columns - 1] = {
				runs: [
					{ label: period, size: metrics.font },
					{ label: `\n${asOf}`, size: metrics.font * 0.9, color: GRAY_HEX }
				],
				right: true,
				frame: 'none'
			};
		}

		// The row of the dates; the columns in front of the days stay grey and empty.
		const band: (Cell | null)[] = Array.from({ length: LEAD_IN }, () => ({
			fill: HEAD_HEX,
			frame: 'cell' as const
		}));
		const dateRow = (front: (Cell | null)[]): Row => ({
			height: metrics.dayHead,
			cells: [...front, ...page.map(date)]
		});

		const rows: Row[] = [{ height: metrics.head, cells: head }, dateRow(band)];

		// The disposition: every event on its own line, the time in bold.
		report.dispo.forEach((row, index) => {
			const cells: (Cell | null)[] = [
				{
					runs: [{ label: row.location, bold: true, size: metrics.dispoFont }],
					top: true,
					frame: 'cell'
				},
				...Array.from({ length: LEAD_IN - 1 }, () => ({ frame: 'cell' as const }))
			];
			(metrics.dispoCells[index] ?? []).forEach((labels) => {
				const runs: Run[] = [];
				labels.forEach((label, offset) => {
					if (offset > 0) runs.push({ label: '\n', size: metrics.dispoFont });
					if (label.time !== '') {
						runs.push({ label: label.time, bold: true, size: metrics.dispoFont });
						runs.push({ label: ` ${label.title}`, size: metrics.dispoFont });
						return;
					}
					runs.push({
						label: `${label.wrapped === true ? '   ' : ''}${label.title}`,
						size: metrics.dispoFont
					});
				});
				cells.push({ runs, top: true, frame: 'cell' });
			});
			rows.push({ height: metrics.dispoRows[index], cells });
		});

		// The empty row between the disposition and the staff.
		rows.push({
			height: metrics.gap,
			cells: Array.from({ length: columns }, () => ({ frame: 'cell' as const }))
		});

		// The dates again, now with the labels of the hours in the first column.
		rows.push(
			dateRow([{ frame: 'cell', fill: HEAD_HEX }, hours('Soll'), hours('Ist'), hours('+/-')])
		);

		// The staff: the name with `Position · Abteilung · Unterabteilung`, then the
		// hours of the month.
		let previousPosition: string | null = null;
		report.staff.forEach((row) => {
			const frame: Cell['frame'] =
				previousPosition !== null && previousPosition !== row.position ? 'block' : 'cell';
			previousPosition = row.position;
			const second = staffLineText(row);
			const runs: Run[] = [{ label: row.name, bold: true, size: metrics.font }];
			if (second !== '') {
				runs.push({ label: `\n${second}`, size: metrics.font * SECOND_LINE_FACTOR, color: GRAY_HEX });
			}
			const cells: (Cell | null)[] = [
				{
					runs,
					fill: row.color === '' ? undefined : cellTint(row.color, 0.3),
					top: true,
					frame
				},
				{ amount: row.target ?? undefined, right: true, top: true, frame },
				{ amount: row.actual, right: true, top: true, frame },
				{ amount: row.difference ?? undefined, sign: true, right: true, top: true, frame }
			];
			page.forEach((day) => {
				const offset = report.days.findIndex((entry) => entry.iso === day.iso);
				const label = row.days[offset] ?? '';
				const time = row.times?.[offset] ?? '';
				const note = row.notes?.[offset] ?? '';
				const color = row.colors?.[offset] ?? '';
				const cellRuns: Run[] = [];
				if (label !== '') {
					// Short name and time share the first line, the note takes the second.
					const caption = time === '' ? label : `${label} - ${time}`;
					cellRuns.push({
						label: caption,
						size: metrics.font,
						color: label === 'frei' ? GRAY_HEX : undefined
					});
					if (note !== '' && metrics.row >= 8) {
						cellRuns.push({ label: `\n${note}`, size: metrics.font * 0.85, color: GRAY_HEX });
					}
				}
				const fill =
					color !== '' ? cellTint(color) : label === '' && day.sunday ? SUNDAY_HEX : undefined;
				cells.push({
					runs: cellRuns.length > 0 ? cellRuns : undefined,
					fill,
					top: true,
					frame
				});
			});
			rows.push({ height: metrics.row, cells });
		});

		// The footer: the legend and the page of the month.
		const foot: (Cell | null)[] = Array.from({ length: columns }, () => ({ frame: 'none' }));
		foot[0] = {
			runs: [{ label: report.legend, size: metrics.font * 0.8, color: GRAY_HEX }],
			wrap: false,
			frame: 'none'
		};
		foot[columns - 1] = {
			runs: [
				{ label: `Seite ${number + 1} von ${pages.length}`, size: metrics.font * 0.8, color: GRAY_HEX }
			],
			right: true,
			wrap: false,
			frame: 'none'
		};
		rows.push({ height: metrics.foot, cells: foot });

		return { metrics, rows };
	});
}

/** The text of a cell: its runs joined, a number with a decimal comma. */
function cellText(cell: Cell | null): string {
	if (cell === null) return '';
	if (cell.amount !== undefined) return hoursText(cell.amount, cell.sign === true);
	return (cell.runs ?? []).map((run) => run.label).join('');
}

// ---------------------------------------------------------------- CSV

/** A field of a German sheet: `;` as separator, quotes only where needed. */
function csvField(value: string): string {
	if (/[";\n\r]/.test(value)) return `"${value.replace(/"/g, '""')}"`;
	return value;
}

/**
 * The plan as text, one block per printed page: the head, the row of the dates, the
 * disposition, the empty row, the dates with the hours and then the staff. A cell that
 * carries two lines keeps them. Decimal comma and semicolon are what a German Excel
 * expects, and the BOM keeps the umlauts readable.
 */
export function toCsv(report: Report): string {
	const rows: string[] = [];
	sheets(report).forEach((sheet, number) => {
		if (number > 0) rows.push('');
		for (const row of sheet.rows) {
			rows.push(row.cells.map((cell) => csvField(cellText(cell))).join(';'));
		}
	});
	return `\ufeff${rows.join('\r\n')}\r\n`;
}

// ---------------------------------------------------------------- XLSX

/** A1, B1, … AA1 for the columns of a sheet. */
function column(index: number): string {
	let remaining = index + 1;
	let label = '';
	while (remaining > 0) {
		const offset = (remaining - 1) % 26;
		label = String.fromCharCode(65 + offset) + label;
		remaining = Math.floor((remaining - 1) / 26);
	}
	return label;
}

function xmlText(value: string): string {
	return value
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;');
}

/** `#rrggbb` → `FFRRGGBB`, the way OOXML writes a colour. */
function argb(color: string): string {
	const found = /^#?([0-9a-f]{6})$/i.exec(color.trim());
	return `FF${(found?.[1] ?? '111111').toUpperCase()}`;
}

/** One run of a cell as the spreadsheet writes it: its own font in front of the text. */
function runXml(run: Run): string {
	const size = (run.size ?? 11).toFixed(1);
	const color = argb(run.color ?? '#111111');
	const bold = run.bold === true ? '<b/>' : '';
	return `<r><rPr>${bold}<sz val="${size}"/><color rgb="${color}"/><rFont val="Calibri"/></rPr><t xml:space="preserve">${xmlText(run.label)}</t></r>`;
}

/**
 * The styles of one workbook: the fonts, fills, borders and cell formats the print
 * asks for, collected as the sheets are written. A format is only written once, so a
 * month of sheets needs a handful of them.
 */
class StyleBook {
	private fonts: string[] = [];
	private fontIndex = new Map<string, number>();
	private fills: string[] = [
		'<fill><patternFill patternType="none"/></fill>',
		'<fill><patternFill patternType="gray125"/></fill>'
	];
	private fillIndex = new Map<string, number>();
	private frame: string[] = ['<border><left/><right/><top/><bottom/><diagonal/></border>'];
	private frameIndex = new Map<string, number>([['none', 0]]);
	private formats: string[] = [];
	private formatIndex = new Map<string, number>();
	private styleBook: string[] = [];
	private key = new Map<string, number>();

	private font(bold: boolean, size: number, color: string): number {
		const key = `${bold}|${size.toFixed(1)}|${color}`;
		const present = this.fontIndex.get(key);
		if (present !== undefined) return present;
		this.fonts.push(
			`<font>${bold ? '<b/>' : ''}<sz val="${size.toFixed(1)}"/><color rgb="${argb(color)}"/><name val="Calibri"/></font>`
		);
		this.fontIndex.set(key, this.fonts.length - 1);
		return this.fonts.length - 1;
	}

	private fill(color: string | undefined): number {
		if (color === undefined) return 0;
		const present = this.fillIndex.get(color);
		if (present !== undefined) return present;
		this.fills.push(
			`<fill><patternFill patternType="solid"><fgColor rgb="${argb(color)}"/><bgColor indexed="64"/></patternFill></fill>`
		);
		this.fillIndex.set(color, this.fills.length - 1);
		return this.fills.length - 1;
	}

	private grid(kind: 'cell' | 'block' | 'none'): number {
		const present = this.frameIndex.get(kind);
		if (present !== undefined) return present;
		const thin = (page: string) => `<${page} style="thin"><color rgb="${argb(FRAME_HEX)}"/></${page}>`;
		const top =
			kind === 'block'
				? `<top style="medium"><color rgb="${argb(BLOCK_HEX)}"/></top>`
				: thin('top');
		this.frame.push(
			`<border>${thin('left')}${thin('right')}${top}${thin('bottom')}<diagonal/></border>`
		);
		this.frameIndex.set(kind, this.frame.length - 1);
		return this.frame.length - 1;
	}

	private format(sign: boolean): number {
		const number = sign ? '+0.00;-0.00;0.00' : '0.00';
		const present = this.formatIndex.get(number);
		if (present !== undefined) return present;
		this.formats.push(number);
		this.formatIndex.set(number, 164 + this.formats.length - 1);
		return 164 + this.formats.length - 1;
	}

	/** The index of the cell format the print asks for. */
	cellFormat(cell: Cell): number {
		const runs = cell.runs ?? [];
		const bold = runs.some((run) => run.bold === true);
		const size = runs[0]?.size ?? 11;
		const color = runs[0]?.color ?? '#111111';
		const font = this.font(bold, size, color);
		const fill = this.fill(cell.fill);
		const grid = this.grid(cell.frame ?? 'cell');
		const number = cell.amount === undefined ? 0 : this.format(cell.sign === true);
		const wrap = cell.wrap !== false;
		const key = `${font}|${fill}|${grid}|${number}|${cell.right === true}|${cell.top === true}|${wrap}`;
		const present = this.key.get(key);
		if (present !== undefined) return present;
		const alignment = [
			cell.right === true ? 'horizontal="right"' : 'horizontal="left"',
			`vertical="${cell.top === true ? 'top' : 'center'}"`,
			wrap ? 'wrapText="1"' : ''
		]
			.filter((part) => part !== '')
			.join(' ');
		this.styleBook.push(
			`<xf numFmtId="${number}" fontId="${font}" fillId="${fill}" borderId="${grid}" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"${number === 0 ? '' : ' applyNumberFormat="1"'}><alignment ${alignment}/></xf>`
		);
		this.key.set(key, this.styleBook.length);
		return this.styleBook.length;
	}

	xml(): string {
		const formats =
			this.formats.length === 0
				? ''
				: `<numFmts count="${this.formats.length}">${this.formats
						.map((kind, index) => `<numFmt numFmtId="${164 + index}" formatCode="${kind}"/>`)
						.join('')}</numFmts>`;
		const styleBook = ['<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>', ...this.styleBook].join('');
		return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">${formats}<fonts count="${this.fonts.length}">${this.fonts.join('')}</fonts><fills count="${this.fills.length}">${this.fills.join('')}</fills><borders count="${this.frame.length}">${this.frame.join('')}</borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="${this.styleBook.length + 1}">${styleBook}</cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>`;
	}
}

/** One worksheet with a printed page: the grid, its widths and the heights of its rows. */
function xlsxSheet(sheet: Sheet, report: Report, styleBook: StyleBook, first: boolean): string {
	const columns = Math.max(...sheet.rows.map((row) => row.cells.length));
	const widths = [
		Math.max(12, sheet.metrics.first - HOUR_WIDTH.reduce((total, value) => total + value, 0)),
		...HOUR_WIDTH
	];
	for (let index = LEAD_IN; index < columns; index += 1) widths.push(sheet.metrics.day);
	const columnsXml = widths
		.map(
			(mm, index) =>
				`<col min="${index + 1}" max="${index + 1}" width="${(mm / 1.85).toFixed(2)}" customWidth="1"/>`
		)
		.join('');

	const rowsXml = sheet.rows
		.map((row, number) => {
			const cells = row.cells
				.map((cell, index) => {
					if (cell === null || cell === undefined) return '';
					const offset = `${column(index)}${number + 1}`;
					const style = ` s="${styleBook.cellFormat(cell)}"`;
					if (cell.amount !== undefined) return `<c r="${offset}"${style}><v>${cell.amount}</v></c>`;
					const runs = cell.runs ?? [];
					if (runs.length === 0) return `<c r="${offset}"${style}/>`;
					return `<c r="${offset}"${style} t="inlineStr"><is>${runs.map(runXml).join('')}</is></c>`;
				})
				.join('');
			// A point of headroom: the paper box has its padding, a spreadsheet cell none.
			const height = row.height * MM + 1;
			return `<row r="${number + 1}" ht="${height.toFixed(2)}" customHeight="1">${cells}</row>`;
		})
		.join('');

	// The month and the legend stand over several columns, as they do on paper.
	const total: string[] = [`A1:${column(Math.min(LEAD_IN, columns) - 1)}1`];
	const footRow = sheet.rows.length;
	if (columns > 2) total.push(`A${footRow}:${column(columns - 2)}${footRow}`);
	const mergeXml = `<mergeCells count="${total.length}">${total
		.map((period) => `<mergeCell ref="${period}"/>`)
		.join('')}</mergeCells>`;

	const shorter = Math.min(report.paper.width, report.paper.height);
	const paper = Math.abs(shorter - 297) < 2 ? 8 : Math.abs(shorter - 210) < 2 ? 9 : 0;
	const portrait = report.paper.height > report.paper.width;
	const margin = (sheet.metrics.margin / 25.4).toFixed(4);
	return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetPr><pageSetUpPr fitToPage="1"/></sheetPr><sheetViews><sheetView${first ? ' tabSelected="1"' : ''} showGridLines="0" workbookViewId="0"/></sheetViews><sheetFormatPr defaultRowHeight="12"/><cols>${columnsXml}</cols><sheetData>${rowsXml}</sheetData>${mergeXml}<pageMargins left="${margin}" right="${margin}" top="${margin}" bottom="${margin}" header="0" footer="0"/><pageSetup paperSize="${paper}" orientation="${portrait ? 'portrait' : 'landscape'}" fitToWidth="1" fitToHeight="1"/></worksheet>`;
}

function xlsxTypes(count: number): string {
	const sheets = Array.from(
		{ length: count },
		(_, index) =>
			`<Override PartName="/xl/worksheets/sheet${index + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>`
	).join('');
	return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>${sheets}<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>`;
}

const XLSX_RELS = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>`;

function xlsxBookRels(count: number): string {
	const sheets = Array.from(
		{ length: count },
		(_, index) =>
			`<Relationship Id="rId${index + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet${index + 1}.xml"/>`
	).join('');
	return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">${sheets}<Relationship Id="rId${count + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>`;
}

function xlsxBook(count: number): string {
	const sheets = Array.from(
		{ length: count },
		(_, index) => `<sheet name="Seite ${index + 1}" sheetId="${index + 1}" r:id="rId${index + 1}"/>`
	).join('');
	return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>${sheets}</sheets></workbook>`;
}

/** CRC-32 of the zip entries — the only checksum a stored archive needs. */
const CRC_TABLE = (() => {
	const table = new Uint32Array(256);
	for (let i = 0; i < 256; i += 1) {
		let value = i;
		for (let bit = 0; bit < 8; bit += 1) {
			value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
		}
		table[i] = value >>> 0;
	}
	return table;
})();

function crc32(data: Uint8Array): number {
	let value = 0xffffffff;
	for (const byte of data) value = CRC_TABLE[(value ^ byte) & 0xff] ^ (value >>> 8);
	return (value ^ 0xffffffff) >>> 0;
}

/** A zip of stored entries: an XLSX is a zip, and packing it needs no library. */
function zip(files: { path: string; data: Uint8Array }[]): Uint8Array {
	const encoder = new TextEncoder();
	const local: Uint8Array[] = [];
	const central: Uint8Array[] = [];
	let archiveOffset = 0;
	for (const file of files) {
		const name = encoder.encode(file.path);
		const crc = crc32(file.data);
		const head = new Uint8Array(30 + name.length);
		const view = new DataView(head.buffer);
		view.setUint32(0, 0x04034b50, true);
		view.setUint16(4, 20, true);
		view.setUint16(6, 0x0800, true);
		view.setUint16(8, 0, true);
		view.setUint16(10, 0, true);
		view.setUint16(12, 0, true);
		view.setUint32(14, crc, true);
		view.setUint32(18, file.data.length, true);
		view.setUint32(22, file.data.length, true);
		view.setUint16(26, name.length, true);
		head.set(name, 30);
		local.push(head, file.data);

		const entry = new Uint8Array(46 + name.length);
		const entryView = new DataView(entry.buffer);
		entryView.setUint32(0, 0x02014b50, true);
		entryView.setUint16(4, 20, true);
		entryView.setUint16(6, 20, true);
		entryView.setUint16(8, 0x0800, true);
		entryView.setUint16(10, 0, true);
		entryView.setUint16(12, 0, true);
		entryView.setUint16(14, 0, true);
		entryView.setUint32(16, crc, true);
		entryView.setUint32(20, file.data.length, true);
		entryView.setUint32(24, file.data.length, true);
		entryView.setUint16(28, name.length, true);
		entryView.setUint16(30, 0, true);
		entryView.setUint16(32, 0, true);
		entryView.setUint16(34, 0, true);
		entryView.setUint16(36, 0, true);
		entryView.setUint32(38, 0, true);
		entryView.setUint32(42, archiveOffset, true);
		entry.set(name, 46);
		central.push(entry);

		archiveOffset += head.length + file.data.length;
	}
	const centerSize = central.reduce((total, part) => total + part.length, 0);
	const end = new Uint8Array(22);
	const visibleEnd = new DataView(end.buffer);
	visibleEnd.setUint32(0, 0x06054b50, true);
	visibleEnd.setUint16(8, files.length, true);
	visibleEnd.setUint16(10, files.length, true);
	visibleEnd.setUint32(12, centerSize, true);
	visibleEnd.setUint32(16, archiveOffset, true);
	const sizeTotal = archiveOffset + centerSize + end.length;
	const result = new Uint8Array(sizeTotal);
	let offset = 0;
	for (const part of [...local, ...central, end]) {
		result.set(part, offset);
		offset += part.length;
	}
	return result;
}

/** The plan as an XLSX workbook — one sheet, the same table as the CSV. */
export function toXlsx(report: Report): Uint8Array {
	const pages = sheets(report);
	const styleBook = new StyleBook();
	const files = pages.map((sheet, index) => xlsxSheet(sheet, report, styleBook, index === 0));
	const encoder = new TextEncoder();
	return zip([
		{ path: '[Content_Types].xml', data: encoder.encode(xlsxTypes(pages.length)) },
		{ path: '_rels/.rels', data: encoder.encode(XLSX_RELS) },
		{ path: 'xl/workbook.xml', data: encoder.encode(xlsxBook(pages.length)) },
		{ path: 'xl/_rels/workbook.xml.rels', data: encoder.encode(xlsxBookRels(pages.length)) },
		{ path: 'xl/styles.xml', data: encoder.encode(styleBook.xml()) },
		...files.map((content, index) => ({
			path: `xl/worksheets/sheet${index + 1}.xml`,
			data: encoder.encode(content)
		}))
	]);
}

// ---------------------------------------------------------------- Tables

/**
 * A table of the exchange as text: the head once, then one line per row. Semicolon
 * and decimal comma are what a German Excel expects, and the BOM keeps the umlauts.
 */
export function tableCsv(head: string[], rows: string[][]): string {
	const alle = [head, ...rows].map((row) => row.map((value) => csvField(value)).join(';'));
	return `\ufeff${alle.join('\r\n')}\r\n`;
}

/**
 * A table of the exchange as a workbook with one sheet: the head bold on a light
 * ground, one line per row. Everything stays text — dates, times and names are no
 * numbers, and a reader that shows `16.11.2026` must get it back unchanged.
 */
export function tableXlsx(head: string[], rows: string[][], sheetName: string): Uint8Array {
	const styleBook = new StyleBook();
	const headStyle = styleBook.cellFormat({
		runs: [{ label: '', bold: true }],
		fill: HEAD_HEX,
		frame: 'cell',
		wrap: false
	});
	const textStyle = styleBook.cellFormat({ runs: [{ label: '' }], frame: 'cell', wrap: false });
	const widths = head.map((_, index) =>
		Math.min(
			60,
			Math.max(8, ...[head, ...rows].map((row) => (row[index] ?? '').length + 2))
		)
	);
	const columnsXml = widths
		.map(
			(char, index) =>
				`<col min="${index + 1}" max="${index + 1}" width="${char.toFixed(2)}" customWidth="1"/>`
		)
		.join('');
	const rowsXml = [head, ...rows]
		.map((row, number) => {
			const style = number === 0 ? headStyle : textStyle;
			const cells = row
				.map((value, index) =>
					value === ''
						? `<c r="${column(index)}${number + 1}" s="${style}"/>`
						: `<c r="${column(index)}${number + 1}" s="${style}" t="inlineStr"><is><t xml:space="preserve">${xmlText(value)}</t></is></c>`
				)
				.join('');
			return `<row r="${number + 1}">${cells}</row>`;
		})
		.join('');
	const sheet = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView tabSelected="1" workbookViewId="0"/></sheetViews><sheetFormatPr defaultRowHeight="14"/><cols>${columnsXml}</cols><sheetData>${rowsXml}</sheetData></worksheet>`;
	const encoder = new TextEncoder();
	return zip([
		{ path: '[Content_Types].xml', data: encoder.encode(xlsxTypes(1)) },
		{ path: '_rels/.rels', data: encoder.encode(XLSX_RELS) },
		{
			path: 'xl/workbook.xml',
			data: encoder.encode(
				`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="${xmlText(sheetName)}" sheetId="1" r:id="rId1"/></sheets></workbook>`
			)
		},
		{ path: 'xl/_rels/workbook.xml.rels', data: encoder.encode(xlsxBookRels(1)) },
		{ path: 'xl/styles.xml', data: encoder.encode(styleBook.xml()) },
		{ path: 'xl/worksheets/sheet1.xml', data: encoder.encode(sheet) }
	]);
}

// ---------------------------------------------------------------- PDF

/**
 * A string of a PDF text operator: the standard fonts take WinAnsi, so every
 * character above ASCII is written as an octal byte — the typographic ones at the
 * place WinAnsi gives them, the rest as their Latin-1 code.
 */
const WIN_ANSI: Record<string, number> = {
	'€': 0x80,
	'‚': 0x82,
	'ƒ': 0x83,
	'„': 0x84,
	'…': 0x85,
	'†': 0x86,
	'‡': 0x87,
	'ˆ': 0x88,
	'‰': 0x89,
	'Š': 0x8a,
	'‹': 0x8b,
	'Œ': 0x8c,
	'‘': 0x91,
	'’': 0x92,
	'“': 0x93,
	'”': 0x94,
	'•': 0x95,
	'–': 0x96,
	'—': 0x97,
	'˜': 0x98,
	'™': 0x99,
	'š': 0x9a,
	'›': 0x9b,
	'œ': 0x9c,
	'Ÿ': 0x9f
};

function pdfText(value: string): string {
	let label = '';
	for (const char of value) {
		const code = WIN_ANSI[char] ?? char.codePointAt(0) ?? 0x20;
		if (char === '(' || char === ')' || char === '\\') {
			label += `\\${char}`;
		} else if (code < 0x20) {
			label += ' ';
		} else if (code < 0x80) {
			label += char;
		} else if (code <= 0xff) {
			label += `\\${code.toString(8).padStart(3, '0')}`;
		} else {
			label += '?';
		}
	}
	return label;
}

/** The widths of Helvetica in thousandths of the font size, per character. */
const WIDTHS: Record<string, number> = (() => {
	const table: Record<string, number> = {
		' ': 278, '!': 278, '"': 355, '#': 556, $: 556, '%': 889, '&': 667, "'": 191,
		'(': 333, ')': 333, '*': 389, '+': 584, ',': 278, '-': 333, '.': 278, '/': 278,
		':': 278, ';': 278, '<': 584, '=': 584, '>': 584, '?': 556, '@': 1015,
		'A': 667, B: 667, C: 722, D: 722, E: 667, F: 611, G: 778, H: 722, I: 278,
		J: 500, K: 667, L: 556, M: 833, N: 722, O: 778, P: 667, Q: 778, R: 722,
		S: 667, T: 611, U: 722, V: 667, W: 944, X: 667, Y: 667, Z: 611,
		'[': 278, '\\': 278, ']': 278, '^': 469, _: 556, '`': 333,
		a: 556, b: 556, c: 500, d: 556, e: 556, f: 278, g: 556, h: 556, i: 222,
		j: 222, k: 500, l: 222, m: 833, n: 556, o: 556, p: 556, q: 556, r: 333,
		s: 500, t: 278, u: 556, v: 500, w: 722, x: 500, y: 500, z: 500,
		'{': 334, '|': 260, '}': 334, '~': 584,
		'ä': 556, 'ö': 556, 'ü': 556, 'Ä': 667, 'Ö': 778, 'Ü': 722, 'ß': 611,
		'–': 556, '—': 1000, '·': 278, '€': 556, '“': 333, '”': 333, '„': 333
	};
	for (let digit = 0; digit <= 9; digit += 1) table[String(digit)] = 556;
	return table;
})();

/** The width of a text in points. */
function textWidth(value: string, size: number): number {
	let total = 0;
	for (const char of value) total += (WIDTHS[char] ?? 556) / 1000;
	return total * size;
}

/** Trims a text to what fits into a box of `breite` points at that font size. */
function truncate(value: string, width: number, size: number): string {
	if (textWidth(value, size) <= width) return value;
	let trimmed = value;
	while (trimmed.length > 1 && textWidth(`${trimmed}-`, size) > width) {
		trimmed = trimmed.slice(0, -1);
	}
	return `${trimmed}-`;
}

interface TextPart {
	label: string;
	bold?: boolean;
}

type Color = [number, number, number];

/** `#3B82F6` → the three channels of the PDF colour operator. */
function toColor(value: string | null | undefined, fallback: Color = [0.4, 0.45, 0.5]): Color {
	const found = /^#?([0-9a-f]{6})$/i.exec((value ?? '').trim());
	if (found === null) return fallback;
	const amount = Number.parseInt(found[1], 16);
	return [(amount >> 16) / 255, ((amount >> 8) & 0xff) / 255, (amount & 0xff) / 255];
}

/** The colour of a cell as the sheet shows it: a fifth of the shift colour on white. */
function lightened(color: Color, share = 0.2): Color {
	return color.map((channel) => 1 - (1 - channel) * share) as Color;
}

function rgb(color: Color): string {
	return `${color[0].toFixed(3)} ${color[1].toFixed(3)} ${color[2].toFixed(3)}`;
}

const GRAY_TEXT: Color = [0.42, 0.45, 0.5];
const SUNDAY: Color = [0.957, 0.957, 0.961];
const HEAD_GRAY: Color = [0.925, 0.937, 0.953];
const RED: Color = [0.541, 0.11, 0.11];

/** The sheets of the report as PDF pages. */
class Pdf {
	private pages: string[] = [];
	private content: string[] = [];
	private height: number;
	private width: number;

	constructor(widthMm: number, heightMm: number) {
		this.width = widthMm * MM;
		this.height = heightMm * MM;
	}

	newPage() {
		if (this.content.length > 0) this.pages.push(this.content.join('\n'));
		this.content = [];
	}

	save(): string[] {
		this.newPage();
		return this.pages;
	}

	label(
		x: number,
		y: number,
		value: string,
		size: number,
		bold = false,
		width?: number,
		color?: Color
	) {
		if (value === '') return;
		const content = width === undefined ? value : truncate(value, width, size);
		const tone = color === undefined ? '' : `${rgb(color)} rg `;
		this.content.push(
			`${tone}BT /${bold ? 'F2' : 'F1'} ${size.toFixed(2)} Tf ${x.toFixed(2)} ${y.toFixed(2)} Td (${pdfText(content)}) Tj ET${color === undefined ? '' : ' 0 g'}`
		);
	}

	/** A text that ends at `rightX` — the hours of a row stand at the same edge. */
	textRight(
		rightX: number,
		y: number,
		value: string,
		size: number,
		bold = false,
		color?: Color
	) {
		if (value === '') return;
		this.label(rightX - textWidth(value, size), y, value, size, bold, undefined, color);
	}

	line(x1: number, y1: number, x2: number, y2: number, weight = 0.4) {
		this.content.push(
			`${weight.toFixed(2)} w ${x1.toFixed(2)} ${y1.toFixed(2)} m ${x2.toFixed(2)} ${y2.toFixed(2)} l S`
		);
	}

	frame(x: number, y: number, width: number, height: number) {
		this.content.push(
			`0.4 w ${x.toFixed(2)} ${y.toFixed(2)} ${width.toFixed(2)} ${height.toFixed(2)} re S`
		);
	}

	fill(x: number, y: number, width: number, height: number, gray: number) {
		this.content.push(
			`${gray.toFixed(2)} g ${x.toFixed(2)} ${y.toFixed(2)} ${width.toFixed(2)} ${height.toFixed(2)} re f 0 g`
		);
	}

	/** A filled box in a colour — the tint of a shift cell or a Sunday. */
	rect(x: number, y: number, width: number, height: number, color: Color) {
		this.content.push(
			`${rgb(color)} rg ${x.toFixed(2)} ${y.toFixed(2)} ${width.toFixed(2)} ${height.toFixed(2)} re f 0 g`
		);
	}

	get pageWidth() {
		return this.width;
	}
	get pageHeight() {
		return this.height;
	}
}

/** One cell of a sheet: its lines, drawn from the top of the box. */
function cell(
	pdf: Pdf,
	x: number,
	y: number,
	width: number,
	height: number,
	rows: TextPart[],
	size: number
) {
	let current = y + height - size - 1.2;
	for (const row of rows) {
		pdf.label(x + 1.5, current, row.label, size, row.bold === true, width - 3);
		current -= size * 1.25;
		if (current < y) return;
	}
}

/**
 * The PDF of the report — the sheet the preview shows.
 *
 * A page is one week: the row of the dates, the disposition of every location, an
 * empty row, the row of the dates again and the staff with the hours of the month.
 * Colours, borders, the tint of a shift cell, the bold time of an event and the
 * thick line between two positions are the same as on the printed page; only the
 * colours follow the paper, which carries no grain.
 */
export function toPdf(report: Report): Uint8Array {
	const pdf = new Pdf(report.paper.width, report.paper.height);
	const pages = weeks(report.days);
	const sizeTotal = pages.length;

	pages.forEach((days, number) => {
		pdf.newPage();
		const metrics = sheetMetrics(report, days);
		const width = report.paper.width * MM;
		const height = report.paper.height * MM;
		const margin = metrics.margin * MM;
		const innerWidth = width - 2 * margin;
		const firstColumn = metrics.first * MM;
		const dayWidth = metrics.day * MM;
		const dayIndex = days.map((day) => report.days.findIndex((entry) => entry.iso === day.iso));
		// The hours stand right-aligned inside the first column, under their labels.
		const targetRight = margin + firstColumn - 33 * MM;
		const actualRight = margin + firstColumn - 17 * MM;
		const diffRight = margin + firstColumn - 1.5 * MM;

		const headHeight = metrics.head * MM;
		const dayHeadHeight = metrics.dayHead * MM;
		const dispoHeight = metrics.dispo * MM;
		const gap = metrics.gap * MM;
		const footHeight = metrics.foot * MM;
		// Two lines of eleven points need about nine millimetres — more would only
		// push the last row off the page.
		const rowHeight = metrics.row * MM;
		// Eleven points is what the theatre reads; a full house on small paper falls
		// back to what the rows can carry.
		const size = metrics.font;
		const timeSize = size * 0.8;

		let y = height - margin;
		// Head: the month and the paper it belongs to.
		pdf.label(margin, y - 5 * MM, report.title, 12, true, innerWidth * 0.6);
		pdf.label(
			margin + innerWidth - 60 * MM,
			y - 5 * MM,
			`${days[0].weekday} ${shortDay(days[0])} – ${days[days.length - 1].weekday} ${shortDay(days[days.length - 1])}`,
			8,
			false,
			40 * MM
		);
		pdf.label(margin + innerWidth - 20 * MM, y - 5 * MM, `Stand ${report.asOf}`, 7, false, 20 * MM, GRAY_TEXT);
		y -= headHeight;

		const columns = (yStart: number) => {
			pdf.line(margin + firstColumn, yStart, margin + firstColumn, yStart - dayHeadHeight);
			days.forEach((day, index) => {
				const x = margin + firstColumn + index * dayWidth;
				if (index > 0) pdf.line(x, yStart, x, yStart - dayHeadHeight);
			});
		};

		// The dates, then the disposition of the locations.
		const headRow = (yStart: number) => {
			days.forEach((day, index) => {
				const x = margin + firstColumn + index * dayWidth;
				const tone = day.sunday ? SUNDAY : HEAD_GRAY;
				pdf.rect(x, yStart - dayHeadHeight, dayWidth, dayHeadHeight, tone);
			});
			pdf.rect(margin, yStart - dayHeadHeight, firstColumn, dayHeadHeight, HEAD_GRAY);
			pdf.frame(margin, yStart - dayHeadHeight, innerWidth, dayHeadHeight);
			columns(yStart);
			days.forEach((day, index) => {
				const x = margin + firstColumn + index * dayWidth;
				const date = `${day.weekday} ${shortDay(day)}`;
				if (day.holiday === null) {
					pdf.label(x + 1.5, yStart - dayHeadHeight + 0.62 * dayHeadHeight, date, size * 0.95, true, dayWidth - 3);
				} else {
					pdf.label(x + 1.5, yStart - size * 0.95 - 2, date, size * 0.95, true, dayWidth - 3);
					pdf.label(x + 1.5, yStart - dayHeadHeight + 2.4, day.holiday, size * 0.78, false, dayWidth - 3, RED);
				}
			});
		};
		headRow(y);
		y -= dayHeadHeight;

		// The disposition: one line per event, the time bold in front of the title.
		// A day with five events needs five lines, so every row is as tall as the
		// busiest day of its location.
		pdf.frame(margin, y - dispoHeight, innerWidth, dispoHeight);
		const dispoFont = metrics.dispoFont;
		const rowsHeight = dispoFont * DISPO_LINE;
		report.dispo.forEach((row, index) => {
			const top = y - metrics.dispoRows.slice(0, index).reduce((total, value) => total + value, 0) * MM;
			const bottom = top - metrics.dispoRows[index] * MM;
			const firstLine = top - 0.8 * MM - rowsHeight * 0.8;
			if (index > 0) pdf.line(margin, top, margin + innerWidth, top);
			pdf.line(margin + firstColumn, top, margin + firstColumn, bottom);
			pdf.label(margin + 1.5, firstLine, row.location, dispoFont, true, firstColumn - 3);
			days.forEach((day, column) => {
				const x = margin + firstColumn + column * dayWidth;
				if (column > 0) pdf.line(x, top, x, bottom);
				const labels = metrics.dispoCells[index]?.[column] ?? [];
				labels.forEach((label, offset) => {
					const baseline = firstLine - offset * rowsHeight;
					let current = x + 1.5 + (label.wrapped === true ? DISPO_INDENT * MM : 0);
					// The time of the event stands bold, the title in the plain font.
					if (label.time !== '') {
						pdf.label(current, baseline, label.time, dispoFont, true);
						current += textWidth(label.time, dispoFont) + 1.5;
					}
					const room = x + dayWidth - 1.5 - current;
					if (room > 4) pdf.label(current, baseline, label.title, dispoFont, false, room);
				});
			});
		});
		y -= dispoHeight;

		// The empty row between the disposition and the staff.
		pdf.frame(margin, y - gap, innerWidth, gap);
		columns(y);
		y -= gap;

		// The dates again, then the staff with its hours.
		headRow(y);
		pdf.textRight(targetRight, y - dayHeadHeight + 0.3 * dayHeadHeight, 'Soll', size * 0.95, true);
		pdf.textRight(actualRight, y - dayHeadHeight + 0.3 * dayHeadHeight, 'Ist', size * 0.95, true);
		pdf.textRight(diffRight, y - dayHeadHeight + 0.3 * dayHeadHeight, '+/-', size * 0.95, true);
		y -= dayHeadHeight;

		let previousPosition: string | null = null;
		for (const row of report.staff) {
			const top = y;
			// The tint of the row: the sub-department (or the department) of the employee,
			// as the board shows it.
			if (row.color !== '') {
				pdf.rect(margin, top - rowHeight, firstColumn, rowHeight, lightened(toColor(row.color), 0.3));
			}
			// The tint of the day cells, then the grid over it.
			days.forEach((day, column) => {
				const x = margin + firstColumn + column * dayWidth;
				const index = dayIndex[column];
				const color = row.colors?.[index] ?? '';
				if (color !== '') {
					pdf.rect(x, top - rowHeight, dayWidth, rowHeight, lightened(toColor(color)));
				} else if (row.days[index] === '' && day.sunday) {
					pdf.rect(x, top - rowHeight, dayWidth, rowHeight, SUNDAY);
				}
			});
			pdf.frame(margin, top - rowHeight, innerWidth, rowHeight);
			pdf.line(margin + firstColumn, top, margin + firstColumn, top - rowHeight);
			// A thicker line where the position changes: `Meister`, `Stellwerker` and
			// `Oberbeleuchter` each keep their own block.
			if (previousPosition !== null && previousPosition !== row.position) {
				pdf.line(margin, top, margin + innerWidth, top, 1.6);
			}
			previousPosition = row.position;

			const firstLine = top - size - 0.6;
			pdf.label(margin + 1.5, firstLine, row.name, size, true, firstColumn - 34 * MM);
			pdf.textRight(targetRight, firstLine, hoursText(row.target) || '–', size);
			pdf.textRight(actualRight, firstLine, hoursText(row.actual), size);
			pdf.textRight(diffRight, firstLine, hoursText(row.difference, true) || '–', size);
			const second = staffLineText(row);
			// Below the hours the whole first column is free — the line uses all of it,
			// so position, department and sub-department fit.
			pdf.label(margin + 1.5, firstLine - size * 1.1, second, size * SECOND_LINE_FACTOR, false, firstColumn - 3 * MM, GRAY_TEXT);

			days.forEach((day, column) => {
				const x = margin + firstColumn + column * dayWidth;
				if (column > 0) pdf.line(x, top, x, top - rowHeight);
				const index = dayIndex[column];
				const label = row.days[index] ?? '';
				if (label === '') return;
				const center = x + dayWidth / 2;
				const free = label === 'frei';
				const tone = free ? GRAY_TEXT : undefined;
				// Short name and time share the first line, the note takes the second.
				const time = row.times?.[index] ?? '';
				const caption = truncate(time === '' ? label : `${label} - ${time}`, dayWidth - 2, size);
				pdf.label(
					center - textWidth(caption, size) / 2,
					firstLine,
					caption,
					size,
					false,
					undefined,
					tone
				);
				const note = row.notes?.[index] ?? '';
				if (note !== '' && rowHeight / MM >= 8) {
					const row2 = truncate(note, dayWidth - 2, timeSize);
					pdf.label(
						center - textWidth(row2, timeSize) / 2,
						firstLine - size * 1.1,
						row2,
						timeSize,
						false,
						undefined,
						GRAY_TEXT
					);
				}
			});
			y -= rowHeight;
		}

		// Footer: the legend, then the page of the month.
		const foot = margin + footHeight / 2;
		pdf.label(margin, foot - 2, report.legend, 6, false, innerWidth - 40 * MM, GRAY_TEXT);
		pdf.label(margin + innerWidth - 30 * MM, foot - 2, `Seite ${number + 1} von ${sizeTotal}`, 6, false, 30 * MM, GRAY_TEXT);
	});

	return pdfFile(pdf, report);
}
/** The file itself: catalogue, pages, fonts and the table of the objects. */
function pdfFile(pdf: Pdf, report: Report): Uint8Array {
	const pages = pdf.save();
	const parts: string[] = [];
	const offsets: number[] = [];
	parts.push('%PDF-1.4\n');
	const pushObject = (content: string) => {
		offsets.push(parts.join('').length);
		parts.push(content);
	};
	const pageCount = pages.length;
	const pageIds: number[] = [];
	for (let index = 0; index < pageCount; index += 1) pageIds.push(4 + index * 2);
	pushObject('1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n');
	pushObject(
		`2 0 obj\n<< /Type /Pages /Kids [${pageIds.map((id) => `${id} 0 R`).join(' ')}] /Count ${pageCount} >>\nendobj\n`
	);
	pushObject(
		'3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n'
	);
	let objectNumber = 4;
	pages.forEach((content) => {
		const pageId = objectNumber;
		const contentId = objectNumber + 1;
		pushObject(
			`${pageId} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${pdf.pageWidth.toFixed(2)} ${pdf.pageHeight.toFixed(2)}] /Resources << /Font << /F1 3 0 R /F2 ${3 + 2 * pageCount + 1} 0 R >> >> /Contents ${contentId} 0 R >>\nendobj\n`
		);
		const pdfStream = `${content}\n`;
		pushObject(`${contentId} 0 obj\n<< /Length ${pdfStream.length} >>\nstream\n${pdfStream}endstream\nendobj\n`);
		objectNumber += 2;
	});
	const boldId = 3 + 2 * pageCount + 1;
	pushObject(
		`${boldId} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>\nendobj\n`
	);
	objectNumber = boldId + 1;
	pushObject(
		`${objectNumber} 0 obj\n<< /Title (${pdfText(report.title)}) /Producer (WHERE) >>\nendobj\n`
	);

	const body = parts.join('');
	const rows: string[] = [];
	const sizeTotal = body.length;
	for (let index = 0; index < offsets.length; index += 1) {
		rows.push(`${String(offsets[index]).padStart(10, '0')} 00000 n \n`);
	}
	const xrefStart = sizeTotal;
	const xref = `xref\n0 ${rows.length + 1}\n0000000000 65535 f \n${rows.join('')}`;
	const trailer = `trailer\n<< /Size ${rows.length + 1} /Root 1 0 R /Info ${objectNumber} 0 R >>\nstartxref\n${xrefStart}\n%%EOF\n`;
	const bytes = new TextEncoder().encode(body + xref + trailer);
	return bytes;
}
