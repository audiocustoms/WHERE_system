import type { ArticleSearchInput, PyroSearchInput, SetOperator } from '../api';

/**
 * Criteria of the article search as query parameters of the article list.
 *
 * The search mask lies as an overlay above the current page; after `Suchen`
 * it switches to `/articles` and loads the hits there. The parameters stay
 * readable in the address so the list can be loaded again.
 */
export function articleSearchParams(criteria: ArticleSearchInput): string {
	const params = new URLSearchParams();
	const set = (key: string, value: string | number | undefined | null) => {
		if (value === undefined || value === null || value === '') return;
		params.set(key, String(value));
	};

	if (criteria.article_type && criteria.article_type !== 'active') set('type', criteria.article_type);
	set('name', criteria.name);
	if (criteria.include_descriptions) set('desc', '1');
	set('no', criteria.article_no);
	set('inv', criteria.inventory_no);
	set('serial', criteria.serial_no);
	if (criteria.with_bundle) set('bundle', '1');
	if (criteria.article_groups?.length) set('groups', criteria.article_groups.join(','));
	if (criteria.article_groups_op && criteria.article_groups_op !== 'contains') {
		set('groups_op', criteria.article_groups_op);
	}
	if (criteria.search_index?.length) set('index', criteria.search_index.join(','));
	if (criteria.search_index_op && criteria.search_index_op !== 'contains') {
		set('index_op', criteria.search_index_op);
	}
	if (criteria.suppliers?.length) set('suppliers', criteria.suppliers.join(','));
	set('manufacturer', criteria.manufacturer);
	set('owner', criteria.owner);
	if (criteria.exclude_virtual) set('novirtual', '1');

	const pyro = pyroValues(criteria.pyro);
	if (Object.keys(pyro).length) set('pyro', JSON.stringify(pyro));

	const query = params.toString();
	return query ? `?${query}` : '';
}

/** Only the set Pyro values; empty fields stay out. */
function pyroValues(pyro: PyroSearchInput | null | undefined): Record<string, string | number> {
	const out: Record<string, string | number> = {};
	for (const [key, value] of Object.entries(pyro ?? {})) {
		if (value === null || value === undefined) continue;
		if (typeof value === 'number') {
			if (Number.isFinite(value)) out[key] = value;
		} else if (value.trim() !== '') {
			out[key] = value.trim();
		}
	}
	return out;
}

/**
 * Nothing chosen: only the preset article type „Aktiv". Then
 * `Suchen` leads to the article list instead of an empty hit list.
 */
export function isArticleSearchEmpty(criteria: ArticleSearchInput): boolean {
	const pyro = pyroValues(criteria.pyro);
	return (
		(criteria.article_type ?? 'active') === 'active' &&
		!criteria.name &&
		!criteria.include_descriptions &&
		!criteria.article_no &&
		!criteria.inventory_no &&
		!criteria.serial_no &&
		!criteria.with_bundle &&
		!criteria.article_groups?.length &&
		!criteria.search_index?.length &&
		!criteria.suppliers?.length &&
		!criteria.manufacturer &&
		!criteria.owner &&
		!criteria.exclude_virtual &&
		Object.keys(pyro).length === 0
	);
}

/** Reads the criteria back; `null` when none are set. */
export function articleSearchFromParams(params: {
	get(key: string): string | null;
}): ArticleSearchInput | null {
	const text = (key: string) => {
		const value = params.get(key)?.trim();
		return value ? value : undefined;
	};
	const number = (key: string) => {
		const value = Number(params.get(key));
		return Number.isFinite(value) && value > 0 ? value : null;
	};
	const list = (key: string) =>
		(text(key) ?? '')
			.split(',')
			.map((s) => s.trim())
			.filter(Boolean);

	let pyro: PyroSearchInput | null = null;
	const rawPyro = text('pyro');
	if (rawPyro) {
		try {
			const parsed: unknown = JSON.parse(rawPyro);
			if (parsed && typeof parsed === 'object') pyro = parsed as PyroSearchInput;
		} catch {
			// A broken parameter must not block the list.
			pyro = null;
		}
	}

	const criteria: ArticleSearchInput = {
		article_type: (text('type') as ArticleSearchInput['article_type']) ?? 'active',
		name: text('name'),
		include_descriptions: params.get('desc') === '1',
		article_no: number('no'),
		inventory_no: text('inv'),
		serial_no: text('serial'),
		with_bundle: params.get('bundle') === '1',
		article_groups: list('groups').map(Number).filter((n) => Number.isInteger(n) && n > 0),
		article_groups_op: (text('groups_op') as SetOperator) ?? 'contains',
		search_index: list('index'),
		search_index_op: (text('index_op') as SetOperator) ?? 'contains',
		suppliers: list('suppliers'),
		manufacturer: text('manufacturer'),
		owner: text('owner'),
		exclude_virtual: params.get('novirtual') === '1',
		pyro
	};

	return isArticleSearchEmpty(criteria) ? null : criteria;
}
