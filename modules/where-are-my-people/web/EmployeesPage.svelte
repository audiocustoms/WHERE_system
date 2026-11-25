<script lang="ts">
	import { onMount } from 'svelte';
	import { api, errorText, type Employee } from '$core/api';
	import { auth } from '$core/auth.svelte';
	import { RIGHT } from '$core/rights';
	import AccessDenied from '$core/AccessDenied.svelte';
	import EmployeeForm from './EmployeeForm.svelte';
	import MasterExchangeDialog from './MasterExchangeDialog.svelte';

	type Active = 'all' | 'active' | 'inactive';

	let employees = $state<Employee[]>([]);
	let quickSearch = $state('');
	/** The search term of the loaded list; empty = all employees. */
		let applied = $state('');
	let loading = $state(false);
	let error = $state<string | null>(null);
	let exchange = $state(false);

	/** Left column like the product groups of the article list: active and department. */
		let activeFilter = $state<Active>('all');
	/** `null` = all, `''` = without department, otherwise the department name. */
		let departmentFilter = $state<string | null>(null);

	/** `null` = closed, `'new'` = empty form, otherwise the edited record. */
	let record = $state<Employee | 'new' | null>(null);

	async function load() {
		loading = true;
		error = null;
		try {
			employees = await 	api.people.list(applied || undefined);
		} catch (e) {
			error = errorText(e);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		// Without read rights the list stays empty; the panel names the missing right.
		if (auth.hasRight(RIGHT.mayReadPersonal)) void load();
	});

	/** Quick search: goes to the server so that it also matches many records. */
	function search() {
			applied = quickSearch.trim();
		void load();
	}

	function reset() {
		quickSearch = '';
			applied = '';
			activeFilter = 'all';
			departmentFilter = null;
		void load();
	}

	/** The row opens the form with the full record — including the qualifications. */
	async function edit(employee: Employee) {
		if (employee.id === null) return;
		try {
			record = await api.people.get(employee.id);
		} catch (e) {
			error = errorText(e);
		}
	}

	async function saved() {
		record = null;
		await load();
	}

	const department = (employee: Employee) => (employee.department ?? '').trim();

	/** The departments of the loaded records — like the product-group tree on the left. */
	const departments = $derived(
		[...new Set(employees.map(department).filter((name) => name !== ''))].sort((a, b) =>
			a.localeCompare(b, 'de')
		)
	);
		const withoutDepartment = $derived(employees.some((employee) => department(employee) === ''));

	const filtered = $derived(
		employees.filter((employee) => {
				if (activeFilter === 'active' && !employee.active) return false;
				if (activeFilter === 'inactive' && employee.active) return false;
				if (departmentFilter === null) return true;
				return department(employee) === departmentFilter;
		})
	);

	/** Header criteria; without any restriction it stays hidden. */
		const hasFilter = $derived(
			applied !== '' || activeFilter !== 'all' || departmentFilter !== null
	);

	/** Columns the list can be sorted by. */
	type SortKey =
		| 'personnel_no'
		| 'last_name'
		| 'department'
		| 'subdepartment'
		| 'position'
		| 'vacation_days';

	/** The list opens sorted by personnel number, ascending. */
	let sortKey = $state<SortKey>('personnel_no');
	let sortDir = $state<'asc' | 'desc'>('asc');

	/** First click sorts ascending, the second reverses the direction. */
		function sortBy(key: SortKey) {
		if (sortKey === key) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortKey = key;
			sortDir = 'asc';
		}
	}

	function sortValue(employee: Employee, key: SortKey): string {
		switch (key) {
			case 'personnel_no':
				return (employee.personnel_no ?? '').trim();
			case 'last_name':
				return employee.last_name.trim();
			case 'department':
					return department(employee);
			case 'subdepartment':
				return (employee.subdepartment ?? '').trim();
			case 'position':
				return (employee.position ?? '').trim();
			case 'vacation_days':
				return employee.vacation_days == null ? '' : String(employee.vacation_days);
		}
	}

	/** Empty values always come last; numbers are compared numerically. */
		const sorted = $derived.by(() => {
		const key = sortKey;
		const factor = sortDir === 'asc' ? 1 : -1;
		return [...filtered].sort((a, b) => {
			const left = sortValue(a, key);
			const right = sortValue(b, key);
			if (left === '' || right === '') {
				if (left === right) return 0;
				return left === '' ? 1 : -1;
			}
			return factor * left.localeCompare(right, 'de', { numeric: true });
		});
	});

	function ariaSort(key: SortKey): 'ascending' | 'descending' | 'none' {
		if (sortKey !== key) return 'none';
		return sortDir === 'asc' ? 'ascending' : 'descending';
	}

	/** `Dr. A. Lovelace` — without salutation, the first name only as initial. */
	function name(employee: Employee): string {
		const title = employee.title ? `${employee.title} ` : '';
		const initial = employee.first_name.trim().charAt(0);
		const firstName = initial ? `${initial.toUpperCase()}. ` : '';
		return `${title}${firstName}${employee.last_name}`;
	}

		const columns = 6;
</script>

<svelte:head><title>Personal — WHERE</title></svelte:head>

{#if !auth.hasRight(RIGHT.mayReadPersonal)}
	<AccessDenied pageName="Personal" right={RIGHT.mayReadPersonal} rightLabel="Personal lesen" />
{:else}
	<h1>Personal</h1>

		<div class="page">
		<aside class="filter" aria-label="Filter">
			<button
				type="button"
				class="entry"
					class:active={activeFilter === 'all' && departmentFilter === null}
					aria-pressed={activeFilter === 'all' && departmentFilter === null}
				onclick={() => {
						activeFilter = 'all';
						departmentFilter = null;
				}}>Alle Mitarbeiter zeigen</button
			>
			<button
				type="button"
				class="entry"
					class:active={activeFilter === 'active'}
					aria-pressed={activeFilter === 'active'}
					onclick={() => (activeFilter = 'active')}>Nur aktive Mitarbeiter zeigen</button
			>
			<button
				type="button"
				class="entry"
					class:active={activeFilter === 'inactive'}
					aria-pressed={activeFilter === 'inactive'}
					onclick={() => (activeFilter = 'inactive')}>Nur inaktive Mitarbeiter zeigen</button
			>

			<hr />

			{#if withoutDepartment}
				<button
					type="button"
					class="entry"
						class:active={departmentFilter === ''}
						aria-pressed={departmentFilter === ''}
					title="Mitarbeiter ohne Abteilung"
						onclick={() => (departmentFilter = '')}>Ohne Abteilung</button
				>
				<hr />
			{/if}

				{#each departments as group (group)}
				<button
					type="button"
					class="entry"
						class:active={departmentFilter === group}
						aria-pressed={departmentFilter === group}
						title={group}
						onclick={() => (departmentFilter = group)}>{group}</button
				>
			{:else}
				<p class="empty">Keine Abteilungen erfasst.</p>
			{/each}
		</aside>

		<section>
			<h2>Mitarbeiterliste</h2>
			<div class="toolbar">
				<input
					placeholder="Schnellsuche (Name, Personalnr., Abteilung, Position)"
					bind:value={quickSearch}
					onkeydown={(e) => {
						if (e.key !== 'Enter') return;
						search();
					}}
				/>
				<button onclick={search} disabled={loading}>{loading ? 'Lade …' : 'Schnellsuche'}</button>
				<button onclick={() => (exchange = true)}>Import / Export</button>
				{#if auth.hasRight(RIGHT.mayCreatePersonal)}
					<button class="search" onclick={() => (record = 'new')}>Mitarbeiter +</button>
				{/if}
			</div>

				{#if hasFilter}
				<div class="criteria">
					<span class="badge">Filter:</span>
						{#if applied}
						<span class="chip">	Suche „{applied}“</span>
					{/if}
						{#if activeFilter === 'active'}
						<span class="chip">Nur aktive</span>
						{:else if activeFilter === 'inactive'}
						<span class="chip">Nur inaktive</span>
					{/if}
						{#if departmentFilter === ''}
						<span class="chip">Ohne Abteilung</span>
						{:else if departmentFilter !== null}
						<span class="chip">	Abteilung „{departmentFilter}“</span>
					{/if}
					<button type="button" class="reset" onclick={reset}>Filter zurücksetzen</button>
				</div>
			{/if}

			<table>
				<thead>
					<tr>
						<th aria-sort={ariaSort('personnel_no')}>
							<button type="button" 	class="sorter" onclick={() => sortBy('personnel_no')}>
								Pers.-Nr.
								{#if sortKey === 'personnel_no'}
									<span class="arrow" aria-hidden="true">{sortDir === 'asc' ? '▲' : '▼'}</span>
								{/if}
							</button>
						</th>
						<th aria-sort={ariaSort('last_name')}>
							<button type="button" 	class="sorter" onclick={() => sortBy('last_name')}>
								Name
								{#if sortKey === 'last_name'}
									<span class="arrow" aria-hidden="true">{sortDir === 'asc' ? '▲' : '▼'}</span>
								{/if}
							</button>
						</th>
						<th aria-sort={ariaSort('department')}>
							<button type="button" 	class="sorter" onclick={() => sortBy('department')}>
								Abteilung
								{#if sortKey === 'department'}
									<span class="arrow" aria-hidden="true">{sortDir === 'asc' ? '▲' : '▼'}</span>
								{/if}
							</button>
						</th>
						<th aria-sort={ariaSort('subdepartment')}>
							<button type="button" 	class="sorter" onclick={() => sortBy('subdepartment')}>
								Unterabteilung
								{#if sortKey === 'subdepartment'}
									<span class="arrow" aria-hidden="true">{sortDir === 'asc' ? '▲' : '▼'}</span>
								{/if}
							</button>
						</th>
						<th aria-sort={ariaSort('position')}>
							<button type="button" 	class="sorter" onclick={() => sortBy('position')}>
								Position
								{#if sortKey === 'position'}
									<span class="arrow" aria-hidden="true">{sortDir === 'asc' ? '▲' : '▼'}</span>
								{/if}
							</button>
						</th>
						<th aria-sort={ariaSort('vacation_days')} title="Resturlaubstage">
							<button type="button" 	class="sorter" onclick={() => sortBy('vacation_days')}>
								Resturlaub
								{#if sortKey === 'vacation_days'}
									<span class="arrow" aria-hidden="true">{sortDir === 'asc' ? '▲' : '▼'}</span>
								{/if}
							</button>
						</th>
					</tr>
				</thead>
				<tbody>
						{#each sorted as employee (employee.id)}
						<tr
							class:inactive={!employee.active}
							tabindex="0"
							title="Mitarbeiter öffnen"
							onclick={() => edit(employee)}
							onkeydown={(e) => {
								if (e.key === 'Enter' || e.key === ' ') {
									e.preventDefault();
									edit(employee);
								}
							}}
						>
							<td class="number">{employee.personnel_no ?? '—'}</td>
								<td class="employeeName">
								{name(employee)}
								{#if employee.user_id != null}
									<span class="userAccess" title="Hat einen Benutzerzugang">✓</span>
								{/if}
							</td>
							<td>{employee.department ?? '—'}</td>
							<td>{employee.subdepartment ?? '—'}</td>
							<td>{employee.position ?? '—'}</td>
							<td class="days">{employee.vacation_days ?? '—'}</td>
						</tr>
					{:else}
						<tr>
								<td colspan={columns}>
									{applied
									? 'Kein Mitarbeiter passt zur Suche.'
										: departmentFilter !== null || activeFilter !== 'all'
										? 'Kein Mitarbeiter passt zum Filter.'
										: 'Noch keine Mitarbeiter erfasst.'}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</section>
	</div>

	{#if error}
		<p class="bad">{error}</p>
	{/if}

	{#if record !== null}
		{#key record}
			<EmployeeForm
				employee={record === 'new' ? null : record}
					onclose={() => (record = null)}
				onsaved={saved}
				onremoved={saved}
			/>
		{/key}
	{/if}

	{#if exchange}
		<MasterExchangeDialog
			area="employees"
			title="Personal"
			filename="personal"
			onimported={saved}
			onclose={() => (exchange = false)}
		/>
	{/if}
{/if}

<style>
	.bad {
		color: var(--bad);
	}
	h2 {
		font-size: 1.05rem;
		margin: 0 0 0.6rem;
	}

	/* Two columns as in the article list: filters left, list right. */
	.page {
		display: grid;
		grid-template-columns: 14rem minmax(0, 1fr);
		gap: 1rem;
		margin-top: 1rem;
		align-items: start;
	}
	@media (max-width: 900px) {
		.page {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	.filter {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 4px;
		padding: 0.35rem;
		font-size: 0.85rem;
	}
	.filter hr {
		border: none;
		border-top: 1px solid var(--border);
		margin: 0.35rem 0;
	}
	.entry {
		display: block;
		width: 100%;
		border: none;
		background: none;
		color: var(--label);
		font: inherit;
		text-align: left;
		padding: 0.2rem 0.25rem;
		border-radius: 3px;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.entry:hover {
		background: var(--surface-3);
	}
	.entry.active {
		background: var(--accent);
		color: #fff;
	}
	.empty {
		color: var(--muted-2);
		font-size: 0.8rem;
		margin: 0.3rem 0.35rem;
	}

	.toolbar {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	input {
		flex: 1;
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: 4px;
	}
	button {
		padding: 0.4rem 1rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		cursor: pointer;
	}
		.toolbar button.search {
		border-color: var(--accent);
		color: var(--accent);
		font-weight: 600;
	}
	.criteria {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.35rem;
		margin-bottom: 0.75rem;
		font-size: 0.8rem;
	}
	.badge {
		color: var(--muted);
	}
	.chip {
		background: var(--surface-3);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 0.1rem 0.5rem;
	}
	.reset {
		border: none;
		background: none;
		color: var(--accent);
		cursor: pointer;
		padding: 0;
		font-size: 0.8rem;
		text-decoration: underline;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		background: var(--surface);
	}
	th,
	td {
		text-align: left;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--surface-3);
		font-size: 0.9rem;
	}
	td.number {
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	td.days {
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	/* The checkmark behind the name: the employee is an account of the user administration. */
		td.employeeName {
		white-space: nowrap;
	}
	.userAccess {
		margin-left: 0.35rem;
		color: var(--good);
		font-weight: 700;
	}
	/* Header: the column names are buttons for sorting. */
	th {
		padding: 0;
	}
	.sorter {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		width: 100%;
		padding: 0.4rem 0.6rem;
		border: none;
		background: none;
		color: inherit;
		font: inherit;
		font-weight: 600;
		text-align: left;
		cursor: pointer;
	}
	.sorter:hover {
		background: var(--surface-3);
	}
	.arrow {
		font-size: 0.7rem;
		color: var(--accent);
	}
	/* Whole row clickable, same hover as in the article list. */
	tbody tr {
		cursor: pointer;
	}
	tbody tr:hover > td,
	tbody tr:focus-visible > td {
		background: var(--surface-3);
	}
	tbody tr:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}
	tr.inactive > td {
		color: var(--muted-2);
	}
</style>
