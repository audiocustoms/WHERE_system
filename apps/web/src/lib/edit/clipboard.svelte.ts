/**
 * Fenster „Zwischenablage anzeigen" (`Bearbeiten`).
 *
 * Es zeigt, was gerade in der Zwischenablage liegt, und setzt den Text auf Wunsch
 * in das zuletzt benutzte Feld ein. Der Browser gibt den Inhalt nur nach einer
 * Berechtigung heraus; verweigert er sie, sagt das Fenster das.
 */

let open = $state(false);

export const clipboardOpen = () => open;

export function showClipboard(): void {
	open = true;
}

export function hideClipboard(): void {
	open = false;
}
