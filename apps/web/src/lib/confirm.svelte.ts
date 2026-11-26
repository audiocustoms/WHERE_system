/**
 * Confirmation before a cancel. Every `Abbrechen` in a mask or an input window
 * runs through `askCancel`, so nothing is discarded by accident. The dialog
 * itself is rendered once in the layout.
 */
import { t } from './i18n.svelte';

export interface ConfirmRequest {
	title: string;
	message: string;
	confirmLabel: string;
	cancelLabel: string;
	resolve: (ok: boolean) => void;
}

let request = $state<ConfirmRequest | null>(null);

export const confirmRequest = () => request;

export function askConfirm(options: {
	title?: string;
	message: string;
	confirmLabel?: string;
	cancelLabel?: string;
}): Promise<boolean> {
	// A second confirmation replaces the first; the old one counts as "no".
	request?.resolve(false);
	return new Promise((resolve) => {
		request = {
			title: options.title ?? t('Rückfrage'),
			message: options.message,
			confirmLabel: options.confirmLabel ?? t('Ja'),
			cancelLabel: options.cancelLabel ?? t('Nein'),
			resolve
		};
	});
}

export function answerConfirm(ok: boolean) {
	const open = request;
	request = null;
	open?.resolve(ok);
}

/** "Abbrechen" in a mask: `true` when the cancel was confirmed. */
export function askCancel(what: string): Promise<boolean> {
	return askConfirm({
		title: t('Abbrechen'),
		message: t('{what} wirklich abbrechen? Nicht gesicherte Eingaben gehen verloren.', { what }),
		confirmLabel: t('Ja, abbrechen'),
		cancelLabel: t('Weiter bearbeiten')
	});
}

/** For `onclick`: asks first and runs `action` only after confirmation. */
export function confirmCancel(what: string, action: () => void) {
	return async () => {
		if (await askCancel(what)) action();
	};
}
