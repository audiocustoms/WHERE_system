<script lang="ts">
	import { escapeWindows } from '../escape.svelte';
	import {
		api,
		errorText,
		type Address,
		type AgendaCategory,
		type AgendaEvent,
		type AgendaEventInput,
		type AgendaFrequency
	} from '../api';
	import { askCancel, askConfirm } from '../confirm.svelte';
	import { openAddressEdit } from '../addresses/addressDialogStore.svelte';
	import { isoDay, shortDate } from './dates';
	import '../search/search-window.css';

	interface Props {
		/** `null` = new event. */
		event: AgendaEvent | null;
		/** Day a new event falls on. */
		date: string;
		/** Day of the clicked recurrence; `null` for a single event. */
		occurrence?: string | null;
		categories: AgendaCategory[];
		onsaved: () => void;
		oncancel: () => void;
	}

	let { event, date, occurrence = null, categories, onsaved, oncancel }: Props = $props();

	/** Defaults of the calendar colours; the first is the „ohne Farbe" choice. */
	const COLORS: { name: string; rgb: [number, number, number] }[] = [
		{ name: 'Blau', rgb: [32, 102, 178] },
		{ name: 'Grün', rgb: [45, 128, 78] },
		{ name: 'Orange', rgb: [204, 128, 24] },
		{ name: 'Rot', rgb: [186, 60, 54] },
		{ name: 'Violett', rgb: [112, 78, 158] },
		{ name: 'Türkis', rgb: [26, 138, 140] },
		{ name: 'Grau', rgb: [110, 118, 128] }
	];

	const FREQUENCIES: { value: AgendaFrequency; label: string }[] = [
		{ value: 'daily', label: 'täglich' },
		{ value: 'weekly', label: 'wöchentlich' },
		{ value: 'monthly', label: 'monatlich' },
		{ value: 'yearly', label: 'jährlich' }
	];

	// The mask starts with the passed-in values; another event replaces the
	// dialog entirely, which is why these are deliberately not derived values.
	// svelte-ignore state_referenced_locally
	const initial = event;
	// svelte-ignore state_referenced_locally
	const startDay = date;

	let title = $state(initial?.title ?? '');
	let headText = $state(initial?.head_text ?? '');
	let notes = $state(initial?.notes ?? '');
	let venue = $state(initial?.venue ?? '');
	let startDate = $state(initial?.start_date ?? startDay);
	let endDate = $state(initial?.end_date ?? initial?.start_date ?? startDay);
	let startTime = $state(initial?.start_time?.slice(0, 5) ?? '09:00');
	let endTime = $state(initial?.end_time?.slice(0, 5) ?? '17:00');
	let allDay = $state(initial?.all_day ?? false);
	let category = $state<string>(initial?.category != null ? String(initial.category) : '');
	let privateEntry = $state(initial?.private_user != null);
	let remind = $state(initial?.remind ?? false);
	let remindMinutes = $state(initial?.remind_minutes ?? 15);

	let repeat = $state<'none' | AgendaFrequency>(initial?.recurrence?.frequency ?? 'none');
	let interval = $state(initial?.recurrence?.interval ?? 1);
	let endMode = $state<'never' | 'until' | 'count'>(
		initial?.recurrence?.until ? 'until' : initial?.recurrence?.count ? 'count' : 'never'
	);
	let until = $state(initial?.recurrence?.until ?? '');
	let count = $state(initial?.recurrence?.count ?? 10);

	let color = $state<[number, number, number] | null>(
		initial?.custom_color && initial.color_red != null
			? [initial.color_red, initial.color_green ?? 0, initial.color_blue ?? 0]
			: null
	);

	let saving = $state(false);
	let error = $state<string | null>(null);

	let addressId = $state<number | null>(initial?.address_id ?? null);
	let addressName = $state<string | null>(initial?.address_name ?? null);
	let addressSearch = $state(false);
	let addressTerm = $state('');
	let addressHits = $state<Address[]>([]);
	let adressLaedt = $state(false);

	const seriesDate = $derived(occurrence ?? event?.start_date ?? null);
	const istSerie = $derived(!!event?.recurrence);

	function rgbStyle(rgb: [number, number, number]): string {
		return `background: rgb(${rgb[0]} ${rgb[1]} ${rgb[2]});`;
	}

	/** Never let the end date fall before the start; the mask pulls it along. */
	function startChanged() {
		if (endDate < startDate) endDate = startDate;
	}

	function input(): AgendaEventInput {
		return {
			title: title.trim(),
			head_text: headText.trim() || null,
			notes: notes.trim() || null,
			start_date: startDate,
			end_date: endDate || startDate,
			start_time: allDay ? null : startTime || null,
			end_time: allDay ? null : endTime || null,
			all_day: allDay,
			category: category ? Number(category) : null,
			venue: venue.trim() || null,
			address_id: addressId,
			color_red: color?.[0] ?? null,
			color_green: color?.[1] ?? null,
			color_blue: color?.[2] ?? null,
			custom_color: color !== null,
			private_user: privateEntry ? 1 : null,
			remind,
			remind_minutes: remind ? remindMinutes : null,
			recurrence:
				repeat === 'none'
					? null
					: {
							frequency: repeat,
							interval: Math.max(1, interval),
							until: endMode === 'until' ? until || null : null,
							count: endMode === 'count' ? Math.max(1, count) : null
						}
		};
	}

	async function save() {
		saving = true;
		error = null;
		try {
			if (event) await api.agenda.replace(event.id, input());
			else await api.agenda.create(input());
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	async function remove() {
		if (!event) return;
		const ok = await askConfirm({
			title: 'Termin löschen',
			message: istSerie
				? `Die ganze Serie „${event.title}" wird gelöscht. Einzelne Tage lassen sich separat entfernen.`
				: `Der Termin „${event.title}" wird gelöscht.`,
			confirmLabel: 'Löschen',
			cancelLabel: 'Behalten'
		});
		if (!ok) return;
		saving = true;
		try {
			await api.agenda.remove(event.id);
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** Take only the clicked day out of the series. */
	async function skip() {
		if (!event || !seriesDate) return;
		const ok = await askConfirm({
			title: 'Nur diesen Tag entfernen',
			message: `Der ${shortDate(seriesDate)} fällt aus, die Serie bleibt bestehen.`,
			confirmLabel: 'Tag entfernen',
			cancelLabel: 'Behalten'
		});
		if (!ok) return;
		saving = true;
		try {
			await api.agenda.skipOccurrence(event.id, seriesDate);
			onsaved();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** `Abbrechen` and closing ask first — as in all other masks. */
	async function cancel() {
		if (await askCancel(event ? 'Das Bearbeiten des Termins' : 'Die Terminanlage')) oncancel();
	}

	async function searchAddresses() {
		adressLaedt = true;
		error = null;
		try {
			const page = await api.addresses.list({ q: addressTerm.trim() || undefined, limit: 8 });
			addressHits = page.items;
		} catch (e) {
			error = errorText(e);
		} finally {
			adressLaedt = false;
		}
	}

	function chooseAddress(address: Address) {
		addressId = address.id;
		addressName = address.name1 ?? `Adresse ${address.id}`;
		addressSearch = false;
		addressTerm = '';
		addressHits = [];
	}

	function removeAddress() {
		addressId = null;
		addressName = null;
	}

	/** Opens the linked record in the address mask (`Adresse` tile of the event). */
	async function openAddress() {
		if (!addressId) return;
		try {
			openAddressEdit(await api.addresses.get(addressId));
		} catch (e) {
			error = errorText(e);
		}
	}

	// `Esc` closes the window like `Abbrechen`.
	$effect(() => escapeWindows(() => void cancel()));
</script>

<div class="sw-backdrop" role="presentation">
	<div
		class="sw-window"
		role="dialog"
		aria-modal="true"
		aria-label="Termin"
		style="width: min(44rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{event ? 'Termin bearbeiten:' : 'Neuer Termin:'}</span>
			<button class="sw-close" type="button" title="Schließen" onclick={cancel}>×</button>
		</div>

		<div class="sw-content">
			{#if error}
				<p class="sw-error">{error}</p>
			{/if}

			<div class="sw-form">
				<label class="sw-row">
					<span class="sw-label">Titel</span>
					<input bind:value={title} placeholder="z. B. Aufbau Bühne 1" />
				</label>
				<label class="sw-row">
					<span class="sw-label">Zusatz</span>
					<input bind:value={headText} placeholder="z. B. Theater Nordstadt" />
				</label>

				<div class="sw-row">
					<span class="sw-label">Zeitraum</span>
					<div class="values">
						<input type="date" bind:value={startDate} oninput={startChanged} aria-label="Von" />
						<span class="to">bis</span>
						<input type="date" bind:value={endDate} min={startDate} aria-label="Bis" />
					</div>
				</div>

				<div class="sw-row">
					<span class="sw-label">Uhrzeit</span>
					<div class="values">
						<input
							type="time"
							bind:value={startTime}
							disabled={allDay}
							aria-label="Von"
						/>
						<span class="to">bis</span>
						<input type="time" bind:value={endTime} disabled={allDay} aria-label="Bis" />
						<label class="check">
							<input type="checkbox" bind:checked={allDay} />
							Ganztägig
						</label>
					</div>
				</div>

				<label class="sw-row">
					<span class="sw-label">Kalender</span>
					<select bind:value={category}>
						<option value="">— ohne Kalender —</option>
						{#each categories as k (k.id)}
							<option value={String(k.id)}>{k.name}</option>
						{/each}
					</select>
				</label>

				<label class="sw-row">
					<span class="sw-label">Veranstaltung</span>
					<input bind:value={venue} placeholder="z. B. Premiere" />
				</label>

				<div class="sw-row">
					<span class="sw-label">Adresse</span>
					<div class="values address">
						{#if addressSearch}
							<input
								bind:value={addressTerm}
								placeholder="Name, Ort oder Nummer"
								aria-label="Adresse suchen"
								oninput={searchAddresses}
							/>
							<button
								class="sw-button"
								type="button"
								onclick={() => (addressSearch = false)}
							>
								Schließen
							</button>
							<ul class="hits">
								{#if adressLaedt}
									<li class="empty">suche …</li>
								{:else if addressHits.length === 0}
									<li class="empty">kein Treffer</li>
								{:else}
									{#each addressHits as a (a.id)}
										<li>
											<button type="button" onclick={() => chooseAddress(a)}>
												<span>{a.name1 ?? `Adresse ${a.id}`}</span>
												<span class="sub">
													{#if a.city}{a.city}{/if}
													{#if a.customer_no}· Kunde {a.customer_no}{/if}
												</span>
											</button>
										</li>
									{/each}
								{/if}
							</ul>
						{:else}
							<span class="chosen">{addressName ?? '— keine Adresse —'}</span>
							<button class="sw-button" type="button" onclick={() => { addressSearch = true; searchAddresses(); }}>
								{addressId ? 'Ändern' : 'Wählen'}
							</button>
							{#if addressId}
								<button
									class="sw-button"
									type="button"
									title="Adresse in der Adressmaske öffnen"
									onclick={openAddress}
								>
									Öffnen
								</button>
								<button class="sw-button" type="button" onclick={removeAddress}>Entfernen</button>
							{/if}
						{/if}
					</div>
				</div>

				<div class="sw-row">
					<span class="sw-label">Farbe</span>
					<div class="colors">
						<button
							type="button"
							class="color none"
							class:active={color === null}
							title="Standardfarbe"
							onclick={() => (color = null)}></button
						>
						{#each COLORS as f (f.name)}
							<button
								type="button"
								class="color"
								class:active={color?.[0] === f.rgb[0] && color?.[1] === f.rgb[1] && color?.[2] === f.rgb[2]}
								title={f.name}
								aria-label={f.name}
								style={rgbStyle(f.rgb)}
								onclick={() => (color = f.rgb)}></button
							>
						{/each}
					</div>
				</div>

				<label class="sw-row">
					<span class="sw-label">Notiz</span>
					<textarea bind:value={notes} rows="3"></textarea>
				</label>

				<div class="sw-row">
					<span class="sw-label">Erinnerung</span>
					<div class="values">
						<label class="check">
							<input type="checkbox" bind:checked={remind} />
							erinnern
						</label>
						<select bind:value={remindMinutes} disabled={!remind}>
							<option value={5}>5 Minuten vorher</option>
							<option value={15}>15 Minuten vorher</option>
							<option value={30}>30 Minuten vorher</option>
							<option value={60}>1 Stunde vorher</option>
							<option value={1440}>1 Tag vorher</option>
						</select>
					</div>
				</div>

				<div class="sw-row">
					<span class="sw-label">Wiederholung</span>
					<div class="values">
						<select bind:value={repeat} aria-label="Wiederholung">
							<option value="none">keine</option>
							{#each FREQUENCIES as f (f.value)}
								<option value={f.value}>{f.label}</option>
							{/each}
						</select>
						{#if repeat !== 'none'}
							<span class="to">alle</span>
							<input
								type="number"
								min="1"
								max="52"
								class="number"
								bind:value={interval}
								aria-label="Intervall"
							/>
							<select bind:value={endMode} aria-label="Ende der Serie">
								<option value="never">ohne Ende</option>
								<option value="until">bis Datum</option>
								<option value="count">nach Anzahl</option>
							</select>
							{#if endMode === 'until'}
								<input type="date" bind:value={until} min={startDate} aria-label="Serie bis" />
							{:else if endMode === 'count'}
								<input
									type="number"
									min="1"
									max="200"
									class="number"
									bind:value={count}
									aria-label="Anzahl"
								/>
								<span class="to">Termine</span>
							{/if}
						{/if}
					</div>
				</div>

				<label class="sw-row">
					<span class="sw-label">Sichtbarkeit</span>
					<span class="values">
						<label class="check">
							<input type="checkbox" bind:checked={privateEntry} />
							nur für mich
						</label>
					</span>
				</label>

				{#if event}
					<p class="sw-small">
						Zuletzt geändert
						{#if event.changed_by}von {event.changed_by}{/if}
						{#if event.changed_at}am {new Date(event.changed_at).toLocaleDateString('de-DE')}{/if}
						{#if istSerie}· Wiederholung: {FREQUENCIES.find((f) => f.value === event?.recurrence?.frequency)?.label}{/if}
					</p>
				{/if}
			</div>
		</div>

		<div class="sw-footer">
			{#if event}
				<button class="sw-button delete" type="button" onclick={remove} disabled={saving}>
					Termin löschen
				</button>
				{#if istSerie}
					<button class="sw-button" type="button" onclick={skip} disabled={saving}>
						Nur diesen Tag entfernen
					</button>
				{/if}
			{/if}
			<span class="spacer"></span>
			<button class="sw-button" type="button" onclick={cancel} disabled={saving}>Abbrechen</button>
			<button
				class="sw-button sw-primary"
				type="button"
				onclick={save}
				disabled={saving || !title.trim()}
			>
				{saving ? 'Sichern …' : 'Sichern'}
			</button>
		</div>
	</div>
</div>

<style>
	.values {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem;
	}
	.to {
		color: var(--muted);
		font-size: 0.8rem;
	}
	.check {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.82rem;
		color: var(--muted);
	}
	input.number {
		width: 4.5rem;
	}
	textarea {
		font: inherit;
		width: 100%;
		min-width: 0;
		padding: 0.3rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		resize: vertical;
	}
	.colors {
		display: flex;
		gap: 0.35rem;
		align-items: center;
	}
	.color {
		width: 1.5rem;
		height: 1.5rem;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		padding: 0;
	}
	.color.none {
		background: var(--surface-3);
		border-color: var(--border);
	}
	.color.active {
		border-color: var(--text);
	}
	/* The address choice unfolds in the row instead of putting a second window
	   over the mask. */
	.values.address {
		flex: 1;
		min-width: 0;
	}
	.values.address > input {
		flex: 1;
		min-width: 8rem;
	}
	.chosen {
		flex: 1;
		min-width: 8rem;
		font-size: 0.85rem;
		color: var(--text);
	}
	.hits {
		flex-basis: 100%;
		list-style: none;
		margin: 0.15rem 0 0;
		padding: 0;
		max-height: 12rem;
		overflow-y: auto;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
	}
	.hits button {
		display: flex;
		justify-content: space-between;
		gap: 0.6rem;
		width: 100%;
		padding: 0.25rem 0.45rem;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.85rem;
		text-align: left;
		cursor: pointer;
	}
	.hits button:hover {
		background: var(--surface-2);
	}
	.hits .sub {
		color: var(--muted-2);
		font-size: 0.78rem;
		white-space: nowrap;
	}
	.hits .empty {
		padding: 0.3rem 0.45rem;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.delete {
		color: var(--bad);
	}
	.spacer {
		flex: 1;
	}
</style>
