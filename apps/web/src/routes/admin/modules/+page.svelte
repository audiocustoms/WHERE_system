<script lang="ts">
	import { onMount } from 'svelte';
	import { errorText } from '../../../lib/api';
	import { modules } from '../../../lib/modules.svelte';

	let message = $state<string | null>(null);
	let error = $state<string | null>(null);
	let busy = $state<string | null>(null);

	onMount(() => {
		void modules.load();
	});

	async function install(id: 'go' | 'people' | 'money') {
		busy = id;
		error = null;
		message = null;
		try {
			const outcome = await modules.install(id);
			const name = modules.status(id)?.product_name ?? id;
			message = outcome.shares_core_database
				? `${name} ist installiert und verwendet Datenbank und Mandanten der Kernanwendung.`
				: `${name} ist installiert und hat die Datenbank angelegt; die Kernanwendung übernimmt sie, sobald sie installiert wird.`;
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = null;
		}
	}

	async function remove(id: 'go' | 'people' | 'money') {
		busy = id;
		error = null;
		message = null;
		try {
			await modules.remove(id);
			message = 'Das Modul wurde entfernt. Die Daten bleiben in der Datenbank.';
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = null;
		}
	}
</script>

<svelte:head><title>Module — WHERE</title></svelte:head>

<h1>Module</h1>
<p class="intro">
	Alle Produkte der WHERE-Familie teilen eine Datenbank und ihre Mandanten.
	<strong>WHERE - is my stuff?</strong> ist die Kernanwendung; die übrigen Produkte werden hier
	hinzugefügt.
</p>

{#if modules.error}<p class="bad">{modules.error}</p>{/if}
{#if error}<p class="bad">{error}</p>{/if}
{#if message}<p class="good">{message}</p>{/if}

{#if !modules.ready}
	<p class="empty">{modules.loading ? 'Lade Modulregister …' : 'Kein Modulregister verfügbar.'}</p>
{:else}
	<p class="meta">
		Installation: {modules.snapshot?.installation_id ?? '—'} · Datenbank angelegt von:
		<strong>{modules.snapshot?.database_owner ?? '—'}</strong>
	</p>

	<table>
		<thead>
			<tr>
				<th>Produkt</th>
				<th>Beschreibung</th>
				<th>Status</th>
				<th></th>
			</tr>
		</thead>
		<tbody>
			{#each modules.snapshot?.modules ?? [] as modul (modul.id)}
				<tr>
					<td>
						<strong>{modul.product_name}</strong>
						{#if modul.is_core}<span class="badge">Kern</span>{/if}
					</td>
					<td>{modul.description}</td>
					<td>
						{#if modul.installed}
							<span class="good">installiert</span>
						{:else}
							<span class="off">nicht installiert</span>
						{/if}
					</td>
					<td class="actions">
						{#if !modul.is_core && !modul.installed}
							<button
								type="button"
								class="primary"
								disabled={busy !== null}
								onclick={() => install(modul.id as 'go' | 'people' | 'money')}
							>
								{busy === modul.id ? 'Installiere …' : 'Installieren'}
							</button>
						{:else if !modul.is_core && modul.installed}
							<button
								type="button"
								class="danger"
								disabled={busy !== null}
								onclick={() => remove(modul.id as 'go' | 'people' | 'money')}
							>
								{busy === modul.id ? 'Entferne …' : 'Entfernen'}
							</button>
						{/if}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>

	{#if !modules.coreInstalled}
		<div class="notice">
			Die Kernanwendung ist in dieser Datenbank noch nicht registriert. Wird
			<em>WHERE - is my stuff?</em> später installiert, verwendet sie die vorhandene Datenbank
			und Mandanten weiter, statt eine zweite anzulegen.
		</div>
	{/if}
{/if}

<style>
	.intro {
		color: var(--muted);
		font-size: 0.88rem;
		margin: 0 0 1rem;
		max-width: 60rem;
	}
	.meta {
		font-size: 0.82rem;
		color: var(--muted);
		margin-bottom: 0.8rem;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.86rem;
	}
	th,
	td {
		padding: 0.45rem 0.6rem;
		border-bottom: 1px solid var(--border);
		text-align: left;
		vertical-align: top;
	}
	.badge {
		margin-left: 0.4rem;
		padding: 0.05rem 0.35rem;
		border-radius: 999px;
		background: var(--surface-3);
		font-size: 0.7rem;
		color: var(--muted);
	}
	.actions {
		white-space: nowrap;
		text-align: right;
	}
	.bad {
		color: var(--bad);
	}
	.good {
		color: var(--good);
	}
	.off,
	.empty {
		color: var(--muted);
	}
	.primary {
		font-weight: 600;
	}
	.danger {
		color: var(--bad);
	}
	.notice {
		margin-top: 1rem;
		padding: 0.8rem;
		border: 1px solid var(--notice-border);
		background: var(--notice-bg);
		border-radius: 6px;
		font-size: 0.86rem;
	}
</style>
