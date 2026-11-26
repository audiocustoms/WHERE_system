<script lang="ts">
	import { t } from '$core/i18n.svelte';

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
		{ name: t('Blau'), rgb: [32, 102, 178] },
		{ name: t('Grün'), rgb: [45, 128, 78] },
		{ name: t('Orange'), rgb: [204, 128, 24] },
		{ name: t('Rot'), rgb: [186, 60, 54] },
		{ name: t('Violett'), rgb: [112, 78, 158] },
		{ name: t('Türkis'), rgb: [26, 138, 140] },
		{ name: t('Grau'), rgb: [110, 118, 128] }
	];

	const FREQUENCIES: { value: AgendaFrequency; label: string }[] = [
		{ value: 'daily', label: t('täglich') },
		{ value: 'weekly', label: t('wöchentlich') },
		{ value: 'monthly', label: t('monatlich') },
		{ value: 'yearly', label: t('jährlich') }
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
			title: t('Termin löschen'),
			message: istSerie
				? t('Die ganze Serie „{title}" wird gelöscht. Einzelne Tage lassen sich separat entfernen.', {
						title: event.title
					})
				: t('Der Termin „{title}" wird gelöscht.', { title: event.title }),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
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
			title: t('Nur diesen Tag entfernen'),
			message: t('Der {date} fällt aus, die Serie bleibt bestehen.', {
				date: shortDate(seriesDate)
			}),
			confirmLabel: t('Tag entfernen'),
			cancelLabel: t('Behalten')
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
		if (await askCancel(t(event ? 'Das Bearbeiten des Termins' : 'Die Terminanlage'))) oncancel();
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
		aria-label={t('Termin')}
		style="width: min(44rem, 100%)"
	>
		<div class="sw-titlebar">
			<span class="sw-icon" aria-hidden="true"></span>
			<span class="sw-title">{event ? t('Termin bearbeiten:') : t('Neuer Termin:')}</span>
			<button class="sw-close" type="button" title={t('Schließen')} onclick={cancel}>×</button>
		</div>

		<div class="sw-content">
			{#if error}
				<p class="sw-error">{error}</p>
			{/if}

			<div class="sw-form">
				<label class="sw-row">
					<span class="sw-label">{t('Titel')}</span>
					<input bind:value={title} placeholder={t('z. B. Aufbau Bühne 1')} />
				</label>
				<label class="sw-row">
					<span class="sw-label">{t('Zusatz')}</span>
					<input bind:value={headText} placeholder={t('z. B. Theater Nordstadt')} />
				</label>

				<div class="sw-row">
					<span class="sw-label">{t('Zeitraum')}</span>
					<div class="values">
						<input type="date" bind:value={startDate} oninput={startChanged} aria-label={t('Von')} />
						<span class="to">{t('bis')}</span>
						<input type="date" bind:value={endDate} min={startDate} aria-label={t('Bis')} />
					</div>
				</div>

				<div class="sw-row">
					<span class="sw-label">{t('Uhrzeit')}</span>
					<div class="values">
						<input
							type="time"
							bind:value={startTime}
							disabled={allDay}
							aria-label={t('Von')}
						/>
						<span class="to">{t('bis')}</span>
						<input type="time" bind:value={endTime} disabled={allDay} aria-label={t('Bis')} />
						<label class="check">
							<input type="checkbox" bind:checked={allDay} />
							{t('Ganztägig')}
						</label>
					</div>
				</div>

				<label class="sw-row">
					<span class="sw-label">{t('Kalender')}</span>
					<select bind:value={category}>
						<option value="">{t('— ohne Kalender —')}</option>
						{#each categories as k (k.id)}
							<option value={String(k.id)}>{k.name}</option>
						{/each}
					</select>
				</label>

				<label class="sw-row">
					<span class="sw-label">{t('Veranstaltung')}</span>
					<input bind:value={venue} placeholder={t('z. B. Premiere')} />
				</label>

				<div class="sw-row">
					<span class="sw-label">{t('Adresse')}</span>
					<div class="values address">
						{#if addressSearch}
							<input
								bind:value={addressTerm}
								placeholder={t('Name, Ort oder Nummer')}
								aria-label={t('Adresse suchen')}
								oninput={searchAddresses}
							/>
							<button
								class="sw-button"
								type="button"
								onclick={() => (addressSearch = false)}
							>
								{t('Schließen')}
							</button>
							<ul class="hits">
								{#if adressLaedt}
									<li class="empty">{t('suche …')}</li>
								{:else if addressHits.length === 0}
									<li class="empty">{t('kein Treffer')}</li>
								{:else}
									{#each addressHits as a (a.id)}
										<li>
											<button type="button" onclick={() => chooseAddress(a)}>
												<span>{a.name1 ?? t('Adresse {id}', { id: a.id })}</span>
												<span class="sub">
													{#if a.city}{a.city}{/if}
													{#if a.customer_no}{t('· Kunde')} {a.customer_no}{/if}
												</span>
											</button>
										</li>
									{/each}
								{/if}
							</ul>
						{:else}
							<span class="chosen">{addressName ?? t('— keine Adresse —')}</span>
							<button class="sw-button" type="button" onclick={() => { addressSearch = true; searchAddresses(); }}>
								{addressId ? t('Ändern') : t('Wählen')}
							</button>
							{#if addressId}
								<button
									class="sw-button"
									type="button"
									title={t('Adresse in der Adressmaske öffnen')}
									onclick={openAddress}
								>
									{t('Öffnen')}
								</button>
								<button class="sw-button" type="button" onclick={removeAddress}>{t('Entfernen')}</button>
							{/if}
						{/if}
					</div>
				</div>

				<div class="sw-row">
					<span class="sw-label">{t('Farbe')}</span>
					<div class="colors">
						<button
							type="button"
							class="color none"
							class:active={color === null}
							title={t('Standardfarbe')}
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
					<span class="sw-label">{t('Notiz')}</span>
					<textarea bind:value={notes} rows="3"></textarea>
				</label>

				<div class="sw-row">
					<span class="sw-label">{t('Erinnerung')}</span>
					<div class="values">
						<label class="check">
							<input type="checkbox" bind:checked={remind} />
							{t('erinnern')}
						</label>
						<select bind:value={remindMinutes} disabled={!remind}>
							<option value={5}>{t('5 Minuten vorher')}</option>
							<option value={15}>{t('15 Minuten vorher')}</option>
							<option value={30}>{t('30 Minuten vorher')}</option>
							<option value={60}>{t('1 Stunde vorher')}</option>
							<option value={1440}>{t('1 Tag vorher')}</option>
						</select>
					</div>
				</div>

				<div class="sw-row">
					<span class="sw-label">{t('Wiederholung')}</span>
					<div class="values">
						<select bind:value={repeat} aria-label={t('Wiederholung')}>
							<option value="none">{t('keine')}</option>
							{#each FREQUENCIES as f (f.value)}
								<option value={f.value}>{f.label}</option>
							{/each}
						</select>
						{#if repeat !== 'none'}
							<span class="to">{t('alle')}</span>
							<input
								type="number"
								min="1"
								max="52"
								class="number"
								bind:value={interval}
								aria-label={t('Intervall')}
							/>
							<select bind:value={endMode} aria-label={t('Ende der Serie')}>
								<option value="never">{t('ohne Ende')}</option>
								<option value="until">{t('bis Datum')}</option>
								<option value="count">{t('nach Anzahl')}</option>
							</select>
							{#if endMode === 'until'}
								<input type="date" bind:value={until} min={startDate} aria-label={t('Serie bis')} />
							{:else if endMode === 'count'}
								<input
									type="number"
									min="1"
									max="200"
									class="number"
									bind:value={count}
									aria-label={t('Anzahl')}
								/>
								<span class="to">{t('Termine')}</span>
							{/if}
						{/if}
					</div>
				</div>

				<label class="sw-row">
					<span class="sw-label">{t('Sichtbarkeit')}</span>
					<span class="values">
						<label class="check">
							<input type="checkbox" bind:checked={privateEntry} />
							{t('nur für mich')}
						</label>
					</span>
				</label>

				{#if event}
					<p class="sw-small">
						{t('Zuletzt geändert')}
						{#if event.changed_by}{t('von {user}', { user: event.changed_by })}{/if}
						{#if event.changed_at}{t('am {date}', { date: new Date(event.changed_at).toLocaleDateString('de-DE') })}{/if}
						{#if istSerie}{t('· Wiederholung: {frequency}', { frequency: FREQUENCIES.find((f) => f.value === event?.recurrence?.frequency)?.label ?? '' })}{/if}
					</p>
				{/if}
			</div>
		</div>

		<div class="sw-footer">
			{#if event}
				<button class="sw-button delete" type="button" onclick={remove} disabled={saving}>
					{t('Termin löschen')}
				</button>
				{#if istSerie}
					<button class="sw-button" type="button" onclick={skip} disabled={saving}>
						{t('Nur diesen Tag entfernen')}
					</button>
				{/if}
			{/if}
			<span class="spacer"></span>
			<button class="sw-button" type="button" onclick={cancel} disabled={saving}>{t('Abbrechen')}</button>
			<button
				class="sw-button sw-primary"
				type="button"
				onclick={save}
				disabled={saving || !title.trim()}
			>
				{saving ? t('Sichern …') : t('Sichern')}
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
