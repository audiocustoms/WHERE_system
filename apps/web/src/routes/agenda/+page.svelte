<script lang="ts">
	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type AgendaBirthday,
		type AgendaCategory,
		type AgendaEvent,
		type AgendaOccurrence,
		type AgendaRange,
		type ModuleId
	} from '../../lib/api';
	import { modules } from '../../lib/modules.svelte';
	import AgendaDialog from '../../lib/agenda/AgendaDialog.svelte';
	import { askConfirm } from '../../lib/confirm.svelte';
	import { auth } from '../../lib/auth.svelte';
	import { RIGHT } from '../../lib/rights';
	import AccessDenied from '../../lib/AccessDenied.svelte';
	import {
		WEEKDAYS,
		addDays,
		addMonths,
		addYears,
		dayMonth,
		endOfMonth,
		endOfYear,
		isoDay,
		isoWeek,
		isWeekend,
		minutesOfDay,
		monthLabel,
		monthShort,
		sameDay,
		shortDate,
		startOfMonth,
		startOfWeek,
		startOfYear,
		timeRange
	} from '../../lib/agenda/dates';

	type View = 'tag' | 'woche' | 'monat' | 'jahr';

	const VIEWS: { value: View; label: string }[] = [
		{ value: 'tag', label: 'Tag' },
		{ value: 'woche', label: 'Woche' },
		{ value: 'monat', label: 'Monat' },
		{ value: 'jahr', label: 'Jahr' }
	];

	/** Views of the template that would stay empty without orders and personnel. */
	const LATER_VIEWS: { label: string; note: string; module?: ModuleId }[] = [
		{ label: 'Events', note: 'braucht die Aufträge', module: 'go' },
		{ label: 'Events 2', note: 'braucht die Aufträge', module: 'go' },
		{ label: 'Detail', note: 'braucht die Aufträge', module: 'go' },
		{ label: 'Artikel', note: 'braucht die Aufträge', module: 'go' },
		{ label: 'Personal', note: 'braucht das Personal', module: 'people' }
	];

	/** Data sources of the template that WHERE does not have yet. */
	const LATER_SOURCES: { label: string; note: string; module?: ModuleId }[] = [
		{ label: 'Aufgaben', note: 'Aufgaben gibt es noch nicht' },
		{ label: 'Angebote', note: 'Angebote gibt es noch nicht', module: 'go' },
		{ label: 'Aufträge', note: 'Aufträge gibt es noch nicht', module: 'go' },
		{ label: 'Personaleinsätze', note: 'Personal gibt es noch nicht', module: 'people' },
		{ label: 'Personalsperrzeiten', note: 'Personal gibt es noch nicht' }
	];

	/** Height of one hour in the time grid of day and week. */
	const HOUR = 38;
	const HOURS = Array.from({ length: 24 }, (_, index) => index);

	let view = $state<View>('monat');
	let anchor = $state(new Date());
	let data = $state<AgendaRange | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let search = $state('');

	/** Deselected calendars (`Storage_TACategory`). */
	let hidden = $state<number[]>([]);
	/** Birthdays from the address master and appointments without a calendar. */
	let withBirthdays = $state(true);
	let withOpenDates = $state(true);

	let dialog = $state<{ event: AgendaEvent | null; date: string; occurrence: string | null } | null>(
		null
	);

	const today = new Date();
	const nowMinutes = today.getHours() * 60 + today.getMinutes();

	const period = $derived.by(() => {
		if (view === 'tag') return { von: anchor, bis: anchor };
		if (view === 'woche') {
			const start = startOfWeek(anchor);
			return { von: start, bis: addDays(start, 6) };
		}
		if (view === 'jahr') return { von: startOfYear(anchor), bis: endOfYear(anchor) };
		return {
			von: startOfWeek(startOfMonth(anchor)),
			bis: addDays(startOfWeek(endOfMonth(anchor)), 6)
		};
	});

	/** Columns of the time grid: one day in the day view, seven in the week. */
	const days = $derived.by(() => {
		if (view === 'tag') return [anchor];
		const start = startOfWeek(anchor);
		return Array.from({ length: 7 }, (_, index) => addDays(start, index));
	});

	/** Rows of the month grid, Monday to Sunday, including the edge days. */
	const weeks = $derived.by(() => {
		const result: Date[][] = [];
		for (let day = period.von; day <= period.bis; day = addDays(day, 1)) {
			const week = result.at(-1);
			if (!week || week.length === 7) result.push([day]);
			else week.push(day);
		}
		return result;
	});

	/** Visible appointments: calendar filter and the two sources. */
	const termine = $derived.by(() =>
		(data?.occurrences ?? []).filter((appointment) =>
			appointment.category == null ? withOpenDates : !hidden.includes(appointment.category)
		)
	);

	const birthdays = $derived(withBirthdays ? (data?.birthdays ?? []) : []);

	const perDay = $derived.by(() => {
		const map = new Map<string, AgendaOccurrence[]>();
		for (const appointment of termine) {
			const list = map.get(appointment.date) ?? [];
			list.push(appointment);
			map.set(appointment.date, list);
		}
		return map;
	});

	const birthdaysPerDay = $derived.by(() => {
		const map = new Map<string, AgendaBirthday[]>();
		for (const birthday of birthdays) {
			const list = map.get(birthday.date) ?? [];
			list.push(birthday);
			map.set(birthday.date, list);
		}
		return map;
	});

	const visibleCategory = $derived.by(
		() => data?.categories.filter((calendar) => !hidden.includes(calendar.id)) ?? []
	);

	async function load() {
		loading = true;
		error = null;
		try {
			data = await api.agenda.range({
				from: isoDay(period.von),
				to: isoDay(period.bis),
				q: search.trim() || undefined
			});
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
		jumpToNow();
	}

	onMount(() => {
		void load();
		mini = startOfMonth(new Date());
	});

	function springe(target: Date) {
		anchor = target;
		void load();
	}

	/** One period further: a day, a week, a month or a year depending on the view. */
	function step(direction: number) {
		if (view === 'tag') springe(addDays(anchor, direction));
		else if (view === 'woche') springe(addDays(anchor, 7 * direction));
		else if (view === 'monat') springe(addMonths(anchor, direction));
		else springe(addYears(anchor, direction));
	}

	function toggle(createNew: View) {
		view = createNew;
		void load();
	}

	function toggleCalendar(id: number) {
		hidden = hidden.includes(id)
			? hidden.filter((x) => x !== id)
			: [...hidden, id];
	}

	function alleAn() {
		hidden = [];
		withBirthdays = true;
		withOpenDates = true;
	}

	function alleAus() {
		hidden = (data?.categories ?? []).map((calendar) => calendar.id);
		withBirthdays = false;
		withOpenDates = false;
	}

	function newAppointment(day: Date) {
		dialog = { event: null, date: isoDay(day), occurrence: null };
	}

	/** The overview knows only the short form; the mask needs the full record. */
	async function terminOeffnen(appointment: AgendaOccurrence) {
		try {
			dialog = {
				event: await api.agenda.get(appointment.event_id),
				date: appointment.date,
				occurrence: appointment.date
			};
		} catch (e) {
			error = errorText(e);
		}
	}

	function closed() {
		dialog = null;
		void load();
	}

	/** Colour of the appointment; without an own colour the tile stays neutral. */
	function appointmentStyle(appointment: AgendaOccurrence): string {
		if (!appointment.custom_color || appointment.color_red == null) return '';
		const rgb = `${appointment.color_red} ${appointment.color_green ?? 0} ${appointment.color_blue ?? 0}`;
		return `border-left-color: rgb(${rgb}); background: color-mix(in srgb, rgb(${rgb}) 12%, var(--surface));`;
	}

	/** Line under the title: time, suffix and address, as far as present. */
	function terminMeta(appointment: AgendaOccurrence): string {
		const parts: string[] = [];
		if (!appointment.all_day) parts.push(timeRange(appointment.start_time, appointment.end_time));
		if (appointment.end_date !== appointment.start_date) parts.push(`bis ${shortDate(appointment.end_date)}`);
		if (appointment.head_text) parts.push(appointment.head_text);
		if (appointment.address_name) parts.push(appointment.address_name);
		return parts.join(' · ');
	}

	function appointmentTitle(appointment: AgendaOccurrence): string {
		const parts = [appointment.title, appointment.head_text, appointment.address_name].filter(Boolean);
		return parts.join(' · ');
	}

	interface Block {
		appointment: AgendaOccurrence;
		top: number;
		height: number;
		links: number;
		width: number;
	}

	/**
	 * Appointments of one day as blocks in the time grid. Overlapping appointments share
	 * the width so a double booking stays visible.
	 */
	function bloecke(day: Date): Block[] {
		const list = (perDay.get(isoDay(day)) ?? []).filter((appointment) => !appointment.all_day);
		const ends: number[] = [];
		const assignment = list.map((appointment) => {
			const start = minutesOfDay(appointment.start_time);
			const end = Math.max(start + 30, minutesOfDay(appointment.end_time) || start + 60);
			let spur = ends.findIndex((free) => free <= start);
			if (spur < 0) {
				spur = ends.length;
				ends.push(end);
			} else {
				ends[spur] = end;
			}
			return { appointment, start, end, spur };
		});
		const columns = Math.max(1, ends.length);
		return assignment.map(({ appointment, start, end, spur }) => ({
			appointment,
			top: (start * HOUR) / 60,
			height: Math.max(20, ((end - start) * HOUR) / 60),
			links: spur / columns,
			width: 1 / columns
		}));
	}

	function allDay(day: Date): AgendaOccurrence[] {
		return (perDay.get(isoDay(day)) ?? []).filter((appointment) => appointment.all_day);
	}

	function isNowOnTop(): number {
		return (nowMinutes * HOUR) / 60;
	}

	let body = $state<HTMLDivElement | null>(null);

	/** The time grid starts at 07:00 instead of midnight. */
	function jumpToNow() {
		requestAnimationFrame(() => {
			if (body) body.scrollTop = 7 * HOUR;
		});
	}

	const monthsYear = $derived(
		Array.from({ length: 12 }, (_, index) => new Date(anchor.getFullYear(), index, 1))
	);

	const yearDays = Array.from({ length: 31 }, (_, index) => index + 1);

	function termineAm(day: Date): number {
		return (perDay.get(isoDay(day))?.length ?? 0) + (birthdaysPerDay.get(isoDay(day))?.length ?? 0);
	}

	// ------------------------------------------------------------ Mini calendar
	let mini = $state(startOfMonth(new Date()));

	function miniMonths(): Date[] {
		return [mini, addMonths(mini, 1)];
	}

	function miniWeeks(month: Date): Date[][] {
		const start = startOfWeek(startOfMonth(month));
		const end = addDays(startOfWeek(endOfMonth(month)), 6);
		const result: Date[][] = [];
		for (let day = start; day <= end; day = addDays(day, 1)) {
			const week = result.at(-1);
			if (!week || week.length === 7) result.push([day]);
			else week.push(day);
		}
		return result;
	}

	// ------------------------------------------------------------------ Calendars
	let createCalendar = $state(false);
	let newCalendar = $state('');
	/** Calendar whose row is currently being edited. */
	let editCalendar = $state<number | null>(null);
	let calendarName = $state('');

	async function saveCalendar() {
		const name = newCalendar.trim();
		if (!name) return;
		try {
			await api.agenda.createCategory(name);
			newCalendar = '';
			createCalendar = false;
			void load();
		} catch (e) {
			error = errorText(e);
		}
	}

	function openCalendarEditor(calendar: AgendaCategory) {
		editCalendar = calendar.id;
		calendarName = calendar.name;
	}

	async function renameCalendar() {
		const name = calendarName.trim();
		if (!name || editCalendar === null) return;
		try {
			await api.agenda.renameCategory(editCalendar, name);
			editCalendar = null;
			void load();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function deleteCalendar(calendar: AgendaCategory) {
		const ok = await askConfirm({
			title: 'Kalender löschen',
			message: `Kalender „${calendar.name}“ wirklich löschen?`,
			confirmLabel: 'Ja, löschen',
			cancelLabel: 'Behalten'
		});
		if (!ok) return;
		try {
			await api.agenda.removeCategory(calendar.id);
			editCalendar = null;
			void load();
		} catch (e) {
			error = errorText(e);
		}
	}

	function heading(): string {
		if (view === 'tag') return `${WEEKDAYS[(anchor.getDay() + 6) % 7]}, ${shortDate(isoDay(anchor))}`;
		if (view === 'woche') {
			const start = startOfWeek(anchor);
			const end = addDays(start, 6);
			const period =
				start.getMonth() === end.getMonth()
					? `${start.getDate()}.–${dayMonth(end)}`
					: `${dayMonth(start)} – ${dayMonth(end)}`;
			return `KW ${isoWeek(start)} · ${period} ${end.getFullYear()}`;
		}
		if (view === 'jahr') return String(anchor.getFullYear());
		return monthLabel(anchor);
	}

	const calendarCount = (calendar: AgendaCategory): number => calendar.appointments;
</script>

{#if !auth.hasRight(RIGHT.mayReadAppointments)}
	<AccessDenied
		pageName="Kalender"
		right={RIGHT.mayReadAppointments}
		rightLabel="Termine lesen"
	/>
{:else}
<header class="page-head">
	<h1>Kalender</h1>
	<div class="tools">
		<div class="paging">
			<button type="button" onclick={() => springe(addYears(anchor, -1))} title="ein Jahr zurück"
				>«</button
			>
			<button type="button" onclick={() => step(-1)} title="ein Zeitraum zurück">‹</button>
			<button type="button" class="today" onclick={() => springe(new Date())}>Heute</button>
			<button type="button" onclick={() => step(1)} title="ein Zeitraum vor">›</button>
			<button type="button" onclick={() => springe(addYears(anchor, 1))} title="ein Jahr vor">»</button>
		</div>
		<h2>{heading()}</h2>
		<div class="views" role="group" aria-label="Ansicht">
			{#each VIEWS as entry (entry.value)}
				<button
					type="button"
					class:active={view === entry.value}
					aria-pressed={view === entry.value}
					onclick={() => toggle(entry.value)}>{entry.label}</button
				>
			{/each}
			{#each LATER_VIEWS.filter((entry) => !entry.module || modules.isInstalled(entry.module)) as spaeter (spaeter.label)}
				<button type="button" class="off" disabled title={`${spaeter.label}: ${spaeter.note}`}>
					{spaeter.label}
				</button>
			{/each}
		</div>
		<span class="spacer"></span>
		{#if auth.hasRight(RIGHT.mayCreateAppointments)}
			<button type="button" class="new-appointment" onclick={() => newAppointment(anchor)}>
				+ Neuer Termin
			</button>
		{/if}
	</div>
</header>

<div class="page">
	<aside class="left" aria-label="Auswahl">
		<div class="group">
			<h3>Kalender</h3>
			<button
				type="button"
				class="entry"
				class:active={hidden.length === 0}
				aria-pressed={hidden.length === 0}
				onclick={alleAn}
			>
				<span>Alle Termine</span>
				<span class="count">{data?.occurrences.length ?? 0}</span>
			</button>
			{#each data?.categories ?? [] as calendar (calendar.id)}
				<div class="calendar-row">
					{#if editCalendar === calendar.id}
						<div class="calendar-edit">
							<input
								class="calendar-name"
								bind:value={calendarName}
								aria-label="Name des Kalenders"
								onkeydown={(e) => {
									if (e.key === 'Enter') void renameCalendar();
									if (e.key === 'Escape') editCalendar = null;
								}}
							/>
							<div class="calendar-buttons">
								<button
									type="button"
									onclick={renameCalendar}
									disabled={!calendarName.trim()}
								>
									Sichern
								</button>
								<button type="button" onclick={() => deleteCalendar(calendar)}>Löschen</button>
								<button type="button" onclick={() => (editCalendar = null)}>
									Abbrechen
								</button>
							</div>
						</div>
					{:else}
						<button
							type="button"
							class="entry"
							class:off={hidden.includes(calendar.id)}
							aria-pressed={!hidden.includes(calendar.id)}
							onclick={() => toggleCalendar(calendar.id)}
						>
							<span class="check" aria-hidden="true">
								{hidden.includes(calendar.id) ? '☐' : '☑'}
							</span>
							<span>{calendar.name}</span>
							<span class="count">{calendarCount(calendar)}</span>
						</button>
						<button
							type="button"
							class="icon"
							title="Kalender umbenennen oder löschen"
							onclick={() => openCalendarEditor(calendar)}
						>
							✎
						</button>
					{/if}
				</div>
			{:else}
				<p class="empty">Keine Kalender angelegt.</p>
			{/each}
		</div>

		<div class="group">
			<h3>Weitere Quellen</h3>
			<label class="check-row">
				<input type="checkbox" bind:checked={withBirthdays} />
				Geburtstage
			</label>
			<label class="check-row">
				<input type="checkbox" bind:checked={withOpenDates} />
				Lose Termine
			</label>
			{#each LATER_SOURCES.filter((entry) => !entry.module || modules.isInstalled(entry.module)) as spaeter (spaeter.label)}
				<label class="check-row off" title={spaeter.note}>
					<input type="checkbox" disabled />
					{spaeter.label}
				</label>
			{/each}
			<div class="switch">
				<button type="button" onclick={alleAn}>Alle auswählen</button>
				<button type="button" onclick={alleAus}>Alle abwählen</button>
			</div>
		</div>

		<div class="group">
			<h3>Suche</h3>
			<input
				bind:value={search}
				placeholder="Titel, Ort, Adresse"
				onkeydown={(e) => {
					if (e.key === 'Enter') void load();
				}}
			/>
		</div>

		{#if createCalendar}
			<div class="group">
				<input
					bind:value={newCalendar}
					placeholder="Name des Kalenders"
					onkeydown={(e) => {
						if (e.key === 'Enter') void saveCalendar();
						if (e.key === 'Escape') createCalendar = false;
					}}
				/>
				<button type="button" onclick={saveCalendar} disabled={!newCalendar.trim()}>
					Anlegen
				</button>
			</div>
		{:else}
			<button type="button" class="entry new" onclick={() => (createCalendar = true)}>
				+ Kalender
			</button>
		{/if}

		<p class="stamp">
			{termine.length}
			{termine.length === 1 ? 'Termin' : 'Termine'}
			{#if birthdays.length}· {birthdays.length} Geburtstage{/if}
			{#if loading}· lade …{/if}
		</p>

		<div class="mini">
			<button type="button" class="mini-today" onclick={() => springe(new Date())}>Heute</button>
			{#each miniMonths() as month (month.toISOString())}
				<div class="mini-month">
					<p class="mini-title">{monthLabel(month)}</p>
					<div class="mini-head">
						<span class="week">KW</span>
						{#each WEEKDAYS as day (day)}
							<span class:weekend={day === 'Sa' || day === 'So'}>{day[0]}</span>
						{/each}
					</div>
					{#each miniWeeks(month) as week, index (index)}
						<div class="mini-week">
							<span class="week">{isoWeek(week[0])}</span>
							{#each week as day (day.toISOString())}
								<button
									type="button"
									class="mini-day"
									class:other={day.getMonth() !== month.getMonth()}
									class:today={sameDay(day, today)}
									class:weekend={isWeekend(day)}
									onclick={() => springe(day)}>{day.getDate()}</button
								>
							{/each}
						</div>
					{/each}
				</div>
			{/each}
			<div class="mini-paging">
				<button type="button" onclick={() => (mini = addMonths(mini, -1))} title="früher">▲</button>
				<button type="button" onclick={() => (mini = addMonths(mini, 1))} title="später">▼</button>
			</div>
		</div>
	</aside>

	<section class="content">
		{#if view === 'monat'}
			<div class="month">
				<div class="weekdays">
					{#each WEEKDAYS as day, index (day)}
						<span class:weekend={index > 4}>{day}</span>
					{/each}
				</div>
				{#each weeks as week, w (w)}
					<div class="week">
						{#each week as day, t (t)}
							{@const entries = perDay.get(isoDay(day)) ?? []}
							{@const geburtstageTag = birthdaysPerDay.get(isoDay(day)) ?? []}
							<div
								class="day"
								class:other={day.getMonth() !== anchor.getMonth()}
								class:today={sameDay(day, today)}
								class:weekend={isWeekend(day)}
							>
								<button
									type="button"
									class="day-head"
										title={auth.hasRight(RIGHT.mayCreateAppointments)
											? 'Neuen Termin an diesem Tag anlegen'
											: 'Tag'}
										onclick={() => {
											if (auth.hasRight(RIGHT.mayCreateAppointments)) newAppointment(day);
										}}
									>
									<span class="number">{day.getDate()}</span>
									{#if day.getDate() === 1 || (t === 0 && day.getMonth() !== anchor.getMonth())}
										<span class="month-name">{monthShort(day)}</span>
									{/if}
								</button>
								<ul class="appointments">
									{#each geburtstageTag as birthday (birthday.address_id)}
										<li>
											<span class="birthday" title={`Geburtstag von ${birthday.name}`}>
												{birthday.name}
											</span>
										</li>
									{/each}
									{#each entries as appointment (appointment.event_id + '@' + appointment.date)}
										<li>
											<button
												type="button"
												class="appointment"
												class:all-day={appointment.all_day}
												style={appointmentStyle(appointment)}
												title={appointmentTitle(appointment)}
												onclick={() => terminOeffnen(appointment)}
											>
												<span class="text">{appointment.title}</span>
												{#if appointment.recurring}
													<span class="series" title="Wiederkehrender Termin">↻</span>
												{/if}
												{#if appointment.private_user != null}
													<span class="private" title="nur für mich">●</span>
												{/if}
												{#if terminMeta(appointment)}
													<span class="meta">{terminMeta(appointment)}</span>
												{/if}
											</button>
										</li>
									{/each}
								</ul>
							</div>
						{/each}
					</div>
				{/each}
			</div>
		{:else if view === 'jahr'}
			<div class="year">
				<div class="year-head">
					<span class="year-month"></span>
					{#each yearDays as number (number)}
						<span class="year-number">{number}</span>
					{/each}
				</div>
				{#each monthsYear as month (month.toISOString())}
					<div class="year-row">
						<button
							type="button"
							class="year-month"
							onclick={() => {
								anchor = startOfMonth(month);
								toggle('monat');
							}}>{monthLabel(month)}</button
						>
						{#each yearDays as number (number)}
							{@const day = new Date(month.getFullYear(), month.getMonth(), number)}
							{#if day.getMonth() !== month.getMonth()}
								<span class="year-day empty"></span>
							{:else}
								<button
									type="button"
									class="year-day"
									class:today={sameDay(day, today)}
									class:weekend={isWeekend(day)}
									class:booked={termineAm(day) > 0}
									title={`${shortDate(isoDay(day))}${termineAm(day) ? ` · ${termineAm(day)} Termine` : ''}`}
									onclick={() => {
										anchor = day;
										toggle('tag');
									}}>{number}</button
								>
							{/if}
						{/each}
					</div>
				{/each}
			</div>
		{:else}
			<div class="time">
				<div class="time-head" style={`grid-template-columns: 3.4rem repeat(${days.length}, minmax(0, 1fr));`}>
					<span class="time-corner"></span>
					{#each days as day (day.toISOString())}
						<button
							type="button"
							class="time-day"
							class:today={sameDay(day, today)}
							class:weekend={isWeekend(day)}
							title={auth.hasRight(RIGHT.mayCreateAppointments)
								? 'Neuen Termin an diesem Tag anlegen'
								: 'Tag'}
							onclick={() => {
								if (auth.hasRight(RIGHT.mayCreateAppointments)) newAppointment(day);
							}}
						>
							<span class="small">{WEEKDAYS[(day.getDay() + 6) % 7]}</span>
							<span class="large">{dayMonth(day)}</span>
						</button>
					{/each}
				</div>

				<div class="time-all-day" style={`grid-template-columns: 3.4rem repeat(${days.length}, minmax(0, 1fr));`}>
					<span class="time-corner small">ganztägig</span>
					{#each days as day (day.toISOString())}
						<div class="all-day-column">
							{#each birthdaysPerDay.get(isoDay(day)) ?? [] as birthday (birthday.address_id)}
								<span class="chip birthday" title={`Geburtstag von ${birthday.name}`}>
									{birthday.name}
								</span>
							{/each}
							{#each allDay(day) as appointment (appointment.event_id + '@' + appointment.date)}
								<button
									type="button"
									class="chip"
									style={appointmentStyle(appointment)}
									title={appointmentTitle(appointment)}
									onclick={() => terminOeffnen(appointment)}>{appointment.title}</button
								>
							{/each}
						</div>
					{/each}
				</div>

				<div
					class="time-body"
					bind:this={body}
					style={`grid-template-columns: 3.4rem repeat(${days.length}, minmax(0, 1fr));`}
				>
					<div class="time-hours">
						{#each HOURS as stunde (stunde)}
							<div class="hour" style={`height:${HOUR}px`}>
								{String(stunde).padStart(2, '0')}:00
							</div>
						{/each}
					</div>
					{#each days as day (day.toISOString())}
						<div
							class="time-column"
							class:today={sameDay(day, today)}
							class:weekend={isWeekend(day)}
							style={`height:${24 * HOUR}px`}
						>
							{#each bloecke(day) as block (block.appointment.event_id + '@' + block.appointment.date)}
								<button
									type="button"
									class="block"
									style={`top:${block.top}px;height:${block.height}px;left:${block.links * 100}%;width:calc(${block.width * 100}% - 2px);${appointmentStyle(block.appointment)}`}
									title={appointmentTitle(block.appointment)}
									onclick={() => terminOeffnen(block.appointment)}
								>
									<span class="block-title">{block.appointment.title}</span>
									<span class="block-time">
										{timeRange(block.appointment.start_time, block.appointment.end_time)}
									</span>
								</button>
							{/each}
							{#if sameDay(day, today)}
								<div class="now" style={`top:${isNowOnTop()}px`} aria-hidden="true"></div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</section>

	{#if modules.isInstalled('go')}
	<aside class="right" aria-label="Auftragsoptionen">
		<h3>Auftragsoptionen</h3>
		<label class="check-row off" title="Aufträge gibt es in WHERE noch nicht">
			<input type="checkbox" disabled />
			Miete
		</label>
		<label class="check-row off" title="Aufträge gibt es in WHERE noch nicht">
			<input type="checkbox" disabled />
			Verkauf
		</label>
		<label class="check-row off" title="Aufträge gibt es in WHERE noch nicht">
			<input type="checkbox" disabled />
			„Nicht Anzeigen“ überschreiben
		</label>
		<p class="right-field">Zeige Aufträge im aktuellen Zeitraum</p>
		<select disabled>
			<option>Keine Auswahl</option>
		</select>
		<p class="notice">
			Aufträge und Angebote gibt es in WHERE noch nicht — die Optionen bleiben solange
			ohne Wirkung. Termine und Geburtstage stehen im Kalender.
		</p>
	</aside>
	{/if}
</div>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if dialog}
	<AgendaDialog
		event={dialog.event}
		date={dialog.date}
		occurrence={dialog.occurrence}
		categories={visibleCategory}
		onsaved={closed}
		oncancel={() => (dialog = null)}
	/>
{/if}
{/if}

<style>
	.page-head {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}
	.tools {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.45rem 0.6rem;
	}
	.spacer {
		flex: 1;
	}
	h2 {
		margin: 0;
		font-size: 1.1rem;
		white-space: nowrap;
	}
	h3 {
		margin: 0 0 0.35rem;
		font-size: 0.8rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--muted);
	}
	.error {
		margin: 0 0 0.75rem;
		padding: 0.5rem 0.7rem;
		border: 1px solid var(--bad);
		border-radius: 4px;
		color: var(--bad);
		background: color-mix(in srgb, var(--bad) 8%, var(--surface));
	}
	.paging,
	.views {
		display: flex;
		gap: 0.25rem;
	}
	.paging button,
	.views button,
	.new-appointment {
		padding: 0.25rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
		cursor: pointer;
	}
	.paging button:hover,
	.views button:hover,
	.new-appointment:hover {
		background: var(--surface-2);
	}
	.views button.active {
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}
	.views button.off {
		color: var(--muted-2);
		border-style: dashed;
		cursor: not-allowed;
	}
	.views button.off:hover {
		background: var(--surface);
	}
	.new-appointment {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}

	.page {
		display: grid;
		grid-template-columns: 15rem minmax(0, 1fr) 13rem;
		gap: 0.9rem;
		align-items: start;
	}
	@media (max-width: 1200px) {
		.page {
			grid-template-columns: 13rem minmax(0, 1fr);
		}
		.right {
			display: none;
		}
	}
	@media (max-width: 900px) {
		.page {
			grid-template-columns: minmax(0, 1fr);
		}
		.left {
			order: 2;
		}
	}

	.left,
	.right {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 0.6rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.entry {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.2rem 0.35rem;
		border: 1px solid transparent;
		border-radius: 3px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.85rem;
		text-align: left;
		cursor: pointer;
	}
	.entry:hover {
		background: var(--surface-2);
	}
	.entry.active {
		background: var(--highlight);
		border-color: var(--accent);
	}
	.entry.off {
		color: var(--muted-2);
	}
	.entry .check {
		color: var(--accent);
		font-size: 0.75rem;
	}
	.entry.off .check {
		color: var(--muted-2);
	}
	.entry .count {
		margin-left: auto;
		color: var(--muted-2);
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
	}
	.entry.new {
		color: var(--accent);
	}
	/* Row of a calendar: toggle on the left, pencil on the right. */
	.calendar-row {
		display: flex;
		align-items: center;
		gap: 0.15rem;
	}
	.calendar-row .entry {
		flex: 1;
		min-width: 0;
	}
	.calendar-row .entry span:not(.check):not(.count) {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.icon {
		padding: 0.1rem 0.25rem;
		border: 1px solid transparent;
		border-radius: 3px;
		background: none;
		color: var(--muted-2);
		font-size: 0.8rem;
		line-height: 1.2;
		cursor: pointer;
	}
	.icon:hover:not(:disabled) {
		border-color: var(--border);
		background: var(--surface-2);
		color: var(--text);
	}
	.icon:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.calendar-name {
		flex: 1;
		min-width: 0;
		padding: 0.2rem 0.35rem;
		border: 1px solid var(--accent);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
	}
	.calendar-edit {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 0.25rem;
		min-width: 0;
	}
	.calendar-buttons {
		display: flex;
		gap: 0.25rem;
	}
	.calendar-buttons button {
		flex: 1;
		padding: 0.15rem 0.25rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.72rem;
		cursor: pointer;
	}
	.calendar-buttons button:hover:not(:disabled) {
		background: var(--highlight);
		border-color: var(--accent);
	}
	.calendar-buttons button:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.check-row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.85rem;
		color: var(--text);
	}
	.check-row.off {
		color: var(--muted-2);
	}
	.switch {
		display: flex;
		gap: 0.35rem;
		margin-top: 0.2rem;
	}
	.switch button {
		flex: 1;
		padding: 0.15rem 0.25rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--muted);
		font: inherit;
		font-size: 0.74rem;
		cursor: pointer;
	}
	.group input:not([type]) {
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
	}
	.empty,
	.stamp {
		margin: 0;
		font-size: 0.78rem;
		color: var(--muted-2);
	}
	.right-field {
		margin: 0.3rem 0 0;
		font-size: 0.78rem;
		color: var(--muted);
	}
	.right select {
		padding: 0.2rem 0.3rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--muted);
		font: inherit;
		font-size: 0.8rem;
	}
	.notice {
		margin: 0.4rem 0 0;
		font-size: 0.72rem;
		line-height: 1.35;
		color: var(--muted-2);
	}

	/* ------------------------------------------------------------ Mini calendar */
	.mini {
		margin-top: auto;
		padding-top: 0.5rem;
		border-top: 1px solid var(--border);
		font-size: 0.78rem;
	}
	.mini-today {
		width: 100%;
		padding: 0.2rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.78rem;
		cursor: pointer;
	}
	.mini-month {
		margin-top: 0.4rem;
	}
	.mini-title {
		margin: 0;
		font-weight: 600;
		text-align: center;
		font-size: 0.78rem;
	}
	.mini-head,
	.mini-week {
		display: grid;
		grid-template-columns: 1.6rem repeat(7, 1fr);
		align-items: center;
	}
	.mini-head span {
		text-align: center;
		color: var(--muted-2);
		font-size: 0.68rem;
	}
	.mini-head span.weekend {
		color: var(--bad);
	}
	.mini-head .week {
		text-align: left;
	}
	.mini-week .week {
		color: var(--muted-2);
		font-size: 0.62rem;
		font-variant-numeric: tabular-nums;
	}
	.mini-day {
		padding: 0.05rem 0;
		border: none;
		border-radius: 2px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.72rem;
		font-variant-numeric: tabular-nums;
		cursor: pointer;
	}
	.mini-day:hover {
		background: var(--surface-3);
	}
	.mini-day.other {
		color: var(--muted-2);
	}
	.mini-day.weekend {
		color: var(--bad);
	}
	.mini-day.today {
		background: var(--accent);
		color: #fff;
		font-weight: 700;
	}
	.mini-paging {
		display: flex;
		gap: 0.25rem;
		margin-top: 0.35rem;
	}
	.mini-paging button {
		flex: 1;
		padding: 0.1rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--muted);
		font: inherit;
		font-size: 0.7rem;
		cursor: pointer;
	}

	/* ------------------------------------------------------------ Month view */
	.month {
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		overflow: hidden;
	}
	.weekdays,
	.week {
		display: grid;
		grid-template-columns: repeat(7, minmax(0, 1fr));
	}
	.weekdays span {
		padding: 0.3rem 0.4rem;
		border-bottom: 1px solid var(--border);
		border-left: 1px solid var(--border);
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--muted);
	}
	.weekdays span:first-child {
		border-left: none;
	}
	.weekdays span.weekend {
		color: var(--bad);
	}
	.week {
		min-height: 6.5rem;
	}
	.week:not(:first-child) .day {
		border-top: 1px solid var(--border);
	}
	.day {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		padding: 0.2rem 0.25rem 0.3rem;
		border-left: 1px solid var(--border);
		background: var(--surface);
		min-width: 0;
	}
	.day:first-child {
		border-left: none;
	}
	.day.weekend {
		background: color-mix(in srgb, var(--bad) 7%, var(--surface));
	}
	.day.today {
		background: color-mix(in srgb, var(--good) 12%, var(--surface));
	}
	.day.other .number {
		color: var(--muted-2);
	}
	.day-head {
		display: flex;
		align-items: baseline;
		gap: 0.3rem;
		padding: 0;
		border: none;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.number {
		font-size: 0.82rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.day.weekend .number {
		color: var(--bad);
	}
	.month-name {
		font-size: 0.68rem;
		color: var(--muted);
		white-space: nowrap;
	}
	.appointments {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.appointment {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		column-gap: 0.25rem;
		align-items: baseline;
		width: 100%;
		padding: 0.12rem 0.28rem;
		border: none;
		border-left: 3px solid var(--accent);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.76rem;
		text-align: left;
		cursor: pointer;
	}
	.appointment:hover {
		background: var(--highlight);
	}
	.appointment .text {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.appointment .meta {
		grid-column: 1 / -1;
		color: var(--muted-2);
		font-size: 0.7rem;
		font-variant-numeric: tabular-nums;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.appointment .series,
	.appointment .private {
		grid-column: 2;
		color: var(--muted-2);
		font-size: 0.7rem;
	}
	.appointment.all-day {
		border-left-style: double;
		border-left-width: 4px;
	}
	.birthday {
		display: block;
		padding: 0.1rem 0.28rem;
		border-left: 3px solid var(--good);
		border-radius: 3px;
		background: color-mix(in srgb, var(--good) 10%, var(--surface));
		color: var(--muted);
		font-size: 0.74rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* -------------------------------------------------------------- Year view */
	.year {
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		overflow-x: auto;
		padding: 0.3rem;
	}
	.year-head,
	.year-row {
		display: grid;
		grid-template-columns: 7.5rem repeat(31, minmax(0, 1fr));
		align-items: center;
		gap: 1px;
	}
	.year-number {
		text-align: center;
		font-size: 0.66rem;
		color: var(--muted-2);
	}
	.year-month {
		padding: 0.1rem 0.3rem;
		border: none;
		border-radius: 3px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.78rem;
		text-align: left;
		cursor: pointer;
	}
	.year-month:hover {
		background: var(--surface-2);
	}
	.year-day {
		height: 1.15rem;
		border: none;
		border-radius: 2px;
		background: var(--surface-2);
		color: transparent;
		font: inherit;
		font-size: 0.6rem;
		cursor: pointer;
	}
	.year-day:hover {
		background: var(--highlight);
		color: var(--text);
	}
	.year-day.weekend {
		background: color-mix(in srgb, var(--bad) 9%, var(--surface));
	}
	.year-day.booked {
		background: color-mix(in srgb, var(--accent) 55%, var(--surface));
	}
	.year-day.weekend.booked {
		background: color-mix(in srgb, var(--accent) 40%, color-mix(in srgb, var(--bad) 20%, var(--surface)));
	}
	.year-day.today {
		background: color-mix(in srgb, var(--good) 40%, var(--surface));
		outline: 1px solid var(--good);
	}
	.year-day.empty {
		background: var(--surface-3);
		opacity: 0.5;
		cursor: default;
	}

	/* ---------------------------------------------------------- Day and week */
	.time {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		overflow: hidden;
	}
	.time-head,
	.time-all-day {
		display: grid;
		border-bottom: 1px solid var(--border);
	}
	.time-all-day {
		min-height: 1.6rem;
		align-items: stretch;
	}
	.time-day {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.05rem;
		padding: 0.25rem 0.2rem;
		border: none;
		border-left: 1px solid var(--border);
		background: none;
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}
	.time-day.weekend {
		background: color-mix(in srgb, var(--bad) 7%, var(--surface));
	}
	.time-day.today {
		background: color-mix(in srgb, var(--good) 14%, var(--surface));
	}
	.time-day .small {
		font-size: 0.7rem;
		color: var(--muted);
		text-transform: uppercase;
	}
	.time-day.weekend .small {
		color: var(--bad);
	}
	.time-day .large {
		font-size: 0.85rem;
		font-weight: 600;
	}
	.time-corner {
		border-right: 1px solid var(--border);
	}
	.time-corner.small {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		padding-right: 0.3rem;
		font-size: 0.66rem;
		color: var(--muted-2);
	}
	.all-day-column {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		padding: 0.15rem;
		border-left: 1px solid var(--border);
		min-width: 0;
	}
	.chip {
		display: block;
		width: 100%;
		padding: 0.1rem 0.25rem;
		border: none;
		border-left: 3px solid var(--accent);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.74rem;
		text-align: left;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: pointer;
	}
	.chip.birthday {
		border-left-color: var(--good);
		background: color-mix(in srgb, var(--good) 10%, var(--surface));
		color: var(--muted);
		cursor: default;
	}
	.time-body {
		display: grid;
		overflow-y: auto;
		max-height: calc(100vh - 20rem);
	}
	.time-hours {
		border-right: 1px solid var(--border);
	}
	.hour {
		display: flex;
		align-items: flex-start;
		justify-content: flex-end;
		padding-right: 0.3rem;
		font-size: 0.68rem;
		color: var(--muted-2);
		font-variant-numeric: tabular-nums;
		border-top: 1px solid var(--border);
	}
	.hour:first-child {
		border-top: none;
	}
	.time-column {
		position: relative;
		border-left: 1px solid var(--border);
		background-image: repeating-linear-gradient(
			to bottom,
			var(--border) 0 1px,
			transparent 1px 38px
		);
	}
	.time-column.weekend {
		background-color: color-mix(in srgb, var(--bad) 5%, var(--surface));
	}
	.time-column.today {
		background-color: color-mix(in srgb, var(--good) 9%, var(--surface));
	}
	.block {
		position: absolute;
		display: flex;
		flex-direction: column;
		gap: 0.05rem;
		padding: 0.1rem 0.25rem;
		border: none;
		border-left: 3px solid var(--accent);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.72rem;
		text-align: left;
		overflow: hidden;
		cursor: pointer;
	}
	.block:hover {
		background: var(--highlight);
	}
	.block-title {
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.block-time {
		color: var(--muted-2);
		font-size: 0.68rem;
		font-variant-numeric: tabular-nums;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.now {
		position: absolute;
		left: 0;
		right: 0;
		height: 0;
		border-top: 2px solid var(--bad);
		pointer-events: none;
	}
	.now::before {
		content: '';
		position: absolute;
		left: -4px;
		top: -5px;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--bad);
	}
</style>
