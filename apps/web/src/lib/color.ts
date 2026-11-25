/**
 * Colour conversions behind the colour picker. The API keeps colours as
 * `#RRGGBB` in upper case, so every helper returns that form.
 */

export interface Rgb {
	r: number;
	g: number;
	b: number;
}

export interface Hsl {
	/** Hue in degrees, 0–360. */
	h: number;
	/** Saturation in percent, 0–100. */
	s: number;
	/** Lightness in percent, 0–100. */
	l: number;
}

const HEX = /^#?([0-9a-f]{6})$/i;
const SHORT_HEX = /^#?([0-9a-f]{3})$/i;

function clamp(value: number, min: number, max: number): number {
	if (!Number.isFinite(value)) return min;
	return Math.min(max, Math.max(min, Math.round(value)));
}

/** Bounds a percentage without rounding it — the colour must stay exact. */
function bound(value: number, min: number, max: number): number {
	if (!Number.isFinite(value)) return min;
	return Math.min(max, Math.max(min, value));
}

/** `#1f6feb`, `1F6FEB` and `#fff` are colours; everything else is not. */
export function isHex(text: string): boolean {
	return HEX.test(text.trim()) || SHORT_HEX.test(text.trim());
}

/** Reads a colour as RGB; `null` when the text is no colour. */
export function rgbFromHex(text: string): Rgb | null {
	const trimmed = text.trim();
	const short = SHORT_HEX.exec(trimmed);
	if (short) {
		const [r, g, b] = short[1].split('');
		return {
			r: parseInt(r + r, 16),
			g: parseInt(g + g, 16),
			b: parseInt(b + b, 16)
		};
	}
	const match = HEX.exec(trimmed);
	if (!match) return null;
	const value = match[1];
	return {
		r: parseInt(value.slice(0, 2), 16),
		g: parseInt(value.slice(2, 4), 16),
		b: parseInt(value.slice(4, 6), 16)
	};
}

/** The colour as `#RRGGBB` in upper case. */
export function hexFromRgb({ r, g, b }: Rgb): string {
	const part = (value: number) => clamp(value, 0, 255).toString(16).padStart(2, '0');
	return `#${part(r)}${part(g)}${part(b)}`.toUpperCase();
}

/** Normalises any readable colour to `#RRGGBB`; `null` when it is none. */
export function normalizeHex(text: string): string | null {
	const rgb = rgbFromHex(text);
	return rgb === null ? null : hexFromRgb(rgb);
}

/** Hue, saturation and lightness of a colour; not rounded, so the way back is exact. */
export function hslFromRgb({ r, g, b }: Rgb): Hsl {
	const rot = r / 255;
	const gruen = g / 255;
	const blue = b / 255;
	const max = Math.max(rot, gruen, blue);
	const min = Math.min(rot, gruen, blue);
	const delta = max - min;
	const l = (max + min) / 2;
	if (delta === 0) {
		return { h: 0, s: 0, l: l * 100 };
	}
	const s = delta / (1 - Math.abs(2 * l - 1));
	let h: number;
	if (max === rot) {
		h = ((gruen - blue) / delta) % 6;
	} else if (max === gruen) {
		h = (blue - rot) / delta + 2;
	} else {
		h = (rot - gruen) / delta + 4;
	}
	h *= 60;
	if (h < 0) h += 360;
	return { h, s: s * 100, l: l * 100 };
}

/** The RGB colour of hue, saturation and lightness. */
export function rgbFromHsl({ h, s, l }: Hsl): Rgb {
	const saettigung = bound(s, 0, 100) / 100;
	const brightness = bound(l, 0, 100) / 100;
	const chroma = (1 - Math.abs(2 * brightness - 1)) * saettigung;
	const winkel = (((h % 360) + 360) % 360) / 60;
	const secondary = chroma * (1 - Math.abs((winkel % 2) - 1));
	const [rot, gruen, blue] =
		winkel < 1
			? [chroma, secondary, 0]
			: winkel < 2
				? [secondary, chroma, 0]
				: winkel < 3
					? [0, chroma, secondary]
					: winkel < 4
						? [0, secondary, chroma]
						: winkel < 5
							? [secondary, 0, chroma]
							: [chroma, 0, secondary];
	const m = brightness - chroma / 2;
	return {
		r: clamp((rot + m) * 255, 0, 255),
		g: clamp((gruen + m) * 255, 0, 255),
		b: clamp((blue + m) * 255, 0, 255)
	};
}
