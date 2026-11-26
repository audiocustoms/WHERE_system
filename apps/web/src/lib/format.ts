/**
 * Make numbers from the API readable.
 *
 * `Decimal` fields arrive as strings ("2574.0000"), counts as numbers. Both are
 * unified through `Number`; whatever cannot be turned into a number stays as
 * text instead of disappearing.
 */
import { t } from './i18n.svelte';

/** Quantity or stock with up to four decimals. */
export function quantity(value: string | number | null): string {
	if (value === null) return '—';
	const n = Number(value);
	if (!Number.isFinite(n)) return String(value);
	if (n === 0) return '0';
	return new Intl.NumberFormat('de-DE', { maximumFractionDigits: 4 }).format(n);
}

/** Whole number with thousands separators. */
export function count(value: number): string {
	return new Intl.NumberFormat('de-DE', { maximumFractionDigits: 0 }).format(value);
}

/**
 * A decimal in a field of a mask: `"33.0000"` → `"33"`, `"11.4000"` → `"11,4"`.
 * No thousands separator, so the text can be read back without guessing; text that
 * is no number stays as it is.
 */
export function decimalText(value: string | null): string {
	if (value === null) return '';
	const text = value.trim();
	if (text === '') return '';
	const n = Number(text);
	if (!Number.isFinite(n)) return value;
	return new Intl.NumberFormat('de-DE', {
		useGrouping: false,
		maximumFractionDigits: 4
	}).format(n);
}

/**
 * The way back for the API: `"11,4"` → `"11.4"`, `"1.234,5"` → `"1234.5"`, an empty
 * field → `null`. Text that is no number is passed on unchanged, so the server
 * reports it instead of the mask dropping it silently.
 */
export function decimalValue(text: string | null): string | null {
	const s = (text ?? '').replace(/[\s\u00a0]/g, '');
	if (s === '') return null;
	let normalized = s.includes(',') ? s.replace(/\./g, '').replace(',', '.') : s;
	// A trailing separator while typing (`12,`) is no number to the database.
	if (normalized.endsWith('.')) normalized = normalized.slice(0, -1);
	if (normalized === '' || normalized === '-') return null;
	return Number.isFinite(Number(normalized)) ? normalized : s;
}

/** Point in time as "13.11.2026, 09:41". */
export function moment(value: string | null): string {
	if (!value) return '—';
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return value;
	return new Intl.DateTimeFormat('de-DE', {
		day: '2-digit',
		month: '2-digit',
		year: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	}).format(date);
}

/** "vor 3 Tagen" for short distances, otherwise the date. */
export function seit(value: string | null): string {
	if (!value) return '—';
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return value;
	const days = Math.floor((Date.now() - date.getTime()) / 86_400_000);
	if (days <= 0) return t('heute');
	if (days === 1) return t('gestern');
	if (days < 30) return t('vor {days} Tagen', { days });
	return moment(value);
}
