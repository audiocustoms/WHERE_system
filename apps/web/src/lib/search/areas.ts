/** Areas of the search window (`img/suche/Suchmaske_main.PNG`). */
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
	{ id: 'addresses', text: 'Adressen / Kunden' },
	{ id: 'articles', text: 'Artikel' },
	{ id: 'orders', text: 'Auftrag', module: 'go' },
	{ id: 'package-slips', text: 'Packschein', module: 'go' },
	{ id: 'invoices', text: 'Rechnung', module: 'money' },
	{ id: 'incoming-invoices', text: 'Eingangsrechnung', module: 'money' },
	{ id: 'documents', text: 'Dokumente' }
];
