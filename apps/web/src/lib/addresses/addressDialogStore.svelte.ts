import type { Address } from '../api';

/**
 * State of the address window. `Adresse +` opens the duplicate check and
 * from there the empty mask; a click on a row of the address list opens
 * the same mask with the existing record.
 */
export type AddressDialog = { mode: 'create' } | { mode: 'edit'; address: Address };

let dialog = $state<AddressDialog | null>(null);

export const addressDialog = () => dialog;

export const addressDialogOpen = () => dialog !== null;

export function openAddressCreate() {
	dialog = { mode: 'create' };
}

export function openAddressEdit(address: Address) {
	dialog = { mode: 'edit', address };
}

export function closeAddressDialog() {
	dialog = null;
}
