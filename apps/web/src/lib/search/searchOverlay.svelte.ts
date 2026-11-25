/**
 * State of the search window. It lies above the current page — the search has
 * no address of its own and is reachable from everywhere.
 *
 * `start` decides what the window opens with: the area choice
 * (button `Suchen`), the address search (button `Adressen`) or the
 * article search (button `Artikel`).
 */
export type SearchStart = 'bereich' | 'addresses' | 'articles';

let isOpen = $state(false);
let start = $state<SearchStart>('bereich');

export const searchOpen = () => isOpen;

/** Start point of the last opened window. */
export const searchStart = () => start;

export function openSearch() {
	start = 'bereich';
	isOpen = true;
}

export function openAddressSearch() {
	start = 'addresses';
	isOpen = true;
}

export function openArticleSearch() {
	start = 'articles';
	isOpen = true;
}

export function closeSearch() {
	isOpen = false;
}
