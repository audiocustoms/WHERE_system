<script lang="ts">
	import SearchWindow from './SearchWindow.svelte';
	import { SEARCH_AREAS, type SearchArea } from './areas';
	import { auth } from '../auth.svelte';
	import { modules } from '../modules.svelte';
	import { RIGHT } from '../rights';

	interface Props {
		onclose: () => void;
		onsearch: (area: SearchArea) => void;
	}

	let { onclose, onsearch }: Props = $props();

	/** Only areas the account has read right for (`null` = always). */
	const RIGHT_PER_AREA: Partial<Record<SearchArea, string>> = {
		addresses: RIGHT.mayReadAddresses,
		articles: RIGHT.mayReadArticles
	};

	const areas = $derived(
		SEARCH_AREAS.filter((entry) => {
			if (entry.module && !modules.isInstalled(entry.module)) return false;
			const right = RIGHT_PER_AREA[entry.id];
			return !right || auth.hasRight(right);
		})
	);

	// „Adressen / Kunden" is preselected in the original — as far as allowed.
	// svelte-ignore state_referenced_locally
	let area = $state<SearchArea>(areas[0]?.id ?? 'addresses');
</script>

<SearchWindow title="Suchen" width="19.5rem" {onclose}>
	<p class="sw-heading">Bereich</p>
	<div class="sw-area">
		{#each areas as entry (entry.id)}
			<label>
				<input type="radio" name="bereich" value={entry.id} bind:group={area} />
				{entry.text}
			</label>
		{/each}
	</div>

	{#snippet actions(cancel)}
		<button class="sw-button" type="button" onclick={cancel}>Abbrechen</button>
		<button class="sw-button sw-primary" type="button" onclick={() => onsearch(area)}>Suchen</button>
	{/snippet}
</SearchWindow>
