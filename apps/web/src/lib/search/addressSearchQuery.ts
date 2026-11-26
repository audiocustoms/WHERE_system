import { t } from '../i18n.svelte';
import type { AddressSearchInput, SetOperator } from '../api';

/**
 * Criteria of the address search as query parameters of the address page.
 *
 * The search mask lies as an overlay above the current page; after `Suchen`
 * it switches to `/addresses` and loads the hits there. The parameters
 * stay readable in the address so the list can be loaded again.
 */
export function addressSearchParams(criteria: AddressSearchInput): string {
	const params = new URLSearchParams();
	const set = (key: string, value: string | number | undefined | null) => {
		if (value === undefined || value === null || value === '') return;
		params.set(key, String(value));
	};

	if (criteria.address_type && criteria.address_type !== 'all') set('type', criteria.address_type);
	set('name', criteria.name);
	if (criteria.name_match && criteria.name_match !== 'contains') set('match', criteria.name_match);
	if (criteria.include_contacts) set('contacts', '1');
	set('group', criteria.group);
	set('no', criteria.customer_no);
	set('debtor', criteria.debtor_no);
	set('address', criteria.address_text);
	set('phone', criteria.phone);
	set('email', criteria.email);
	if (criteria.search_index?.length) set('index', criteria.search_index.join(','));
	if (criteria.search_index_op && criteria.search_index_op !== 'contains_any') {
		set('index_op', criteria.search_index_op);
	}
	if (criteria.include_contact_index) set('contact_index', '1');

	const query = params.toString();
	return query ? `?${query}` : '';
}

/** Reads the criteria back; `null` when none are set. */
export function addressSearchFromParams(params: {
	get(key: string): string | null;
}): AddressSearchInput | null {
	const text = (key: string) => {
		const value = params.get(key)?.trim();
		return value ? value : undefined;
	};
	const number = (key: string) => {
		const value = Number(params.get(key));
		return Number.isFinite(value) && value > 0 ? value : null;
	};

	const criteria: AddressSearchInput = {
		address_type: (text('type') as AddressSearchInput['address_type']) ?? 'all',
		name: text('name'),
		name_match: (text('match') as AddressSearchInput['name_match']) ?? 'contains',
		include_contacts: params.get('contacts') === '1',
		group: text('group'),
		customer_no: number('no'),
		debtor_no: text('debtor'),
		address_text: text('address'),
		phone: text('phone'),
		email: text('email'),
		search_index: (text('index') ?? '').split(',').map((s) => s.trim()).filter(Boolean),
		search_index_op: (text('index_op') as SetOperator) ?? 'contains_any',
		include_contact_index: params.get('contact_index') === '1'
	};

	const empty =
		criteria.address_type === 'all' &&
		!criteria.name &&
		!criteria.group &&
		!criteria.customer_no &&
		!criteria.debtor_no &&
		!criteria.address_text &&
		!criteria.phone &&
		!criteria.email &&
		!criteria.search_index?.length &&
		!criteria.include_contacts &&
		!criteria.include_contact_index;

	return empty ? null : criteria;
}

const TYP_TEXT: Record<string, string> = {
	customers: t('Kunden'),
	suppliers: t('Lieferanten')
};

const OPERATOR_TEXT: Record<string, string> = {
	contains: t('enthält'),
	contains_any: t('oder enthält'),
	contains_all: t('und enthält'),
	contains_none: t('enthält nicht')
};

/** Short form of the criteria for the header of the hit list. */
export function describeAddressSearch(criteria: AddressSearchInput): string[] {
	const parts: string[] = [];
	if (criteria.address_type && criteria.address_type !== 'all') {
		parts.push(TYP_TEXT[criteria.address_type] ?? criteria.address_type);
	}
	const labelled: { label: string; value: string | null | undefined }[] = [
		{ label: t('Name'), value: criteria.name },
		{ label: t('Adressgruppe'), value: criteria.group },
		{ label: t('Anschrift'), value: criteria.address_text },
		{ label: t('Telefon'), value: criteria.phone },
		{ label: t('eMail'), value: criteria.email },
		{ label: t('Debitor'), value: criteria.debtor_no }
	];
	for (const { label, value } of labelled) {
		if (value) parts.push(t('{label} „{value}"', { label, value }));
	}
	if (criteria.customer_no) parts.push(t('Kunden-Nr. {no}', { no: criteria.customer_no }));
	if (criteria.search_index?.length) {
		const op = OPERATOR_TEXT[criteria.search_index_op ?? 'contains_any'];
		parts.push(t('Suchindex {op} {terms}', { op, terms: criteria.search_index.join(', ') }));
	}
	if (criteria.include_contacts) parts.push(t('inkl. Ansprechpartner'));
	if (criteria.include_contact_index) parts.push(t('inkl. Ansprechpartner im Suchindex'));
	return parts;
}
