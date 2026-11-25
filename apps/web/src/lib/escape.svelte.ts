/**
 * The house rule: `Esc` cancels. It closes the **topmost** open window — by the same
 * path as that window's `Abbrechen`, including the confirmation attached there.
 *
 * A window registers on mount with `escapeFenster(abbrechen)`; the return value
 * unregisters it again (in `$effect` the return value suffices). Of several open
 * windows only the last registered one reacts, so that `Esc` never closes two
 * windows at once. Popups inside a page (colour picker, cell menu, rename) keep
 * their own `Esc`.
 *
 * Stack and listener live on `window` and not in the module: on hot reload of the
 * development server an open window therefore stays reachable, and the listener is
 * not registered a second time.
 */
type Cancel = () => void;

interface Guard {
	stack: Cancel[];
	zuhoerer: boolean;
}

const KEYS = '__whereEscape';

function guard(): Guard {
	const global = window as unknown as Record<string, Guard | undefined>;
	global[KEYS] ??= { stack: [], zuhoerer: false };
	return global[KEYS] as Guard;
}

/** One listener for all windows — registered before the first window opens. */
export function startEscape(): void {
	if (typeof window === 'undefined') return;
	const w = guard();
	if (w.zuhoerer) return;
	w.zuhoerer = true;
	window.addEventListener(
		'keydown',
		(event: KeyboardEvent) => {
			if (event.key !== 'Escape') return;
			const top = w.stack[w.stack.length - 1];
			if (top === undefined) return;
			event.preventDefault();
			event.stopImmediatePropagation();
			top();
		},
		true
	);
}

/** Registers an open window; the return value unregisters it again. */
export function escapeWindows(cancel: Cancel): () => void {
	startEscape();
	const { stack } = guard();
	stack.push(cancel);
	return () => {
		const offset = stack.lastIndexOf(cancel);
		if (offset >= 0) stack.splice(offset, 1);
	};
}