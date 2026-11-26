<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import { api, errorText, type PeopleListEntry, type PeopleListKind } from '$core/api';
	import { auth } from '$core/auth.svelte';
	import { askConfirm } from '$core/confirm.svelte';
	import { RIGHT } from '$core/rights';
	import ColorPicker from '$core/ColorPicker.svelte';

	interface Props {
		/** `departments`, `subdepartments`, `positions` or `locations`. */
		kind: PeopleListKind;
		/** Heading of the list, e.g. `Abteilungen`. */
		title: string;
		/** One entry in the singular, e.g. `Abteilung`. */
		singular: string;
	}

	let { kind, title, singular }: Props = $props();

	let entries = $state<PeopleListEntry[]>([]);
	/** The lists a row can point at: the locations take a sub-department, the
	 * sub-departments take a department. */
	let departments = $state<PeopleListEntry[]>([]);
	let subDepartments = $state<PeopleListEntry[]>([]);
	let newName = $state('');
	/** Entry whose name is currently being edited. */
	let editing = $state<number | null>(null);
	let draft = $state('');
	let loading = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);

	/**
	 * `Personaldispo anlegen` allows new entries, `Personaldispo editieren` changes
	 * the existing ones; the administration may do both (the API checks it as well —
	 * `where-api/src/authz.rs`).
	 */
	const mayCreate = $derived(auth.isAdmin || auth.hasRight(RIGHT.mayCreateDispo));
	const mayChange = $derived(auth.isAdmin || auth.hasRight(RIGHT.mayEditDispo));

	async function load() {
		loading = true;
		error = null;
		try {
			entries = await api.people.lists.list(kind);
			// Rows the board types itself are no master data of the houses.
			if (kind === 'locations') {
				entries = entries.filter((entry) => entry.is_text !== true);
			}
			if (kind === 'locations') {
				subDepartments = await api.people.lists.list('subdepartments');
			} else if (kind === 'subdepartments') {
				departments = await api.people.lists.list('departments');
			}
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	onMount(load);

	/** The list a row of this panel names as its parent. */
	const parents = $derived(kind === 'locations' ? subDepartments : departments);

	/** Which sub-department works in that location (or which department a
	 * sub-department sits under); empty clears the binding. */
	async function setParent(id: number, department: string) {
		saving = true;
		error = null;
		try {
			await api.people.lists.setParent(kind, id, department);
			await load();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** The tint a department or a location gives its rows in the schedule board. */
	async function setTint(entry: PeopleListEntry, tint: string) {
		saving = true;
		error = null;
		try {
			await api.people.lists.setColor(kind, entry.id, tint);
			await load();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	async function create() {
		const name = newName.trim();
		if (name === '') return;
		saving = true;
		error = null;
		try {
			await api.people.lists.create(kind, name);
			newName = '';
			await load();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	function startRename(entry: PeopleListEntry) {
		editing = entry.id;
		draft = entry.name;
		error = null;
	}

	function cancelRename() {
		editing = null;
		draft = '';
	}

	async function saveRename(id: number) {
		const name = draft.trim();
		if (name === '') return;
		saving = true;
		error = null;
		try {
			await api.people.lists.rename(kind, id, name);
			cancelRename();
			await load();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	async function removeEntry(entry: PeopleListEntry) {
		if (
			!(await askConfirm({
				message: t('{what} „{name}“ entfernen?', { what: singular, name: entry.name }),
				confirmLabel: t('Entfernen')
			}))
		)
			return;
		saving = true;
		error = null;
		try {
			await api.people.lists.remove(kind, entry.id);
			await load();
		} catch (e) {
			error = errorText(e);
		} finally {
			saving = false;
		}
	}

	/** Swaps the entry with its neighbor and saves the new order. */
	async function move(index: number, direction: -1 | 1) {
		const target = index + direction;
		if (target < 0 || target >= entries.length) return;
		const next = [...entries];
		[next[index], next[target]] = [next[target], next[index]];
		entries = next;
		error = null;
		try {
			entries = await api.people.lists.reorder(
				kind,
				next.map((entry) => entry.id)
			);
		} catch (e) {
			error = errorText(e);
			await load();
		}
	}
</script>

<section class="list">
	<h2>{title} ({entries.length})</h2>

	{#if mayCreate}
		<div class="toolbar">
			<input
				placeholder={t('Neue {what} …', { what: singular })}
				maxlength="100"
				bind:value={newName}
				onkeydown={(e) => {
					if (e.key !== 'Enter') return;
					e.preventDefault();
					create();
				}}
			/>
			<button type="button" onclick={create} disabled={saving || loading}>{t('Anlegen')}</button>
		</div>
	{/if}

	{#if error}<p class="bad">{error}</p>{/if}

	<ul class="entries">
		{#each entries as entry, i (entry.id)}
			<li>
				{#if editing === entry.id}
					<input
						class="rename"
						maxlength="100"
						bind:value={draft}
						onkeydown={(e) => {
							if (e.key === 'Enter') {
								e.preventDefault();
								saveRename(entry.id);
							}
							if (e.key === 'Escape') cancelRename();
						}}
					/>
					<div class="actions">
						<button
							type="button"
							onclick={() => saveRename(entry.id)}
							disabled={saving}>{t('Sichern')}</button
						>
						<button type="button" onclick={cancelRename}>{t('Abbrechen')}</button>
					</div>
				{:else}
					<div class="row">
						<span class="name">{entry.name}</span>
						{#if mayChange}
							<div class="sorting">
								<button
									type="button"
									class="arrow"
									title={t('nach oben')}
									aria-label={t('{name}: nach oben', { name: entry.name })}
									disabled={i === 0}
									onclick={() => move(i, -1)}>↑</button
								>
								<button
									type="button"
									class="arrow"
									title={t('nach unten')}
									aria-label={t('{name}: nach unten', { name: entry.name })}
									disabled={i === entries.length - 1}
									onclick={() => move(i, 1)}>↓</button
								>
							</div>
						{/if}
					</div>
					{#if mayChange}
						<div class="actions">
							<ColorPicker
								value={entry.color ?? ''}
								label={t('{name}: Farbe', { name: entry.name })}
								disabled={saving}
								onchange={(tint) => void setTint(entry, tint)}
							/>
							{#if entry.color}
								<button
									type="button"
									disabled={saving}
									onclick={() => void setTint(entry, '')}>{t('ohne Farbe')}</button
								>
							{/if}
							{#if kind === 'locations' || kind === 'subdepartments'}
								<select
									class="parent"
									title={kind === 'locations'
										? t('Unterabteilung, die hier arbeitet')
										: t('Abteilung, zu der sie gehört')}
									aria-label={t('{name}: {what}', {
										name: entry.name,
										what: kind === 'locations' ? t('Unterabteilung') : t('Abteilung')
									})}
									value={entry.department ?? ''}
									disabled={saving}
									onchange={(e) => setParent(entry.id, e.currentTarget.value)}
								>
									<option value="">
										{kind === 'locations' ? t('— alle Unterabteilungen —') : t('— keine —')}
									</option>
									{#each parents as entry_ (entry_.id)}
										<option value={entry_.name}>{entry_.name}</option>
									{/each}
									{#if entry.department && !parents.some((parent) => parent.name === entry.department)}
										<option value={entry.department}>{entry.department}</option>
									{/if}
								</select>
							{/if}
							<button type="button" onclick={() => startRename(entry)}>{t('umbenennen')}</button>
							<button type="button" class="danger" onclick={() => removeEntry(entry)}>
								{t('entfernen')}
							</button>
						</div>
					{/if}
				{/if}
			</li>
		{:else}
			<li class="empty">{loading ? t('Lade …') : t('Noch keine {what} erfasst.', { what: title })}</li>
		{/each}
	</ul>
</section>

<style>
	.list {
		min-width: 0;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 0.75rem 0.9rem 0.85rem;
	}
	h2 {
		font-size: 1.05rem;
		margin: 0 0 0.6rem;
	}
	.toolbar {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	input {
		flex: 1;
		min-width: 0;
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
	}
	.rename {
		box-sizing: border-box;
		width: 100%;
		font: inherit;
		font-size: 0.9rem;
		padding: 0.15rem 0.35rem;
	}
	button {
		padding: 0.4rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
	.entries {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.entries li {
		padding: 0.4rem 0;
	}
	.entries li + li {
		border-top: 1px solid var(--surface-3);
	}
	.entries li.empty {
		color: var(--muted);
		font-size: 0.85rem;
	}
	/* Name on the left of the first line, the sorting at its right end. */
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.name {
		flex: 1 1 auto;
		min-width: 0;
		font-size: 0.9rem;
		overflow-wrap: anywhere;
	}
	/* The tint a department gives its employees in the schedule board. */
	.sorting {
		display: flex;
		flex: 0 0 auto;
		gap: 0.25rem;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-start;
		gap: 0.25rem;
		margin-top: 0.3rem;
	}
	.actions button,
	.sorting button {
		padding: 0.15rem 0.4rem;
		font-size: 0.75rem;
	}
	.sorting button.arrow {
		padding: 0.15rem 0.3rem;
		min-width: 1.6rem;
		line-height: 1;
	}
	.actions button:disabled,
	.sorting button:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.danger {
		color: var(--bad);
	}
	.parent {
		max-width: 12rem;
		padding: 0.15rem 0.3rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: inherit;
		font: inherit;
		font-size: 0.75rem;
	}
	.bad {
		color: var(--bad);
		margin: 0 0 0.5rem;
		font-size: 0.85rem;
	}
</style>
