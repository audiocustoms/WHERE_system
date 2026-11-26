/**
 * Date arithmetic for the calendar.
 *
 * All functions work with local time and with `YYYY-MM-DD` texts, so that the
 * time zone of a workstation never pushes an appointment to the previous day.
 */
import { t } from '../i18n.svelte';

/** `2026-11-13` → Date (local midnight). */
export function parseDay(text: string): Date {
	const [year, month, day] = text.split('-').map(Number);
	return new Date(year, (month ?? 1) - 1, day ?? 1);
}

/** Date → `2026-11-13`. */
export function isoDay(date: Date): string {
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${date.getFullYear()}-${month}-${day}`;
}

export function addDays(date: Date, days: number): Date {
	const copy = new Date(date);
	copy.setDate(copy.getDate() + days);
	return copy;
}

export function addMonths(date: Date, months: number): Date {
	return new Date(date.getFullYear(), date.getMonth() + months, 1);
}

export function startOfMonth(date: Date): Date {
	return new Date(date.getFullYear(), date.getMonth(), 1);
}

export function endOfMonth(date: Date): Date {
	return new Date(date.getFullYear(), date.getMonth() + 1, 0);
}

export function startOfYear(date: Date): Date {
	return new Date(date.getFullYear(), 0, 1);
}

export function endOfYear(date: Date): Date {
	return new Date(date.getFullYear(), 11, 31);
}

export function addYears(date: Date, years: number): Date {
	return new Date(date.getFullYear() + years, date.getMonth(), date.getDate());
}

/** Calendar week per ISO 8601 (the week belongs to the year the Thursday falls in). */
export function isoWeek(date: Date): number {
	const thursday = startOfWeek(date);
	thursday.setDate(thursday.getDate() + 3);
	const firstThursday = new Date(thursday.getFullYear(), 0, 4);
	firstThursday.setDate(firstThursday.getDate() - ((firstThursday.getDay() + 6) % 7) + 3);
	const days = Math.round((thursday.getTime() - firstThursday.getTime()) / 86400000);
	return 1 + Math.round(days / 7);
}

/** Monday of the week `date` falls in. */
export function startOfWeek(date: Date): Date {
	const weekday = (date.getDay() + 6) % 7;
	return addDays(date, -weekday);
}

export function sameDay(a: Date, b: Date): boolean {
	return (
		a.getFullYear() === b.getFullYear() &&
		a.getMonth() === b.getMonth() &&
		a.getDate() === b.getDate()
	);
}

export function isWeekend(date: Date): boolean {
	const weekday = date.getDay();
	return weekday === 0 || weekday === 6;
}

export const WEEKDAYS = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

const MONTHS = [
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

export function monthLabel(date: Date): string {
	return `${t(MONTHS[date.getMonth()])} ${date.getFullYear()}`;
}

/** `Nov` */
export function monthShort(date: Date): string {
	return t(MONTHS[date.getMonth()]).slice(0, 3);
}

/** `13. Nov` — this is how the calendar writes the days in the week header. */
export function dayMonth(date: Date): string {
	return `${date.getDate()}. ${monthShort(date)}`;
}

/** `00:00` as minutes since midnight. */
export function minutesOfDay(time: string | null): number {
	if (!time) return 0;
	const [hours, minutes] = time.split(':').map(Number);
	return (hours || 0) * 60 + (minutes || 0);
}

/** `10.11.2026` */
export function shortDate(text: string): string {
	const date = parseDay(text);
	return `${String(date.getDate()).padStart(2, '0')}.${String(date.getMonth() + 1).padStart(2, '0')}.${date.getFullYear()}`;
}

/** `Mo, 10.11.` */
export function dayLabel(date: Date): string {
	const weekday = t(WEEKDAYS[(date.getDay() + 6) % 7]);
	return `${weekday}, ${String(date.getDate()).padStart(2, '0')}.${String(date.getMonth() + 1).padStart(2, '0')}.`;
}

/** `09:00 – 17:00`, empty for all-day events. */
export function timeRange(start: string | null, end: string | null): string {
	const from = start?.slice(0, 5) ?? '';
	const to = end?.slice(0, 5) ?? '';
	if (!from && !to) return '';
	return to ? `${from} – ${to}` : from;
}
