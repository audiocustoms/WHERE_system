/**
 * The language of the interface.
 *
 * German is the source language: every visible text in the code is written in
 * German and `t()` turns it into English when the interface runs in English. So
 * `t('Abbrechen')` needs no invented key, a text that has no translation yet
 * stays readable (it falls back to German), and the German interface does not
 * depend on a dictionary at all.
 *
 * The choice lives in `localStorage` — it is a property of this browser, like
 * the design switch, and it has to be known before the first paint.
 */
import { en } from './i18n/en';

export type Locale = 'de' | 'en';

const STORAGE_KEY = 'where.locale';

/** German when the system speaks German, English for everything else. */
function systemLocale(): Locale {
	if (typeof navigator === 'undefined') return 'de';
	const languages = navigator.languages?.length ? navigator.languages : [navigator.language];
	return languages.some((tag) => tag.toLowerCase().startsWith('de')) ? 'de' : 'en';
}

function storedLocale(): Locale | null {
	if (typeof localStorage === 'undefined') return null;
	const value = localStorage.getItem(STORAGE_KEY);
	return value === 'de' || value === 'en' ? value : null;
}

let current = $state<Locale>(storedLocale() ?? systemLocale());

export const LANGUAGES: { value: Locale; text: string }[] = [
	{ value: 'de', text: 'Deutsch' },
	{ value: 'en', text: 'English' }
];

/** The language the interface runs in. */
export function locale(): Locale {
	return current;
}

/** Whether the user picked a language herself (otherwise the system decides). */
export function hasChosenLocale(): boolean {
	return storedLocale() !== null;
}

export function setLocale(next: Locale): void {
	current = next;
	if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, next);
}

/** `{name}` in a translation is replaced by `params.name`. */
export function t(
	text: string,
	params?: Record<string, string | number>
): string {
	const entry = current === 'en' ? en[text] : undefined;
	let translated = text;
	if (typeof entry === 'string') translated = entry;
	else if (entry) translated = Number(params?.n) === 1 ? entry.one : entry.other;
	if (!params) return translated;
	return translated.replace(/\{(\w+)\}/g, (whole, name: string) =>
		Object.hasOwn(params, name) ? String(params[name]) : whole
	);
}
