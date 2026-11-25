<script lang="ts">
	import { escapeWindows } from '$core/escape.svelte';
	import {
		api,
		errorText,
		type DutyDemandDay,
		type DutyDemandEntry,
		type DispoDayProposal,
		type Shift
	} from '$core/api';
	import '$core/search/search-window.css';

	interface Props {
		/** Day the dialog belongs to (`YYYY-MM-DD` and its German label). */
		day: { iso: string; weekday: string; day: string };
		/** Maintained positions, in their master-data order. */
		positions: string[];
		/** Shifts in their maintained order. */
		shifts: Shift[];
		/** The stored demand of that day. */
		demand: DutyDemandEntry[];
		/** Employees already planned per `shiftId|position` on that day. */
		planned: Record<string, number>;
		/** Who would fill an open row — the employee with the fewest hours. */
		proposal: string | null;
		/** What the disposition of the day asks for, when it has entries. */
		dispo: DispoDayProposal | null;
		/** The rules the written plan of that day breaks, with the employee. */
		violations: { name: string; reason: string }[];
		/** Whether the user may change the plan. */
		mayEdit: boolean;
		/** Called on close; `geaendert` tells the board to reload. */
		onclose: (changed: boolean) => void;
		/** Called after an event was set aside, so the board reads the day again. */
		onrefresh: () => void;
	}

	let {
		day,
		positions,
		shifts,
		demand,
		planned,
		proposal,
		dispo,
		violations,
		mayEdit,
		onclose,
		onrefresh
	}: Props = $props();

	/** The position value that does not care which employee covers the row. */
	const NOT_RELEVANT = '*';

	/** One editable line: a position within a shift. */
	interface Row {
		shiftId: number;
		position: string;
		/** Location the row belongs to; empty when it is not bound to one. */
		location: string;
		/** Number as text, so an empty field stays possible. */
		value: string;
	}

	/** Rows of the day, one shift block after the other. */
	let rows = $state<Row[]>(initialRows());
	/** Positions each shift does not have — `shiftId` → names. */
	let exclusion = $state<Record<number, string[]>>(startExclusion());
	let copyCount = $state('1');
	let saving = $state(false);
	let error = $state<string | null>(null);
	let notice = $state<string | null>(null);
	let changed = $state(false);

	/** The excluded positions as the shifts bring them. */
	function startExclusion(): Record<number, string[]> {
		return Object.fromEntries(
			shifts.map((shift) => [shift.id, [...(shift.excluded_positions ?? [])]])
		);
	}

	/** The demand the disposition asks for, grouped by shift. */
	const dispoPerShift = $derived.by(() => {
		const card = new Map<number, DutyDemandEntry[]>();
		for (const entry of dispo?.entries ?? []) {
			const list = card.get(entry.shift_id);
			if (list) list.push(entry);
			else card.set(entry.shift_id, [entry]);
		}
		return card;
	});

	/** `true` when the disposition asks for nothing and names nothing unclear. */
	const withoutDispo = $derived(
		(dispo?.entries.length ?? 0) === 0 && (dispo?.unclear.length ?? 0) === 0
	);

	/** Why an event of the disposition could not be read into a shift. */
	function reasonText(reason: string): string {
		if (reason === 'no_time') return 'ohne Uhrzeit';
		if (reason === 'unknown_flag') return 'unbekanntes Flag';
		if (reason === 'no_staffing') return 'Flag ohne Besetzung';
		if (reason === 'no_shift') return 'keine passende Schicht';
		return reason;
	}

	/** Sets an event aside — or puts it back into the plan. */
	async function setIgnored(entryId: number, ignored: boolean) {
		error = null;
		try {
			await api.people.duty.dispo.setIgnored(entryId, ignored);
			onrefresh();
		} catch (e) {
			error = errorText(e);
		}
	}

	/** Takes the demand of the disposition over into the rows of the day. */
	function takeOverDispo() {
		const next: Row[] = [...rows];
		for (const entry of dispo?.entries ?? []) {
			const place = entry.location ?? '';
			const present = next.find(
				(row) =>
					row.shiftId === entry.shift_id &&
					row.position.trim().toLowerCase() === entry.position.trim().toLowerCase() &&
					row.location.trim().toLowerCase() === place.trim().toLowerCase()
			);
			if (present) present.value = String(entry.needed);
			else
				next.push({
					shiftId: entry.shift_id,
					position: entry.position,
					location: place,
					value: String(entry.needed)
				});
		}
		// Rows that stayed empty go away when the proposal filled the block.
		rows = next.filter(
			(row) => row.position !== '' || row.value !== '' || row.location !== ''
		);
		notice = 'Bedarf aus der Dispo übernommen — mit `Speichern` festschreiben.';
	}

	/** Stored rows first, otherwise one empty row for every shift. */
	function initialRows(): Row[] {
		return shifts.map((shift) => {
			const own = demand.filter((entry) => entry.shift_id === shift.id);
			return own.length > 0
				? own.map((entry) => ({
						shiftId: shift.id,
						position: entry.position,
						location: entry.location ?? '',
						value: String(entry.needed)
					}))
				: [{ shiftId: shift.id, position: '', location: '', value: '' }];
		}).flat();
	}

	/** Shifts may arrive after the dialog opened — every one keeps its row. */
	$effect(() => {
		const missing = shifts
			.map((shift) => shift.id)
			.filter((id) => !rows.some((row) => row.shiftId === id));
		if (missing.length === 0) return;
		const order = shifts.map((shift) => shift.id);
		rows = [
			...rows,
			...missing.map((id) => ({ shiftId: id, position: '', location: '', value: '' }))
		].sort((a, b) => order.indexOf(a.shiftId) - order.indexOf(b.shiftId));
	});

	function rowsOf(shiftId: number): Row[] {
		return rows.filter((row) => row.shiftId === shiftId);
	}

	function amount(row: Row): number {
		const value = Number(String(row.value ?? '').replace(',', '.'));
		if (!Number.isFinite(value) || value <= 0) return 0;
		return Math.min(99, Math.round(value));
	}

	const total = $derived(rows.reduce((sizeTotal, row) => sizeTotal + amount(row), 0));
	const plannedTotal = $derived(
		Object.values(planned).reduce((sizeTotal, count) => sizeTotal + count, 0)
	);

	/** Employees planned in that shift and position; `*` counts the whole shift. */
	function plannedAmount(row: Row): number {
		const key = `${row.shiftId}|${row.position.trim().toLowerCase()}`;
		return planned[key] ?? 0;
	}

	/** A row whose position does not matter. */
	function isOpen(row: Row): boolean {
		return row.position.trim() === NOT_RELEVANT;
	}

	/** The positions this shift does not have. */
	function exclusionFrom(shiftId: number): string[] {
		return exclusion[shiftId] ?? [];
	}

	function isExcluded(shiftId: number, position: string): boolean {
		const name = position.trim().toLowerCase();
		return exclusionFrom(shiftId).some((excluded) => excluded.trim().toLowerCase() === name);
	}

	/** Positions of one shift that are not used in another row yet. */
	function freePositions(row: Row): string[] {
		const taken = rows
			.filter((other) => other !== row && other.shiftId === row.shiftId)
			.map((other) => other.position.trim().toLowerCase());
		return positions.filter(
			(position) =>
				!taken.includes(position.trim().toLowerCase()) &&
				!isExcluded(row.shiftId, position)
		);
	}

	/** Positions of the shift that could still be excluded. */
	function openExclusions(shiftId: number): string[] {
		return positions.filter((position) => !isExcluded(shiftId, position));
	}

	/** Writes the excluded positions of a shift; the list is replaced as a whole. */
	async function setExclusion(shift: Shift, positions: string[]) {
		const previous = exclusionFrom(shift.id);
		exclusion = { ...exclusion, [shift.id]: positions };
		error = null;
		try {
			const stored = await api.people.shifts.exclusions(shift.id, positions);
			exclusion = { ...exclusion, [shift.id]: [...(stored.excluded_positions ?? [])] };
			changed = true;
		} catch (e) {
			exclusion = { ...exclusion, [shift.id]: previous };
			error = errorText(e);
		}
	}

	function addExclusion(shift: Shift, position: string) {
		if (position.trim() === '') return;
		void setExclusion(shift, [...exclusionFrom(shift.id), position]);
	}

	function removeExclusion(shift: Shift, position: string) {
		void setExclusion(
			shift,
			exclusionFrom(shift.id).filter((excluded) => excluded !== position)
		);
	}

	function addRow(shiftId: number) {
		rows = [...rows, { shiftId, position: '', location: '', value: '' }];
	}

	function removeRow(row: Row) {
		const own = rowsOf(row.shiftId);
		if (own.length <= 1) {
			row.position = '';
			row.value = '';
			return;
		}
		rows = rows.filter((other) => other !== row);
	}

	function toIso(date: Date): string {
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const dayOfMonth = String(date.getDate()).padStart(2, '0');
		return `${date.getFullYear()}-${month}-${dayOfMonth}`;
	}

	function date(): Date {
		const [year, month, dayOfMonth] = day.iso.split('-').map(Number);
		return new Date(year, month - 1, dayOfMonth);
	}

	function plusDays(dateValue: Date, days: number): Date {
		return new Date(dateValue.getFullYear(), dateValue.getMonth(), dateValue.getDate() + days);
	}

	/** Days after the opened date that the settings can be copied onto. */
	const targets = $derived.by(() => {
		const start = date();
		const monday = plusDays(start, -((start.getDay() + 6) % 7));
		const week: string[] = [];
		for (let index = 0; index < 7; index += 1) week.push(toIso(plusDays(monday, index)));
		const month: string[] = [];
		const lastDay = new Date(start.getFullYear(), start.getMonth() + 1, 0).getDate();
		for (let index = 1; index <= lastDay; index += 1) {
			month.push(toIso(new Date(start.getFullYear(), start.getMonth(), index)));
		}
		const count = Math.max(1, Math.min(400, Math.round(Number(copyCount) || 1)));
		const following: string[] = [];
		for (let index = 1; index <= count; index += 1) following.push(toIso(plusDays(start, index)));
		return {
			week: week.filter((iso) => iso !== day.iso),
			month: month.filter((iso) => iso !== day.iso),
			remaining: month.filter((iso) => iso > day.iso),
			following
		};
	});

	/** Saves the numbers of the dialog and keeps the local state in step. */
	async function save(): Promise<boolean> {
		saving = true;
		error = null;
		try {
			const entries = rows
				.filter((row) => row.position.trim() !== '' && amount(row) > 0)
				.map((row) => ({
					shift_id: row.shiftId,
					position: row.position.trim(),
					location: row.location.trim(),
					needed: amount(row)
				}));
			const saved: DutyDemandDay = await api.people.duty.demand.set(day.iso, entries);
			rows = initialRowsFrom(saved);
			changed = true;
			return true;
		} catch (e) {
			error = errorText(e);
			return false;
		} finally {
			saving = false;
		}
	}

	function initialRowsFrom(day: DutyDemandDay): Row[] {
		return shifts
			.map((shift) => {
				const own = day.entries.filter((entry) => entry.shift_id === shift.id);
				return own.length > 0
					? own.map((entry) => ({
							shiftId: shift.id,
							position: entry.position,
							location: entry.location ?? '',
							value: String(entry.needed)
						}))
					: [{ shiftId: shift.id, position: '', location: '', value: '' }];
			})
			.flat();
	}

	async function carryOver(days: string[], rightLabel: string) {
		if (days.length === 0) return;
		if (!(await save())) return;
		saving = true;
		error = null;
		notice = null;
		try {
			const result = await api.people.duty.demand.copy(day.iso, days);
			notice = `${rightLabel}: ${result.days} ${result.days === 1 ? 'Tag' : 'Tage'} übernommen.`;
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	function close() {
		onclose(changed);
	}

	/** `Speichern` closes the window once the rows are stored. */
	async function saveAndClose() {
		if (await save()) close();
	}


	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void close()));
</script>


<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window demand-window"
		role="dialog"
		aria-modal="true"
		aria-label="Personalbedarf"
		style="width: min(38rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">Personalbedarf</span>
			<span class="date">{day.weekday} {day.day}.</span>
			<button class="sw-close" type="button" title="Schließen" onclick={close}>
				×
			</button>
		</div>

		<div class="content">
				{#if violations.length > 0}
					<section class="block conflicts">
						<header>
							<span class="title">Regelverstöße</span>
						</header>
						<ul class="violationList">
							{#each violations as violation (`${violation.name}|${violation.reason}`)}
								<li>
									<span class="who">{violation.name}</span>
									<span class="reason">{violation.reason}</span>
								</li>
							{/each}
						</ul>
					</section>
				{/if}
				{#if !withoutDispo}
					<section class="block dispo">
						<header>
							<span class="title">Bedarf aus der Dispo</span>
							{#if mayEdit && (dispo?.entries.length ?? 0) > 0}
								<button type="button" class="sw-button small" onclick={takeOverDispo}>
									Übernehmen
								</button>
							{/if}
						</header>
						{#if (dispo?.entries.length ?? 0) === 0}
							<p class="empty">Die Dispo dieses Tages ergibt keinen Bedarf.</p>
						{:else}
							<ul class="dispoList">
								{#each shifts as shift (shift.id)}
									{#each dispoPerShift.get(shift.id) ?? [] as entry (`${entry.shift_id}|${entry.location}|${entry.position}`)}
										<li>
											<span class="where">{shift.short_name ?? shift.name}</span>
											<span class="what"
												>{entry.position === NOT_RELEVANT ? 'nicht relevant' : entry.position}{#if entry.location}<span
														class="place">{entry.location}</span
													>{/if}</span
											>
											<span class="count">{entry.needed}</span>
										</li>
									{/each}
								{/each}
							</ul>
						{/if}
						{#if (dispo?.unclear.length ?? 0) > 0}
							<ul class="unclear">
								{#each dispo?.unclear ?? [] as event (event.entry_id)}
									<li class:ignored={event.ignored}>
										<span class="question">?</span>
										<span class="event"
											>{event.starts_at ? `${event.starts_at} ` : ''}{event.title} ·
											{reasonText(event.reason)}</span
										>
										{#if mayEdit}
											<button
												type="button"
												class="sw-button small"
												title={event.ignored
													? 'den Eintrag wieder beachten'
													: 'den Eintrag nicht mehr beachten'}
												onclick={() => setIgnored(event.entry_id, !event.ignored)}
											>
												{event.ignored ? 'Beachten' : 'Ignorieren'}
											</button>
										{/if}
									</li>
								{/each}
							</ul>
						{/if}
					</section>
				{/if}
				{#each shifts as shift (shift.id)}
				<section class="block" class:inactive={!shift.active}>
					<header>
						<span class="badge" style={`--badge:${shift.color ?? '#94a3b8'}`}></span>
						<span class="title">
							{#if shift.short_name}<span class="shortName">{shift.short_name}</span>{/if}
							{shift.name}{shift.active ? '' : ' · inaktiv'}
						</span>
						<span class="time">{shift.starts_at}–{shift.ends_at}</span>
						{#if mayEdit}
							<button
								type="button"
								class="add"
								title="Position hinzufügen"
								aria-label="Position zu {shift.name} hinzufügen"
								disabled={saving}
								onclick={() => addRow(shift.id)}>+</button
							>
						{/if}
					</header>

					<div class="head">
						<span>Position</span>
						<span>Bedarf</span>
						<span>Geplant</span>
						<span></span>
					</div>

					{#each rowsOf(shift.id) as row (row)}
						<div class="row">
							<span class="position">
								<select
									aria-label="Position in {shift.name}"
									disabled={!mayEdit || saving}
									bind:value={row.position}
								>
									<option value="">—</option>
									<option value={NOT_RELEVANT}>nicht relevant</option>
									{#each freePositions(row) as position (position)}
										<option value={position}>{position}</option>
									{/each}
									{#if row.position !== '' && row.position !== NOT_RELEVANT && !freePositions(row).includes(row.position)}
										<option value={row.position}>{row.position}</option>
									{/if}
								</select>
								{#if row.location}
									<span class="location" title="Standort der Dispo">{row.location}</span>
								{/if}
							</span>
							<input
								type="number"
								min="0"
								max="99"
								step="1"
								inputmode="numeric"
								placeholder="0"
								disabled={!mayEdit || saving}
								aria-label="Bedarf {shift.name} {row.position}"
								value={row.value}
								oninput={(event) => (row.value = event.currentTarget.value)}
							/>
							<span class="planned" class:missing={plannedAmount(row) < amount(row)}>
								{plannedAmount(row)}
							</span>
							{#if mayEdit}
								<button
									type="button"
									class="remove"
									title="Zeile entfernen"
									aria-label="Zeile entfernen"
									disabled={saving}
									onclick={() => removeRow(row)}>−</button
								>
							{/if}
						</div>
						{#if isOpen(row)}
							<p class="proposal">
								Position egal — es zählt jeder der Schicht. Vorschlag (wenigste Stunden):
								<strong>{proposal ?? 'niemand frei'}</strong>
							</p>
						{/if}
					{/each}

					<div class="exclusion">
						<span class="exclusion-text">Positionen ausschließen</span>
						{#each exclusionFrom(shift.id) as excluded (excluded)}
							<span class="chip">
								{excluded}
								{#if mayEdit}
									<button
										type="button"
										class="chipRemove"
										title="{excluded} wieder zulassen"
										aria-label="{excluded} wieder zulassen"
										disabled={saving}
										onclick={() => removeExclusion(shift, excluded)}>×</button
									>
								{/if}
							</span>
						{:else}
							<span class="none">keine</span>
						{/each}
						{#if mayEdit && openExclusions(shift.id).length > 0}
							<select
								class="choice"
								aria-label="Position in {shift.name} ausschließen"
								disabled={saving}
								value=""
								onchange={(event) => {
									addExclusion(shift, event.currentTarget.value);
									event.currentTarget.value = '';
								}}
							>
								<option value="">+ ausschließen …</option>
								{#each openExclusions(shift.id) as position (position)}
									<option value={position}>{position}</option>
								{/each}
							</select>
						{/if}
					</div>
				</section>
			{:else}
				<p class="empty">
					Noch keine Schicht angelegt — lege sie unter <em>Stammdaten → Personal → Dienstplan</em> an.
				</p>
			{/each}

			<div class="summary">
				<span>Summe</span>
				<span>{total}</span>
				<span class:missing={plannedTotal < total}>{plannedTotal}</span>
			</div>

			{#if mayEdit}
				<fieldset class="copy">
					<legend>Einstellungen übertragen</legend>
					<div class="copy-row">
						<label>
							<span>Nächste</span>
							<input
								class="count"
								type="number"
								min="1"
								max="400"
								step="1"
								bind:value={copyCount}
								disabled={saving}
							/>
							<span>Tage</span>
						</label>
						<button
							class="sw-button"
							type="button"
							disabled={saving}
							onclick={() => carryOver(targets.following, 'Nächste Tage')}
						>
							Übertragen
						</button>
					</div>
					<div class="copy-row quick">
						<button
							class="sw-button"
							type="button"
							disabled={saving}
							onclick={() => carryOver(targets.week, 'Ganze Woche')}
						>
							Ganze Woche
						</button>
						<button
							class="sw-button"
							type="button"
							disabled={saving}
							onclick={() => carryOver(targets.remaining, 'Rest des Monats')}
						>
							Rest des Monats
						</button>
						<button
							class="sw-button"
							type="button"
							disabled={saving}
							onclick={() => carryOver(targets.month, 'Alle Tage des Monats')}
						>
							Alle Tage des Monats
						</button>
					</div>
				</fieldset>
			{/if}

			{#if error}<p class="error">{error}</p>{/if}
			{#if notice}<p class="notice">{notice}</p>{/if}
		</div>

		<div class="sw-footer">
			<button class="sw-button" type="button" onclick={close}>Schließen</button>
			{#if mayEdit}
				<button
					class="sw-button sw-primary"
					type="button"
					disabled={saving}
					onclick={saveAndClose}>Speichern</button
				>
			{/if}
		</div>
	</div>
</div>

<style>
	.demand-window {
		max-height: min(48rem, 90vh);
		display: flex;
		flex-direction: column;
	}
	.date {
		margin-left: auto;
		margin-right: 0.6rem;
		color: var(--muted);
		font-size: 0.82rem;
		font-variant-numeric: tabular-nums;
	}
	.content {
		padding: 0.6rem 0.75rem;
		overflow: auto;
	}
	.block {
		margin-bottom: 0.7rem;
		padding: 0.35rem 0.45rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 6px;
	}
	.block.inactive .title {
		color: var(--muted-2);
	}
	.block header {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		padding-bottom: 0.25rem;
	}
	.badge {
		flex: none;
		width: 0.35rem;
		height: 1.2rem;
		border-radius: 2px;
		background: var(--badge, var(--border));
	}
	.title {
		display: flex;
		align-items: baseline;
		gap: 0.3rem;
		font-size: 0.85rem;
		font-weight: 600;
	}
	.shortName {
		padding: 0.05rem 0.28rem;
		border-radius: 3px;
		background: var(--surface-3);
		color: var(--muted);
		font-size: 0.68rem;
		font-weight: 600;
	}
	.time {
		color: var(--muted-2);
		font-size: 0.72rem;
		font-variant-numeric: tabular-nums;
	}
	.add,
	.remove {
		width: 1.3rem;
		height: 1.3rem;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--muted);
		font-size: 0.9rem;
		line-height: 1;
		cursor: pointer;
	}
	.add {
		margin-left: auto;
	}
	.add:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.remove:hover {
		border-color: var(--bad);
		color: var(--bad);
	}
	.head,
	.row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 5rem 4.5rem 1.4rem;
		gap: 0.4rem;
		align-items: center;
	}
	.sw-button.small {
		padding: 0.15rem 0.5rem;
		font-size: 0.75rem;
	}
	.head {
		padding: 0 0.1rem 0.15rem;
		color: var(--muted-2);
		font-size: 0.66rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}
	.head span:not(:first-child) {
		text-align: right;
	}
	.row {
		padding: 0.1rem 0.1rem;
	}
	.row select,
	.row input,
	.count {
		width: 100%;
		padding: 0.15rem 0.3rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: inherit;
		font: inherit;
		font-size: 0.8rem;
	}
	.row input,
	.count {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	/* Position and the location the row belongs to. */
	.position {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		min-width: 0;
	}
	.location {
		flex: 0 0 auto;
		padding: 0.1rem 0.3rem;
		border-radius: 3px;
		background: var(--surface-3);
		color: var(--muted);
		font-size: 0.68rem;
		white-space: nowrap;
	}
	.planned {
		color: var(--muted);
		font-size: 0.8rem;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.planned.missing,
	.summary .missing {
		color: var(--bad);
		font-weight: 600;
	}
	/* An open row: any employee of the shift counts, the fewest hours is suggested. */
	.proposal {
		margin: 0 0 0.25rem;
		padding: 0 0.1rem 0.25rem 0.15rem;
		color: var(--muted);
		font-size: 0.72rem;
	}
	.proposal strong {
		color: var(--label, inherit);
		font-variant-numeric: tabular-nums;
	}
	/* Positions the shift does not have. */
	.exclusion {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.3rem;
		margin: 0.3rem 0 0.1rem;
		padding: 0.25rem 0.1rem 0;
		border-top: 1px dashed var(--surface-3);
	}
	.exclusion-text {
		color: var(--muted-2);
		font-size: 0.66rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.15rem;
		padding: 0 0.15rem 0 0.4rem;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface-2);
		font-size: 0.72rem;
	}
	.chipRemove {
		padding: 0 0.25rem;
		border: none;
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: 0.8rem;
		line-height: 1.2;
		cursor: pointer;
	}
	.chipRemove:hover {
		color: var(--bad);
	}
	.none,
	.choice {
		color: var(--muted-2);
		font-size: 0.72rem;
	}
	.choice {
		padding: 0.05rem 0.2rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--muted);
		font: inherit;
		font-size: 0.72rem;
	}
	.summary {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 5rem 4.5rem 1.4rem;
		gap: 0.4rem;
		margin-top: 0.2rem;
		padding: 0.35rem 0.55rem;
		border-top: 1px solid var(--border);
		font-size: 0.84rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.summary span:not(:first-child) {
		text-align: right;
	}
	.copy {
		margin: 0.75rem 0 0;
		padding: 0.5rem 0.6rem 0.6rem;
		border: 1px solid var(--border);
		border-radius: 6px;
	}
	.copy legend {
		padding: 0 0.3rem;
		color: var(--muted);
		font-size: 0.75rem;
		font-weight: 600;
	}
	.copy-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem;
	}
	.copy-row + .copy-row {
		margin-top: 0.4rem;
	}
	.copy-row label {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.8rem;
	}
	.count {
		width: 4rem;
	}
	.quick .sw-button {
		flex: 1 1 auto;
	}
	.empty {
		margin: 0.4rem 0;
		color: var(--muted);
		font-size: 0.82rem;
	}
	/* The demand the disposition asks for, above the shift blocks. */
	.block.dispo {
		border-color: var(--accent, #1f6feb);
	}
	.block.conflicts {
		border-color: var(--bad);
	}
	.violationList {
		margin: 0.2rem 0 0.1rem;
		padding: 0;
		list-style: none;
		font-size: 0.82rem;
		color: var(--bad);
	}
	.violationList li {
		display: flex;
		gap: 0.5rem;
		padding: 0.1rem 0;
	}
	.violationList .who {
		min-width: 9rem;
		font-weight: 600;
	}
	.violationList .reason {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.dispoList {
		margin: 0.2rem 0 0.4rem;
		padding: 0;
		list-style: none;
		font-size: 0.82rem;
	}
	.dispoList li {
		display: flex;
		gap: 0.5rem;
		padding: 0.1rem 0;
	}
	.dispoList .where {
		min-width: 3.5rem;
		color: var(--muted);
	}
	.dispoList .what {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.dispoList .place {
		margin-left: 0.35rem;
		color: var(--muted);
		font-size: 0.76rem;
	}
	.dispoList .count {
		font-variant-numeric: tabular-nums;
	}
	.unclear {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		margin: 0.2rem 0;
		padding: 0;
		list-style: none;
		font-size: 0.78rem;
		color: var(--warn, #b45309);
	}
	.unclear li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.3rem 0.5rem;
	}
	.unclear li.ignored {
		color: var(--muted);
	}
	.unclear .question {
		font-weight: 700;
	}
	.unclear .event {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.error {
		margin: 0.6rem 0 0;
		color: var(--bad);
		font-size: 0.82rem;
	}
	.notice {
		margin: 0.6rem 0 0;
		color: var(--good);
		font-size: 0.82rem;
	}
</style>
