<script lang="ts">
	import { escapeWindows } from '$core/escape.svelte';
	import { onMount } from 'svelte';
	import { errorText, type Employee, type Shift } from '$core/api';
	import '$core/search/search-window.css';
	import {
		PAPERS,
		sheetMetrics,
		staffLineText,
		pageCount,
		hoursText,
		shortDay,
		weeks,
		SECOND_LINE_FACTOR,
		type Report
	} from './roster-export';
	import {
		toText,
		toDate,
		buildReport,
		loadMonth,
		type ReportSources,
		type SheetRow,
		type MonthData
	} from './roster-report';

	/**
	 * „Dienstplan drucken" — the window behind the print button of the board.
	 *
	 * It prints one month, one week per page: the row of the dates, the disposition
	 * of every location, an empty row, the row of the dates again and the staff with
	 * its department, position and hours. The exchange — CSV, XLSX and PDF — lives in
	 * the window `Import / Export` of the board.
	 */
	interface Props {
		/** First day the board shows — the month is taken from it. */
		from: string;
		to: string;
		employees: Employee[];
		shifts: Shift[];
		colors: { sick: string; vacation: string };
		/** Hours one day of vacation and one day of sickness count. */
		absence: { sick: number; vacation: number };
		/** Tints of the master data; the sub-department wins over the department. */
		departmentColors: Map<string, string>;
		subdepartmentColors: Map<string, string>;
		onclose: () => void;
	}

	let {
		from,
		employees,
		shifts,
		colors,
		absence,
		departmentColors,
		subdepartmentColors,
		onclose
	}: Props = $props();

	/** Paper of the print dialog; DIN A3 landscape is what the theatre prints on. */
	let paper = $state('a3-quer');
	let data = $state<MonthData | null>(null);
	let loading = $state(false);
	let error = $state('');
	let previewWidth = $state(900);
	let pagesElement = $state<HTMLElement | null>(null);

	/** The month of the board: the first to the last day of it. */
	const period = $derived.by(() => {
		const anchor = toDate(from);
		return {
			from: toIso(new Date(anchor.getFullYear(), anchor.getMonth(), 1)),
			to: toIso(new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0))
		};
	});

	function toIso(date: Date): string {
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${date.getFullYear()}-${month}-${day}`;
	}

	$effect(() => {
		const startIso = period.from;
		const endIso = period.to;
		void load(startIso, endIso);
	});

	async function load(startIso: string, endIso: string) {
		loading = true;
		error = '';
		try {
			data = await loadMonth({ from: startIso, to: endIso });
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	const sheet = $derived(PAPERS.find((entry) => entry.value === paper) ?? PAPERS[0]);

	const report = $derived.by<Report>(() => {
		const sources: ReportSources = {
			period,
			data: data ?? { cells: {}, dispo: [], locations: [], holidays: {} },
			employees,
			shifts,
			colors,
			absence,
			paper: { width: sheet.width, height: sheet.height },
			staffColors: { subdepartment: subdepartmentColors, department: departmentColors }
		};
		return buildReport(sources);
	});

	const title = $derived(report.title);
	const days = $derived(report.days);
	const pages = $derived(weeks(report.days));
	const count = $derived(pageCount(report));

	const zoom = $derived(Math.min(1, (previewWidth - 24) / (sheet.width * 3.7795)));

	/** `true` when the row opens a new block of the same position. */
	function startsBlock(rows: SheetRow[], index: number): boolean {
		return index === 0 || rows[index - 1].position !== rows[index].position;
	}

	/** The sheet styles have to be global: the preview and the iframe share them. */
	onMount(() => {
		const element = document.createElement('style');
		element.id = 'where-print-style';
		element.textContent = PRINT_STYLE;
		document.head.append(element);
		return () => element.remove();
	});

	/**
	 * Hands the pages to the print dialog of the system. The pages go into an
	 * iframe of their own, so paper size and margins arrive at the printer without
	 * the board in the way; everything else (printer, duplex, copies) is left to
	 * the dialog of the operating system.
	 */
	function print() {
		if (pagesElement === null) return;
		const frame = document.createElement('iframe');
		frame.setAttribute('aria-hidden', 'true');
		frame.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0;';
		document.body.append(frame);
		const doc = frame.contentDocument;
		if (doc === null) {
			frame.remove();
			return;
		}
		doc.open();
		doc.write(
			`<!doctype html><html lang="de"><head><meta charset="utf-8"><title>${title}</title>` +
				`<style>@page{size:${sheet.css};margin:${sheetMetrics(report, pages[0] ?? []).margin}mm}${PRINT_STYLE}${PRINT_PAGE}</style>` +
				`</head><body>${pagesElement.innerHTML}</body></html>`
		);
		doc.close();
		const window = frame.contentWindow;
		setTimeout(() => {
			window?.focus();
			window?.print();
			setTimeout(() => frame.remove(), 2000);
		}, 200);
	}

	/**
	 * What changes on the printed page: the sheet gives its padding to the page
	 * margin, so the content sits exactly inside the printable area of the paper.
	 */
	const PRINT_PAGE = `
		.sheet {
			width: calc(var(--width) - 2 * var(--margin));
			height: calc(var(--height) - 2 * var(--margin) - 0.5mm);
			padding: 0;
		}
	`;

	/**
	 * The styles of a sheet — once for the preview and once for the iframe.
	 *
	 * Everything is in millimetres: a page is exactly the paper it is printed on,
	 * so the preview and the print agree.
	 */
	const PRINT_STYLE = `
		.sheet {
			box-sizing: border-box;
			display: flex;
			flex-direction: column;
			width: var(--width);
			height: calc(var(--height) - 0.5mm);
			padding: var(--margin);
			background: #fff;
			color: #111;
			font-family: 'Segoe UI', Arial, sans-serif;
			font-size: var(--font);
			line-height: 1.15;
			break-after: page;
			overflow: hidden;
			/* The tints have to reach the paper, not only the screen. */
			-webkit-print-color-adjust: exact;
			print-color-adjust: exact;
		}
		.sheet:last-child { break-after: auto; }
		.sheet .sheet-head {
			display: flex;
			align-items: baseline;
			gap: 4mm;
			height: var(--head);
			flex: none;
		}
		.sheet .sheet-head .title { font-size: calc(var(--font) * 1.9); font-weight: 700; }
		.sheet .sheet-head .asOf { margin-left: auto; font-size: calc(var(--font) * 0.9); color: #555; }
		.sheet table { width: 100%; border-collapse: collapse; table-layout: fixed; }
		.sheet thead th { height: var(--day-head); font-size: calc(var(--font) * 0.95); background: #eceff3; }
		.sheet th, .sheet td { border: 0.25mm solid #8a8a8a; padding: 0 1mm; }
		.sheet th.empty, .sheet th.who { text-align: left; }
		.sheet th.who { overflow: hidden; }
		.sheet .line-one {
			display: grid;
			grid-template-columns: 1fr 13mm 13mm 12mm;
			align-items: baseline;
			gap: 0 1.5mm;
		}
		.sheet .line-one .amount { text-align: right; font-weight: 400; }
		.sheet th.who .line-one .name { font-weight: 700; }
		.sheet th.who .line-two {
			display: block;
			font-weight: 400;
			font-size: calc(var(--font) * var(--line2, 0.74));
			color: #444;
			line-height: 1.1;
			/* Position, department and sub-department on one line — the row keeps its height. */
			overflow: hidden;
			white-space: nowrap;
			text-overflow: ellipsis;
		}
		.sheet .dispo tr.empty-row { height: var(--gap); }
		.sheet .dispo td {
			font-size: var(--dispo-font);
			line-height: 1.25;
			vertical-align: top;
			padding: 0.8mm 1mm 0;
			overflow: hidden;
		}
		.sheet .dispo td span { display: block; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
		.sheet .dispo td span.wrapped { padding-left: 3.5mm; }
		.sheet .dispo td span b { font-weight: 700; margin-right: 1mm; }
		.sheet .dispo th.who { font-size: var(--dispo-font); vertical-align: top; padding-top: 0.8mm; }
		.sheet .staff tbody tr { height: var(--row); }
		/* The sub-department (or department) of the employee, as on the board. */
		.sheet .staff tbody tr th.who {
			background: color-mix(in srgb, var(--row-color, #fff) 30%, #fff);
		}
		.sheet .staff tbody tr.block th, .sheet .staff tbody tr.block td { border-top: 0.8mm solid #333; }
		.sheet td.cell { text-align: center; vertical-align: middle; }
		.sheet td.off { background: color-mix(in srgb, var(--cell-color) 20%, #fff); }
		.sheet td.free-day { color: #6b7280; }
		.sheet .shortName { display: block; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
		.sheet .note { display: block; font-size: calc(var(--font) * 0.85); color: #333; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
		.sheet .holiday { display: block; font-size: calc(var(--font) * 0.78); color: #8a1c1c; }
		.sheet .sunday { background: #f4f4f5; }
		.sheet .foot {
			display: flex;
			align-items: center;
			gap: 4mm;
			height: var(--foot);
			flex: none;
			font-size: calc(var(--font) * 0.8);
			color: #444;
		}
		.sheet .foot .legend { flex: 1; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
	`;

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => onclose()));
</script>

<svelte:head>
	<title>{title} — WHERE</title>
</svelte:head>

<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) onclose();
	}}
></div>

<div class="window" role="dialog" aria-modal="true" aria-label="Dienstplan drucken">
	<header class="head">
		<h2>Dienstplan drucken</h2>
		<button type="button" class="close" aria-label="Schließen" onclick={onclose}>×</button>
	</header>

	<div class="choice">
		<label class="field"
			>Papier
			<select bind:value={paper}>
				{#each PAPERS as entry (entry.value)}
					<option value={entry.value}>{entry.label}</option>
				{/each}
			</select>
		</label>
		<span class="period"
			>{toText(period.from)} – {toText(period.to)} · {count} Seite{count === 1 ? '' : 'n'}</span
		>
		<button type="button" class="print" onclick={print} disabled={loading || count === 0}>
			Drucken …
		</button>
	</div>

	{#if error}<p class="bad">{error}</p>{/if}

	<div class="preview" bind:clientWidth={previewWidth}>
		{#if loading}
			<p class="loading">Lade …</p>
		{/if}
		<div class="pages" style={`zoom:${zoom}`}>
			<div bind:this={pagesElement}>
				{#each pages as page, number (page[0].iso)}
					{@const indexes = page.map((day) => days.findIndex((entry) => entry.iso === day.iso))}
					{@const m = sheetMetrics(report, page)}
					<section
						class="sheet"
						style={`--width:${sheet.width}mm; --height:${sheet.height}mm; --margin:${m.margin}mm; --head:${m.head}mm; --foot:${m.foot}mm; --day-head:${m.dayHead}mm; --row:${m.row}mm; --font:${m.font}pt; --dispo-font:${m.dispoFont}pt; --gap:${m.gap}mm; --first:${m.first}mm; --line2:${SECOND_LINE_FACTOR}`}
					>
						<div class="sheet-head">
							<span class="title">{title}</span>
							<span>{shortDay(page[0])} – {shortDay(page[page.length - 1])}</span>
							<span class="asOf">Stand {report.asOf}</span>
						</div>

						<table class="dispo">
							<colgroup>
								<col style="width:var(--first)" />
								{#each page as day (day.iso)}<col />{/each}
							</colgroup>
							<thead>
								<tr>
									<th class="empty"></th>
									{#each page as day (day.iso)}
										<th class:sunday={day.sunday}>
											{day.weekday} {shortDay(day)}
											{#if day.holiday}<span class="holiday">{day.holiday}</span>{/if}
										</th>
									{/each}
								</tr>
							</thead>
							<tbody>
								{#each report.dispo as row, locationNumber (row.location)}
									<tr style={`height:${m.dispoRows[locationNumber]}mm`}>
										<th class="who">{row.location}</th>
										{#each m.dispoCells[locationNumber] ?? [] as labels, column (column)}
											<td>
												{#each labels as label, index (index)}
													<span class:wrapped={label.wrapped === true}
														>{#if label.time}<b>{label.time}</b>{/if}{label.title}</span
													>
												{/each}
											</td>
										{/each}
									</tr>
								{/each}
								<tr class="empty-row">
									{#each Array(page.length + 1) as _, index (index)}
										<td></td>
									{/each}
								</tr>
							</tbody>
						</table>

						<table class="staff">
							<colgroup>
								<col style="width:var(--first)" />
								{#each page as day (day.iso)}<col />{/each}
							</colgroup>
							<thead>
								<tr>
									<th class="empty">
										<span class="line-one">
											<span></span>
											<span class="amount">Soll</span>
											<span class="amount">Ist</span>
											<span class="amount">+/-</span>
										</span>
									</th>
									{#each page as day (day.iso)}
										<th class:sunday={day.sunday}>
											{day.weekday} {shortDay(day)}
											{#if day.holiday}<span class="holiday">{day.holiday}</span>{/if}
										</th>
									{/each}
								</tr>
							</thead>
							<tbody>
								{#each report.staff as row, rowNumber (rowNumber)}
									<tr class:block={startsBlock(report.staff, rowNumber)}>
										<th
											class="who"
											style={row.color === '' ? '' : `--row-color:${row.color}`}
										>
											<span class="line-one">
												<span class="name">{row.name}</span>
												<span class="amount">{hoursText(row.target) || '–'}</span>
												<span class="amount">{hoursText(row.actual)}</span>
												<span class="amount">{hoursText(row.difference, true) || '–'}</span>
											</span>
											<span class="line-two">{staffLineText(row)}</span>
										</th>
										{#each indexes as offset, column (column)}
											{@const label = row.days[offset] ?? ''}
											{@const color = row.colors[offset] ?? ''}
											<td
												class="cell"
												class:off={color !== ''}
												class:free-day={label === 'frei'}
												class:sunday={label === ''}
												style={color === '' ? '' : `--cell-color:${color}`}
											>
												{#if label !== ''}
													{@const time = row.times[offset] ?? ''}
													<span class="shortName">{time === '' ? label : `${label} - ${time}`}</span>
													{#if m.row >= 8 && (row.notes[offset] ?? '') !== ''}
														<span class="note">{row.notes[offset]}</span>
													{/if}
												{/if}
											</td>
										{/each}
									</tr>
								{/each}
							</tbody>
						</table>

						<div class="foot">
							<span class="legend">{report.legend}</span>
							<span>Seite {number + 1} from {count}</span>
						</div>
					</section>
				{/each}
			</div>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(15, 23, 42, 0.55);
		z-index: 40;
	}
	.window {
		position: fixed;
		inset: 2vh 2vw;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 16px 20px;
		overflow: hidden;
		background: var(--where-surface, #fff);
		color: var(--where-label, #111);
		border-radius: 12px;
		box-shadow: 0 18px 50px rgba(15, 23, 42, 0.35);
		z-index: 41;
		/* The window is paper: its controls stay light, whatever the app is set to. */
		color-scheme: light;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.head h2 {
		margin: 0;
		font-size: 18px;
	}
	.close {
		margin-left: auto;
		font-size: 20px;
		line-height: 1;
		padding: 2px 10px;
	}
	.choice {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}
	.field {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.period {
		margin-left: auto;
		font-size: 13px;
		opacity: 0.8;
	}

	.print {
		font-weight: 700;
	}
	.preview {
		flex: 1;
		overflow: auto;
		background: #4b5563;
		border-radius: 8px;
		padding: 12px;
	}
	.pages {
		transform-origin: top left;
	}
	.pages > div {
		display: flex;
		flex-direction: column;
		gap: 7mm;
	}
	.bad {
		color: #b91c1c;
		margin: 0;
	}
	.loading {
		color: #fff;
		margin: 0 0 8px;
	}
</style>
