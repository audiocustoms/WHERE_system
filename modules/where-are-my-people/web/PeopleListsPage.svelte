<script lang="ts">
	import { auth } from '$core/auth.svelte';
	import { RIGHT } from '$core/rights';
	import AccessDenied from '$core/AccessDenied.svelte';
	import MasterExchangeDialog from './MasterExchangeDialog.svelte';
	import PeopleListPanel from './PeopleListPanel.svelte';

	/** Bumped to reload the panels after an import. */
	let round = $state(0);
	let exchange = $state(false);

	/** Reading is open to the personnel manager and to the disposition alike. */
	const mayRead = $derived(
		auth.isAdmin ||
			auth.hasRight(RIGHT.mayReadPersonal) ||
			auth.hasRight(RIGHT.mayReadDispo)
	);
</script>

<svelte:head><title>Abteilungen und Positionen — WHERE</title></svelte:head>

{#if !mayRead}
	<AccessDenied
		pageName="Abteilungen und Positionen"
		right={RIGHT.mayReadDispo}
		rightLabel="Personaldispo lesen"
	/>
{:else}
	<div class="wide">
		<div class="head-row">
			<h1>Abteilungen und Positionen</h1>
			{#if auth.isAdmin}
				<button type="button" class="exchange" onclick={() => (exchange = true)}>
					Import / Export
				</button>
			{/if}
		</div>

		<div class="columns">
			{#key round}
				<PeopleListPanel kind="departments" title="Abteilungen" singular="Abteilung" />
				<PeopleListPanel
					kind="subdepartments"
					title="Unterabteilungen"
					singular="Unterabteilung"
				/>
				<PeopleListPanel kind="locations" title="Standorte" singular="Standort" />
				<PeopleListPanel kind="positions" title="Positionen" singular="Position" />
			{/key}
		</div>
	</div>

	{#if exchange}
		<MasterExchangeDialog
			area="lists"
			title="Abteilungen und Positionen"
			filename="abteilungen-positionen"
			onimported={() => (round += 1)}
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
	.columns {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
		gap: 1rem;
		margin-top: 1rem;
		align-items: start;
	}
</style>
