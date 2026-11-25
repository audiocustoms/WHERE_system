<script lang="ts">
	import { goto } from '$app/navigation';
	import { emptyAddress } from '../api';
	import { auth } from '../auth.svelte';
	import { RIGHT } from '../rights';
	import { addressDialog, closeAddressDialog } from './addressDialogStore.svelte';
	import DuplicateCheck from './DuplicateCheck.svelte';
	import AddressFormDialog from './AddressFormDialog.svelte';

	const dialog = addressDialog();

	/** First the duplicate check, then the mask (`img/adressen`). */
	let stage = $state<'check' | 'form'>('check');
	let draft = $state(emptyAddress());

	function toForm(term: string) {
		// The checked term is the name of the new address — it is already in the field.
		draft = { ...emptyAddress(), name1: term };
		stage = 'form';
	}

	async function saved() {
		closeAddressDialog();
		// If the list already sits behind, it has to reload the record.
		if (location.pathname === '/addresses') location.reload();
		else await goto('/addresses');
	}
</script>

{#if dialog?.mode === 'edit'}
	<AddressFormDialog
		initial={dialog.address}
		id={dialog.address.id}
		schreibbar={auth.hasRight(RIGHT.mayEditAddress)}
		loeschbar={auth.hasRight(RIGHT.mayDeleteAddress)}
		oncancel={closeAddressDialog}
		onsaved={saved}
	/>
{:else if stage === 'check'}
	<DuplicateCheck onexit={closeAddressDialog} oncontinue={toForm} />
{:else}
	<AddressFormDialog
		initial={draft}
		schreibbar={auth.hasRight(RIGHT.mayCreateAddress)}
		oncancel={closeAddressDialog}
		onsaved={saved}
	/>
{/if}
