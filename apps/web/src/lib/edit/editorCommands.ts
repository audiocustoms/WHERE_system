/**
 * Die Befehle des Menüs `Bearbeiten`.
 *
 * Sie wirken auf das Eingabefeld, in dem zuletzt geschrieben wurde — ein Klick ins
 * Menü nimmt dem Feld den Fokus, deshalb merkt sich dieses Modul das letzte Feld.
 * Ohne Feld gibt der Befehl `false` zurück; der Aufrufer meldet das kurz.
 */

export type EditorCommand =
	| 'undo'
	| 'redo'
	| 'cut'
	| 'copy'
	| 'paste'
	| 'delete'
	| 'select-all';

type Field = HTMLInputElement | HTMLTextAreaElement;

let lastField: HTMLElement | null = null;
let tracking = false;

/** Einmal beim Start: merkt sich jedes Feld, das den Fokus bekommt. */
export function startEditorTracking(): void {
	if (tracking || typeof window === 'undefined') return;
	tracking = true;
	window.addEventListener(
		'focusin',
		(event) => {
			const field = editable(event.target);
			if (field) lastField = field;
		},
		true
	);
}

function editable(target: unknown): HTMLElement | null {
	if (!(target instanceof HTMLElement)) return null;
	if (target instanceof HTMLInputElement) {
		const withoutText = ['checkbox', 'radio', 'file', 'button', 'submit', 'reset', 'range', 'color'];
		if (withoutText.includes(target.type) || target.disabled || target.readOnly) return null;
		return target;
	}
	if (target instanceof HTMLTextAreaElement) {
		return target.disabled || target.readOnly ? null : target;
	}
	return target.isContentEditable ? target : null;
}

/** Das Feld, auf das ein Befehl wirkt: das fokussierte, sonst das zuletzt benutzte. */
export function activeEditor(): HTMLElement | null {
	if (typeof document === 'undefined') return null;
	const current = editable(document.activeElement);
	if (current) return current;
	return lastField && lastField.isConnected ? lastField : null;
}

/** Ob überhaupt ein Feld bereitsteht (für den Zustand der Menüeinträge). */
export function hasEditor(): boolean {
	return activeEditor() !== null;
}

/**
 * Führt einen Befehl aus. `false` heißt: kein Feld, oder der Browser hat den
 * Zugriff verweigert — der Aufrufer sagt es kurz an.
 */
export async function runEditorCommand(command: EditorCommand): Promise<boolean> {
	const field = activeEditor();
	if (!field) return false;
	field.focus({ preventScroll: true });

	switch (command) {
		case 'undo':
			return document.execCommand('undo');
		case 'redo':
			return document.execCommand('redo');
		case 'cut':
			return document.execCommand('cut');
		case 'copy':
			return document.execCommand('copy');
		case 'select-all':
			if (isTextField(field)) {
				field.select();
				return true;
			}
			return document.execCommand('selectAll');
		case 'delete':
			return deleteForward(field);
		case 'paste':
			return await paste(field);
	}
}

function isTextField(field: HTMLElement): field is Field {
	return field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement;
}

/** Meldet dem Feld die Änderung, damit `bind:value` sie mitbekommt. */
function announce(field: HTMLElement): void {
	field.dispatchEvent(new Event('input', { bubbles: true }));
}

/**
 * `Löschen` ist die Taste `Entf`: die Auswahl fällt weg, ohne Auswahl das Zeichen
 * hinter dem Cursor.
 */
function deleteForward(field: HTMLElement): boolean {
	if (!isTextField(field)) return document.execCommand('delete');

	const start = field.selectionStart ?? 0;
	const end = field.selectionEnd ?? start;
	if (end > start) {
		field.setRangeText('', start, end, 'end');
	} else if (start < field.value.length) {
		field.setRangeText('', start, start + 1, 'end');
	} else {
		return true;
	}
	announce(field);
	return true;
}

/**
 * `Einsetzen`: Chrome verbietet `execCommand('paste')`; dann liest die
 * Zwischenablage-API den Text und er wird an der Einfügemarke eingesetzt.
 */
async function paste(field: HTMLElement): Promise<boolean> {
	if (document.execCommand('paste')) return true;

	let text = '';
	try {
		text = await navigator.clipboard.readText();
	} catch {
		return false;
	}
	if (!text) return true;

	if (isTextField(field)) {
		const start = field.selectionStart ?? field.value.length;
		const end = field.selectionEnd ?? start;
		field.setRangeText(text, start, end, 'end');
		announce(field);
		return true;
	}
	return document.execCommand('insertText', false, text);
}

/** Inhalt der Zwischenablage als Text; `null`, wenn der Browser ihn nicht hergibt. */
export async function readClipboard(): Promise<string | null> {
	try {
		return await navigator.clipboard.readText();
	} catch {
		return null;
	}
}
