/**
 * Windows the way Windows handles them: drag by the title bar, resize at the edges and
 * corners, a double click on the title bar puts place and size back.
 *
 * One listener on the document is enough for every window of the application
 * (`.sw-window`): the search window, the address and article masks, the number and
 * confirmation windows.
 *
 * As soon as a window is touched it leaves the centring of its backdrop and is fixed
 * with `left`/`top`, so edge and corner follow the pointer exactly — even when the
 * width changes on the way.
 */

/** That much of a window stays visible while it is moved. */
const VISIBLE = 120;
/** The title bar is at least this high — it stays below the top edge. */
const TITLE_HEIGHT = 28;
/** Smallest size while dragging at the edges. */
const MIN_WIDTH = 320;
const MIN_HEIGHT = 160;
/** Distance a window keeps to the edge of its backdrop. */
const MARGIN = 8;

/** The eight handles: four edges, four corners. */
const EDGES = ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'] as const;
type Edge = (typeof EDGES)[number];

type Place = { left: number; top: number };

/** Place per window, once it was touched. Without an entry it stays centred. */
const places = new WeakMap<HTMLElement, Place>();
/** The `style` of a window before it was touched — for the double click. */
const startStyles = new WeakMap<HTMLElement, string>();

function backdrop(element: HTMLElement): HTMLElement | null {
	return element.closest<HTMLElement>('.sw-backdrop');
}

function setPlace(element: HTMLElement, place: Place) {
	places.set(element, place);
	element.style.left = `${Math.round(place.left)}px`;
	element.style.top = `${Math.round(place.top)}px`;
}

/** Takes the window out of the centring and remembers its place. */
function pinPlace(element: HTMLElement): Place {
	const known = places.get(element);
	if (known) return known;
	// The original `style` (width from the component) has to be able to come back.
	startStyles.set(element, element.getAttribute('style') ?? '');
	const area = backdrop(element);
	const rect = element.getBoundingClientRect();
	const base = area?.getBoundingClientRect();
	const place: Place = {
		left: rect.left - (base?.left ?? 0) + (area?.scrollLeft ?? 0),
		top: rect.top - (base?.top ?? 0) + (area?.scrollTop ?? 0)
	};
	element.style.position = 'absolute';
	element.style.margin = '0';
	setPlace(element, place);
	return place;
}

/** Keeps the window in view: a piece to the side, the title bar at the top. */
function clampPlace(element: HTMLElement, place: Place): Place {
	const area = backdrop(element) ?? document.documentElement;
	const minLeft = Math.min(0, VISIBLE - element.offsetWidth);
	const maxLeft = Math.max(minLeft, area.clientWidth - VISIBLE);
	const minTop = 0;
	const maxTop = Math.max(
		minTop,
		area.clientHeight - Math.min(TITLE_HEIGHT, element.offsetHeight)
	);
	return {
		left: Math.min(Math.max(place.left, minLeft), maxLeft),
		top: Math.min(Math.max(place.top, minTop), maxTop)
	};
}

/** Puts the eight handles into a window that has none yet. */
function withHandles(element: HTMLElement) {
	if (element.dataset.handles === '1') return;
	element.dataset.handles = '1';
	for (const edge of EDGES) {
		const handle = document.createElement('div');
		handle.className = `sw-grip sw-griff-${edge}`;
		handle.dataset.edge = edge;
		handle.setAttribute('aria-hidden', 'true');
		element.appendChild(handle);
	}
}

/** Every window that appears in the document gets its handles. */
function watchWindows(): MutationObserver {
	const observer = new MutationObserver((entries) => {
		for (const entry of entries) {
			for (const node of entry.addedNodes) {
				if (!(node instanceof HTMLElement)) continue;
				const element = node.matches('.sw-window')
					? node
					: node.querySelector<HTMLElement>('.sw-window');
				if (element) withHandles(element);
			}
		}
	});
	observer.observe(document.body, { childList: true, subtree: true });
	for (const element of document.querySelectorAll<HTMLElement>('.sw-window')) withHandles(element);
	return observer;
}

export function installWindowChrome(): () => void {
	let drag: {
		element: HTMLElement;
		pointerId: number;
		startX: number;
		startY: number;
		place: Place;
	} | null = null;

	let resize: {
		element: HTMLElement;
		edge: Edge;
		pointerId: number;
		startX: number;
		startY: number;
		place: Place;
		width: number;
		height: number;
	} | null = null;

	const onPointerDown = (event: PointerEvent) => {
		if (event.button !== 0) return;
		const target = event.target;
		if (!(target instanceof Element)) return;

		// 1. Handle at an edge or a corner: change the size.
		const handle = target.closest<HTMLElement>('.sw-grip');
		const handleWindow = handle?.closest<HTMLElement>('.sw-window');
		if (handle && handleWindow && handle.dataset.edge) {
			resize = {
				element: handleWindow,
				edge: handle.dataset.edge as Edge,
				pointerId: event.pointerId,
				startX: event.clientX,
				startY: event.clientY,
				place: pinPlace(handleWindow),
				width: handleWindow.offsetWidth,
				height: handleWindow.offsetHeight
			};
			handleWindow.classList.add('is-dragging');
			document.body.classList.add('window-resize');
			event.preventDefault();
			return;
		}

		// 2. Title bar: move. Buttons and fields stay usable.
		if (target.closest('button, input, select, textarea, a')) return;
		const title = target.closest('.sw-titlebar');
		const element = title?.closest<HTMLElement>('.sw-window');
		if (!element) return;

		drag = {
			element,
			pointerId: event.pointerId,
			startX: event.clientX,
			startY: event.clientY,
			place: pinPlace(element)
		};
		element.classList.add('is-dragging');
		document.body.classList.add('window-drag');
		event.preventDefault();
	};

	const onPointerMove = (event: PointerEvent) => {
		if (resize && event.pointerId === resize.pointerId) {
			const dx = event.clientX - resize.startX;
			const dy = event.clientY - resize.startY;
			const { edge, place, width, height, element } = resize;
			const area = backdrop(element);
			const maxWidth = (area?.clientWidth || window.innerWidth) - place.left - MARGIN;
			const maxHeight = (area?.clientHeight || window.innerHeight) - place.top - MARGIN;

			let nextWidth = edge.includes('e') ? width + dx : edge.includes('w') ? width - dx : width;
			let nextHeight = edge.includes('s') ? height + dy : edge.includes('n') ? height - dy : height;
			nextWidth = Math.min(Math.max(nextWidth, MIN_WIDTH), Math.max(MIN_WIDTH, maxWidth));
			nextHeight = Math.min(Math.max(nextHeight, MIN_HEIGHT), Math.max(MIN_HEIGHT, maxHeight));

			element.style.width = `${Math.round(nextWidth)}px`;
			element.style.height = `${Math.round(nextHeight)}px`;
			element.style.maxHeight = 'none';
			setPlace(element, {
				// The dragged edge follows the pointer, the other side stays.
				left: edge.includes('w') ? place.left + (width - nextWidth) : place.left,
				top: edge.includes('n') ? place.top + (height - nextHeight) : place.top
			});
			return;
		}

		if (!drag || event.pointerId !== drag.pointerId) return;
		setPlace(
			drag.element,
			clampPlace(drag.element, {
				left: drag.place.left + (event.clientX - drag.startX),
				top: drag.place.top + (event.clientY - drag.startY)
			})
		);
	};

	const onPointerUp = (event: PointerEvent) => {
		if (resize && event.pointerId === resize.pointerId) {
			resize.element.classList.remove('is-dragging');
			document.body.classList.remove('window-resize');
			resize = null;
			return;
		}
		if (!drag || event.pointerId !== drag.pointerId) return;
		drag.element.classList.remove('is-dragging');
		document.body.classList.remove('window-drag');
		drag = null;
	};

	const onDoubleClick = (event: MouseEvent) => {
		const target = event.target;
		if (!(target instanceof Element)) return;
		if (target.closest('button, input, select, textarea, a')) return;
		const element = target.closest('.sw-titlebar')?.closest<HTMLElement>('.sw-window');
		if (!element) return;
		// Back to the middle, in the size it started with: the stored `style` returns.
		element.setAttribute('style', startStyles.get(element) ?? '');
		startStyles.delete(element);
		places.delete(element);
	};

	const observer = watchWindows();
	document.addEventListener('pointerdown', onPointerDown);
	document.addEventListener('pointermove', onPointerMove);
	document.addEventListener('pointerup', onPointerUp);
	document.addEventListener('pointercancel', onPointerUp);
	document.addEventListener('dblclick', onDoubleClick);

	return () => {
		observer.disconnect();
		document.removeEventListener('pointerdown', onPointerDown);
		document.removeEventListener('pointermove', onPointerMove);
		document.removeEventListener('pointerup', onPointerUp);
		document.removeEventListener('pointercancel', onPointerUp);
		document.removeEventListener('dblclick', onDoubleClick);
		document.body.classList.remove('window-drag', 'window-resize');
	};
}
