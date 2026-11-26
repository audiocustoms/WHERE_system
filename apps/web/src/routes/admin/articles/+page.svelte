<script lang="ts">
	import { t } from '$core/i18n.svelte';

	import { onMount } from 'svelte';
	import {
		api,
		errorText,
		type ArticleGroup,
		type SettingsArea,
		type SettingsRow
	} from '../../../lib/api';
	import { askConfirm } from '../../../lib/confirm.svelte';
	import ListWindow from '../../../lib/settings/ListWindow.svelte';
	import '../../../lib/search/search-window.css';

	/** The two tabs of the left column from `img/Admin Menü/Artikel/Artikel.PNG`. */
	type Tab = 'warengruppen' | 'suchindex';

	/** What is currently selected in the list. */
	interface Marker {
		kind: Tab;
		id: number;
		label: string;
		/** Path of the search index, otherwise empty. */
		path: string;
	}

	/** An input window for `+` and `✎`. */
	interface TextDialog {
		title: string;
		value: string;
		create: boolean;
		/** Parent group, only for the product groups. */
		parent: number | null;
	}

	/** The buttons of the „Artikel“ group — each opens a list (`area: ''` = no list). */
	const KNOEPFE: { label: string; area: string }[] = [
		{ label: t('Hersteller ändern…'), area: 'hersteller' },
		{ label: t('Eigentümer ändern…'), area: 'eigentuemer' },
		{ label: t('Artikelzustände ändern…'), area: 'artikelzustand' },
		{ label: t('TeamAgenda-Gruppen ändern…'), area: '' },
		{ label: t('Artikelseitige Provisionsklassen'), area: 'provisionsklassen' }
	];

	const SUCHINDEX = 'artikel-suchindex';

	let areas = $state<SettingsArea[]>([]);
	let tab = $state<Tab>('warengruppen');
	let groups = $state<ArticleGroup[]>([]);
	let suchindex = $state<SettingsRow[]>([]);
	let marker = $state<Marker | null>(null);
	let dialog = $state<TextDialog | null>(null);
	let isOpen = $state<string | null>(null);
	let error = $state<string | null>(null);
	let load = $state(true);

	onMount(loadAll);

	async function loadAll() {
		load = true;
		error = null;
		try {
			areas = await api.settings.areas();
			groups = await api.masterData.articleGroups();
			const index = await api.settings.list(SUCHINDEX);
			suchindex = index.rows;
		} catch (e) {
			error = errorText(e);
		} finally {
			load = false;
		}
	}

	function area(key: string): SettingsArea | null {
		return areas.find((area) => area.key === key) ?? null;
	}

	function oeffne(key: string) {
		if (!area(key)) {
			error = t('Die Liste „{area}“ fehlt in dieser Installation.', { area: key });
			return;
		}
		isOpen = key;
	}

	/** Product groups as a tree: children are indented below their parent group. */
	const groupTree = $derived.by(() => {
		const children = new Map<number | null, ArticleGroup[]>();
		for (const group of groups) {
			const list = children.get(group.parent_id) ?? [];
			list.push(group);
			children.set(group.parent_id, list);
		}
		const rows: { group: ArticleGroup; depth: number }[] = [];
		const visited = new Set<number>();
		function run(parent: number | null, depth: number) {
			for (const group of children.get(parent) ?? []) {
				if (visited.has(group.id)) continue;
				visited.add(group.id);
				rows.push({ group, depth });
				run(group.id, depth + 1);
			}
		}
		run(null, 0);
		// Do not swallow groups whose parent group is unknown.
		for (const group of groups) if (!visited.has(group.id)) rows.push({ group, depth: 0 });
		return rows;
	});

	/** The search index carries its path as text — the depth sits in the slashes. */
	const indexRows = $derived(
		suchindex
			.map((row) => {
				const label = row.values['Index'] ?? '';
				const path = row.values['Pfad'] ?? '';
				return { id: row.id, label, path, depth: path ? path.split('/').length - 1 : 0 };
			})
			// Sorted by path, children sit directly below their parent group.
			.sort((a, b) => (a.path || a.label).localeCompare(b.path || b.label, 'de'))
	);

	/** Possible parent groups: all except the group itself. */
	const topGroups = $derived(
		groups.filter((group) => group.id !== (dialog?.create ? null : marker?.id))
	);

	function remember(kind: Tab, id: number, label: string, path = '') {
		marker = marker?.kind === kind && marker.id === id ? null : { kind, id, label, path };
	}

	/** `+` creates, `✎` edits — both through a small input window. */
	function startCreate() {
		dialog = {
			title: tab === 'warengruppen' ? t('Neue Warengruppe') : t('Neuer Suchbegriff'),
			value: '',
			create: true,
			// A selected group becomes the parent group of the new entry.
			parent: marker?.kind === 'warengruppen' ? marker.id : null
		};
	}

	function edit() {
		if (!marker) return;
		dialog = {
			title: tab === 'warengruppen' ? t('Warengruppe ändern') : t('Suchbegriff ändern'),
			value: marker.label,
			create: false,
			parent: groups.find((group) => group.id === marker?.id)?.parent_id ?? null
		};
	}

	async function save() {
		if (!dialog) return;
		const value = dialog.value.trim();
		if (!value) return;
		const { create, parent } = dialog;
		const target = marker;
		dialog = null;
		error = null;
		try {
			if (tab === 'warengruppen') {
				if (create) await api.masterData.articleGroupCreate({ name: value, parent_id: parent });
				else if (target) await api.masterData.articleGroupUpdate(target.id, { name: value, parent_id: parent });
			} else if (create) {
				const topGroup = target?.kind === 'suchindex' ? target.path : '';
				await api.settings.create(SUCHINDEX, {
					Index: value,
					Path: topGroup ? `${topGroup}/${value}` : value,
					Parent_ID: '0',
					Child_ID: '0'
				});
			} else if (target) {
				const row = suchindex.find((r) => r.id === target.id);
				await api.settings.update(SUCHINDEX, target.id, { ...(row?.values ?? {}), Index: value });
			}
			await loadAll();
		} catch (e) {
			error = errorText(e);
		}
	}

	async function loeschen() {
		if (!marker) return;
		const target = marker;
		const ok = await askConfirm({
			title: t('Löschen'),
			message:
				target.kind === 'warengruppen'
					? t(
							'Warengruppe „{name}“ löschen? Das geht nur, solange keine Artikel und keine Untergruppen daran hängen.',
							{ name: target.label }
						)
					: t('Suchbegriff „{name}“ löschen?', { name: target.label }),
			confirmLabel: t('Löschen'),
			cancelLabel: t('Behalten')
		});
		if (!ok) return;
		error = null;
		try {
			if (target.kind === 'warengruppen') await api.masterData.articleGroupDelete(target.id);
			else await api.settings.remove(SUCHINDEX, target.id);
			await loadAll();
		} catch (e) {
			error = errorText(e);
		}
	}
</script>

<svelte:head><title>{t('Artikeleinstellungen — WHERE')}</title></svelte:head>

<h1>{t('Artikeleinstellungen')}</h1>

<section class="selection" aria-label={t('Artikeleinstellungen')}>
	<div class="tabs" role="tablist" aria-label={t('Artikel')}>
		<button
			type="button"
			role="tab"
			aria-selected={tab === 'warengruppen'}
			class:active={tab === 'warengruppen'}
			onclick={() => {
				tab = 'warengruppen';
				marker = null;
			}}>{t('Warengruppen')}</button
		>
		<button
			type="button"
			role="tab"
			aria-selected={tab === 'suchindex'}
			class:active={tab === 'suchindex'}
			onclick={() => {
				tab = 'suchindex';
				marker = null;
			}}>{t('Suchindex')}</button
		>
	</div>

	<div
		class="tree"
		role="tabpanel"
		aria-label={tab === 'warengruppen' ? t('Warengruppen') : t('Suchindex')}
	>
		{#if load}
			<p class="small">{t('Lade…')}</p>
		{:else if tab === 'warengruppen'}
			{#each groupTree as row (row.group.id)}
				<button
					type="button"
					class="tree-row"
					class:marked={marker?.kind === 'warengruppen' && marker.id === row.group.id}
					style="padding-left: {0.3 + row.depth * 1.1}rem"
					onclick={() => remember('warengruppen', row.group.id, row.group.name ?? '')}
				>
					<span class="name">{row.group.name ?? '—'}</span>
					<span class="count">{row.group.article_count}</span>
				</button>
			{:else}
				<p class="small">{t('Noch keine Warengruppen.')}</p>
			{/each}
		{:else}
			{#each indexRows as row (row.id)}
				<button
					type="button"
					class="tree-row"
					class:marked={marker?.kind === 'suchindex' && marker.id === row.id}
					style="padding-left: {0.3 + row.depth * 1.1}rem"
					onclick={() => remember('suchindex', row.id, row.label, row.path)}
				>
					<span class="name">{row.label || '—'}</span>
					{#if row.path && row.path !== row.label}
						<span class="path">{row.path}</span>
					{/if}
				</button>
			{:else}
				<p class="small">{t('Noch keine Suchbegriffe.')}</p>
			{/each}
		{/if}
	</div>

	<div class="list-foot">
		<div class="button-group" role="group" aria-label={t('Eintrag')}>
			<button type="button" class="square" title={t('Neuer Eintrag')} onclick={startCreate}>+</button>
			<button type="button" class="square" title={t('Ändern')} disabled={!marker} onclick={edit}
				>✎</button
			>
			<button type="button" class="square" title={t('Löschen')} disabled={!marker} onclick={loeschen}
				>−</button
			>
		</div>
		<span class="small">{t('Markierte Zeile bearbeiten · Listen sichern sofort')}</span>
	</div>

	<h2>{t('Artikel')}</h2>
	{#each KNOEPFE as knopf (knopf.label)}
		{#if knopf.area}
			<button type="button" class="entry" onclick={() => oeffne(knopf.area)}>
				{knopf.label}
			</button>
		{:else}
			<button
				type="button"
				class="entry"
				title={t('TeamAgenda-Gruppen sind freier Text am Artikel')}
				onclick={() =>
					(error = t(
						'TeamAgenda-Gruppen stehen als freier Text am Artikel („Artikel“.„TeamAgenda_Gruppe“); eine eigene Liste führt die Altanwendung nicht.'
					))}
			>
				{knopf.label}
			</button>
		{/if}
	{/each}
</section>

<footer class="foot">
	<span class="spacer"></span>
	<a class="btn" href="/">{t('Abbrechen')}</a>
	<button type="button" class="btn" disabled title={t('Die Listen sichern sofort')}>{t('Sichern')}</button>
</footer>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if dialog}
	<div class="sw-backdrop" role="presentation">
		<div
			class="sw-window"
			role="dialog"
			aria-modal="true"
			aria-label={dialog.title}
			style="width: min(26rem, 100%)"
		>
			<div class="sw-titlebar">
				<span class="sw-icon" aria-hidden="true"></span>
				<span class="sw-title">{dialog.title}</span>
				<button
					class="sw-close"
					type="button"
					title={t('Schließen')}
					onclick={() => (dialog = null)}>×</button
				>
			</div>
			<div class="sw-content">
				<label class="field">
					{tab === 'warengruppen' ? t('Bezeichnung') : t('Suchbegriff')}
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="text"
						autofocus
						maxlength="255"
						value={dialog.value}
						oninput={(e) => (dialog!.value = e.currentTarget.value)}
						onkeydown={(e) => {
							if (e.key === 'Enter') save();
						}}
					/>
				</label>
				{#if tab === 'warengruppen'}
					<label class="field">
						{t('Obergruppe')}
						<select
							class="text"
							value={dialog.parent ?? ''}
							onchange={(e) =>
								(dialog!.parent = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))}
						>
							<option value="">{t('— keine (oberste Ebene) —')}</option>
							{#each topGroups as group (group.id)}
								<option value={group.id}>{group.name ?? `#${group.id}`}</option>
							{/each}
						</select>
					</label>
				{/if}
			</div>
			<div class="sw-footer">
				<span class="spacer"></span>
				<button type="button" class="btn" onclick={() => (dialog = null)}>{t('Abbrechen')}</button>
				<button
					type="button"
					class="btn primary"
					disabled={!dialog.value.trim()}
					onclick={save}>{dialog.create ? t('Anlegen') : t('Sichern')}</button
				>
			</div>
		</div>
	</div>
{/if}

{#if isOpen && area(isOpen)}
	<ListWindow area={area(isOpen)!} onclose={() => (isOpen = null)} onchange={loadAll} />
{/if}

<style>
	.error {
		margin: 0 0 1rem;
		padding: 0.5rem 0.7rem;
		border: 1px solid var(--bad);
		border-radius: 4px;
		color: var(--bad);
		background: color-mix(in srgb, var(--bad) 8%, var(--surface));
	}
	/* As with the address settings: one column, same width. */
	.selection {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		max-width: 30rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
	}
	.tabs {
		display: flex;
		gap: 0.25rem;
		margin: -0.75rem -0.75rem 0.5rem;
		padding: 0.5rem 0.75rem 0;
	}
	.tabs button {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-bottom: none;
		border-radius: 3px 3px 0 0;
		background: var(--surface-2);
		color: var(--muted);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
	}
	.tabs button.active {
		background: var(--surface);
		color: var(--accent);
		font-weight: 600;
	}
	.tree {
		display: flex;
		flex-direction: column;
		min-height: 14rem;
		max-height: 24rem;
		overflow: auto;
		padding: 0.35rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
	}
	.tree-row {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		padding: 0.16rem 0.3rem;
		border: none;
		border-radius: 2px;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		text-align: left;
		cursor: pointer;
	}
	.tree-row:nth-child(even) {
		background: var(--surface);
	}
	.tree-row.marked {
		background: var(--highlight);
		outline: 1px solid var(--accent);
	}
	.count {
		margin-left: auto;
		color: var(--muted-2);
		font-size: 0.78rem;
	}
	.path {
		color: var(--muted-2);
		font-size: 0.78rem;
	}
	h2 {
		margin: 0.9rem 0 0.35rem;
		font-size: 0.95rem;
		color: var(--accent);
	}
	.entry {
		padding: 0.28rem 0.5rem;
		margin-bottom: 0.2rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		text-align: center;
		cursor: pointer;
	}
	.entry:hover {
		background: var(--highlight);
		border-color: var(--accent);
	}
	/* Edit and create directly below the list. */
	.list-foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0.45rem 0 0;
	}
	.list-foot .small {
		flex: 1;
		min-width: 0;
	}
	.foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 30rem;
		margin-top: 0.35rem;
		padding-top: 0.6rem;
		border-top: 1px solid var(--border);
	}
	.spacer {
		flex: 1;
	}
	.button-group {
		display: flex;
		gap: 0.2rem;
	}
	.square {
		width: 1.9rem;
		height: 1.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.9rem;
		line-height: 1;
		cursor: pointer;
	}
	.square:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.btn {
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
		text-decoration: none;
	}
	.btn:disabled {
		color: var(--muted-2);
		cursor: default;
	}
	.btn.primary {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 0.5rem;
		font-size: 0.8rem;
		color: var(--muted);
	}
	.text {
		width: 100%;
		padding: 0.3rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.86rem;
	}
	.small {
		font-size: 0.78rem;
		color: var(--muted-2);
	}
	.sw-window .sw-content {
		padding: 0.75rem 0.9rem;
	}
</style>
