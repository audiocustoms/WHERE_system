/**
 * Make numbers from the API readable.
 *
 * `Decimal` fields arrive as strings ("2574.0000"), counts as numbers. Both are
 * unified through `Number`; whatever cannot be turned into a number stays as
 * text instead of disappearing.
 */

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
	if (days <= 0) return 'heute';
	if (days === 1) return 'gestern';
	if (days < 30) return `vor ${days} Tagen`;
	return moment(value);
}
