/**
 * Kurze Rückmeldung am unteren Rand — für Sätze, die der Anwender sehen muss,
 * weil sein Klick sonst ins Leere ginge („Kein Eingabefeld aktiv.").
 *
 * Sie verschwindet von selbst; ein neuer Satz ersetzt den alten.
 */

let text = $state<string | null>(null);
let timer: ReturnType<typeof setTimeout> | null = null;

export const statusMessage = () => text;

export function showStatus(message: string, milliseconds = 2800): void {
	text = message;
	if (timer) clearTimeout(timer);
	timer = setTimeout(() => (text = null), milliseconds);
}

export function clearStatus(): void {
	if (timer) clearTimeout(timer);
	text = null;
}
