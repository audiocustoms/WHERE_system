/**
 * Conversion of the cell contents of a CSV file into the values the API expects.
 *
 * The mask knows a type per field (`T` text, `N` whole number, `Z` decimal,
 * `B` yes/no, `D` date). Here the text of the cell is mapped onto it — with the
 * spellings that come from the legacy system: `1.234,56` as a price,
 * `15.09.2024 20:46:52` as a date, `True`/`Ja`/`1` as a check mark.
 */

import { t } from '../i18n.svelte';
import type { FieldType } from './importFields';

/** Result of a conversion: the value or the reason why it does not fit. */
export type Conversion = { ok: true; value: unknown } | { ok: false; reason: string };

/** Empty cells stay empty — they do not overwrite a default of the mask. */
export function isEmpty(text: string | undefined): boolean {
	return (text ?? '').trim() === '';
}

export function convert(kind: FieldType, text: string): Conversion {
	switch (kind) {
		case 'T':
			return { ok: true, value: text.trim() };
		case 'N': {
			const value = count(text);
			if (value === null)
				return { ok: false, reason: t('„{value}“ ist keine ganze Zahl', { value: text.trim() }) };
			if (!Number.isInteger(value))
				return { ok: false, reason: t('„{value}“ ist keine ganze Zahl', { value: text.trim() }) };
			return { ok: true, value };
		}
		case 'Z': {
			const value = count(text);
			if (value === null)
				return { ok: false, reason: t('„{value}“ ist keine Zahl', { value: text.trim() }) };
			// The API takes decimals as text with a dot — that keeps the precision.
			return { ok: true, value: String(value) };
		}
		case 'B': {
			const value = checked(text);
			if (value === null)
				return { ok: false, reason: t('„{value}“ ist kein Ja/Nein', { value: text.trim() }) };
			return { ok: true, value };
		}
		case 'D': {
			const value = date(text);
			if (value === null)
				return { ok: false, reason: t('„{value}“ ist kein Datum', { value: text.trim() }) };
			return { ok: true, value };
		}
	}
}

/**
 * Reads a number in German or English notation.
 *
 * When a dot and a comma both occur, the later character is the decimal separator
 * (`1.234,56` and `1,234.56`). With only a comma it is the decimal character
 * (`1,5`); with only a dot, the dot counts as decimal character (`1.5`) — a
 * thousands dot without decimals (`1.234`) is the rarer reading in an export
 * file. Currency signs and spaces are dropped.
 */
export function count(text: string): number | null {
	let value = text.trim().replace(/[^\d,.\-+]/g, '');
	if (value === '' || value === '-' || value === '+') return null;
	const comma = value.lastIndexOf(',');
	const punkt = value.lastIndexOf('.');
	if (comma >= 0 && punkt >= 0) {
		if (comma > punkt) value = value.replace(/\./g, '').replace(/,/g, '.');
		else value = value.replace(/,/g, '');
	} else if (comma >= 0) {
		value = value.replace(/,/g, '.');
	}
	const result = Number(value);
	return Number.isFinite(result) ? result : null;
}

const JA = ['1', 'ja', 'j', 'true', 'wahr', 'x', 'y', 'yes', 'an'];
const NEIN = ['0', 'nein', 'n', 'false', 'falsch', '-', 'no', 'aus'];

/** Reads a check mark: `True`, `Ja`, `1`, `x` count as set. */
export function checked(text: string): boolean | null {
	const value = text.trim().toLowerCase();
	if (value === '') return null;
	if (JA.includes(value)) return true;
	if (NEIN.includes(value)) return false;
	return null;
}

/**
 * Reads a date and returns it as `YYYY-MM-DD` — the form the API requires.
 * Recognised are `2024-09-15`, `15.09.2024`, `15.9.2024`, `15/09/2024` and all
 * three with an appended time.
 */
export function date(text: string): string | null {
	const value = text.trim().split(/[ T]/)[0];
	if (!value) return null;
	let day: number | undefined;
	let month: number | undefined;
	let year: number | undefined;
	const iso = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(value);
	const german = /^(\d{1,2})[./](\d{1,2})[./](\d{4})$/.exec(value);
	if (iso) {
		year = Number(iso[1]);
		month = Number(iso[2]);
		day = Number(iso[3]);
	} else if (german) {
		day = Number(german[1]);
		month = Number(german[2]);
		year = Number(german[3]);
	} else return null;
	if (year === undefined || month === undefined || day === undefined) return null;
	if (month < 1 || month > 12 || day < 1 || day > 31) return null;
	return `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}
