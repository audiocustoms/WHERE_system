<script lang="ts">
	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type HolidayCounts,
		type HolidayImportReport,
		type HorizonUnit,
		type PlanningRules,
		type Shift,
		type ShiftBand,
		type ShiftInput
	} from '$core/api';
	import { auth } from '$core/auth.svelte';
	import { RIGHT } from '$core/rights';
	import AccessDenied from '$core/AccessDenied.svelte';
	import { askConfirm } from '$core/confirm.svelte';
	import ColorPicker from '$core/ColorPicker.svelte';
	import MasterExchangeDialog from './MasterExchangeDialog.svelte';

	/** The draft of one shift — numbers as text so empty fields stay possible. */
	interface ShiftDraft {
		id: number | null;
		name: string;
		short_name: string;
		starts_at: string;
		ends_at: string;
		break_minutes: string;
		band: ShiftBand;
		apprentice_only: boolean;
		color: string;
		department: string;
		active: boolean;
	}

	function emptyDraft(): ShiftDraft {
		return {
			id: null,
			name: '',
			short_name: '',
			starts_at: '08:00',
			ends_at: '16:00',
			break_minutes: '30',
			band: 'early',
			apprentice_only: false,
			color: '#1F6FEB',
			department: '',
			active: true
		};
	}

	function draftOf(shift: Shift): ShiftDraft {
		return {
			id: shift.id,
			name: shift.name,
			short_name: shift.short_name ?? '',
			starts_at: shift.starts_at,
			ends_at: shift.ends_at,
			break_minutes: String(shift.break_minutes),
			band: shift.band,
			apprentice_only: shift.apprentice_only,
			color: shift.color ?? '',
			department: shift.department ?? '',
			active: shift.active
		};
	}

	/** Numeric fields of the rules, each as text in the form. */	type AmountField =
		| 'min_rest_hours'
		| 'max_daily_hours'
		| 'max_weekly_hours'
		| 'break_after_hours'
		| 'break_minutes'
		| 'long_break_after_hours'
		| 'long_break_minutes'
		| 'max_consecutive_days'
		| 'min_consecutive_days'
		| 'shift_rotation_days'
		| 'min_days_off_per_week'
		| 'planning_horizon'
		| 'vacation_day_hours'
		| 'sick_day_hours';

	type ToggleField =
		| 'sunday_work_allowed'
		| 'compensate_sunday'
		| 'extended_sunday_rest'
		| 'free_weekend_every_six_weeks'
		| 'apprentice_needs_company'
		| 'import_public_holidays'
		| 'import_regional_holidays'
		| 'import_school_holidays'
		| 'show_holidays'
		| 'show_school_holidays'
		| 'holiday_work_allowed'
		| 'respect_absence';

	/** Color fields of the rules. */
	type ColorField = 'vacation_color' | 'sick_color' | 'holiday_color' | 'school_holiday_color';

	/** The rules as a draft — numbers as text like in the personnel form. */
	type RulesDraft = Record<AmountField, string> &
		Record<ColorField, string> &
		Record<ToggleField, boolean> & {
			planning_horizon_unit: HorizonUnit;
			/** `YYYY-MM-DD`; empty counts from 1 January. */
			balance_from: string;
			/** Code of the federal state (`BY`, …); empty = none. */
			holiday_state: string;
		};

	interface AmountFieldLabel {
		key: AmountField;
		label: string;
		unit: string;
		min: number;
		max: number;
		/** Step of the input; decimals need `0.25` instead of `1`. */
		step?: string;
	}

	interface ColorFieldLabel {
		key: ColorField;
		label: string;
	}

	interface RuleGroup {
		title: string;
		fields: AmountFieldLabel[];
		colors?: ColorFieldLabel[];
	}

	const ruleGroups: RuleGroup[] = [
		{
			title: 'Ruhezeit und Arbeitszeit',
			fields: [
				{
					key: 'min_rest_hours',
					label: 'Ruhezeit zwischen zwei Schichten',
					unit: 'Std.',
					min: 0,
					max: 24
				},
				{
					key: 'max_daily_hours',
					label: 'Höchstarbeitszeit pro Tag',
					unit: 'Std.',
					min: 1,
					max: 24
				},
				{
					key: 'max_weekly_hours',
					label: 'Höchstarbeitszeit pro Woche',
					unit: 'Std.',
					min: 1,
					max: 168
				},
				{
					key: 'max_consecutive_days',
					label: 'Max. Arbeitstage in Folge',
					unit: 'Tage',
					min: 1,
					max: 31
				},
				{
					key: 'min_consecutive_days',
					label: 'Min. Arbeitstage in Folge',
					unit: 'Tage',
					min: 1,
					max: 31
				},
				{
					key: 'shift_rotation_days',
					label: 'Schichtwechsel nach',
					unit: 'Tage',
					min: 1,
					max: 365
				},
				{
					key: 'min_days_off_per_week',
					label: 'freie Tage pro Woche',
					unit: 'Tage',
					min: 0,
					max: 7
				}
			]
		},
		{
			title: 'Pausen',
			fields: [
				{
					key: 'break_after_hours',
					label: 'Pause ab Arbeitszeit',
					unit: 'Std.',
					min: 1,
					max: 24
				},
				{
					key: 'break_minutes',
					label: 'Dauer der Pause',
					unit: 'Min.',
					min: 0,
					max: 240
				},
				{
					key: 'long_break_after_hours',
					label: 'lange Pause ab Arbeitszeit',
					unit: 'Std.',
					min: 1,
					max: 24
				},
				{
					key: 'long_break_minutes',
					label: 'Dauer der langen Pause',
					unit: 'Min.',
					min: 0,
					max: 240
				}
			]
		},
		{
			title: 'Urlaub und Krankheit',
			fields: [
				{
					key: 'vacation_day_hours',
					label: '1 Urlaubstag entspricht',
					unit: 'Std.',
					min: 0,
					max: 24,
					step: '0.25'
				},
				{
					key: 'sick_day_hours',
					label: '1 Krankheitstag entspricht',
					unit: 'Std.',
					min: 0,
					max: 24,
					step: '0.25'
				}
			],
			colors: [
				{ key: 'vacation_color', label: 'Farbe Urlaub' },
				{ key: 'sick_color', label: 'Farbe Krank' }
			]
		}
	];

	const toggles: { key: ToggleField; label: string }[] = [
		{ key: 'sunday_work_allowed', label: 'Sonntagsarbeit erlaubt' },
		{
			key: 'free_weekend_every_six_weeks',
			label: '6-Tage-Woche: alle 6 Wochen ein ganzes Wochenende (Sa + So) frei'
		},
		{
			key: 'apprentice_needs_company',
			label: 'Auszubildende/r nie allein in einer Schicht'
		},
		{ key: 'holiday_work_allowed', label: 'Feiertagsarbeit erlaubt' },
		{
			key: 'respect_absence',
			label: 'Urlaub und Abwesenheiten berücksichtigen'
		}
	];

	let shifts = $state<Shift[]>([]);
	/** The departments offered for `Abteilung` of a shift. */
	let departments = $state<{ id: number; name: string }[]>([]);
	/** Open window `Import / Export` of the shifts. */
	let exchange = $state(false);
	let draft = $state<ShiftDraft | null>(null);
	let rules = $state<RulesDraft | null>(null);
	let loading = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);
	let note = $state<string | null>(null);

	/** The federal states as the API knows them. */
	const FEDERAL_STATES: { code: string; name: string }[] = [
		{ code: 'BW', name: 'Baden-Württemberg' },
		{ code: 'BY', name: 'Bayern' },
		{ code: 'BE', name: 'Berlin' },
		{ code: 'BB', name: 'Brandenburg' },
		{ code: 'HB', name: 'Bremen' },
		{ code: 'HH', name: 'Hamburg' },
		{ code: 'HE', name: 'Hessen' },
		{ code: 'MV', name: 'Mecklenburg-Vorpommern' },
		{ code: 'NI', name: 'Niedersachsen' },
		{ code: 'NW', name: 'Nordrhein-Westfalen' },
		{ code: 'RP', name: 'Rheinland-Pfalz' },
		{ code: 'SL', name: 'Saarland' },
		{ code: 'SN', name: 'Sachsen' },
		{ code: 'ST', name: 'Sachsen-Anhalt' },
		{ code: 'SH', name: 'Schleswig-Holstein' },
		{ code: 'TH', name: 'Thüringen' }
	];

	/** Year of the holiday import (the settings only say *what* is imported). */
	let importYear = $state(String(new Date().getFullYear()));
	let holidayRun = $state(false);
	let holidayMessage = $state<string | null>(null);
	let holidayError = $state<string | null>(null);
	/** What the last import stored — the display switches need it. */
	let imported = $state<HolidayCounts>({ holidays: 0, school_holidays: 0 });

	/**
	 * The right of the personal disposition opens the page; `anlegen` brings new
	 * shifts, `editieren` changes the existing ones — per department of the
	 * permission group (the API checks both). The rules of the planning belong to
	 * the whole house and stay with the administration.
	 */
	const mayRead = $derived(auth.isAdmin || auth.hasRight(RIGHT.mayReadDispo));
	const mayCreate = $derived(auth.isAdmin || auth.hasRight(RIGHT.mayCreateDispo));
	const mayChange = $derived(auth.isAdmin || auth.hasRight(RIGHT.mayEditDispo));
	/** Only holders of the administrative right may edit (the API checks it as well). */
	const mayEdit = $derived(auth.isAdmin);

	function rulesFrom(stored: PlanningRules): RulesDraft {
		return {
			min_rest_hours: String(stored.min_rest_hours),
			max_daily_hours: String(stored.max_daily_hours),
			max_weekly_hours: String(stored.max_weekly_hours),
			break_after_hours: String(stored.break_after_hours),
			break_minutes: String(stored.break_minutes),
			long_break_after_hours: String(stored.long_break_after_hours),
			long_break_minutes: String(stored.long_break_minutes),
			max_consecutive_days: String(stored.max_consecutive_days),
			min_consecutive_days: String(stored.min_consecutive_days),
			shift_rotation_days: String(stored.shift_rotation_days),
			min_days_off_per_week: String(stored.min_days_off_per_week),
			planning_horizon: String(stored.planning_horizon),
			vacation_day_hours: String(stored.vacation_day_hours),
			sick_day_hours: String(stored.sick_day_hours),
			vacation_color: stored.vacation_color,
			sick_color: stored.sick_color,
			holiday_color: stored.holiday_color,
			school_holiday_color: stored.school_holiday_color,
			balance_from: stored.balance_from ?? '',
			planning_horizon_unit: stored.planning_horizon_unit,
			sunday_work_allowed: stored.sunday_work_allowed,
			compensate_sunday: stored.compensate_sunday,
			extended_sunday_rest: stored.extended_sunday_rest,
			free_weekend_every_six_weeks: stored.free_weekend_every_six_weeks,
			apprentice_needs_company: stored.apprentice_needs_company,
			import_public_holidays: stored.import_public_holidays,
			import_regional_holidays: stored.import_regional_holidays,
			import_school_holidays: stored.import_school_holidays,
			show_holidays: stored.show_holidays,
			show_school_holidays: stored.show_school_holidays,
			holiday_state: stored.holiday_state ?? '',
			holiday_work_allowed: stored.holiday_work_allowed,
			respect_absence: stored.respect_absence
		};
	}

	function amount(label: string): number {
		const value = Number(label.trim().replace(',', '.'));
		return Number.isFinite(value) ? value : 0;
	}

	/**
	 * 24-hour time: `6`, `630`, `06.30` or `6:3` become `HH:MM`.
	 * Anything that cannot be parsed stays as typed — the API then answers with 422.
	 */
	function timeText(label: string): string {
		const raw = label.trim().replace(/[.,;\s]+/g, ':');
		const parts = raw.split(':').filter((part) => part !== '');
		let hours: number;
		let minutes: number;
		if (parts.length === 1) {
			const digits = parts[0].replace(/\D/g, '');
			if (digits === '') return label;
			if (digits.length === 3) {
				hours = Number(digits.slice(0, 1));
				minutes = Number(digits.slice(1));
			} else if (digits.length === 4) {
				hours = Number(digits.slice(0, 2));
				minutes = Number(digits.slice(2));
			} else if (digits.length <= 2) {
				hours = Number(digits);
				minutes = 0;
			} else {
				return label;
			}
		} else if (parts.length === 2) {
			hours = Number(parts[0]);
			minutes = Number(parts[1]);
		} else {
			return label;
		}
		if (!Number.isInteger(hours) || !Number.isInteger(minutes)) return label;
		if (hours < 0 || hours > 23 || minutes < 0 || minutes > 59) return label;
		return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`;
	}

	/** `HH:MM` in 24-hour format. */
	function isTime(label: string): boolean {
		return /^([01][0-9]|2[0-3]):[0-5][0-9]$/.test(label);
	}

	function rulesForApi(draft: RulesDraft): PlanningRules {
		return {
			min_rest_hours: amount(draft.min_rest_hours),
			max_daily_hours: amount(draft.max_daily_hours),
			max_weekly_hours: amount(draft.max_weekly_hours),
			break_after_hours: amount(draft.break_after_hours),
			break_minutes: amount(draft.break_minutes),
			long_break_after_hours: amount(draft.long_break_after_hours),
			long_break_minutes: amount(draft.long_break_minutes),
			max_consecutive_days: amount(draft.max_consecutive_days),
			min_consecutive_days: amount(draft.min_consecutive_days),
			shift_rotation_days: amount(draft.shift_rotation_days),
			min_days_off_per_week: amount(draft.min_days_off_per_week),
			planning_horizon: amount(draft.planning_horizon),
			// Decimal values go as text so that no rounding happens.
			vacation_day_hours: String(amount(draft.vacation_day_hours)),
			sick_day_hours: String(amount(draft.sick_day_hours)),
			vacation_color: draft.vacation_color,
			sick_color: draft.sick_color,
			holiday_color: draft.holiday_color,
			school_holiday_color: draft.school_holiday_color,
			balance_from: draft.balance_from === '' ? null : draft.balance_from,
			planning_horizon_unit: draft.planning_horizon_unit,
			sunday_work_allowed: draft.sunday_work_allowed,
			compensate_sunday: draft.sunday_work_allowed && draft.compensate_sunday,
			extended_sunday_rest: draft.sunday_work_allowed && draft.extended_sunday_rest,
			free_weekend_every_six_weeks: draft.free_weekend_every_six_weeks,
			apprentice_needs_company: draft.apprentice_needs_company,
			import_public_holidays: draft.import_public_holidays,
			import_regional_holidays: draft.import_regional_holidays,
			import_school_holidays: draft.import_school_holidays,
			show_holidays: draft.show_holidays,
			show_school_holidays: draft.show_school_holidays,
			holiday_state: draft.holiday_state === '' ? null : draft.holiday_state,
			holiday_work_allowed: draft.holiday_work_allowed,
			respect_absence: draft.respect_absence
		};
	}

	async function load() {
		loading = true;
		error = null;
		try {
			[shifts, rules, imported] = await Promise.all([
				api.people.shifts.list(),
				api.people.planning.get().then(rulesFrom),
				api.people.holidays.summary()
			]);
			if (!departments.length) {
				departments = (await api.people.lists.list('departments')).map((entry) => ({
					id: entry.id,
					name: entry.name
				}));
			}
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		if (mayRead) void load();
	});

	function createShift() {
		draft = emptyDraft();
		error = null;
		note = null;
	}

	function edit(shift: Shift) {
		draft = draftOf(shift);
		error = null;
		note = null;
	}

	async function saveShift() {
		if (draft === null) return;
		// A free day keeps no times and no break: the mask shows none of them.
		if (draft.band === 'free') {
			draft.starts_at = '00:00';
			draft.ends_at = '23:59';
			draft.break_minutes = '0';
			draft.apprentice_only = false;
		}
		const start = timeText(draft.starts_at);
		const end = timeText(draft.ends_at);
		if (draft.band !== 'free' && (!isTime(start) || !isTime(end))) {
			error = 'Bitte Beginn und Ende im 24-Stunden-Format angeben, z. B. 06:00.';
			return;
		}
		draft.starts_at = start;
		draft.ends_at = end;
		const body: ShiftInput = {
			name: draft.name.trim(),
			short_name: draft.short_name.trim() === '' ? null : draft.short_name.trim(),
			starts_at: start,
			ends_at: end,
			break_minutes: amount(draft.break_minutes),
			band: draft.band,
			apprentice_only: draft.apprentice_only,
			color: draft.color.trim() === '' ? null : draft.color.trim(),
			department: draft.department.trim() === '' ? null : draft.department.trim(),
			active: draft.active
		};
		saving = true;
		error = null;
		note = null;
		try {
			if (draft.id === null) {
				await api.people.shifts.create(body);
			} else {
				await api.people.shifts.update(draft.id, body);
			}
			draft = null;
			await load();
			note = 'Schicht gespeichert.';
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	async function removeShift(shift: Shift) {
		if (
			!(await askConfirm({
				message: `Schicht „${shift.name}“ entfernen?`,
				confirmLabel: 'Entfernen'
			}))
		)
			return;
		saving = true;
		error = null;
		try {
			await api.people.shifts.remove(shift.id);
			if (draft?.id === shift.id) draft = null;
			await load();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** Swaps the shift with its neighbor and saves the new order. */
	async function move(index: number, direction: -1 | 1) {
		const target = index + direction;
		if (target < 0 || target >= shifts.length) return;
		const next = [...shifts];
		[next[index], next[target]] = [next[target], next[index]];
		shifts = next;
		error = null;
		try {
			shifts = await api.people.shifts.reorder(next.map((shift) => shift.id));
		} catch (e) {
			error = errorText(e);
			await load();
		}
	}

	async function saveRules() {
		if (rules === null) return;
		saving = true;
		error = null;
		note = null;
		try {
			rules = rulesFrom(await api.people.planning.save(rulesForApi(rules)));
			note = 'Regeln gespeichert.';
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** Whether the chosen imports need a federal state. */
	const needsFederalState = $derived(
		rules !== null && (rules.import_regional_holidays || rules.import_school_holidays)
	);

	/** The German name of a federal state code. */
	function federalStateName(code: string | null): string | null {
		if (code === null) return null;
		return FEDERAL_STATES.find((entry) => entry.code === code)?.name ?? code;
	}

	/** What one import run brought, in one German line. */
	function reportText(report: HolidayImportReport): string {
		const state = federalStateName(report.state);
		const parts = [
			`${report.national_holidays} gesetzliche Feiertage`,
			`${report.regional_holidays} regionale Feiertage`,
			`${report.school_holidays} Ferienzeiträume`
		];
		return `${report.year}: ${parts.join(', ')} geladen${state ? ` (${state})` : ''}.`;
	}

	/**
	 * Saves the settings and reads the selected lists of the year from the
	 * source. The addresses are fixed in the API (`openholidaysapi.org`), so the
	 * operator only presses the button.
	 */
	async function loadHolidays() {
		if (rules === null || !mayEdit) return;
		holidayMessage = null;
		holidayError = null;
		if (needsFederalState && rules.holiday_state === '') {
			holidayError = 'Regionale Feiertage und Ferien brauchen ein Bundesland.';
			return;
		}
		const year = amount(importYear);
		if (!Number.isInteger(year) || year < 2000 || year > 2100) {
			holidayError = 'Bitte ein Jahr zwischen 2000 und 2100 angeben.';
			return;
		}
		holidayRun = true;
		try {
			rules = rulesFrom(await api.people.planning.save(rulesForApi(rules)));
			const report = await api.people.holidays.importYear(year);
			holidayMessage = reportText(report);
			if (report.errors.length > 0) holidayError = report.errors.join(' ');
			imported = await api.people.holidays.summary();
		} catch (e) {
			holidayError = errorText(e);
		} finally {
			holidayRun = false;
		}
	}

	/** A free day is no work at all — the entry only marks the day. */
	function isFree(shift: { band: ShiftBand }): boolean {
		return shift.band === 'free';
	}

	/** `06:00 – 14:00`; an end before the start gets the day change appended. */
	function time(shift: Shift): string {
		if (isFree(shift)) return 'ganzer Tag';
		const dayChange = shift.ends_at <= shift.starts_at ? ' (+1 Tag)' : '';
		return `${shift.starts_at} – ${shift.ends_at}${dayChange}`;
	}

	/** The band of the shift as the list shows it. */
	function bandLabel(band: ShiftBand): string {
		if (band === 'late') return 'Spät';
		if (band === 'intermediate') return 'Zwischen';
		if (band === 'free') return 'Freier Tag';
		return 'Früh';
	}

	/** Working hours of the shift: the span minus its break. */
	function hours(shift: Shift): string {
		if (isFree(shift)) return '0,00 h';
		const [startHour, startMinute] = shift.starts_at.split(':').map(Number);
		const [endHour, endMinute] = shift.ends_at.split(':').map(Number);
		let minutes = endHour * 60 + endMinute - (startHour * 60 + startMinute);
		if (minutes <= 0) minutes += 24 * 60;
		const net = Math.max(0, minutes - shift.break_minutes);
		return `${(net / 60).toLocaleString('de-DE', {
			minimumFractionDigits: 2,
			maximumFractionDigits: 2
		})} h`;
	}

	const headColumns = $derived(mayChange ? 7 : 6);
</script>

<svelte:head><title>Dienstplan — WHERE</title></svelte:head>

{#if !mayRead}
	<AccessDenied pageName="Dienstplan" right={RIGHT.mayReadDispo} rightLabel="Personaldispo lesen" />
{:else}
	<div class="head-row">
		<h1>Dienstplan</h1>
		{#if auth.isAdmin}
			<button type="button" class="exchange" onclick={() => (exchange = true)}>Import / Export</button>
		{/if}
	</div>

	{#if error}<p class="bad">{error}</p>{/if}
	{#if note}<p class="good">{note}</p>{/if}

	<section class="card">
		<header>
			<h2>Schichten ({shifts.length})</h2>
			{#if mayCreate}
				<button type="button" class="primary" onclick={createShift} disabled={saving}>
					Schicht +
				</button>
			{/if}
		</header>

		{#if draft !== null}
			<form
				class="editor"
				onsubmit={(event) => {
					event.preventDefault();
					saveShift();
				}}
			>
				<h3>{draft.id === null ? 'Neue Schicht' : `Schicht „${draft.name}“`}</h3>
				<div class="fields">
					<label>
						<span>Name</span>
						<input
							bind:value={draft.name}
							maxlength="100"
							required
							placeholder="Frühschicht"
						/>
					</label>
					<label>
						<span>Kürzel</span>
						<input bind:value={draft.short_name} maxlength="10" placeholder="F" />
					</label>
					<label>
						<span>Von (24 h)</span>
						<input
							type="text"
							inputmode="numeric"
							autocomplete="off"
							maxlength="5"
							placeholder="HH:MM"
							title="Uhrzeit im 24-Stunden-Format, z. B. 06:00"
							bind:value={draft.starts_at}
							onblur={(event) => (draft!.starts_at = timeText(event.currentTarget.value))}
							disabled={draft.band === 'free'}
							required={draft.band !== 'free'}
						/>
					</label>
					<label>
						<span>Bis (24 h)</span>
						<input
							type="text"
							inputmode="numeric"
							autocomplete="off"
							maxlength="5"
							placeholder="HH:MM"
							title="Uhrzeit im 24-Stunden-Format, z. B. 14:30"
							bind:value={draft.ends_at}
							onblur={(event) => (draft!.ends_at = timeText(event.currentTarget.value))}
							disabled={draft.band === 'free'}
							required={draft.band !== 'free'}
						/>
					</label>
					<label>
						<span>Pause (Min.)</span>
						<input
							type="number"
							inputmode="numeric"
							min="0"
							max="720"
							step="1"
							value={draft.break_minutes}
							oninput={(event) => (draft!.break_minutes = event.currentTarget.value)}
							disabled={draft.band === 'free'}
						/>
					</label>
					<label>
						<span>Art</span>
						<select bind:value={draft.band}>
							<option value="frueh">Früh</option>
							<option value="spaet">Spät</option>
							<option value="zwischen">Zwischen</option>
							<option value="frei">Freier Tag</option>
						</select>
					</label>
					<label>
						<span>Abteilung</span>
						<select bind:value={draft.department}>
							<option value="">—</option>
							{#each departments as entry (entry.id)}
								<option value={entry.name}>{entry.name}</option>
							{/each}
						</select>
					</label>
					<label>
						<span>Farbe</span>
						<ColorPicker bind:value={draft.color} label="Farbe der Schicht" />
					</label>
					<label class="switch">
						<input
							type="checkbox"
							bind:checked={draft.apprentice_only}
							disabled={draft.band === 'free'}
						/>
						<span>nur Azubis</span>
					</label>
					<label class="switch">
						<input type="checkbox" bind:checked={draft.active} />
						<span>aktiv</span>
					</label>
				</div>
				<footer>
					<button type="submit" class="primary" disabled={saving}>
						{saving ? 'Sichere …' : 'Sichern'}
					</button>
					<button type="button" onclick={() => (draft = null)}>Abbrechen</button>
					<span class="hint"
						>Ein Ende vor dem Beginn gilt als Schicht über Mitternacht. Uhrzeiten im
						24-Stunden-Format.</span
					>
				</footer>
			</form>
		{/if}

		<table>
			<thead>
				<tr>
					<th>Schicht</th>
					<th>Kürzel</th>
					<th>Zeit</th>
					<th class="amount">Pause</th>
					<th class="amount">Stunden</th>
					<th>Status</th>
					{#if mayChange}<th></th>{/if}
				</tr>
			</thead>
			<tbody>
				{#each shifts as shift, i (shift.id)}
					<tr class:inactive={!shift.active}>
						<td>
							{#if shift.color}
								<span class="color" style={`background:${shift.color}`} aria-hidden="true"></span>
							{/if}
							{shift.name}
						</td>
						<td>{shift.short_name ?? '—'}</td>
						<td class="amount">{time(shift)}</td>
						<td class="amount">{shift.break_minutes} Min.</td>
						<td class="amount">{hours(shift)}</td>
						<td>
							{shift.active ? 'active' : 'inaktiv'}
							<span class="kind">{bandLabel(shift.band)}</span>
							{#if shift.department}<span class="kind">{shift.department}</span>{/if}
							{#if shift.apprentice_only}<span class="kind">nur Azubis</span>{/if}
						</td>
						{#if mayChange}
							<td class="actions">
								{#if mayEdit}
									<button
										type="button"
										class="arrow"
										title="nach oben"
										aria-label={`${shift.name} nach oben`}
										disabled={i === 0}
										onclick={() => move(i, -1)}>↑</button
									>
									<button
										type="button"
										class="arrow"
										title="nach unten"
										aria-label={`${shift.name} nach unten`}
										disabled={i === shifts.length - 1}
										onclick={() => move(i, 1)}>↓</button
									>
								{/if}
								<button type="button" onclick={() => edit(shift)}>edit</button>
								<button type="button" class="danger" onclick={() => removeShift(shift)}>
									entfernen
								</button>
							</td>
						{/if}
					</tr>
				{:else}
					<tr>
						<td colspan={headColumns}>
							{loading ? 'Lade …' : 'Noch keine Schicht erfasst.'}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</section>

	{#if rules !== null}
		<form
			class="card"
			onsubmit={(event) => {
				event.preventDefault();
				saveRules();
			}}
		>
			<header>
				<h2>Regeln der Planung</h2>
			</header>

			<div class="groups">
				{#each ruleGroups as group (group.title)}
					<fieldset>
						<legend>{group.title}</legend>
						{#each group.fields as field (field.key)}
							<label>
								<span>{field.label}</span>
								<span class="input">
									<input
										type="number"
										inputmode={field.step ? 'decimal' : 'numeric'}
										min={field.min}
										max={field.max}
										step={field.step ?? '1'}
										value={rules[field.key]}
										disabled={!mayEdit}
										oninput={(event) => (rules![field.key] = event.currentTarget.value)}
									/>
									<span class="unit">{field.unit}</span>
								</span>
							</label>
						{/each}
						{#each group.colors ?? [] as color (color.key)}
							<label>
								<span>{color.label}</span>
								<span class="input">
									<ColorPicker
										bind:value={rules[color.key]}
										label={color.label}
										disabled={!mayEdit}
									/>
								</span>
							</label>
						{/each}
					</fieldset>
				{/each}

				<fieldset>
					<legend>Planung</legend>
					<label>
						<span>Planungshorizont</span>
						<span class="input">
							<input
								type="number"
								inputmode="numeric"
								min="1"
								max={rules.planning_horizon_unit === 'months' ? 12 : 52}
								step="1"
								value={rules.planning_horizon}
								disabled={!mayEdit}
								oninput={(event) => (rules!.planning_horizon = event.currentTarget.value)}
							/>
							<select
								aria-label="Einheit des Planungshorizonts"
								disabled={!mayEdit}
								value={rules.planning_horizon_unit}
								onchange={(event) => {
									const unit = event.currentTarget.value as HorizonUnit;
									rules!.planning_horizon_unit = unit;
									const limit = unit === 'months' ? 12 : 52;
									if (amount(rules!.planning_horizon) > limit) {
										rules!.planning_horizon = String(limit);
									}
								}}
							>
								<option value="weeks">Wochen</option>
								<option value="months">Monate</option>
							</select>
						</span>
					</label>
				</fieldset>

				<fieldset>
					<legend>Stundenkonto</legend>
					<label>
						<span>Über-/Minusstunden zählen ab</span>
						<span class="input">
							<input
								type="date"
								value={rules.balance_from}
								disabled={!mayEdit}
								oninput={(event) => (rules!.balance_from = event.currentTarget.value)}
							/>
							{#if rules.balance_from !== '' && mayEdit}
								<button
									type="button"
									class="clear"
									title="ab 1. Januar zählen"
									onclick={() => (rules!.balance_from = '')}>×</button
								>
							{/if}
						</span>
					</label>
				</fieldset>

				<fieldset>
					<legend>Woche und Abwesenheiten</legend>
					{#each toggles as entry (entry.key)}
						<label class="switch">
							<input
								type="checkbox"
								checked={rules[entry.key]}
								disabled={!mayEdit}
								onchange={(event) => {
									const checked = event.currentTarget.checked;
									rules![entry.key] = checked;
									// Sunday work is compensated and followed by the long rest by
									// default; both options can be switched off afterwards.
									if (entry.key === 'sunday_work_allowed' && checked) {
										rules!.compensate_sunday = true;
										rules!.extended_sunday_rest = true;
									}
								}}
							/>
							<span>{entry.label}</span>
						</label>
						{#if entry.key === 'sunday_work_allowed'}
							<label class="switch subOption">
								<input
									type="checkbox"
									checked={rules.compensate_sunday}
									disabled={!mayEdit || !rules.sunday_work_allowed}
									onchange={(event) => (rules!.compensate_sunday = event.currentTarget.checked)}
								/>
								<span>Kompensation</span>
							</label>
							<label
								class="switch subOption"
								title="nach einem Sonntagdienst einen Ersatzruhetag: 24 Stunden frei, dazu die normale Ruhezeit zum nächsten Dienst"
							>
								<input
									type="checkbox"
									checked={rules.extended_sunday_rest}
									disabled={!mayEdit || !rules.sunday_work_allowed}
									onchange={(event) =>
										(rules!.extended_sunday_rest = event.currentTarget.checked)}
								/>
								<span>24 + 11 Stunden Ruhezeit</span>
							</label>
						{/if}
					{/each}
				</fieldset>

				<fieldset>
					<legend>Feiertage und Ferien</legend>
					<label class="switch">
						<input
							type="checkbox"
							checked={rules.import_public_holidays}
							disabled={!mayEdit}
							onchange={(event) =>
								(rules!.import_public_holidays = event.currentTarget.checked)}
						/>
						<span>Importiere gesetzliche Feiertage</span>
					</label>
					<label class="switch">
						<input
							type="checkbox"
							checked={rules.import_regional_holidays}
							disabled={!mayEdit}
							onchange={(event) =>
								(rules!.import_regional_holidays = event.currentTarget.checked)}
						/>
						<span>Importiere regionale Feiertage nach Bundesland</span>
					</label>
					<label class="switch">
						<input
							type="checkbox"
							checked={rules.import_school_holidays}
							disabled={!mayEdit}
							onchange={(event) =>
								(rules!.import_school_holidays = event.currentTarget.checked)}
						/>
						<span>Importiere Ferien nach Bundesland</span>
					</label>
					<label class="switch">
						<input
							type="checkbox"
							checked={rules.show_holidays}
							disabled={!mayEdit || imported.holidays === 0}
							title={imported.holidays === 0 ? 'Noch keine Feiertage importiert' : undefined}
							onchange={(event) => (rules!.show_holidays = event.currentTarget.checked)}
						/>
						<span>Feiertage anzeigen</span>
					</label>
					<label class="switch">
						<input
							type="checkbox"
							checked={rules.show_school_holidays}
							disabled={!mayEdit || imported.school_holidays === 0}
							title={imported.school_holidays === 0 ? 'Noch keine Ferien importiert' : undefined}
							onchange={(event) => (rules!.show_school_holidays = event.currentTarget.checked)}
						/>
						<span>Ferien anzeigen</span>
					</label>
					<label>
						<span>Bundesland</span>
						<span class="input">
							<select
								aria-label="Bundesland"
								disabled={!mayEdit || !needsFederalState}
								value={rules.holiday_state}
								onchange={(event) => (rules!.holiday_state = event.currentTarget.value)}
							>
								<option value="">—</option>
								{#each FEDERAL_STATES as state (state.code)}
									<option value={state.code}>{state.name}</option>
								{/each}
							</select>
						</span>
					</label>
					<label>
						<span>Jahr</span>
						<span class="input">
							<input
								type="number"
								inputmode="numeric"
								min="2000"
								max="2100"
								step="1"
								value={importYear}
								disabled={!mayEdit}
								oninput={(event) => (importYear = event.currentTarget.value)}
							/>
							<button
								type="button"
								class="primary"
								disabled={!mayEdit || holidayRun}
								onclick={loadHolidays}
							>
								{holidayRun ? 'Lade …' : 'Prüfen und laden'}
							</button>
						</span>
					</label>
					<label>
						<span>Farbe Feiertag</span>
						<span class="input">
							<ColorPicker
								bind:value={rules.holiday_color}
								label="Farbe der Feiertage im Dienstplan"
								disabled={!mayEdit}
							/>
						</span>
					</label>
					<label>
						<span>Farbe Ferien</span>
						<span class="input">
							<ColorPicker
								bind:value={rules.school_holiday_color}
								label="Farbe der Ferien im Dienstplan"
								disabled={!mayEdit}
							/>
						</span>
					</label>
					<p class="hint">
						Quelle: openholidaysapi.org — wird beim Prüfen abgerufen, Adressen müssen nicht
						eingetragen werden.
					</p>
					{#if holidayMessage}<p class="good">{holidayMessage}</p>{/if}
					{#if holidayError}<p class="bad">{holidayError}</p>{/if}
				</fieldset>
			</div>

			{#if mayEdit}
				<footer>
					<button type="submit" class="primary" disabled={saving}>
						{saving ? 'Sichere …' : 'Regeln sichern'}
					</button>
					<button type="button" onclick={load} disabled={saving}>Verwerfen</button>
				</footer>
			{/if}
		</form>
	{/if}

	{#if exchange}
		<MasterExchangeDialog
			area="shifts"
			title="Dienstplan"
			filename="schichten"
			onimported={load}
			onclose={() => (exchange = false)}
		/>
	{/if}
{/if}

<style>
	.head-row {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.exchange {
		margin-left: auto;
	}
	.card {
		margin-top: 1rem;
		padding: 0.9rem 1rem 1rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 0.6rem;
	}
	h2 {
		font-size: 1.05rem;
		margin: 0;
	}
	h3 {
		font-size: 0.95rem;
		margin: 0 0 0.6rem;
	}
	button {
		padding: 0.4rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
	button.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}
	button:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.editor {
		border: 1px solid var(--surface-3);
		border-radius: 6px;
		padding: 0.75rem;
		margin-bottom: 0.8rem;
		background: var(--surface-2, var(--surface));
	}
	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
		gap: 0.5rem 0.75rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		font-size: 0.85rem;
	}
	label > span:first-child {
		color: var(--muted);
	}
	.kind {
		margin-left: 0.4rem;
		padding: 0.05rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 999px;
		font-size: 0.7rem;
		color: var(--muted);
		white-space: nowrap;
	}
	input {
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font: inherit;
		background: var(--surface);
	}
	select {
		padding: 0.35rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		font: inherit;
		background: var(--surface);
	}
	.editor footer {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.7rem;
	}
	.hint {
		color: var(--muted);
		font-size: 0.8rem;
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		text-align: left;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.9rem;
	}
	th {
		color: var(--muted);
		font-weight: 600;
	}
	td.amount,
	th.amount {
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	tr.inactive > td {
		color: var(--muted-2);
	}
	.color {
		display: inline-block;
		width: 0.65rem;
		height: 0.65rem;
		border-radius: 2px;
		margin-right: 0.35rem;
		vertical-align: baseline;
	}
	.actions {
		white-space: nowrap;
		text-align: right;
	}
	.actions button {
		padding: 0.2rem 0.5rem;
		font-size: 0.8rem;
	}
	.actions button.arrow {
		padding: 0.2rem 0.35rem;
		min-width: 1.6rem;
		line-height: 1;
	}
	.actions button:disabled {
		color: var(--muted-2);
	}
	.danger {
		color: var(--bad);
	}
	.bad {
		color: var(--bad);
		margin: 0.6rem 0 0;
		font-size: 0.85rem;
	}
	.good {
		color: var(--good, var(--accent));
		margin: 0.6rem 0 0;
		font-size: 0.85rem;
	}
	.groups {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
		gap: 0.8rem 1.2rem;
	}
	fieldset {
		border: 1px solid var(--surface-3);
		border-radius: 6px;
		padding: 0.6rem 0.8rem 0.8rem;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
	}
	legend {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--muted);
		padding: 0 0.3rem;
	}
	.input {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.input input {
		width: 6rem;
	}
	.input input[type='date'] {
		width: 9rem;
	}
	.clear {
		padding: 0 0.35rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--muted);
		cursor: pointer;
	}
	.clear:hover {
		border-color: var(--bad);
		color: var(--bad);
	}
	.unit {
		color: var(--muted);
		font-size: 0.8rem;
	}
	label.switch {
		flex-direction: row;
		align-items: flex-start;
		gap: 0.45rem;
	}
	label.switch input {
		margin-top: 0.2rem;
	}
	/* Sub-option of a setting, e.g. the compensation of Sunday work. */
	label.subOption {
		margin-left: 1.5rem;
	}
	label.subOption input:disabled + span {
		color: var(--muted-2);
	}
	label.switch span {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}
	form > footer {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.8rem;
	}
</style>
