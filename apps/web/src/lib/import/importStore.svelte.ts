import type { ImportTarget } from './importFields';

/**
 * State of the import manager (`Datei → Aktion → Import-Manager`).
 *
 * One window for both targets: it opens with the chosen target and can be
 * switched inside the window.
 */
let isOpen = $state(false);
let target = $state<ImportTarget>('addresses');

export const importOpen = () => isOpen;

/** Target the window was opened with. */
export const importTarget = () => target;

export function openImport(newTarget: ImportTarget = 'addresses') {
	target = newTarget;
	isOpen = true;
}

export function closeImport() {
	isOpen = false;
}
