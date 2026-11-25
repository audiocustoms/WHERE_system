/**
 * Colour scheme (light/dark). The initial value is set in `app.html` before the
 * application renders; here it is only toggled and stored.
 */
const STORAGE_KEY = 'where-theme';

let dark = $state(initial());

function initial(): boolean {
	if (typeof document === 'undefined') return false;
	return document.documentElement.dataset.theme === 'dark';
}

export const isDark = () => dark;

export function setDark(next: boolean) {
	dark = next;
	if (typeof document !== 'undefined') {
		document.documentElement.dataset.theme = next ? 'dark' : 'light';
	}
	try {
		localStorage.setItem(STORAGE_KEY, next ? 'dark' : 'light');
	} catch {
		// Without localStorage the scheme applies to this session only.
	}
}

export const toggleTheme = () => setDark(!dark);
