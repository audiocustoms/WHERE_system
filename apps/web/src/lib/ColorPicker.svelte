<script lang="ts">
	import { t } from '$core/i18n.svelte';

	/**
	 * Colour picker with two tabs: 32 predefined colours, and a colour wheel
	 * with HEX, RGB and HSL. The value is a `#RRGGBB` in upper case, the form
	 * the API stores.
	 */
	import { hexFromRgb, hslFromRgb, normalizeHex, rgbFromHsl, rgbFromHex } from '$core/color';

	const PALETTE: readonly string[] = [
		'#FFFFFF',
		'#F4F4F5',
		'#D4D4D8',
		'#A1A1AA',
		'#71717A',
		'#3F3F46',
		'#18181B',
		'#000000',
		'#FEE2E2',
		'#FCA5A5',
		'#EF4444',
		'#B91C1C',
		'#FED7AA',
		'#FB923C',
		'#EA580C',
		'#9A3412',
		'#FEF3C7',
		'#FDE047',
		'#EAB308',
		'#A16207',
		'#DCFCE7',
		'#86EFAC',
		'#22C55E',
		'#15803D',
		'#DBEAFE',
		'#93C5FD',
		'#3B82F6',
		'#1D4ED8',
		'#E9D5FF',
		'#C084FC',
		'#9333EA',
		'#6B21A8'
	];

	interface Props {
		value?: string;
		/** Name of the field, used as the accessible label. */
		label?: string;
		disabled?: boolean;
		/** Called with the new `#RRGGBB` whenever the picker changes the value. */
		onchange?: (value: string) => void;
	}

	let {
		value = $bindable('#000000'),
		label = t('Farbe'),
		disabled = false,
		onchange
	}: Props = $props();

	let isOpen = $state(false);
	let tab = $state<'palette' | 'rad'>('palette');
	let hexText = $state(value);
	let wurzel = $state<HTMLDivElement | null>(null);
	let radFlaeche = $state<HTMLDivElement | null>(null);
	let dragging = false;

	/** The colour the picker works with: any readable value, else black. */
	const color = $derived(normalizeHex(value) ?? '#000000');
	/** Whether the field carries a colour at all; empty shows the crossed square. */
	const applied = $derived(normalizeHex(value) !== null);
	const rgb = $derived(rgbFromHex(color) ?? { r: 0, g: 0, b: 0 });
	const hsl = $derived(hslFromRgb(rgb));
	/** What the fields show: whole numbers, while the hue stays exact inside. */
	const display = $derived({
		h: Math.round(hsl.h),
		s: Math.round(hsl.s),
		l: Math.round(hsl.l)
	});
	const marker = $derived({
		left: `${50 + Math.sin((hsl.h * Math.PI) / 180) * (hsl.s / 2)}%`,
		top: `${50 - Math.cos((hsl.h * Math.PI) / 180) * (hsl.s / 2)}%`
	});

	// The text of the HEX field follows the value, whoever changed it.
	$effect(() => {
		hexText = color;
	});

	$effect(() => {
		if (!isOpen) return;
		function outside(event: PointerEvent) {
			if (wurzel !== null && !wurzel.contains(event.target as Node)) isOpen = false;
		}
		function taste(event: KeyboardEvent) {
			if (event.key === 'Escape') isOpen = false;
		}
		document.addEventListener('pointerdown', outside);
		document.addEventListener('keydown', taste);
		return () => {
			document.removeEventListener('pointerdown', outside);
			document.removeEventListener('keydown', taste);
		};
	});

	function set(color: string) {
		const clean = normalizeHex(color);
		if (clean === null) return;
		value = clean;
		onchange?.(clean);
	}

	function choose(color: string) {
		set(color);
		isOpen = false;
	}

	function setzeHsl(h: number, s: number, l: number) {
		set(hexFromRgb(rgbFromHsl({ h, s, l })));
	}

	function setzeTeilRgb(part: 'r' | 'g' | 'b', value: number) {
		const createNew = { ...rgb };
		createNew[part] = value;
		set(hexFromRgb(createNew));
	}

	function setzeTeilHsl(part: 'h' | 's' | 'l', value: number) {
		const createNew = { ...hsl };
		createNew[part] = value;
		setzeHsl(createNew.h, createNew.s, createNew.l);
	}

	/** Hue and saturation under the pointer; the wheel starts at the top. */
	function wheelValue(event: PointerEvent): { h: number; s: number } | null {
		const flaeche = radFlaeche;
		if (flaeche === null) return null;
		const rect = flaeche.getBoundingClientRect();
		const radius = Math.min(rect.width, rect.height) / 2;
		if (radius <= 0) return null;
		const dx = event.clientX - (rect.left + rect.width / 2);
		const dy = event.clientY - (rect.top + rect.height / 2);
		const gap = Math.min(1, Math.hypot(dx, dy) / radius);
		let winkel = (Math.atan2(dy, dx) * 180) / Math.PI + 90;
		if (winkel < 0) winkel += 360;
		return { h: winkel, s: gap * 100 };
	}

	function showWheel(event: PointerEvent) {
		const value = wheelValue(event);
		if (value === null) return;
		setzeHsl(value.h, value.s, hsl.l);
	}

	function wheelStart(event: PointerEvent) {
		dragging = true;
		(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
		showWheel(event);
	}

	function dragWheel(event: PointerEvent) {
		if (dragging) showWheel(event);
	}

	function wheelEnd(event: PointerEvent) {
		dragging = false;
		(event.currentTarget as HTMLElement).releasePointerCapture?.(event.pointerId);
	}

	function count(event: Event): number {
		return (event.currentTarget as HTMLInputElement).valueAsNumber;
	}
</script>

<div class="picker" bind:this={wurzel}>
	<button
		type="button"
		class="field"
		{disabled}
		title={label}
		aria-label={label}
		aria-expanded={isOpen}
		onclick={() => (isOpen = !isOpen)}
	>
		<span
			class="preview"
			class:empty={!applied}
			style={applied ? `background: ${color}` : undefined}
		></span>
		{#if applied}
			<span class="value">{color}</span>
		{/if}
	</button>

	{#if isOpen}
		<div class="menu">
			<div class="tabs" role="tablist">
				<button
					type="button"
					role="tab"
					class:active={tab === 'palette'}
					aria-selected={tab === 'palette'}
					onclick={() => (tab = 'palette')}
				>
					{t('Palette')}
				</button>
				<button
					type="button"
					role="tab"
					class:active={tab === 'rad'}
					aria-selected={tab === 'rad'}
					onclick={() => (tab = 'rad')}
				>
					{t('Farbrad')}
				</button>
			</div>

			{#if tab === 'palette'}
				<div class="palette">
					{#each PALETTE as platte (platte)}
						<button
							type="button"
							class="swatch"
							class:selected={color === platte}
							style="background: {platte}"
							title={platte}
							aria-label={platte}
							onclick={() => choose(platte)}
						></button>
					{/each}
				</div>
			{:else}
				<div
					class="wheel"
					bind:this={radFlaeche}
					role="presentation"
					onpointerdown={wheelStart}
					onpointermove={dragWheel}
					onpointerup={wheelEnd}
					onpointercancel={wheelEnd}
				>
					<span class="marker" style="left: {marker.left}; top: {marker.top}"></span>
				</div>

				<label class="lightness">
					<span>{t('Helligkeit')}</span>
					<input
						type="range"
						min="0"
						max="100"
						step="1"
						value={display.l}
						aria-label={t('Helligkeit')}
						oninput={(event) => setzeTeilHsl('l', count(event))}
					/>
					<span class="number">{display.l} %</span>
				</label>

				<label class="row">
					<span>{t('HEX')}</span>
					<input
						class="text"
						maxlength="7"
						value={hexText}
						oninput={(event) => {
							hexText = event.currentTarget.value;
							set(hexText);
						}}
						onblur={() => (hexText = color)}
					/>
				</label>

				<div class="row">
					<span>{t('RGB')}</span>
					<input
						type="number"
						min="0"
						max="255"
						value={rgb.r}
						aria-label={t('Rot')}
						oninput={(event) => setzeTeilRgb('r', count(event))}
					/>
					<input
						type="number"
						min="0"
						max="255"
						value={rgb.g}
						aria-label={t('Grün')}
						oninput={(event) => setzeTeilRgb('g', count(event))}
					/>
					<input
						type="number"
						min="0"
						max="255"
						value={rgb.b}
						aria-label={t('Blau')}
						oninput={(event) => setzeTeilRgb('b', count(event))}
					/>
				</div>

				<div class="row">
					<span>{t('HSL')}</span>
					<input
						type="number"
						min="0"
						max="360"
						value={display.h}
						aria-label={t('Farbton')}
						oninput={(event) => setzeTeilHsl('h', count(event))}
					/>
					<input
						type="number"
						min="0"
						max="100"
						value={display.s}
						aria-label={t('Sättigung')}
						oninput={(event) => setzeTeilHsl('s', count(event))}
					/>
					<input
						type="number"
						min="0"
						max="100"
						value={display.l}
						aria-label={t('Helligkeit in Prozent')}
						oninput={(event) => setzeTeilHsl('l', count(event))}
					/>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.picker {
		position: relative;
		display: inline-block;
	}

	.field {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.15rem 0.45rem 0.15rem 0.2rem;
		border: 1px solid var(--border);
		border-radius: 0.3rem;
		background: var(--surface);
		color: var(--text);
		cursor: pointer;
		font: inherit;
	}

	.field:disabled {
		cursor: not-allowed;
		opacity: 0.55;
	}

	.preview {
		width: 1.6rem;
		height: 1.35rem;
		border: 1px solid var(--muted-2);
		border-radius: 0.2rem;
	}

	/* No colour yet: a white square with a stroke through it. */
	.preview.empty {
		background-color: #fff;
		background-image: linear-gradient(
			to bottom right,
			transparent 44%,
			var(--muted-2) 44%,
			var(--muted-2) 56%,
			transparent 56%
		);
	}

	.value {
		font-size: 0.78rem;
		font-variant-numeric: tabular-nums;
		color: var(--muted);
	}

	.menu {
		position: absolute;
		top: calc(100% + 0.25rem);
		left: 0;
		z-index: 60;
		width: 15.5rem;
		padding: 0.55rem;
		border: 1px solid var(--border);
		border-radius: 0.4rem;
		background: var(--surface);
		color: var(--text);
		box-shadow: 0 8px 24px rgba(15, 23, 42, 0.28);
	}

	.tabs {
		display: flex;
		gap: 0.25rem;
		margin-bottom: 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	.tabs button {
		flex: 1;
		padding: 0.3rem 0.2rem;
		border: none;
		border-bottom: 2px solid transparent;
		background: none;
		cursor: pointer;
		font: inherit;
		font-size: 0.8rem;
		color: var(--muted);
	}

	.tabs button.active {
		border-bottom-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}

	.palette {
		display: grid;
		grid-template-columns: repeat(8, 1fr);
		gap: 0.2rem;
	}

	.swatch {
		height: 1.55rem;
		border: 1px solid var(--border);
		border-radius: 0.2rem;
		cursor: pointer;
		padding: 0;
	}

	.swatch:hover {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.swatch.selected {
		outline: 2px solid var(--text);
		outline-offset: 1px;
	}

	.wheel {
		position: relative;
		width: 100%;
		aspect-ratio: 1 / 1;
		border-radius: 50%;
		cursor: crosshair;
		touch-action: none;
		background-image:
			radial-gradient(circle closest-side, #fff 0%, rgba(255, 255, 255, 0) 100%),
			conic-gradient(
				from 0deg,
				#f00 0deg,
				#ff0 60deg,
				#0f0 120deg,
				#0ff 180deg,
				#00f 240deg,
				#f0f 300deg,
				#f00 360deg
			);
		box-shadow: inset 0 0 0 1px var(--border);
	}

	.marker {
		position: absolute;
		width: 0.7rem;
		height: 0.7rem;
		margin: -0.35rem 0 0 -0.35rem;
		border: 2px solid #fff;
		border-radius: 50%;
		box-shadow: 0 0 0 1px var(--text);
		pointer-events: none;
	}

	.lightness {
		display: grid;
		grid-template-columns: auto 1fr auto;
		align-items: center;
		gap: 0.4rem;
		margin: 0.45rem 0;
		font-size: 0.75rem;
		color: var(--muted);
	}

	.lightness input[type='range'] {
		width: 100%;
		background: linear-gradient(90deg, #000, #fff);
	}

	.number {
		font-variant-numeric: tabular-nums;
	}

	.row {
		display: grid;
		grid-template-columns: 2.4rem 1fr 1fr 1fr;
		align-items: center;
		gap: 0.25rem;
		margin-top: 0.35rem;
		font-size: 0.75rem;
		color: var(--muted);
	}

	.row > span:first-child {
		font-weight: 600;
	}

	.row input {
		width: 100%;
		min-width: 0;
		padding: 0.15rem 0.25rem;
		border: 1px solid var(--border);
		border-radius: 0.25rem;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
	}

	.row input.text {
		grid-column: 2 / span 3;
		text-transform: uppercase;
	}
</style>
