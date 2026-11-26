/** Areas of the search window (`img/suche/Suchmaske_main.PNG`). */
import { t } from '../i18n.svelte';
import type { ModuleId } from '../api';

export type SearchArea =
	| 'addresses'
	| 'articles'
	| 'orders'
	| 'package-slips'
	| 'invoices'
	| 'incoming-invoices'
	| 'documents';

export interface SearchAreaEntry {
	id: SearchArea;
	text: string;
	/** Module that brings the area (`go` = orders/package slips, `money` = invoices). */
	module?: ModuleId;
}

/** Order and label as in the original. */
export const SEARCH_AREAS: SearchAreaEntry[] = [
	{ id: 'addresses', text: t('Adressen / Kunden') },
	{ id: 'articles', text: t('Artikel') },
	{ id: 'orders', text: t('Auftrag'), module: 'go' },
	{ id: 'package-slips', text: t('Packschein'), module: 'go' },
	{ id: 'invoices', text: t('Rechnung'), module: 'money' },
	{ id: 'incoming-invoices', text: t('Eingangsrechnung'), module: 'money' },
	{ id: 'documents', text: t('Dokumente') }
];
