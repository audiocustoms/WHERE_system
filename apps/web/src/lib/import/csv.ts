/**
 * Reading a CSV file for the import manager (`img/import/*.PNG`).
 *
 * Two tasks: turn the bytes into text with the chosen character encoding and
 * split the text into header row and data rows. The browser reads all encodings
 * of the WHATWG list through `TextDecoder`; the character sets of the legacy
 * system that are missing there (DOS, EBCDIC, the Mac variants) as well as UTF-32
 * and UTF-7 come from the tables in `legacy-codepages.ts` and from the small
 * readers further down.
 */

import { t } from '../i18n.svelte';
import { CP037, CP437, MAC_ARABIC, MAC_CE, MAC_GREEK, MAC_HEBREW, MAC_TURKISH } from './legacy-codepages';

export interface Encoding {
	/** Unique key of the list entry. */
	key: string;
	/** The label of the list — it comes from the legacy system. */
	label: string;
	/** Whatever works with `TextDecoder` or a key of the own readers. */
	decoding: string;
	/** Addition below the selection when the label does not apply literally. */
	note?: string;
}

/**
 * The encodings of the selection list, in the order of the template.
 *
 * `windows-1215` does not exist; meant is the western european Windows code page
 * `windows-1252`. `x-mac-chineseimp` is the Mac version of GB2312 — the browser
 * reads it as `gbk`, the Chinese characters are kept.
 */
export const KODIERUNGEN: Encoding[] = [
	{ key: 'utf-8', label: 'UTF-8', decoding: 'utf-8' },
	{ key: 'macintosh', label: 'macintosh', decoding: 'macintosh' },
	{
		key: 'windows-1215',
		label: 'windows-1215',
		decoding: 'windows-1252',
		note: t('gelesen als windows-1252')
	},
	{ key: 'iso-8859-1', label: 'ISO 8859-1', decoding: 'iso-8859-1' },
	{ key: 'iso-8859-2', label: 'ISO 8859-2', decoding: 'iso-8859-2' },
	{ key: 'iso-8859-10', label: 'ISO 8859-10', decoding: 'iso-8859-10' },
	{ key: 'iso-8859-15', label: 'ISO 8859-15', decoding: 'iso-8859-15' },
	{ key: 'us-ascii', label: 'US-ASCII', decoding: 'us-ascii' },
	{ key: 'utf-32', label: 'UTF-32', decoding: 'utf-32' },
	{ key: 'utf-16', label: 'UTF-16', decoding: 'utf-16' },
	{ key: 'utf-7', label: 'UTF-7', decoding: 'utf-7' },
	{ key: 'ibm437', label: 'IBM437', decoding: 'ibm437' },
	{ key: 'ibm037', label: 'IBM037', decoding: 'ibm037' },
	{ key: 'macce', label: 'MacCE', decoding: 'macce' },
	{ key: 'x-mac-cyrillic', label: 'x-mac-cyrilic', decoding: 'x-mac-cyrillic' },
	{ key: 'x-mac-turkish', label: 'x-mac-turkish', decoding: 'x-mac-turkish' },
	{ key: 'x-mac-arabic', label: 'x-mac-arabic', decoding: 'x-mac-arabic' },
	{ key: 'x-mac-greek', label: 'x-mac-greek', decoding: 'x-mac-greek' },
	{ key: 'x-mac-hebrew', label: 'x-mac-hebrew', decoding: 'x-mac-hebrew' },
	{ key: 'x-mac-ce', label: 'x-mac-ce', decoding: 'x-mac-ce' },
	{
		key: 'x-mac-chineseimp',
		label: 'x-mac-chineseimp',
		decoding: 'x-mac-chineseimp',
		note: t('gelesen als GBK (Mac-Fassung von GB2312)')
	},
	{ key: 'iso-8859-3', label: 'ISO 8859-3', decoding: 'iso-8859-3' },
	{ key: 'iso-8859-4', label: 'ISO 8859-4', decoding: 'iso-8859-4' },
	{ key: 'iso-8859-5', label: 'ISO 8859-5', decoding: 'iso-8859-5' },
	{ key: 'iso-8859-6', label: 'ISO 8859-6', decoding: 'iso-8859-6' },
	{ key: 'iso-8859-7', label: 'ISO 8859-7', decoding: 'iso-8859-7' },
	{ key: 'iso-8859-8', label: 'ISO 8859-8', decoding: 'iso-8859-8' },
	{ key: 'iso-8859-9', label: 'ISO 8859-9', decoding: 'iso-8859-9' },
	{ key: 'iso-8859-13', label: 'ISO 8859-13', decoding: 'iso-8859-13' },
	{ key: 'windows-1250', label: 'windows-1250', decoding: 'windows-1250' },
	{ key: 'windows-1252', label: 'windows-1252', decoding: 'windows-1252' },
	{ key: 'windows-1253', label: 'windows-1253', decoding: 'windows-1253' },
	{ key: 'windows-1254', label: 'windows-1254', decoding: 'windows-1254' },
	{ key: 'windows-1255', label: 'windows-1255', decoding: 'windows-1255' },
	{ key: 'windows-1256', label: 'windows-1256', decoding: 'windows-1256' },
	{ key: 'windows-1257', label: 'windows-1257', decoding: 'windows-1257' },
	{ key: 'windows-31j', label: 'windows-31J', decoding: 'windows-31j' },
	{ key: 'shift_jis', label: 'Shift_JIS', decoding: 'shift_jis' },
	{ key: 'iso-2022-jp', label: 'ISO-2022-JP', decoding: 'iso-2022-jp' },
	{ key: 'big5', label: 'Big5', decoding: 'big5' },
	{ key: 'euc-kr', label: 'EUC-KR', decoding: 'euc-kr' },
	{ key: 'koi8-r', label: 'KOI8-R', decoding: 'koi8-r' },
	{ key: 'gbk', label: 'GBK', decoding: 'gbk' },
	{ key: 'gb_2312-80', label: 'GB_2312-80', decoding: 'gb_2312-80' }
];

/** Character sets whose table ships along (key → 256 characters). */
const TABLES: Record<string, string> = {
	ibm437: CP437,
	ibm037: CP037,
	macce: MAC_CE,
	'x-mac-ce': MAC_CE,
	'x-mac-turkish': MAC_TURKISH,
	'x-mac-arabic': MAC_ARABIC,
	'x-mac-greek': MAC_GREEK,
	'x-mac-hebrew': MAC_HEBREW
};

/** An encoding read without `TextDecoder`. */
const OWN: Record<string, (bytes: Uint8Array) => string> = {
	'utf-32': utf32,
	'utf-16': utf16,
	'utf-16le': (bytes) => utf16(bytes, 'le'),
	'utf-16be': (bytes) => utf16(bytes, 'be'),
	'utf-7': utf7,
	'x-mac-chineseimp': (bytes) => new TextDecoder('gbk').decode(bytes)
};

/** Turns the bytes of the file into text. */
export function dekodiere(bytes: Uint8Array, encoding: string): string {
	const own = OWN[encoding];
	if (own) return own(bytes);
	const table = TABLES[encoding];
	if (table) return einByte(bytes, table);
	try {
		return new TextDecoder(encoding).decode(bytes);
	} catch {
		// An unknown encoding must not stop the import — UTF-8 is the default of
		// the mask.
		return new TextDecoder('utf-8').decode(bytes);
	}
}

/** Every byte through a table — the character sets with one byte per character. */
function einByte(bytes: Uint8Array, table: string): string {
	let text = '';
	for (const byte of bytes) text += table[byte] ?? '\uFFFD';
	return text;
}

/** UTF-16 with the byte order detected at the BOM; little endian without a BOM. */
function utf16(bytes: Uint8Array, order?: 'le' | 'be'): string {
	const detection =
		order ??
		(bytes[0] === 0xff && bytes[1] === 0xfe
			? 'le'
			: bytes[0] === 0xfe && bytes[1] === 0xff
				? 'be'
				: 'le');
	return new TextDecoder(detection === 'le' ? 'utf-16le' : 'utf-16be').decode(bytes);
}

/** UTF-32, byte order detected at the BOM; little endian without a BOM. */
function utf32(bytes: Uint8Array): string {
	let start = 0;
	let gross = false;
	if (bytes.length >= 4) {
		if (bytes[0] === 0xff && bytes[1] === 0xfe && bytes[2] === 0x00 && bytes[3] === 0x00) {
			start = 4;
		} else if (bytes[0] === 0x00 && bytes[1] === 0x00 && bytes[2] === 0xfe && bytes[3] === 0xff) {
			start = 4;
			gross = true;
		}
	}
	let text = '';
	for (let i = start; i + 3 < bytes.length; i += 4) {
		const value = gross
			? ((bytes[i] << 24) | (bytes[i + 1] << 16) | (bytes[i + 2] << 8) | bytes[i + 3]) >>> 0
			: ((bytes[i + 3] << 24) | (bytes[i + 2] << 16) | (bytes[i + 1] << 8) | bytes[i]) >>> 0;
		text += value <= 0x10ffff ? String.fromCodePoint(value) : '\uFFFD';
	}
	return text;
}

/** The alphabet of the modified Base64 encoding of UTF-7. */
const UTF7_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

/** UTF-7: plain characters literally, `+…-` as modified Base64 with UTF-16 units. */
function utf7(bytes: Uint8Array): string {
	// UTF-7 is seven bits wide; a byte above 0x7F counts as a Latin-1 character.
	const text = einByte(bytes, String.fromCharCode(...Array.from({ length: 128 }, (_, i) => i)));
	let result = '';
	let i = 0;
	while (i < text.length) {
		const character = text[i];
		if (character !== '+') {
			result += character;
			i += 1;
			continue;
		}
		i += 1;
		if (text[i] === '-') {
			result += '+';
			i += 1;
			continue;
		}
		let bits = 0;
		let count = 0;
		while (i < text.length) {
			const offset = UTF7_ALPHABET.indexOf(text[i]);
			if (offset < 0) {
				if (text[i] === '-') i += 1;
				break;
			}
			bits = (bits << 6) | offset;
			count += 6;
			i += 1;
			while (count >= 16) {
				count -= 16;
				result += String.fromCharCode((bits >> count) & 0xffff);
			}
		}
	}
	return result;
}

/** The separators the mask offers, together with their label. */
export const SEPARATORS: { value: string; label: string }[] = [
	{ value: 'auto', label: t('Automatisch') },
	{ value: '\t', label: t('Tabulator') },
	{ value: ';', label: t('Semikolon') },
	{ value: ',', label: t('Komma') },
	{ value: '|', label: t('Senkrechter Strich') }
];

/**
 * Guesses the separator from the first row: the character that occurs most often
 * outside of quotation marks. When several candidates have the same count, the
 * order of the mask decides (semicolon before comma).
 */
export function detectSeparator(text: string): string {
	const candidates = [';', '\t', ',', '|'];
	const counter = new Map<string, number>(candidates.map((k) => [k, 0]));
	let inAnfuehrung = false;
	for (let i = 0; i < text.length; i += 1) {
		const character = text[i];
		if (character === '"') {
			if (inAnfuehrung && text[i + 1] === '"') i += 1;
			else inAnfuehrung = !inAnfuehrung;
			continue;
		}
		if (inAnfuehrung) continue;
		if (character === '\n' || character === '\r') break;
		if (counter.has(character)) counter.set(character, (counter.get(character) ?? 0) + 1);
	}
	let bester = ';';
	for (const candidate of candidates) {
		if ((counter.get(candidate) ?? 0) > (counter.get(bester) ?? 0)) bester = candidate;
	}
	return bester;
}

/**
 * Splits the text into rows and fields (RFC 4180 with concessions): fields in
 * quotation marks may contain the separator and line breaks, `""` stands for a
 * quotation mark. `\r\n`, `\n` and a single `\r` end a row. Completely empty rows
 * are dropped.
 */
export function liesCsv(text: string, separator: string): string[][] {
	const rows: string[][] = [];
	let row: string[] = [];
	let field = '';
	let inAnfuehrung = false;

	const endRow = () => {
		row.push(field);
		field = '';
		if (row.some((value) => value.trim() !== '')) rows.push(row);
		row = [];
	};

	for (let i = 0; i < text.length; i += 1) {
		const character = text[i];
		if (inAnfuehrung) {
			if (character === '"') {
				if (text[i + 1] === '"') {
					field += '"';
					i += 1;
				} else inAnfuehrung = false;
			} else field += character;
			continue;
		}
		if (character === '"') {
			inAnfuehrung = true;
			continue;
		}
		if (character === separator) {
			row.push(field);
			field = '';
			continue;
		}
		if (character === '\r') {
			if (text[i + 1] === '\n') i += 1;
			endRow();
			continue;
		}
		if (character === '\n') {
			endRow();
			continue;
		}
		field += character;
	}
	if (field !== '' || row.length) endRow();
	return rows;
}
