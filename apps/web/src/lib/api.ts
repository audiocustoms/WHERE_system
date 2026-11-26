/**
 * Typed client for the WHERE API.
 *
 * Resource names and payloads mirror the Axum routes in `crates/where-api`.
 * Errors surface as `ApiError` carrying the RFC 9457 problem document the
 * backend returns, so callers can show `detail` directly.
 */

const BASE: string = resolveBase();

/**
 * Where to send API calls.
 *
 * - Tauri with a runtime configuration: the shell injects
 *   `window.__WHERE_API_BASE__` before the SPA starts, so a workstation of a
 *   central server talks to that server instead of its own machine.
 * - `VITE_API_BASE` set at build time: wins over the Tauri default.
 * - Browser behind the Vite proxy, or served same-origin by Axum: relative URLs.
 * - Tauri without either: the local API on 127.0.0.1:8090.
 */
function resolveBase(): string {
	const injected =
		typeof window !== 'undefined'
			? ((window as { __WHERE_API_BASE__?: string }).__WHERE_API_BASE__ ?? '')
			: '';
	if (injected) return injected.replace(/\/+$/, '');

	const configured = import.meta.env.VITE_API_BASE as string | undefined;
	if (configured) return configured.replace(/\/+$/, '');

	const isTauri =
		typeof window !== 'undefined' &&
		('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
	return isTauri ? 'http://127.0.0.1:8090' : '';
}

export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly title: string,
		detail: string
	) {
		super(detail);
		this.name = 'ApiError';
	}
}

/** Uniform error text; `mitTitel` additionally prefixes the problem kind. */
export function errorText(e: unknown, withTitle = false): string {
	if (e instanceof ApiError) {
		return withTitle ? `${e.title}: ${e.message}` : e.message;
	}
	if (e instanceof Error) return e.message;
	return String(e);
}

export interface Page<T> {
	items: T[];
	total: number;
	limit: number;
	offset: number;
}

async function problemOf(response: Response): Promise<ApiError> {
	const problem = (await response.json().catch(() => null)) as {
		title?: string;
		detail?: string;
	} | null;
	return new ApiError(
		response.status,
		problem?.title ?? 'error',
		problem?.detail ?? `${response.status} ${response.statusText}`
	);
}

/**
 * Token of the signed-in session. `lib/auth.svelte.ts` sets it after a login and on
 * restore; every request carries it as `Authorization: Bearer …`.
 */
let authToken: string | null = null;

export function setAuthToken(token: string | null): void {
	authToken = token;
}

/**
 * What happens when the server rejects a request with `401`: the session has
 * expired or was ended. `lib/auth.svelte.ts` hooks in here so that the
 * application returns to the mask instead of stacking error messages.
 */
let onUnauthorized: (() => void) | null = null;

export function setUnauthorizedHandler(handler: (() => void) | null): void {
	onUnauthorized = handler;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	const headers: Record<string, string> = {
		'content-type': 'application/json',
		...(init?.headers as Record<string, string> | undefined)
	};
	if (authToken) headers.authorization = `Bearer ${authToken}`;
	const response = await fetch(`${BASE}${path}`, { ...init, headers });

	if (!response.ok) {
		// An expired session leads back to the login mask.
		if (response.status === 401 && !path.startsWith('/api/v1/auth/login')) onUnauthorized?.();
		throw await problemOf(response);
	}
	if (response.status === 204) return undefined as T;
	return (await response.json()) as T;
}

function query(params: Record<string, string | number | undefined>): string {
	const search = new URLSearchParams();
	for (const [key, value] of Object.entries(params)) {
		if (value !== undefined && value !== '') search.set(key, String(value));
	}
	const s = search.toString();
	return s ? `?${s}` : '';
}

/**
 * `multipart/form-data` with the session. Do not set `content-type`: the boundary
 * string is written by the browser itself.
 */
async function sendForm<T>(path: string, form: FormData): Promise<T> {
	const headers: Record<string, string> = {};
	if (authToken) headers.authorization = `Bearer ${authToken}`;
	const response = await fetch(`${BASE}${path}`, { method: 'POST', body: form, headers });
	if (!response.ok) {
		if (response.status === 401) onUnauthorized?.();
		throw await problemOf(response);
	}
	return (await response.json()) as T;
}

/**
 * Posts a body of raw bytes (a file the user picked) and reads JSON back.
 * `content-type` names what the server expects, not what the browser guesses.
 */
async function sendBytes<T>(path: string, bytes: ArrayBuffer, type: string): Promise<T> {
	const headers: Record<string, string> = { 'content-type': type };
	if (authToken) headers.authorization = `Bearer ${authToken}`;
	const response = await fetch(`${BASE}${path}`, { method: 'POST', body: bytes, headers });
	if (!response.ok) {
		if (response.status === 401) onUnauthorized?.();
		throw await problemOf(response);
	}
	return (await response.json()) as T;
}

/**
 * Loads a body as a blob. Images and PDFs are not attached to the page as
 * `<img src>`: the server demands the session token in the header, which an
 * element does not send along. The caller turns it into an object URL.
 */
async function fetchBlob(url: string): Promise<Blob> {
	const headers: Record<string, string> = {};
	if (authToken) headers.authorization = `Bearer ${authToken}`;
	const response = await fetch(url, { headers });
	if (!response.ok) {
		if (response.status === 401) onUnauthorized?.();
		throw await problemOf(response);
	}
	return await response.blob();
}

// ---------------------------------------------------------------- Address

/** "Bestand" (stock) above the address list. */
export type AddressState = 'active' | 'inactive' | 'all';

/** Sortable columns of the address list. */
export type AddressSort = 'type' | 'number' | 'name' | 'country' | 'postal_code' | 'city';

/** Parameters of the address list (`GET /api/v1/addresses`). */
export interface AddressListParams {
	limit?: number;
	offset?: number;
	q?: string;
	active?: AddressState;
	sort?: AddressSort;
	dir?: 'asc' | 'desc';
}

export interface Address {
	id: number;
	customer_no: number | null;
	title: string | null;
	name1: string | null;
	name2: string | null;
	name3: string | null;
	salutation: string | null;
	street: string | null;
	country: string | null;
	city: string | null;
	vat_id: string | null;
	debtor: string | null;
	phone: string | null;
	email: string | null;
	/** Free text of the "Adressgruppe" (address group, `GruppenNameAdresse`). */
	group: string | null;
	is_customer: boolean;
	is_supplier: boolean;
	auftragssperre: boolean;
	mahnsperre: boolean;

	// ---- Base data
	active: boolean;
	/** `adr_Einzelperson` — "Firma" (company) vs. "Einzelperson" (individual). */
	is_person: boolean;
	birthday: string | null;
	address_addition: string | null;
	postal_code: string | null;
	state: string | null;
	fax: string | null;
	mobile: string | null;
	website: string | null;
	is_interested: boolean;
	is_location: boolean;
	is_employee: boolean;
	is_vip: boolean;

	// ---- Prospect / customer
	eori_no: string | null;
	tax_number: string | null;
	register_entry: string | null;
	register_seat: string | null;
	credit_limit: string | null;
	discount_rental: string | null;
	discount_sale: string | null;
	bank: string | null;
	bank_account: string | null;
	bank_code: string | null;
	iban: string | null;
	bic: string | null;

	// ---- Supplier
	supplier_no: number | null;
	creditor: string | null;
	supplier_revenue: string | null;
	supplier_discount_rental: string | null;
	supplier_discount_purchase: string | null;
	supplier_bank: string | null;
	supplier_bank_account: string | null;
	supplier_bank_code: string | null;
	supplier_iban: string | null;
	supplier_bic: string | null;
}

export type AddressInput = Omit<Address, 'id'>;

/** Empty address for the creation form (customer and prospect are mutually exclusive). */
export function emptyAddress(): AddressInput {
	return {
		customer_no: null,
		title: null,
		name1: '',
		name2: null,
		name3: null,
		salutation: null,
		street: null,
		country: null,
		city: null,
		vat_id: null,
		debtor: null,
		phone: null,
		email: null,
		group: null,
		is_customer: false,
		is_supplier: false,
		auftragssperre: false,
		mahnsperre: false,
		active: true,
		is_person: false,
		birthday: null,
		address_addition: null,
		postal_code: null,
		state: null,
		fax: null,
		mobile: null,
		website: null,
		is_interested: false,
		is_location: false,
		is_employee: false,
		is_vip: false,
		eori_no: null,
		tax_number: null,
		register_entry: null,
		register_seat: null,
		credit_limit: null,
		discount_rental: null,
		discount_sale: null,
		bank: null,
		bank_account: null,
		bank_code: null,
		iban: null,
		bic: null,
		supplier_no: null,
		creditor: null,
		supplier_revenue: null,
		supplier_discount_rental: null,
		supplier_discount_purchase: null,
		supplier_bank: null,
		supplier_bank_account: null,
		supplier_bank_code: null,
		supplier_iban: null,
		supplier_bic: null
	};
}

/** Criteria of the address search (`img/suche/Adresssuche.PNG`). */
export interface AddressSearchInput {
	address_type?: 'all' | 'customers' | 'suppliers';
	name?: string;
	name_match?: 'contains' | 'starts_with' | 'equals';
	include_contacts?: boolean;
	group?: string;
	customer_no?: number | null;
	debtor_no?: string;
	address_text?: string;
	phone?: string;
	email?: string;
	search_index?: string[];
	search_index_op?: SetOperator;
	include_contact_index?: boolean;
	limit?: number;
	offset?: number;
}

// ---------------------------------------------------------------- Article master

/** The six text blocks of the form (tabs Kurz … WebShop). */
export interface ArticleTexts {
	short: string | null;
	long: string | null;
	technical_details: string | null;
	scope_of_delivery: string | null;
	packaging_info: string | null;
	webshop: string | null;
}

/** Prices and properties of the "Miete" (rental) and "Verkauf" (sale) tabs. */
export interface ArticlePricing {
	// ---- Rental: calculation
	rental_roll_days_active: boolean;
	rental_off_days_active: boolean;
	rental_weekend_active: boolean;

	// ---- Rental: prices
	rental_price_per_day: string | null;
	rental_base_price: string | null;
	rental_cost_price: string | null;
	rental_price_per_day_roll_days: string | null;
	rental_base_price_roll_days: string | null;
	rental_cost_price_roll_days: string | null;
	rental_price_per_day_off_days: string | null;
	rental_base_price_off_days: string | null;
	rental_cost_price_off_days: string | null;
	rental_price_weekend: string | null;
	rental_cost_price_weekend: string | null;
	deposit: string | null;
	used_sale_price: string | null;
	/** Percentage (0–100). */
	rental_vat_rate: number | null;
	rental_revenue_account: string | null;
	rental_cost_center: string | null;
	capital_binding: string | null;
	approximate_location: string | null;
	factor_scale: number | null;
	factor_scale_roll_days: number | null;
	factor_scale_off_days: number | null;

	// ---- Rental: properties
	packaging_release: boolean;
	rental_no_discount: boolean;
	rental_not_insurable: boolean;
	material_only: boolean;
	unique_article: boolean;
	checkout_requires_serial: boolean;
	fixed_installation: boolean;
	inspection_on_checkin: boolean;
	function_check_on_checkin: boolean;
	consumption_recording: boolean;

	// ---- Sale: prices
	sale_price_net: string | null;
	sale_price_gross: string | null;
	sale_retail_price: string | null;
	sale_cost_price_net: string | null;
	sale_markup_percent: string | null;
	/** Percentage (0–100). */
	sale_vat_rate: number | null;
	sale_revenue_account: string | null;
	sale_cost_center: string | null;
	quantity_scale: number | null;

	// ---- Sale: properties
	sale_no_discount: boolean;
	sell_full_packages: boolean;
	package_unit: string | null;
	package_size: string | null;
	reorder_level: string | null;
}

/** Scale for usage days (rental) or quantities (sale). */
export interface ScaleCategory {
	id: number;
	name: string | null;
	for_sale: boolean;
	is_default: boolean;
}

/** One tier of a scale. */
export interface ScaleStep {
	from_value: string | null;
	factor: string | null;
	discount: string | null;
}

/** Complete article — matches the form `img/articles_01..05.PNG`. */
export interface Article {
	id: number | null;
	article_no: number;
	inventory_no: string | null;
	name: string;
	short_name: string | null;
	match_code: string | null;
	unit: string | null;

	active: boolean;
	inventory: boolean;
	rental: boolean;
	sale: boolean;
	text_sync: boolean;

	stock_tracking: boolean;
	no_own_stock: boolean;
	individual_items: boolean;
	allocation_control: boolean;
	virtual_article: boolean;

	ean_code: string | null;
	manufacturer_no: string | null;
	manufacturer: string | null;
	manufacturer_country: string | null;
	retail_price: string | null;
	customs_tariff_export: string | null;
	customs_tariff_import: string | null;

	insurance_value_net: string | null;
	insurance_value_gross: string | null;
	team_group: string | null;
	commission_class: string | null;

	width_net: string | null;
	height_net: string | null;
	depth_net: string | null;
	volume_net: string | null;
	weight_net: string | null;

	width_gross: string | null;
	height_gross: string | null;
	depth_gross: string | null;
	volume_gross: string | null;
	weight_gross: string | null;

	voltage_from: string | null;
	voltage_to: string | null;
	current_draw: string | null;
	power: string | null;
	setup_minutes: number | null;
	teardown_minutes: number | null;
	check_out_seconds: number | null;
	check_in_seconds: number | null;

	inspection_class: number | null;
	inspection_interval_months: number | null;
	inspection_minutes: number | null;
	workshop_by_usage_days: boolean;
	workshop_max_usage_days: string | null;
	workshop_fixed_intervals: boolean;
	workshop_by_days: boolean;
	workshop_by_months: boolean;
	workshop_days: number | null;
	workshop_months: number | null;

	pricing: ArticlePricing;

	texts: ArticleTexts;
	article_groups: number[];
	/** "Lieferanten" (suppliers) tab (`"Artikel_Lieferanten"`). */
	suppliers: ArticleSupplier[];
	/** "Pyro" tab (`pyro_*` fields of the article). */
	pyro: ArticlePyro;
	erstellt_am: string | null;
	changed_at: string | null;
}

/** Supplier of an article — an address with `Type_Lieferant`. */
export interface ArticleSupplier {
	supplier_id: number;
	supplier_no: number | null;
	name: string | null;
	position: number | null;
	purchase_price: string | null;
	delivery_days: number | null;
	favorite: boolean;
	sales_article: boolean;
	supplier_article_no: string | null;
}

/** Pyro tab of the article form. */
export interface ArticlePyro {
	un_number: string | null;
	un_description: string | null;
	hazard_class: string | null;
	caliber: number | null;
	lateral_safety_distance: string | null;
	vertical_safety_distance: string | null;
	ascending_height: string | null;
	sound_pressure_level: string | null;
	sound_pressure_range: string | null;
	tunnel_restriction_code: string | null;
	delay_time: string | null;
	effect_duration: string | null;
	category: string | null;
	bam_id: string | null;
	classification: string | null;
	packing_group: string | null;
	approval_number: string | null;
	neq: string | null;
	neq_unit: string | null;
	un_points: string | null;
	transport_category: string | null;
}

/** Empty Pyro tab for a new article. */
export function emptyPyro(): ArticlePyro {
	return {
		un_number: null,
		un_description: null,
		hazard_class: null,
		caliber: null,
		lateral_safety_distance: null,
		vertical_safety_distance: null,
		ascending_height: null,
		sound_pressure_level: null,
		sound_pressure_range: null,
		tunnel_restriction_code: null,
		delay_time: null,
		effect_duration: null,
		category: null,
		bam_id: null,
		classification: null,
		packing_group: null,
		approval_number: null,
		neq: null,
		neq_unit: null,
		un_points: null,
		transport_category: null
	};
}

/** List row of the article overview. */
export interface ArticleList {
	id: number;
	article_no: number;
	name: string | null;
	manufacturer: string | null;
	inventory_no: string | null;
	active: boolean;
	rental: boolean;
	sale: boolean;
	/** Total across all warehouses; `null` = no booking exists. */
	stock_on_hand: string | null;
	/** Stock minus fixed reservations and withdrawals. */
	available_stock: string | null;
	/** `"Bestand_GesamtMiete"` across all warehouses (column "Bestand Miete"). */
	rental_stock: string | null;
	/** Sale stock (new + used) across all warehouses (column "Bestand Verkauf"). */
	sale_stock: string | null;
	/** `"Mietpreis"` (column "Preis Miete"). */
	rental_price: string | null;
	/** `"VKPreisNetto"` (column "Preis Verkauf"). */
	sale_price: string | null;
}

/** Stock of one article in one warehouse (upper grid of the form). */
export interface StockRow {
	stock_id: number;
	stock_name: string;
	is_default: boolean;
	/** The warehouse takes part in the working masks (`where_zugriff`). */
	zugriff: boolean;
	/** Material may be taken out of it (`where_entnahme`). */
	entnahme: boolean;
	/** `null` = no booking exists for this warehouse yet. */
	total: string | null;
	rental: string | null;
	sale: string | null;
	inventory: string | null;
	used_sale: string | null;
	reserved: string | null;
	withdrawal: string | null;
	available: string | null;
}

/** One device or one batch (lower grid, right side = device data). */
export interface Device {
	id: number;
	serial_no: string | null;
	inventory_no: string | null;
	inventory_no2: string | null;
	quantity: string | null;
	/** Usage areas — several at the same time are possible. */
	inventory: boolean;
	rental: boolean;
	sale: boolean;
	used_sale: boolean;
	/** e.g. "Inventar, Miete". */
	usage_text: string;
	stock_id: number | null;
	stock_name: string | null;
	bin_location: string | null;
	condition: string | null;
	discarded: boolean;
	/** "im Lager", "ausgegeben: <Ort>" or "ausgesondert". */
	status: string;
	purchase_price: string | null;
	purchase_date: string | null;
	supplier_no: number | null;
	owner: string | null;
	warranty_months: number | null;
	ext_barcode: string | null;
	year_built: number | null;
	note: string | null;
	without_serial: boolean;
	recorded_at: string | null;
	/** Place of the house the device stands at while it is issued. */
	place_id: number | null;
	place_name: string | null;
	issued_from: string | null;
	issued_until: string | null;
}

/** A place of the house where material stands while it is in use (`Foyer`, `Podium`). */
export interface Place {
	id: number;
	name: string;
	note: string | null;
	active: boolean;
}

/** Input for issuing material to a place of the house. */
export interface IssueInput {
	place_id: number;
	/** Amount; the whole row when omitted. */
	quantity: string | null;
	/** Day the material is expected back, `YYYY-MM-DD`. */
	until: string | null;
	note: string | null;
}

/** Input for moving material to another warehouse. */
export interface TransferInput {
	stock_id: number;
	quantity: string | null;
	note: string | null;
}

/** One stocktake (`Inventur`) of a warehouse. */
export interface Stocktake {
	id: number;
	warehouse_id: number;
	warehouse_name: string | null;
	name: string;
	note: string | null;
	/** `open`, `closed` or `cancelled`. */
	state: string;
	started_at: string;
	started_by: string;
	finished_at: string | null;
	finished_by: string | null;
	lines: number;
	counted: number;
	differences: number;
}

/** One line of a stocktake: a device/batch as the book had it, plus what was counted. */
export interface StocktakeLine {
	id: number;
	device_id: number;
	article_id: number;
	article_no: number | null;
	article_name: string | null;
	serial_no: string | null;
	bin_location: string | null;
	book_quantity: string;
	counted_quantity: string | null;
	/** `counted − book`; `null` while the line is open. */
	difference: string | null;
	counted_at: string | null;
	counted_by: string | null;
	note: string | null;
	device_gone: boolean;
}

export interface StocktakeDetail {
	stocktake: Stocktake;
	lines: StocktakeLine[];
}

/** One line of the reorder list (`Beschaffung`). */
export interface PurchasingSuggestion {
	article_id: number;
	article_no: number | null;
	article_name: string | null;
	warehouse_id: number;
	warehouse_name: string | null;
	/** `Gesamt − Festblockade − Entnahme` of that warehouse. */
	available: string;
	/** `Meldebestand` of the article. */
	reorder_level: string;
	/** What has to be ordered. */
	missing: string;
	/** Consumed within the window. */
	used: string;
	lead_time_days: number | null;
	supplier_no: number | null;
	supplier_name: string | null;
	supplier_article_no: string | null;
}

/** One purchase order (`Bestellung`) of the warehouse. */
export interface PurchaseOrder {
	id: number;
	/** Number from the series `Bestellnummer`. */
	number: string;
	supplier_no: number | null;
	supplier_name: string | null;
	ordered_at: string;
	expected_at: string | null;
	note: string | null;
	/** `open`, `done` or `cancelled`. */
	state: string;
	created_at: string;
	created_by: string;
	lines: number;
	open_lines: number;
	ordered: string;
	received: string;
	missing: string;
}

/** One line of an order: what was ordered and what arrived. */
export interface PurchaseOrderLine {
	id: number;
	order_id: number;
	line_no: number;
	article_id: number;
	article_no: number | null;
	article_name: string | null;
	quantity: string;
	/** Sum of the devices that arrived on this line. */
	received: string;
	/** `quantity − received`, never negative. */
	open: string;
	unit_price: string | null;
	note: string | null;
}

export interface PurchaseOrderDetail {
	order: PurchaseOrder;
	lines: PurchaseOrderLine[];
}

/** A line that still expects material — what the goods receipt offers. */
export interface OpenOrderLine {
	line_id: number;
	order_id: number;
	number: string;
	supplier_name: string | null;
	expected_at: string | null;
	line_no: number;
	article_id: number;
	quantity: string;
	received: string;
	open: string;
}

/** One line of the stock value report (`Bestandswert`). */
export interface StockValueLine {
	warehouse_id: number;
	warehouse_name: string | null;
	/** Empty when the article belongs to no group. */
	group_name: string;
	articles: number;
	amount: string;
	/** `Menge × Kaufpreis`. */
	value: string;
}

/** Contents of the "Lagerbestand" tab. */
export interface StockLevel {
	stock: StockRow[];
	devices: Device[];
	/** `Meldebestand` of the article; `null` = none set. */
	reorder_level: string | null;
}

/** Input for the goods receipt. */
export interface GoodsReceiptInput {
	serial_no: string | null;
	without_serial: boolean;
	stock_id: number;
	bin_location: string | null;
	condition: string | null;
	purchase_price: string | null;
	purchase_date: string | null;
	supplier_no: number | null;
	inventory_no: string | null;
	reason: string | null;
	/** Line of a purchase order this delivery belongs to. */
	order_line: number | null;
	/** At least one; several at the same time are possible. */
	inventory: boolean;
	rental: boolean;
	sale: boolean;
	used_sale: boolean;
	quantity: string | null;
}

/**
 * Result of a goods receipt.
 *
 * With serial numbers each piece becomes a device with its own number; a batch
 * without serial numbers stays one record whose `quantity` is the amount.
 */
export interface GoodsReceiptResult {
	devices: Device[];
	/** Received quantity — the sum over all created records. */
	quantity: string;
}

/** Editable device data of the right-hand side. */
export interface DeviceInput {
	serial_no: string | null;
	inventory_no: string | null;
	inventory_no2: string | null;
	stock_id: number | null;
	bin_location: string | null;
	condition: string | null;
	inventory: boolean;
	rental: boolean;
	sale: boolean;
	used_sale: boolean;
	discarded: boolean;
	purchase_price: string | null;
	purchase_date: string | null;
	supplier_no: number | null;
	owner: string | null;
	warranty_months: number | null;
	ext_barcode: string | null;
	year_built: number | null;
	note: string | null;
}

/** One line of the stock history of an article or a device. */
export interface StockMovement {
	id: number;
	device_id: number;
	serial_no: string | null;
	/** `receipt`, `change`, `move`, `issue`, `return`, `retire`, `reinstate`, `delete`. */
	kind: string;
	quantity: string | null;
	warehouse_from: number | null;
	warehouse_from_name: string | null;
	warehouse_to: number | null;
	warehouse_to_name: string | null;
	bin_from: string | null;
	bin_to: string | null;
	place_from_name: string | null;
	place_to_name: string | null;
	condition_from: string | null;
	condition_to: string | null;
	discarded_from: boolean | null;
	discarded_to: boolean | null;
	note: string | null;
	/** ISO date and time of the booking. */
	at: string;
	/** Account that booked. */
	by: string;
}

export interface ArticleGroup {
	id: number;
	name: string | null;
	/** Parent group; `null` = top level. */
	parent_id: number | null;
	/** Number of articles assigned to this group. */
	article_count: number;
}

/** Input for creating/changing a product group. */
export interface ArticleGroupInput {
	name: string;
	parent_id: number | null;
}

/** A number series with its counter and a preview of the next number. */
export interface NumberSeries {
	key: string;
	name: string;
	prefix: string;
	format: string;
	digits: number;
	start_value: number;
	step: number;
	current_value: number;
	yearly_reset: boolean;
	year: number | null;
	active: boolean;
	legacy_source: string | null;
	changed_at: string;
	next_value: number | null;
	next_number: string | null;
}

export interface NumberSeriesInput {
	name: string;
	prefix: string;
	format: string;
	digits: number;
	start_value: number;
	step: number;
	yearly_reset: boolean;
	active: boolean;
	current_value: number;
}

export interface Language {
	id: number;
	language: string | null;
	native_language: boolean;
	iso_639_1: string | null;
}

// ---------------------------------------------------------------- Article search

/** "Artikeltyp" (article type) option list of the search form. */
export type ArticleType = 'active' | 'inactive' | 'all';

/** Combination of several values of one criterion. */
export type SetOperator = 'contains' | 'contains_any' | 'contains_all' | 'contains_none';

/** "Pyro" tab of the search form; numbers are ranges (from/to). */
export interface PyroSearchInput {
	un_number?: string;
	hazard_class?: string;
	ce_nr?: string;
	classification?: string;
	category?: string;
	approval_number?: string;
	packing_group?: string;
	tunnel_code?: string;
	transport_category?: string;
	neq_from?: number | null;
	neq_to?: number | null;
	un_points?: number | null;
	lateral_distance_from?: number | null;
	lateral_distance_to?: number | null;
	vertical_distance_from?: number | null;
	vertical_distance_to?: number | null;
	ascending_height_from?: number | null;
	ascending_height_to?: number | null;
	sound_pressure_level_from?: number | null;
	sound_pressure_level_to?: number | null;
	sound_pressure_range_from?: number | null;
	sound_pressure_range_to?: number | null;
	delay_time_from?: number | null;
	delay_time_to?: number | null;
	effect_duration_from?: number | null;
	effect_duration_to?: number | null;
}

/** Criteria of the search form (`img/Artikel_Suchmaske.PNG`). */
export interface ArticleSearchInput {
	article_type?: ArticleType;
	name?: string;
	include_descriptions?: boolean;
	article_no?: number | null;
	inventory_no?: string;
	serial_no?: string;
	with_bundle?: boolean;
	article_groups?: number[];
	article_groups_op?: SetOperator;
	search_index?: string[];
	search_index_op?: SetOperator;
	suppliers?: string[];
	manufacturer?: string;
	owner?: string;
	exclude_virtual?: boolean;
	pyro?: PyroSearchInput | null;
	limit?: number;
	offset?: number;
}

/** Saved search (row "Vorgang" of the form). */
export interface SavedSearch {
	id: number;
	name: string;
	criteria: ArticleSearchInput;
	created_at: string;
	changed_at: string;
}

/** Node of the search-index tree (dialog "Index wählen"). */
export interface SearchIndexNode {
	id: number;
	name: string | null;
	parent_id: number | null;
}

// ------------------------------------------------------------------- Calendar

/** One appointment as it appears in the form. */
export interface AgendaEvent {
	id: number;
	title: string;
	head_text: string | null;
	notes: string | null;
	start_date: string;
	end_date: string;
	start_time: string | null;
	end_time: string | null;
	all_day: boolean;
	category: number | null;
	category_name: string | null;
	venue: string | null;
	address_id: number | null;
	address_name: string | null;
	color_red: number | null;
	color_green: number | null;
	color_blue: number | null;
	custom_color: boolean;
	private_user: number | null;
	remind: boolean;
	remind_minutes: number | null;
	recurrence: AgendaRecurrence | null;
	changed_at: string | null;
	changed_by: string | null;
}

export type AgendaFrequency = 'daily' | 'weekly' | 'monthly' | 'yearly';

export interface AgendaRecurrence {
	frequency: AgendaFrequency;
	interval: number;
	until: string | null;
	count: number | null;
}

/** What the form sends; missing fields get their default in the backend. */
export interface AgendaEventInput {
	title: string;
	head_text?: string | null;
	notes?: string | null;
	start_date: string;
	end_date?: string | null;
	start_time?: string | null;
	end_time?: string | null;
	all_day?: boolean;
	category?: number | null;
	venue?: string | null;
	address_id?: number | null;
	color_red?: number | null;
	color_green?: number | null;
	color_blue?: number | null;
	custom_color?: boolean;
	private_user?: number | null;
	remind?: boolean;
	remind_minutes?: number | null;
	recurrence?: AgendaRecurrence | null;
}

/** One day of an appointment — a series yields several. */
export interface AgendaOccurrence {
	event_id: number;
	date: string;
	title: string;
	head_text: string | null;
	start_date: string;
	end_date: string;
	start_time: string | null;
	end_time: string | null;
	all_day: boolean;
	category: number | null;
	category_name: string | null;
	venue: string | null;
	address_id: number | null;
	address_name: string | null;
	color_red: number | null;
	color_green: number | null;
	color_blue: number | null;
	custom_color: boolean;
	private_user: number | null;
	recurring: boolean;
}

export interface AgendaCategory {
	id: number;
	name: string;
	appointments: number;
}

export interface AgendaRange {
	from: string;
	to: string;
	occurrences: AgendaOccurrence[];
	birthdays: AgendaBirthday[];
	categories: AgendaCategory[];
}

/** Birthday from the address master; the calendar shows it as an all-day row. */
export interface AgendaBirthday {
	address_id: number;
	name: string;
	date: string;
	birth_year: number;
}

// ------------------------------------------------------------- Settings

export type SettingsKind = 'text' | 'integer' | 'number' | 'bool' | 'date';

/** One column of a settings list. */
export interface SettingsField {
	column: string;
	label: string;
	kind: SettingsKind;
}

/** One list of the admin menu. */
export interface SettingsArea {
	key: string;
	title: string;
	group: string;
	table: string;
	/** Column that names an entry (usually the first one). */
	name_column: string;
	description: string;
	fields: SettingsField[];
}

/** Values of one row, all as text — the column kind says how they behave. */
export type SettingsValues = Record<string, string | null>;

export interface SettingsRow {
	id: number;
	values: SettingsValues;
}

export interface SettingsList {
	area: SettingsArea;
	rows: SettingsRow[];
}

/** Options of the forms, key → value. */
export interface Prefs {
	values: Record<string, string>;
}

// ------------------------------------------------------- Order settings

/** One row of the schedule (`img/Admin Menü/Aufträge/Auftragseinstellungen_Zeiteplan.PNG`). */
export interface TimelineEntry {
	key: string;
	label: string;
	position: number;
	shown: boolean;
}

/** One of the nine headings of the order form. */
export interface HeadingEntry {
	number: number;
	shown: boolean;
	label: string | null;
}

/** One macro of the order form. */
export interface OrderMacro {
	id: number;
	label: string;
	for_sale: boolean;
	active: boolean;
	created_by: string | null;
	body: string | null;
	changed_at: string | null;
}

export interface MacroInput {
	label: string;
	for_sale: boolean;
	active: boolean;
	body: string | null;
}

// -------------------------------------------------------------- Bank list

/** One row of the bank list (`img/Admin Menü/Bank - Swift BIC Liste`). */
export interface Bank {
	id: number;
	bic: string;
	name: string;
	country: string | null;
	blz: string | null;
	city: string | null;
	alt_name: string | null;
}

export interface BankInput {
	bic: string;
	name: string;
	country: string | null;
	blz: string | null;
	city: string | null;
	alt_name: string | null;
}

export interface BankListResponse {
	rows: Bank[];
	total: number;
}

/** Result of an update from an online source. */
export interface BankSyncReport {
	source: string;
	read: number;
	inserted: number;
	updated: number;
	unchanged: number;
	skipped: number;
}

/** The built-in source of the bank list. */
export interface BankSource {
	url: string;
	note: string;
}

// --------------------------------------------------------- Bank accounts

/** One account of the company (`img/Admin Menü/Bankverbindungen`). */
export interface BankAccount {
	id: number;
	label: string;
	holder: string | null;
	iban: string | null;
	bic: string | null;
	institute: string | null;
	account_number: string | null;
	blz: string | null;
	bank_address: string | null;
	standard: boolean;
	fibu_account: string | null;
	sepa_creditor_id: string | null;
	sepa_direct_debit: boolean;
	sepa_lead_first: number | null;
	sepa_lead_follow: number | null;
	sepa_submit_time: string | null;
	ch_street: string | null;
	ch_house_number: string | null;
	ch_postal_code: string | null;
	ch_city: string | null;
	ch_country: string | null;
	ch_holder_address: string | null;
	/** `EB` of the list: account folder for electronic banking on file. */
	electronic_banking: boolean;
}

export type BankAccountInput = Omit<BankAccount, 'id' | 'electronic_banking'>;

/** Result of "Kto & BLZ in IBAN & BIC umwandeln" (convert account & bank code into IBAN & BIC). */
export interface ConversionResult {
	iban: string;
	bic: string | null;
	institute: string | null;
	bank_address: string | null;
}

// ------------------------------------------------------------------ Tenants

/** One tenant of the installation (`img/Admin Menü/Mandanten`). */
export interface Tenant {
	id: number;
	/** `Abk.` (abbreviation) of the list. */
	short_name: string | null;
	/** `Firmename` (company name). */
	name: string;
	tenant_id: number;
	/** `Stammdaten-Sync` of the list; `0` means "nicht eingerichtet" (not configured). */
	sync_id: number;
	parent: number | null;
	web_identifier: string | null;
	currency: string | null;
	currency_id: number | null;
	print_language: string | null;
	print_language_id: number | null;
	hide_actions: boolean;
	color: number;
	color_on: boolean;
	address_extra: string | null;
	street: string | null;
	postal_code: string | null;
	city: string | null;
	country_iso3: string | null;
	country: string | null;
	state: string | null;
	phone: string | null;
	fax: string | null;
	email: string | null;
	email_accounting: string | null;
	website: string | null;
	vat_id: string | null;
	tax_number: string | null;
	register_entry: string | null;
	register_place: string | null;
	linked_address: number | null;
	linked_address_name: string | null;
}

export type TenantInput = Omit<
	Tenant,
	'id' | 'linked_address_name' | 'tenant_id' | 'sync_id' | 'color'
> & {
	tenant_id: number | null;
	sync_id: number | null;
	color: number | null;
};

/** Countries and print languages of the tenant form. */
export interface TenantReference {
	countries: { iso3: string; name: string }[];
	print_languages: { label: string; id: number }[];
}

// ------------------------------------------------------------ User management

/** One account of the option list on the login form. */
export interface LoginUser {
	id: number;
	user_name: string;
	display_name: string | null;
	short_name: string | null;
	/** `#rrggbb` built from `TA_ColorRed/Green/Blue`. */
	color: string | null;
	has_password: boolean;
	/** Tenants (`"User_Mandanten"`) the account is enabled for. */
	mandants: number[];
}

/** One tenant of the selection left of `Benutzer`. */
export interface LoginTenant {
	id: number;
	short_name: string | null;
	name: string;
}

/** The signed-in account as the header shows it. */
export interface SessionUser {
	id: number;
	user_name: string;
	display_name: string | null;
	short_name: string | null;
	color: string | null;
	/** The rights from the permission groups, e.g. `artikel.artikel-anlegen`. */
	rights: string[];
	/** While `true`, the application first leads into the password change. */
	change_next_login: boolean;
	/** The tenant chosen at login. */
	mandant: number | null;
	/** Its name, so the header shows the installation. */
	mandant_name: string | null;
	/** The tenants the account is assigned to (`"User_Mandanten"`). */
	mandants: number[];
}

/** Result of a login: token and account. */
export interface LoginResponse {
	token: string;
	user: SessionUser;
}

/** The four lists behind `Aktuelle Ansicht` of the `Benutzerverwaltung` window. */
export type UserView = 'local-active' | 'mobile-active' | 'local-inactive' | 'mobile-inactive';

/** One permission group as it appears in the right-hand list. */
export interface GroupRef {
	id: number;
	name: string;
}

/** One row of the user list. */
export interface UserSummary {
	id: number;
	/** `User_anzeigeName` of the list: `Administrator [Adm]`. */
	name: string;
	short_name: string | null;
	groups: GroupRef[];
}

/** One user with all columns of the four tabs of its dialog. */
export interface User {
	id: number;
	user_name: string;
	display_name: string | null;
	short_name: string | null;
	local_login: boolean;
	mobile_login: boolean;
	has_password: boolean;
	change_next_login: boolean;
	cannot_change: boolean;
	never_expires: boolean;
	disabled: boolean;
	language: string | null;
	two_factor: boolean;
	/** `#rrggbb` built from `TA_ColorRed/Green/Blue`. */
	color: string | null;
	sso_ad_active: boolean;
	sso_ad_user: string | null;
	sso_ad_domain: string | null;
	sso_mb_active: boolean;
	sso_mb_os_user: string | null;
	sso_mb_machine: string | null;
	sso_default_entries: boolean;
	sso_default_mandant: number | null;
	sso_default_lager: number | null;
	function: string | null;
	phone: string | null;
	mobile: string | null;
	fax: string | null;
	email: string | null;
	/** `0` = SMTP Mailserver, `1` = OAuth 2.0. */
	email_provider: number;
	smtp_server: string | null;
	smtp_port: number | null;
	email_signature: string | null;
	bcc: string | null;
	im1_type: string | null;
	im1: string | null;
	im2_type: string | null;
	im2: string | null;
	profile_mandant: number | null;
	extension: string | null;
	address_linked: boolean;
	personal_linked: boolean;
	mandants: number[];
	lager: number[];
	groups: GroupRef[];
}

export type UserInput = Omit<User, 'id' | 'has_password' | 'two_factor' | 'address_linked' | 'personal_linked' | 'groups'> & {
	/** Empty keeps the stored password unchanged. */
	password: string | null;
};

/** One user of a group — the child row below the group. */
export interface GroupMember {
	id: number;
	name: string;
}

/** One permission group with both of its right sets. */
export interface UserGroup {
	id: number;
	name: string;
	max_rights: string[];
	min_rights: string[];
	mandants: number[];
	lager: number[];
	/** Departments the group may work in — `Personal`; empty = all of them. */
	departments: number[];
	/** The same for `Personal-Disposition`; empty = all of them. */
	dispo_departments: number[];
	members: number;
	/** The users of the group (`Rechtevergabe.PNG`). */
	member_users: GroupMember[];
}

export type UserGroupInput = Omit<UserGroup, 'id' | 'members' | 'member_users'>;

/** One right of the rights tree. */
export interface PermissionRight {
	key: string;
	label: string;
}

/** One leaf of the rights tree with its rights. */
export interface PermissionArea {
	key: string;
	label: string;
	rights: PermissionRight[];
}

/** One branch of the rights tree. */
export interface PermissionGroup {
	key: string;
	label: string;
	areas: PermissionArea[];
}

// ---------------------------------------------------------------- File storage

export interface ArticleFile {
	id: number;
	article_id: number;
	folder: string;
	name: string;
	is_link: boolean;
	link_target: string | null;
	mime: string | null;
	size: number | null;
	sha256: string | null;
	version: number;
	is_current: boolean;
	uploaded_by: string | null;
	uploaded_at: string | null;
}

// ------------------------------------------------- Test reports / article image

/** One test report: a PDF bound to a device (serial and inventory no.). */
export interface InspectionReport {
	id: number;
	device_id: number;
	article_id: number;
	serial_no: string | null;
	inventory_no: string | null;
	title: string;
	mime: string | null;
	size: number | null;
	sha256: string;
	uploaded_by: string | null;
	uploaded_at: string | null;
}

/** The uploaded article image; `null` as long as none is stored. */
export interface ArticleImageFile {
	id: number;
	name: string;
	mime: string | null;
	size: number | null;
	uploaded_at: string | null;
}

export interface ArticleImage {
	file: ArticleImageFile | null;
	placeholder: boolean;
}

// ------------------------------------------------------------------- Dashboard

/**
 * Metrics and lists of the start page. Everything is counted in the database
 * so the numbers match across several workstations; decimal values
 * (stocks) arrive as strings like all `Decimal` fields.
 */
export interface Dashboard {
	articles: {
		total: number;
		active: number;
		inactive: number;
		virtual_articles: number;
		created_last_30_days: number;
		changed_last_30_days: number;
	};
	addresses: {
		total: number;
		active: number;
		customers: number;
		suppliers: number;
		interested: number;
		locations: number;
		vip: number;
		employees: number;
	};
	stock: {
		devices: number;
		discarded_devices: number;
		rental: string;
		sale: string;
		reserved: string;
		withdrawn: string;
	};
	pyro: {
		articles: number;
		articles_with_hazard_class: number;
		hazard_classes: { label: string; count: number }[];
	};
	quality: {
		articles_without_group: number;
		articles_without_rental_price: number;
		articles_without_stock: number;
		addresses_without_email: number;
		addresses_without_number: number;
	};
	groups: { id: number; name: string | null; articles: number }[];
	warehouses: { id: number; name: string; rental: string; sale: string }[];
	recent_articles: {
		id: number;
		article_no: number;
		name: string | null;
		group: string | null;
		changed_at: string | null;
	}[];
	recent_addresses: {
		id: number;
		customer_no: number | null;
		name: string | null;
		city: string | null;
		group: string | null;
		active: boolean;
	}[];
}

/** One row the import did not take over. */
export interface ImportFailure {
	/** Position in the request, starting at 1 — the window maps it to the file line. */
	row: number;
	message: string;
}

/** Result of one import run. */
export interface ImportReport {
	created: number;
	failed: ImportFailure[];
}

/** One row of the article import: the record and the product group named by the file. */
export interface ArticleImportRow {
	article: Article;
	/** Name of the product group — the API creates a missing group. */
	warengruppe?: string | null;
}

/** One warehouse of the tenant (`img/Admin Menü/Lager und Lagerplätze`, "Lager" tab). */
export interface WarehouseAdmin {
	id: number;
	mandant_id: number | null;
	name: string | null;
	/** `__DefaultLager` — the preselection of the forms. */
	is_default: boolean;
	zugriff: boolean;
	prioritaet: boolean;
	such_std: boolean;
	entnahme: boolean;
	anmelden: boolean;
	fremd: boolean;
	/** Group-internal warehouse or without disposition. */
	konzern: boolean;
}

/** Storage location (Symetric) — name, short code, value range. */
export interface WarehouseLocation {
	id: number;
	warehouse_id: number;
	name: string | null;
	prefix: string | null;
	values: string | null;
	position: number | null;
}

/** Storage location (Self defined) — freely named, with path. */
export interface WarehouseBin {
	id: number;
	warehouse_id: number;
	label: string | null;
	path: string | null;
	parent_id: number | null;
}

// ------------------------------------------------------------------ Module
// `WHERE - is my stuff?` is the core application; the other three products are
// modules that share the same database and the same tenants.

export type ModuleId = 'stuff' | 'go' | 'people' | 'money';

/** One row of the module registry (`/api/v1/modules`). */
export interface ModuleStatus {
	id: ModuleId;
	product_name: string;
	description: string;
	kind: 'core' | 'module';
	is_core: boolean;
	installed: boolean;
	/** Capabilities the module contributes (e.g. `article-pricing`). */
	capabilities: string[];
}

export interface ModuleSnapshot {
	installation_id: string | null;
	/** Product that created the database. */
	database_owner: string | null;
	core_installed: boolean;
	modules: ModuleStatus[];
	unknown_modules: string[];
}

/** What `/api/v1/installation` answers: the product this installation runs. */
export interface InstallationInfo {
	product_name: string;
}

/** The master data that can leave and come back as a file. */
export type MasterArea = 'lists' | 'employees' | 'shifts';

/** What a master-data import did — or, as a scan, would do. */
export interface MasterReport {
	/** Rows the file carried, without its head. */
	rows: number;
	created: number;
	changed: number;
	unchanged: number;
	/** Everything that was skipped, with the line of the file. */
	warnings: string[];
}

/** One employee from `WHERE - are my people?`. */
export interface Employee {
	id: number | null;
	personnel_no: string | null;
	salutation: string | null;
	title: string | null;
	first_name: string;
	last_name: string;
	birth_date: string | null;
	birth_place: string | null;
	nationality: string | null;
	marital_status: string | null;
	street: string | null;
	postal_code: string | null;
	city: string | null;
	country: string | null;
	email: string | null;
	phone: string | null;
	mobile: string | null;
	entry_date: string | null;
	exit_date: string | null;
	employment_type: string | null;
	weekly_hours: string | null;
	/** Working days per week: 1 to 7 (null = not set). */
	work_days: number | null;
	/** Vacation / remaining vacation days (null = not set). */
	vacation_days: number | null;
	position: string | null;
	department: string | null;
	/** Sub-department under the department, for example `Podium` under `Beleuchtung`. */
	subdepartment: string | null;
	/** The house of the disposition the employee belongs to. */
	location: string | null;
	cost_center: string | null;
	manager: string | null;
	social_security_no: string | null;
	tax_id: string | null;
	notes: string | null;
	active: boolean;
	/** The employee is an apprentice. */
	is_apprentice: boolean;
	/** The plan generator may place this employee; `false` = planned by hand only. */
	auto_plan: boolean;
	/** Place on the schedule board once its rows were arranged by hand. */
	board_order?: number | null;
	/** Account of the user administration behind this employee, if one names it. */
	user_id?: number | null;
	qualifications: string[];
	created_at: string | null;
	changed_at: string | null;
}

export type EmployeeInput = Omit<
	Employee,
	'id' | 'created_at' | 'changed_at' | 'board_order' | 'user_id'
>;

/** One public holiday of the tenant. */
export interface Holiday {
	/** `YYYY-MM-DD`. */
	day: string;
	name: string;
	/** `national` or `regional`. */
	scope: 'national' | 'regional';
	/** Federal state of a regional holiday, `null` when nationwide. */
	state: string | null;
}

/** One stretch of school holidays. */
export interface SchoolHoliday {
	/** `YYYY-MM-DD`. */
	starts_on: string;
	/** `YYYY-MM-DD`. */
	ends_on: string;
	name: string;
	state: string;
}

/** Holidays and school holidays of a period. */
export interface HolidayCalendar {
	holidays: Holiday[];
	school_holidays: SchoolHoliday[];
}

/** How many holidays and school holidays are stored. */
export interface HolidayCounts {
	holidays: number;
	school_holidays: number;
}

/** What one holiday import did. */
export interface HolidayImportReport {
	year: number;
	state: string | null;
	national_holidays: number;
	regional_holidays: number;
	school_holidays: number;
	/** Sources that answered with something unusable; the rest was stored. */
	errors: string[];
}

/** One department or position of the personnel form. */
export interface PeopleListEntry {
	id: number;
	name: string;
	/** Department that works in that location; only a location carries one. */
	department?: string | null;
	/** Only a location carries this: `false` = the house is run without a foreman. */
	needs_foreman?: boolean;
	/** Only a location carries this: the row was typed on the board, so its name is
	 *  free text and it stays out of the master data and the employee mask. */
	is_text?: boolean;
	/** Only a department carries this: the tint of its employees in the board. */
	color?: string | null;
}

/** The master-data lists of the personnel form. */
export type PeopleListKind = 'departments' | 'subdepartments' | 'positions' | 'locations';

/** One shift of schedulening. */
/** Half of the day a shift belongs to; `frei` is a whole free day. */
export type ShiftBand = 'early' | 'late' | 'intermediate' | 'free';

export interface Shift {
	id: number;
	name: string;
	short_name: string | null;
	/** Start `HH:MM`. */
	starts_at: string;
	/** End `HH:MM`; if it lies before the start, the shift runs past midnight. */
	ends_at: string;
	/** Break in minutes. */
	break_minutes: number;
	/** Half of the day the shift belongs to: `frueh`, `spaet`, `zwischen` or `frei`. */
	band: ShiftBand;
	/** Only apprentices may be planned into this shift. */
	apprentice_only: boolean;
	/** Color as `#RRGGBB`. */
	color: string | null;
	/** Department the shift belongs to; `null` = every dispatcher may give it. */
	department: string | null;
	active: boolean;
	sort_order: number;
	/** Positions of the personnel master data that do not occur in this shift. */
	excluded_positions: string[];
}

/** What the form sends for a shift. */
export interface ShiftInput {
	name: string;
	short_name: string | null;
	starts_at: string;
	ends_at: string;
	break_minutes: number;
	band: ShiftBand;
	apprentice_only: boolean;
	color: string | null;
	department: string | null;
	active: boolean;
}

/** One schedule-plan entry: shift, free time, sick leave or vacation. */
export interface ScheduleEntry {
	id: number;
	employee_id: number;
	/** `shift`, `sick`, `vacation` or `free`. */
	kind: ScheduleKind;
	/** Only set when `kind = 'shift'`. */
	shift_id: number | null;
	/** Only set when `kind = 'free'`; `HH:MM`. */
	starts_at: string | null;
	/** Only set when `kind = 'free'`; `HH:MM`. */
	ends_at: string | null;
	/** Only set when `kind = 'free'`. */
	break_minutes: number | null;
	/** Free text of the cell — shown under the shift on the board and on the sheet. */
	note: string | null;
	/** `YYYY-MM-DD`. */
	day: string;
}

/** What one cell of the schedule holds. */
export type ScheduleKind = 'shift' | 'sick' | 'vacation' | 'free';

/** What a written series did and where the planning rules stopped it. */
export interface ScheduleSeriesResult {
	/** The days that were written, in the order of their days. */
	entries: ScheduleEntry[];
	/** Days the board asked for. */
	requested: number;
	/** First day that stayed free; `null` when every day was written. */
	blocked: ScheduleBlocked | null;
}

/** The day a series stopped at and the rule that stopped it. */
export interface ScheduleBlocked {
	/** `YYYY-MM-DD`. */
	day: string;
	/** `sunday`, `absence`, `max_consecutive_days`, `min_rest_hours`,
	 *  `max_daily_hours`, `max_weekly_hours` or `min_days_off_per_week`. */
	reason: string;
}

/** One event of a location in a PeM disposition export. */
export interface DispoEvent {
	/** `YYYY-MM-DD`. */
	day: string;
	/** Start as `HH:MM`; `null` when the cell carried no time. */
	starts_at: string | null;
	/** The text without its time, for example `Max und Moritz Vst`. */
	title: string;
}

/** A location block of a PeM export: the merge of its name cell spans its rows. */
export interface DispoLocation {
	name: string;
	first_row: number;
	last_row: number;
	events: DispoEvent[];
}

/** What a PeM export contains, read without writing anything. */
export interface DispoScan {
	locations: DispoLocation[];
	/** Days found in the date header. */
	days: number;
	/** Skipped rows and shortened texts. */
	warnings: string[];
	/** The same disposition was already imported for this month. */
	already_imported: boolean;
	/** Entries the month already holds, written by an earlier import. */
	existing_entries: number;
}

/** What the import of a PeM export wrote. */
export interface DispoImportReport {
	locations: number;
	/** Locations that did not exist yet and were added to the list. */
	locations_created: number;
	/** Flags that did not exist yet and were created from the legend. */
	flags_created: number;
	days: number;
	entries_created: number;
	warnings: string[];
}

/** What a reset of a month removed. */
export interface ResetReport {
	cleared: number;
}

/** What an exchange import did: the rows of the file and everything it skipped. */
export interface ExchangeReport {
	rows: number;
	written: number;
	warnings: string[];
}

/** One name of an exchange preview with the rows behind it. */
export interface ExchangeGroup {
	name: string;
	rows: number;
}

/** What a preview of an exchange file shows before anything is written. */
export interface ExchangeScan {
	/** Rows of the file that can be written. */
	rows: number;
	/** Distinct days those rows cover. */
	days: number;
	/** The locations of a disposition, the people of a plan. */
	groups: ExchangeGroup[];
	warnings: string[];
	/** Entries the month already holds. */
	existing_entries: number;
	/** `true` when this very file was already written into the month. */
	already_imported: boolean;
}

/** One entry of the disposition: an event of a location on a day. */
export interface DispoEntry {
	id: number;
	location_id: number;
	/** `YYYY-MM-DD`. */
	day: string;
	/** `HH:MM`; `null` when the event has no time. */
	starts_at: string | null;
	title: string;
	/** Code of the flag (`Vst`), `null` when the text carries none. */
	flag: string | null;
	sort_order: number;
	/** `true` when the planner set the event aside: it asks for nobody. */
	ignored: boolean;
}

/** One entry as the window sends it back; without `id` it is new. */
export interface DispoEntryInput {
	id?: number | null;
	starts_at: string;
	title: string;
	/** Code of the flag; `null` or `''` clears it. */
	flag?: string | null;
	/** Keeps the event but stops it from asking for anybody. */
	ignored?: boolean;
}

/** One position a flag needs in one house. */
export interface FlagStaffing {
	/** The location the staffing belongs to. */
	location_id: number;
	/** A position of the master data, or `*` for "any position". */
	position: string;
	needed: number;
}

/** A flag of the disposition with its default staffing. */
export interface PeopleFlag {
	id: number;
	/** The short code as the export writes it, for example `Vst`. */
	code: string;
	/** What the code means, for example `Vorstellung`. */
	name: string;
	sort_order: number;
	staffing: FlagStaffing[];
	/** Positions this flag never uses. */
	excluded_positions: string[];
}

/** How many employees of a position a shift needs on a day. */
export interface ScheduleDemandEntry {
	shift_id: number;
	position: string;
	/** Location of the disposition; empty when the row is not bound to one. */
	location?: string;
	needed: number;
}

/** The staffing demand of one day. */
export interface ScheduleDemandDay {
	/** `YYYY-MM-DD`. */
	day: string;
	/** Only pairs of shift and position that need somebody. */
	entries: ScheduleDemandEntry[];
}

/** An event of the disposition that adds nothing to the demand. */
export interface DispoUnclearEvent {
	entry_id: number;
	location_id: number;
	/** Start as `HH:MM` when the event has one. */
	starts_at: string | null;
	title: string;
	/** The flag the text names, when it names one. */
	flag: string | null;
	/** `no_time`, `no_flag`, `unknown_flag` or `no_shift`. */
	reason: string;
	/** Set aside by the planner: listed, but the day is no longer marked. */
	ignored: boolean;
}

/** What the disposition of one day asks for. */
export interface DispoDayProposal {
	/** `YYYY-MM-DD`. */
	day: string;
	/** Demand per shift and position, derived from the events of the day. */
	entries: ScheduleDemandEntry[];
	/** Events that could not be read into a shift. */
	unclear: DispoUnclearEvent[];
	/** `true` when the day carries an unclear event — the board shows a `?`. */
	marked: boolean;
}

/** A demand row the generator could not fill. */
export interface ScheduleGap {
	/** `YYYY-MM-DD`. */
	day: string;
	shift_id: number;
	position: string;
	/** How many employees are missing for that row. */
	missing: number;
	/** `no_employee`, `subdepartment` or `holiday`. */
	reason: string;
}

/** What the generator did. */
/** One cell of the plan that breaks a rule of the planning. */
export interface ScheduleViolation {
	employee_id: number;
	day: string;
	/** `min_rest_hours`, `max_consecutive_days`, `weekend`, `apprentice_only`, … */
	reason: string;
}

/** The result of checking a written plan. */
export interface ScheduleAudit {
	/** Worked cells that were checked. */
	checked: number;
	/** The cells that break a rule, in the order of the days. */
	violations: ScheduleViolation[];
}

export interface ScheduleGenerateReport {
	days: number;
	/** Cells written. */
	assigned: number;
	/** Cells that stayed as they were. */
	kept: number;
	/** Shift cells removed because of `overwrite`. */
	replaced: number;
	/** Cells the hours balance added beyond the demand. */
	extra: number;
	/** Free days written (`FT`, and the `SoFT` after a worked Sunday). */
	free_days: number;
	/** Rows that stayed open. */
	gaps: ScheduleGap[];
	/** Days whose disposition could not be read into shifts (`?` on the board). */
	unclear_days: string[];
}

/** Unit of the planning horizon: weeks or months. */
export type HorizonUnit = 'weeks' | 'months';

/** The rules that schedulening follows. */
export interface PlanningRules {
	min_rest_hours: number;
	max_daily_hours: number;
	max_weekly_hours: number;
	break_after_hours: number;
	break_minutes: number;
	long_break_after_hours: number;
	long_break_minutes: number;
	max_consecutive_days: number;
	/**
	 * Working days in a row before a day off may follow (1 = no minimum). The
	 * generator keeps a block that has not reached it going first.
	 */
	min_consecutive_days: number;
	/**
	 * Days an employee keeps the same shift before the planning may move them
	 * from early to late or the other way round (default 14).
	 */
	shift_rotation_days: number;
	min_days_off_per_week: number;
	sunday_work_allowed: boolean;
	/** Is a worked Sunday compensated on another weekday? */
	compensate_sunday: boolean;
	/**
	 * After a Sunday shift the rest period is one full day plus the regular rest
	 * period (24 + 11 hours). Germany only.
	 */
	extended_sunday_rest: boolean;
	/**
	 * With a six-day week every sixth week has to leave a whole weekend
	 * (Saturday and Sunday together) free. Germany only.
	 */
	free_weekend_every_six_weeks: boolean;
	/** An apprentice may never be alone in a shift. Germany only. */
	apprentice_needs_company: boolean;
	/** Import the nationwide public holidays of Germany. */
	import_public_holidays: boolean;
	/** Import the regional public holidays of `holiday_state`. */
	import_regional_holidays: boolean;
	/** Import the school holidays of `holiday_state`. */
	import_school_holidays: boolean;
	/** Federal state (`BW`, `BY`, …) for the regional and school holidays. */
	holiday_state: string | null;
	/** Color of the holiday columns in the schedule (`#RRGGBB`). */
	holiday_color: string;
	/** Show the public holidays on the board. */
	show_holidays: boolean;
	/** Show the school holidays on the board. */
	show_school_holidays: boolean;
	/** Color of the school holidays in the schedule (`#RRGGBB`). */
	school_holiday_color: string;
	holiday_work_allowed: boolean;
	respect_absence: boolean;
	/** Unit of the planning horizon. */
	planning_horizon: number;
	/** `weeks` or `months`. */
	planning_horizon_unit: HorizonUnit;
	/** Hours one vacation day counts as. */
	vacation_day_hours: string;
	/** Hours one sick day counts as. */
	sick_day_hours: string;
	/** Color of the vacation cells in the schedule (`#RRGGBB`). */
	vacation_color: string;
	/** Color of the sick cells in the schedule (`#RRGGBB`). */
	sick_color: string;
	/** From this date the plan counts over- and under-hours (`null` = 1 January). */
	balance_from: string | null;
}

export interface ModuleInstallOutcome {
	module: ModuleId;
	newly_installed: boolean;
	shares_core_database: boolean;
	owns_database: boolean;
	reused_existing_database: boolean;
	installation_id: string | null;
	database_owner: string | null;
}

export const api = {
	health: () => request<{ status: string; version: string }>('/health'),

	/**
	 * Which product this installation runs.
	 *
	 * The login mask needs the name before a session exists, so the API answers it
	 * without signing in.
	 */
	installation: () => request<InstallationInfo>('/api/v1/installation'),

	/** Module registry: which products share this installation. */
	modules: {
		list: () => request<ModuleSnapshot>('/api/v1/modules'),
		install: (id: ModuleId) =>
			request<ModuleInstallOutcome>(`/api/v1/modules/${id}`, { method: 'POST' }),
		remove: (id: ModuleId) =>
			request<ModuleSnapshot>(`/api/v1/modules/${id}`, { method: 'DELETE' })
	},

	/** Personnel master of `WHERE - are my people?`. */
	people: {
		list: (q?: string) => request<Employee[]>(`/api/v1/people${query({ q })}`),
		get: (id: number) => request<Employee>(`/api/v1/people/${id}`),
		/** Next employee number from the number series (the form's suggestion). */
		nextNumber: () => request<{ personnel_no: string }>('/api/v1/people/new'),
		create: (body: EmployeeInput) =>
			request<Employee>('/api/v1/people', { method: 'POST', body: JSON.stringify(body) }),
		update: (id: number, body: EmployeeInput) =>
			request<Employee>(`/api/v1/people/${id}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/people/${id}`, { method: 'DELETE' }),

		/** The order of the rows on the schedule board; ids from top to bottom. */
		boardOrder: (ids: number[]) =>
			request<{ written: number }>('/api/v1/people/board-order', {
				method: 'PUT',
				body: JSON.stringify({ ids })
			}),

		/** Names the account of the user administration behind an employee; `null`
		 *  takes the reference away. */
		setUser: (id: number, userId: number | null) =>
			request<Employee>(`/api/v1/people/${id}/user`, {
				method: 'PUT',
				body: JSON.stringify({ user_id: userId })
			}),

		/** Departments and positions — the suggestion lists of the form. */
		lists: {
			list: (kind: PeopleListKind) =>
				request<PeopleListEntry[]>(`/api/v1/people/lists/${kind}`),
			create: (kind: PeopleListKind, name: string) =>
				request<PeopleListEntry>(`/api/v1/people/lists/${kind}`, {
					method: 'POST',
					body: JSON.stringify({ name })
				}),
			rename: (kind: PeopleListKind, id: number, name: string) =>
				request<PeopleListEntry>(`/api/v1/people/lists/${kind}/${id}`, {
					method: 'PUT',
					body: JSON.stringify({ name })
				}),
			remove: (kind: PeopleListKind, id: number) =>
				request<{ deleted: boolean }>(`/api/v1/people/lists/${kind}/${id}`, {
					method: 'DELETE'
				}),
			/** The sub-department that works in a location, or the department a
			 * sub-department sits under; an empty string clears it. */
			setParent: (kind: PeopleListKind, id: number, department: string) =>
				request<PeopleListEntry>(`/api/v1/people/lists/${kind}/${id}/department`, {
					method: 'PUT',
					body: JSON.stringify({ department })
				}),
			/** Whether a house is staffed with a foreman (`Meister`). */
			setLocationForeman: (id: number, needsForeman: boolean) =>
				request<PeopleListEntry>(`/api/v1/people/lists/locations/${id}/foreman`, {
					method: 'PUT',
					body: JSON.stringify({ needs_foreman: needsForeman })
				}),
			/** The tint of a department or a location; an empty string removes it. */
			setColor: (kind: PeopleListKind, id: number, color: string) =>
				request<PeopleListEntry>(`/api/v1/people/lists/${kind}/${id}/color`, {
					method: 'PUT',
					body: JSON.stringify({ color })
				}),
			/** New order, ids from top to bottom. */
			reorder: (kind: PeopleListKind, ids: number[]) =>
				request<PeopleListEntry[]>(`/api/v1/people/lists/${kind}/order`, {
					method: 'PUT',
					body: JSON.stringify({ ids })
				})
		},

		/** Flags of the disposition and the staffing each of them needs. */
		flags: {
			list: () => request<PeopleFlag[]>('/api/v1/people/flags'),
			create: (code: string, name: string) =>
				request<PeopleFlag>('/api/v1/people/flags', {
					method: 'POST',
					body: JSON.stringify({ code, name })
				}),
			rename: (id: number, code: string, name: string) =>
				request<PeopleFlag>(`/api/v1/people/flags/${id}`, {
					method: 'PUT',
					body: JSON.stringify({ code, name })
				}),
			remove: (id: number) =>
				request<{ deleted: boolean }>(`/api/v1/people/flags/${id}`, { method: 'DELETE' }),
			/** Replaces the staffing of a flag in one location. */
			setStaffing: (id: number, location: number, entries: FlagStaffing[]) =>
				request<FlagStaffing[]>(`/api/v1/people/flags/${id}/staffing/${location}`, {
					method: 'PUT',
					body: JSON.stringify({
						entries: entries.map((entry) => ({
							position: entry.position,
							needed: entry.needed
						}))
					})
				}),
			/** Replaces the positions a flag does not use. */
			exclusions: (id: number, positions: string[]) =>
				request<string[]>(`/api/v1/people/flags/${id}/exclusions`, {
					method: 'PUT',
					body: JSON.stringify({ positions })
				}),
			/** Creates the flags of the PeM legend that are missing. */
			presets: () =>
				request<{ created: number; kept: number }>('/api/v1/people/flags/presets', {
					method: 'POST'
				})
		},

		/** Shifts of schedulening. */
		shifts: {
			list: () => request<Shift[]>('/api/v1/people/shifts'),
			create: (body: ShiftInput) =>
				request<Shift>('/api/v1/people/shifts', {
					method: 'POST',
					body: JSON.stringify(body)
				}),
			update: (id: number, body: ShiftInput) =>
				request<Shift>(`/api/v1/people/shifts/${id}`, {
					method: 'PUT',
					body: JSON.stringify(body)
				}),
			remove: (id: number) =>
				request<{ deleted: boolean }>(`/api/v1/people/shifts/${id}`, { method: 'DELETE' }),
			/** New order, ids from top to bottom. */
			reorder: (ids: number[]) =>
				request<Shift[]>('/api/v1/people/shifts/order', {
					method: 'PUT',
					body: JSON.stringify({ ids })
				}),
			/** Replaces the positions the shift does not have. */
			exclusions: (id: number, positions: string[]) =>
				request<Shift>(`/api/v1/people/shifts/${id}/exclusions`, {
					method: 'PUT',
					body: JSON.stringify({ positions })
				})
		},

		/** Master data as exchange files: the lists, the employees and the shifts. */
		masterdata: {
			/** Reads the file and reports what it would do; nothing is written. */
			scan: (area: MasterArea, file: ArrayBuffer, replace: boolean) =>
				sendBytes<MasterReport>(
					`/api/v1/people/masterdata/${area}/scan${query({ replace: replace ? 'true' : 'false' })}`,
					file,
					'application/octet-stream'
				),
			/** Writes the file into the master data. */
			import: (area: MasterArea, file: ArrayBuffer, replace: boolean) =>
				sendBytes<MasterReport>(
					`/api/v1/people/masterdata/${area}${query({ replace: replace ? 'true' : 'false' })}`,
					file,
					'application/octet-stream'
				)
		},

		/** Rules of schedulening (one row). */
		planning: {
			get: () => request<PlanningRules>('/api/v1/people/planning'),
			save: (body: PlanningRules) =>
				request<PlanningRules>('/api/v1/people/planning', {
					method: 'PUT',
					body: JSON.stringify(body)
				})
		},

		/** Public and school holidays, imported from the online source. */
		holidays: {
			range: (from: string, to: string) =>
				request<HolidayCalendar>(`/api/v1/people/holidays?from=${from}&to=${to}`),
			summary: () => request<HolidayCounts>('/api/v1/people/holidays/summary'),
			importYear: (year: number) =>
				request<HolidayImportReport>('/api/v1/people/holidays/import', {
					method: 'POST',
					body: JSON.stringify({ year })
				})
		},

		/** Schedule: who works when in which shift. */
		schedule: {
			range: (from: string, to: string) =>
				request<ScheduleEntry[]>(`/api/v1/people/schedule${query({ from, to })}`),
			/** Checks the written plan of a period against the rules of the planning. */
			audit: (from: string, to: string) =>
				request<ScheduleAudit>(`/api/v1/people/schedule/audit${query({ from, to })}`),
			/** Sets the cell; without `kind` it is cleared. `starts_at`, `ends_at`
			 *  and `break_minutes` belong to `kind = 'free'`, `note` is the free text
			 *  of the cell. */
			set: (body: {
				employee_id: number;
				day: string;
				kind: ScheduleKind | null;
				shift_id?: number | null;
				starts_at?: string | null;
				ends_at?: string | null;
				break_minutes?: number | null;
				note?: string | null;
			}) =>
				request<ScheduleEntry | null>('/api/v1/people/schedule', {
					method: 'PUT',
					body: JSON.stringify(body)
				}),
			/** Writes the same cell onto `days` days in a row, the first day included.
			 *  A shift only reaches as far as the planning rules allow. */
			series: (body: {
				employee_id: number;
				day: string;
				days: number;
				kind: ScheduleKind;
				shift_id?: number | null;
				starts_at?: string | null;
				ends_at?: string | null;
				break_minutes?: number | null;
			}) =>
				request<ScheduleSeriesResult>('/api/v1/people/schedule/series', {
					method: 'POST',
					body: JSON.stringify(body)
				}),
			/** Removes the shift cells of a month; sick, vacation and free days stay. */
			resetShifts: (month: number, year: number) =>
				request<ResetReport>(`/api/v1/people/schedule/reset${query({ month, year })}`, {
					method: 'POST'
				}),
			/** The exchange of the plan: the sheet of a month out and back in. */
			exchange: {
				/** Reads a file without writing anything and names what it carries. */
				scan: (
					kind: 'dispo' | 'plan',
					file: ArrayBuffer,
					month: number,
					year: number,
					onlyWithTime = false
				) =>
					sendBytes<ExchangeScan>(
						`/api/v1/people/schedule/exchange/${kind}/scan${query({
							month,
							year,
							only_with_time: onlyWithTime ? 'true' : undefined
						})}`,
						file,
						'application/octet-stream'
					),
				/** Writes the disposition of a month; locations the file names are created. */
				dispo: (
					file: ArrayBuffer,
					month: number,
					year: number,
					replace: boolean,
					onlyWithTime = false
				) =>
					sendBytes<ExchangeReport>(
						`/api/v1/people/schedule/exchange/dispo${query({
							month,
							year,
							replace: replace ? 'true' : 'false',
							only_with_time: onlyWithTime ? 'true' : undefined
						})}`,
						file,
						'application/octet-stream'
					),
				/** Writes the cells of a month. */
				plan: (file: ArrayBuffer, month: number, year: number, replace: boolean) =>
					sendBytes<ExchangeReport>(
						`/api/v1/people/schedule/exchange/plan${query({
							month,
							year,
							replace: replace ? 'true' : 'false'
						})}`,
						file,
						'application/octet-stream'
					)
			},
			/** Rows the board types itself: locations with a free-text name. */
			rows: {
				/** Adds a row with a default name at the end of the disposition. */
				create: () =>
					request<PeopleListEntry>('/api/v1/people/schedule/rows', { method: 'POST' }),
				/** Renames a typed row; an empty name answers `422`. */
				rename: (rowId: number, name: string) =>
					request<PeopleListEntry>(`/api/v1/people/schedule/rows/${rowId}`, {
						method: 'PUT',
						body: JSON.stringify({ name })
					}),
				/** Removes a typed row with its events; a house stays untouched. */
				remove: (rowId: number) =>
					request<{ deleted: boolean }>(`/api/v1/people/schedule/rows/${rowId}`, {
						method: 'DELETE'
					})
			},
			/** Staffing demand per day and position. */
			demand: {
				range: (from: string, to: string) =>
					request<ScheduleDemandDay[]>(`/api/v1/people/schedule/demand${query({ from, to })}`),
				/** Replaces the day; positions without a number are dropped. */
				set: (day: string, entries: ScheduleDemandEntry[]) =>
					request<ScheduleDemandDay>(`/api/v1/people/schedule/demand/${day}`, {
						method: 'PUT',
						body: JSON.stringify({ entries })
					}),
				/** Writes the demand of `from` onto the given days. */
				copy: (from: string, to: string[]) =>
					request<{ days: number }>('/api/v1/people/schedule/demand/copy', {
						method: 'POST',
						body: JSON.stringify({ from, to })
					}),
				/** The demand the disposition of the days asks for. */
				dispoDemand: (from: string, to: string, department?: string) =>
					request<DispoDayProposal[]>(
						`/api/v1/people/schedule/dispo-demand${query({ from, to, department })}`
					)
			},
			/** Builds the plan of a period from the stored demand, the disposition,
			 *  the employees and the planning rules. */
			generate: (body: { from: string; to: string; overwrite: boolean }) =>
				request<ScheduleGenerateReport>('/api/v1/people/schedule/generate', {
					method: 'POST',
					body: JSON.stringify(body)
				}),
			/** The month of an employee as a calendar, sent by mail. */
			ical: {
				/** Sends one mail per chosen employee through the e-mail account of the
				 *  signed-in user; the answer names who got none and why. */
				send: (from: string, to: string, employeeIds: number[]) =>
					request<{ sent: number; skipped: { name: string; reason: string }[] }>(
						'/api/v1/people/schedule/ical/send',
						{ method: 'POST', body: JSON.stringify({ from, to, employee_ids: employeeIds }) }
					)
			},
			/** The disposition export of KOKOS.event PeM. */
			dispo: {
				/** The locations of the departments this session disposes — the rows
				 *  of the disposition on the board; `department` narrows the view. */
				locations: (department?: string) =>
					request<PeopleListEntry[]>(
						`/api/v1/people/schedule/locations${query({ department })}`
					),
				/** The entries of a period, in location and day order. */
				range: (from: string, to: string, department?: string) =>
					request<DispoEntry[]>(
						`/api/v1/people/schedule/dispo${query({ from, to, department })}`
					),
				/** Replaces the entries of one location on one day. */
				setDay: (locationId: number, day: string, entries: DispoEntryInput[]) =>
					request<DispoEntry[]>(
						`/api/v1/people/schedule/dispo/${locationId}/${day}`,
						{ method: 'PUT', body: JSON.stringify({ entries }) }
					),
				/** Keeps an event but stops it from asking for anybody — or puts it
				 *  back into the plan. */
				setIgnored: (id: number, ignored: boolean) =>
					request<DispoEntry>(`/api/v1/people/schedule/dispo/entry/${id}/ignored`, {
						method: 'PUT',
						body: JSON.stringify({ ignored })
					}),
				/** Reads the file and reports the locations it contains. */
				scan: (file: ArrayBuffer, month: number, year: number, onlyWithTime: boolean) =>
					sendBytes<DispoScan>(
						`/api/v1/people/schedule/dispo/scan${query({
							month,
							year,
							only_with_time: onlyWithTime ? 'true' : undefined
						})}`,
						file,
						'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
					),
				/** Writes a location per block and one entry per event. */
				import: (
					file: ArrayBuffer,
					month: number,
					year: number,
					replace: boolean,
					onlyWithTime: boolean
				) =>
					sendBytes<DispoImportReport>(
						`/api/v1/people/schedule/dispo/import${query({
							month,
							year,
							replace: replace ? 'true' : 'false',
							only_with_time: onlyWithTime ? 'true' : undefined
						})}`,
						file,
						'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
					),
				/** Removes every entry of the disposition of a month. */
				reset: (month: number, year: number) =>
					request<ResetReport>(`/api/v1/people/schedule/dispo/reset${query({ month, year })}`, {
						method: 'POST'
					})
			}
		}
	},

	/** Login (`img/Admin Menü/Login Logout`). */
	auth: {
		/** Accounts of the form's option list. */
		users: (mandant?: number | null) =>
			request<LoginUser[]>(`/api/v1/auth/users${mandant ? `?mandant=${mandant}` : ''}`),
		/** The tenants of the selection left of `Benutzer`. */
		tenants: () => request<LoginTenant[]>('/api/v1/auth/tenants'),
		login: (body: { user_name: string; password: string; mandant?: number | null }) =>
			request<LoginResponse>('/api/v1/auth/login', {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		/** Checks the token and returns the account (restore after reload). */
		session: () => request<SessionUser>('/api/v1/auth/session'),
		/** Tenant switch at runtime: the session moves to the other tenant. */
		switchMandant: (mandant: number) =>
			request<SessionUser>('/api/v1/auth/mandant', {
				method: 'POST',
				body: JSON.stringify({ mandant })
			}),
		logout: () => request<void>('/api/v1/auth/logout', { method: 'POST' }),
		/** Set a new password — the form of the first login demands it. */
		changePassword: (body: { password: string; repeat: string }) =>
			request<void>('/api/v1/auth/password', {
				method: 'POST',
				body: JSON.stringify(body)
			})
	},

	/** Start page: one call, all metrics. */
	dashboard: () => request<Dashboard>('/api/v1/dashboard'),

	agenda: {
		/** Appointments of a period; series arrive as individual days. */
		range: (params: { from: string; to: string; category?: number; user?: number; q?: string }) =>
			request<AgendaRange>(
				`/api/v1/agenda${query({
					from: params.from,
					to: params.to,
					category: params.category,
					user: params.user,
					q: params.q
				})}`
			),
		get: (id: number) => request<AgendaEvent>(`/api/v1/agenda/${id}`),
		create: (body: AgendaEventInput) =>
			request<AgendaEvent>('/api/v1/agenda', { method: 'POST', body: JSON.stringify(body) }),
		replace: (id: number, body: AgendaEventInput) =>
			request<AgendaEvent>(`/api/v1/agenda/${id}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/agenda/${id}`, { method: 'DELETE' }),
		/** Remove only this one day of the series. */
		skipOccurrence: (id: number, date: string) =>
			request<{ deleted: boolean }>(
				`/api/v1/agenda/${id}/occurrence${query({ date })}`,
				{ method: 'DELETE' }
			),
		categories: () => request<AgendaCategory[]>('/api/v1/agenda/categories'),
		createCategory: (name: string) =>
			request<AgendaCategory>('/api/v1/agenda/categories', {
				method: 'POST',
				body: JSON.stringify({ name })
			}),
		renameCategory: (id: number, name: string) =>
			request<AgendaCategory>(`/api/v1/agenda/categories/${id}`, {
				method: 'PUT',
				body: JSON.stringify({ name })
			}),
		removeCategory: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/agenda/categories/${id}`, { method: 'DELETE' })
	},

	/** Lists of the admin menu (`img/Admin Menü`). */
	settings: {
		areas: () => request<SettingsArea[]>('/api/v1/settings'),
		list: (key: string) => request<SettingsList>(`/api/v1/settings/${key}`),
		create: (key: string, values: SettingsValues) =>
			request<SettingsRow>(`/api/v1/settings/${key}`, {
				method: 'POST',
				body: JSON.stringify({ values })
			}),
		update: (key: string, id: number, values: SettingsValues) =>
			request<SettingsRow>(`/api/v1/settings/${key}/${id}`, {
				method: 'PUT',
				body: JSON.stringify({ values })
			}),
		remove: (key: string, id: number) =>
			request<{ deleted: boolean }>(`/api/v1/settings/${key}/${id}`, { method: 'DELETE' })
	},

	/** Options of the forms (checkboxes and small entries of a form). */
	prefs: {
		load: (prefix?: string) => request<Prefs>(`/api/v1/prefs${query({ prefix })}`),
		save: (key: string, value: string | null) =>
			request<void>(`/api/v1/prefs/${key}`, {
				method: 'PUT',
				body: JSON.stringify({ value })
			})
	},

	/** Order settings (`img/Admin Menü/Aufträge`). */
	orders: {
		timeline: () => request<TimelineEntry[]>('/api/v1/orders/timeline'),
		saveTimeline: (key: string, input: { label?: string | null; shown: boolean }) =>
			request<TimelineEntry>(`/api/v1/orders/timeline/${key}`, {
				method: 'PUT',
				body: JSON.stringify(input)
			}),
		headings: () => request<HeadingEntry[]>('/api/v1/orders/headings'),
		saveHeading: (number: number, input: { shown: boolean; label?: string | null }) =>
			request<HeadingEntry>(`/api/v1/orders/headings/${number}`, {
				method: 'PUT',
				body: JSON.stringify(input)
			}),
		macros: () => request<OrderMacro[]>('/api/v1/orders/macros'),
		createMacro: (input: MacroInput) =>
			request<OrderMacro>('/api/v1/orders/macros', {
				method: 'POST',
				body: JSON.stringify(input)
			}),
		updateMacro: (id: number, input: MacroInput) =>
			request<OrderMacro>(`/api/v1/orders/macros/${id}`, {
				method: 'PUT',
				body: JSON.stringify(input)
			}),
		removeMacro: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/orders/macros/${id}`, { method: 'DELETE' })
	},

	/** Bank list (`img/Admin Menü/Bank - Swift BIC Liste`). */
	banks: {
		list: (params: { q?: string; limit?: number; offset?: number } = {}) =>
			request<BankListResponse>(`/api/v1/banks${query(params)}`),
		create: (body: BankInput) =>
			request<Bank>('/api/v1/banks', { method: 'POST', body: JSON.stringify(body) }),
		update: (id: number, body: BankInput) =>
			request<Bank>(`/api/v1/banks/${id}`, { method: 'PUT', body: JSON.stringify(body) }),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/banks/${id}`, { method: 'DELETE' }),
		/** Sync with an online source: `url` or a pasted list. */
		sync: (body: { url?: string | null; csv?: string | null }) =>
			request<BankSyncReport>('/api/v1/banks/sync', {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		/** The built-in source for `Update`. */
		source: () => request<BankSource>('/api/v1/banks/source')
	},

	/** Bank accounts (`img/Admin Menü/Bankverbindungen`). */
	bankAccounts: {
		list: () => request<BankAccount[]>('/api/v1/bank-accounts'),
		get: (id: number) => request<BankAccount>(`/api/v1/bank-accounts/${id}`),
		create: (body: BankAccountInput) =>
			request<BankAccount>('/api/v1/bank-accounts', {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		update: (id: number, body: BankAccountInput) =>
			request<BankAccount>(`/api/v1/bank-accounts/${id}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/bank-accounts/${id}`, { method: 'DELETE' }),
		setStandard: (id: number, standard: boolean) =>
			request<BankAccount>(`/api/v1/bank-accounts/${id}/standard`, {
				method: 'PUT',
				body: JSON.stringify({ standard })
			}),
		convert: (body: { blz: string; account_number: string }) =>
			request<ConversionResult>('/api/v1/bank-accounts/convert', {
				method: 'POST',
				body: JSON.stringify(body)
			})
	},

	/** Tenants (`img/Admin Menü/Mandanten`). */
	tenants: {
		list: () => request<Tenant[]>('/api/v1/tenants'),
		reference: () => request<TenantReference>('/api/v1/tenants/reference'),
		get: (id: number) => request<Tenant>(`/api/v1/tenants/${id}`),
		create: (body: TenantInput) =>
			request<Tenant>('/api/v1/tenants', { method: 'POST', body: JSON.stringify(body) }),
		update: (id: number, body: TenantInput) =>
			request<Tenant>(`/api/v1/tenants/${id}`, { method: 'PUT', body: JSON.stringify(body) }),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/tenants/${id}`, { method: 'DELETE' })
	},

	/** Warehouses and storage locations (`img/Admin Menü/Lager und Lagerplätze`). */
	warehouses: {
		/** Warehouses of a tenant — without a tenant there is no list (`422`). */
		list: (mandant: number) =>
			request<WarehouseAdmin[]>(`/api/v1/warehouses${query({ mandant })}`),
		create: (mandant: number, name: string) =>
			request<WarehouseAdmin>('/api/v1/warehouses', {
				method: 'POST',
				body: JSON.stringify({ mandant, name })
			}),
		update: (
			id: number,
			body: {
				name: string;
				zugriff: boolean;
				prioritaet: boolean;
				such_std: boolean;
				entnahme: boolean;
				anmelden: boolean;
				fremd: boolean;
				konzern: boolean;
			}
		) =>
			request<WarehouseAdmin>(`/api/v1/warehouses/${id}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/warehouses/${id}`, { method: 'DELETE' }),
		/** Assign a legacy warehouse to the tenant (it carries no `Mandanten_ID` yet). */
		assign: (id: number, mandant: number) =>
			request<WarehouseAdmin>(`/api/v1/warehouses/${id}/assign`, {
				method: 'POST',
				body: JSON.stringify({ mandant })
			}),

		/** Storage location (Symetric). */
		locations: (id: number) =>
			request<WarehouseLocation[]>(`/api/v1/warehouses/${id}/locations`),
		locationCreate: (id: number, name: string) =>
			request<WarehouseLocation>(`/api/v1/warehouses/${id}/locations`, {
				method: 'POST',
				body: JSON.stringify({ name })
			}),
		locationUpdate: (
			id: number,
			locationId: number,
			body: { name: string; prefix: string; values: string }
		) =>
			request<WarehouseLocation>(`/api/v1/warehouses/${id}/locations/${locationId}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		locationRemove: (id: number, locationId: number) =>
			request<{ deleted: boolean }>(`/api/v1/warehouses/${id}/locations/${locationId}`, {
				method: 'DELETE'
			}),

		/** Storage location (Self defined). */
		bins: (id: number) => request<WarehouseBin[]>(`/api/v1/warehouses/${id}/bins`),
		binCreate: (id: number, label: string) =>
			request<WarehouseBin>(`/api/v1/warehouses/${id}/bins`, {
				method: 'POST',
				body: JSON.stringify({ label })
			}),
		binUpdate: (id: number, binId: number, label: string) =>
			request<WarehouseBin>(`/api/v1/warehouses/${id}/bins/${binId}`, {
				method: 'PUT',
				body: JSON.stringify({ label })
			}),
		binRemove: (id: number, binId: number) =>
			request<{ deleted: boolean }>(`/api/v1/warehouses/${id}/bins/${binId}`, {
				method: 'DELETE'
			})
	},

	/** Places of the house (`Foyer`, `Podium`) — where material stands while it is issued. */
	places: {
		list: () => request<Place[]>('/api/v1/places'),
		create: (body: { name: string; note: string | null; active: boolean }) =>
			request<Place>('/api/v1/places', { method: 'POST', body: JSON.stringify(body) }),
		update: (id: number, body: { name: string; note: string | null; active: boolean }) =>
			request<Place>(`/api/v1/places/${id}`, { method: 'PUT', body: JSON.stringify(body) }),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/places/${id}`, { method: 'DELETE' })
	},

	/** Stocktaking (`Inventur`): counting a warehouse against the book. */
	stocktaking: {		list: () => request<Stocktake[]>('/api/v1/stocktaking'),
		get: (id: number) => request<StocktakeDetail>(`/api/v1/stocktaking/${id}`),
		start: (body: { warehouse_id: number; name: string; note: string | null }) =>
			request<Stocktake>('/api/v1/stocktaking', { method: 'POST', body: JSON.stringify(body) }),
		count: (id: number, lineId: number, body: { counted: string | null; note: string | null }) =>
			request<void>(`/api/v1/stocktaking/${id}/lines/${lineId}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		finish: (id: number) =>
			request<Stocktake>(`/api/v1/stocktaking/${id}/finish`, { method: 'POST' }),
		cancel: (id: number) =>
			request<Stocktake>(`/api/v1/stocktaking/${id}/cancel`, { method: 'POST' })
	},

	/** Reorder suggestions (`Beschaffung`). */
	procurement: {
		suggestions: (days = 90) =>
			request<PurchasingSuggestion[]>(`/api/v1/procurement/suggestions${query({ days })}`)
	},

	/** Stock value (`Bestandswert`): per warehouse and article group. */
	stockValue: {
		report: (warehouse: number | null = null) =>
			request<StockValueLine[]>(
				`/api/v1/stock-value${query({ warehouse: warehouse ?? undefined })}`
			)
	},

	/** Purchase orders of the warehouse (`Bestellungen`). */
	purchaseOrders: {
		list: (state: string | null = null) =>
			request<PurchaseOrder[]>(`/api/v1/purchase-orders${query({ state: state ?? undefined })}`),
		/** Open lines of one article — what a delivery could still belong to. */
		openLines: (articleId: number) =>
			request<OpenOrderLine[]>(
				`/api/v1/purchase-orders/open-lines${query({ article_id: articleId })}`
			),
		get: (id: number) => request<PurchaseOrderDetail>(`/api/v1/purchase-orders/${id}`),
		create: (body: {
			supplier_no: number | null;
			ordered_at: string | null;
			expected_at: string | null;
			note: string | null;
			lines: { article_id: number; quantity: string; unit_price: string | null; note: string | null }[];
		}) => request<PurchaseOrder>('/api/v1/purchase-orders', {
			method: 'POST',
			body: JSON.stringify(body)
		}),
		/** Supplier, delivery date and note of an open order. */
		updateHeader: (
			id: number,
			body: { supplier_no: number | null; expected_at: string | null; note: string | null }
		) =>
			request<PurchaseOrder>(`/api/v1/purchase-orders/${id}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		addLine: (
			id: number,
			body: { article_id: number; quantity: string; unit_price: string | null; note: string | null }
		) =>
			request<PurchaseOrder>(`/api/v1/purchase-orders/${id}/lines`, {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		updateLine: (
			id: number,
			lineId: number,
			body: { quantity: string; unit_price: string | null; note: string | null }
		) =>
			request<PurchaseOrder>(`/api/v1/purchase-orders/${id}/lines/${lineId}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		removeLine: (id: number, lineId: number) =>
			request<PurchaseOrder>(`/api/v1/purchase-orders/${id}/lines/${lineId}`, {
				method: 'DELETE'
			}),
		close: (id: number) =>
			request<PurchaseOrder>(`/api/v1/purchase-orders/${id}/close`, { method: 'POST' }),
		cancel: (id: number) =>
			request<PurchaseOrder>(`/api/v1/purchase-orders/${id}/cancel`, { method: 'POST' })
	},

	/** User management (`img/Admin Menü/Benutzerverwaltung`). */
	users: {
		list: (view: UserView) => request<UserSummary[]>(`/api/v1/users${query({ view })}`),
		get: (id: number) => request<User>(`/api/v1/users/${id}`),
		create: (body: UserInput) =>
			request<User>('/api/v1/users', { method: 'POST', body: JSON.stringify(body) }),
		update: (id: number, body: UserInput) =>
			request<User>(`/api/v1/users/${id}`, { method: 'PUT', body: JSON.stringify(body) }),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/users/${id}`, { method: 'DELETE' }),
		addGroup: (id: number, group: number) =>
			request<User>(`/api/v1/users/${id}/groups/${group}`, { method: 'PUT' }),
		removeGroup: (id: number, group: number) =>
			request<User>(`/api/v1/users/${id}/groups/${group}`, { method: 'DELETE' }),
		/** The rights tree of the group dialog. */
		permissions: () => request<PermissionGroup[]>('/api/v1/permissions'),
		/** All groups with their rights and their users (right-hand tree). */
		groups: () => request<UserGroup[]>('/api/v1/user-groups'),
		group: (id: number) => request<UserGroup>(`/api/v1/user-groups/${id}`),
		groupCreate: (body: UserGroupInput) =>
			request<UserGroup>('/api/v1/user-groups', { method: 'POST', body: JSON.stringify(body) }),
		groupUpdate: (id: number, body: UserGroupInput) =>
			request<UserGroup>(`/api/v1/user-groups/${id}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		groupCopy: (id: number) =>
			request<UserGroup>(`/api/v1/user-groups/${id}/copy`, { method: 'POST' }),
		groupRemove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/user-groups/${id}`, { method: 'DELETE' })
	},

	addresses: {
		list: (params: AddressListParams = {}) =>
			request<Page<Address>>(
				`/api/v1/addresses${query({
					limit: params.limit,
					offset: params.offset,
					q: params.q,
					active: params.active,
					sort: params.sort,
					dir: params.dir
				})}`
			),
		/** Duplicate check: one search term, kept broad. */
		duplicates: (q: string, limit = 25) =>
			request<Page<Address>>(`/api/v1/addresses/duplicates${query({ q, limit })}`),
		/** Search form (`img/suche/Adresssuche.PNG`): criteria in the body. */
		search: (criteria: AddressSearchInput) =>
			request<Page<Address>>('/api/v1/addresses/search', {
				method: 'POST',
				body: JSON.stringify(criteria)
			}),
		get: (id: number) => request<Address>(`/api/v1/addresses/${id}`),
		create: (body: AddressInput) =>
			request<Address>('/api/v1/addresses', { method: 'POST', body: JSON.stringify(body) }),
		replace: (id: number, body: AddressInput) =>
			request<Address>(`/api/v1/addresses/${id}`, { method: 'PUT', body: JSON.stringify(body) }),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/addresses/${id}`, { method: 'DELETE' })
	},

	article: {
		list: (
			params: { limit?: number; offset?: number; q?: string; group?: number; active?: 'active' | 'inactive' | 'all' } = {}
		) =>
			request<Page<ArticleList>>(
				`/api/v1/articles${query({
					limit: params.limit,
					offset: params.offset,
					q: params.q,
					group: params.group,
					active: params.active
				})}`
			),
		/** Search form: criteria as body, hits like in the list. */
		search: (criteria: ArticleSearchInput) =>
			request<Page<ArticleList>>('/api/v1/articles/search', {
				method: 'POST',
				body: JSON.stringify(criteria)
			}),
		savedSearches: () => request<SavedSearch[]>('/api/v1/articles/searches'),
		saveSearch: (name: string, criteria: ArticleSearchInput) =>
			request<SavedSearch>('/api/v1/articles/searches', {
				method: 'POST',
				body: JSON.stringify({ name, criteria })
			}),
		deleteSearch: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/articles/searches/${id}`, { method: 'DELETE' }),
		get: (id: number) => request<Article>(`/api/v1/articles/${id}`),
		/** Template for "new": already carries the next free article number. */
		new: () => request<Article>('/api/v1/articles/new'),
		create: (body: Article) =>
			request<Article>('/api/v1/articles', { method: 'POST', body: JSON.stringify(body) }),
		replace: (id: number, body: Article) =>
			request<Article>(`/api/v1/articles/${id}`, { method: 'PUT', body: JSON.stringify(body) }),
		remove: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/articles/${id}`, { method: 'DELETE' }),

		stock: (articleId: number, q?: string) =>
			request<StockLevel>(`/api/v1/articles/${articleId}/stock${query({ q })}`),
		/** History of the article, or of one device (`device`). Newest first. */
		movements: (articleId: number, device?: number, limit = 100) =>
			request<StockMovement[]>(
				`/api/v1/articles/${articleId}/movements${query({ device, limit })}`
			),
		goodsReceipt: (articleId: number, body: GoodsReceiptInput) =>
			request<GoodsReceiptResult>(`/api/v1/articles/${articleId}/devices`, {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		updateDevice: (articleId: number, deviceId: number, body: DeviceInput) =>
			request<Device>(`/api/v1/articles/${articleId}/devices/${deviceId}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		deleteDevice: (articleId: number, deviceId: number) =>
			request<{ deleted: boolean }>(`/api/v1/articles/${articleId}/devices/${deviceId}`, {
				method: 'DELETE'
			}),
		/** Hands material out to a place of the house (`Ausgeben`). */
		issueDevice: (articleId: number, deviceId: number, body: IssueInput) =>
			request<void>(`/api/v1/articles/${articleId}/devices/${deviceId}/issue`, {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		/** Takes issued material back into the warehouse. */
		returnDevice: (
			articleId: number,
			deviceId: number,
			body: { condition: string | null; note: string | null }
		) =>
			request<void>(`/api/v1/articles/${articleId}/devices/${deviceId}/return`, {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		/** Moves material to another warehouse of the house (`Umlegen`). */
		transferDevice: (articleId: number, deviceId: number, body: TransferInput) =>
			request<void>(`/api/v1/articles/${articleId}/devices/${deviceId}/transfer`, {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		/** Consumes material out of a batch (`Verbrauchen`). */
		consumeDevice: (articleId: number, deviceId: number, body: { quantity: string; note: string | null }) =>
			request<void>(`/api/v1/articles/${articleId}/devices/${deviceId}/consume`, {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		/** Moves several rows of one article into another warehouse (`Sammel-Umlagerung`). */
		transferDevices: (
			articleId: number,
			body: { devices: number[]; stock_id: number; note: string | null }
		) =>
			request<{ moved: number; failed: { device_id: number; reason: string }[] }>(
				`/api/v1/articles/${articleId}/stock-transfer`,
				{ method: 'POST', body: JSON.stringify(body) }
			),
		/** `Meldebestand` of the article — the level the reorder suggestion compares with. */
		setReorderLevel: (articleId: number, level: string | null) =>
			request<void>(`/api/v1/articles/${articleId}/reorder-level`, {
				method: 'PUT',
				body: JSON.stringify({ level })
			}),

		files: {
			list: (articleId: number) =>
				request<ArticleFile[]>(`/api/v1/articles/${articleId}/files`),
			history: (articleId: number) =>
				request<ArticleFile[]>(`/api/v1/articles/${articleId}/files/history`),
			upload: async (articleId: number, file: File, folder = '', user?: string) => {
				const form = new FormData();
				// Field names as in the Rust handler (`datei`, `folder`, `benutzer`).
				form.append('datei', file, file.name);
				if (folder) form.append('folder', folder);
				if (user) form.append('benutzer', user);
				return (await sendForm<ArticleFile>(
					`/api/v1/articles/${articleId}/files`,
					form
				)) as ArticleFile;
			},
			link: (articleId: number, body: { folder: string; name: string; target: string }) =>
				request<ArticleFile>(`/api/v1/articles/${articleId}/files/links`, {
					method: 'POST',
					body: JSON.stringify(body)
				}),
			contentUrl: (articleId: number, fileId: number) =>
				`${BASE}/api/v1/articles/${articleId}/files/${fileId}/content`,
			/** The body with the session — for display and download in the browser. */
			content: (articleId: number, fileId: number) =>
				fetchBlob(`${BASE}/api/v1/articles/${articleId}/files/${fileId}/content`),
			remove: (articleId: number, fileId: number) =>
				request<{ deleted: boolean }>(`/api/v1/articles/${articleId}/files/${fileId}`, {
					method: 'DELETE'
				})
		},

		/** Test reports of one device (`Artikel mit Seriennummern`). */
		reports: {
			list: (articleId: number) =>
				request<InspectionReport[]>(`/api/v1/articles/${articleId}/reports`),
			ofDevice: (articleId: number, deviceId: number) =>
				request<InspectionReport[]>(
					`/api/v1/articles/${articleId}/devices/${deviceId}/reports`
				),
			upload: async (
				articleId: number,
				deviceId: number,
				file: File,
				title?: string
			): Promise<InspectionReport> => {
				const form = new FormData();
				form.append('datei', file, file.name);
				if (title?.trim()) form.append('titel', title.trim());
				return (await sendForm<InspectionReport>(
					`/api/v1/articles/${articleId}/devices/${deviceId}/reports`,
					form
				)) as InspectionReport;
			},
			content: (articleId: number, deviceId: number, reportId: number) =>
				fetchBlob(
					`${BASE}/api/v1/articles/${articleId}/devices/${deviceId}/reports/${reportId}/content`
				),
			remove: (articleId: number, deviceId: number, reportId: number) =>
				request<{ deleted: boolean }>(
					`/api/v1/articles/${articleId}/devices/${deviceId}/reports/${reportId}`,
					{ method: 'DELETE' }
				)
		},

		/** Article image (thumbnail via `Kategorie`). */
		image: {
			get: (articleId: number) => request<ArticleImage>(`/api/v1/articles/${articleId}/image`),
			upload: async (articleId: number, file: File): Promise<ArticleImage> => {
				const form = new FormData();
				form.append('datei', file, file.name);
				return (await sendForm<ArticleImage>(
					`/api/v1/articles/${articleId}/image`,
					form
				)) as ArticleImage;
			},
			placeholder: (articleId: number, placeholder: boolean) =>
				request<ArticleImage>(`/api/v1/articles/${articleId}/image/placeholder`, {
					method: 'PUT',
					body: JSON.stringify({ placeholder })
				}),
			remove: (articleId: number) =>
				request<ArticleImage>(`/api/v1/articles/${articleId}/image`, { method: 'DELETE' }),
			content: (articleId: number, fileId: number) =>
				fetchBlob(`${BASE}/api/v1/articles/${articleId}/files/${fileId}/content`)
		}
	},

	/**
	 * Import manager (`Datei → Aktion → Import-Manager`): the rows of a
	 * CSV file, already mapped onto the fields of the form.
	 */
	import: {
		articles: (rows: ArticleImportRow[]) =>
			request<ImportReport>('/api/v1/import/articles', {
				method: 'POST',
				body: JSON.stringify({ rows })
			}),
		addresses: (rows: AddressInput[]) =>
			request<ImportReport>('/api/v1/import/addresses', {
				method: 'POST',
				body: JSON.stringify({ rows })
			})
	},

	masterData: {
		articleGroups: (params: { q?: string } = {}) =>
			request<ArticleGroup[]>(`/api/v1/master-data/article-groups${query(params)}`),
		languages: () => request<Language[]>('/api/v1/master-data/languages'),
		/** Values of the address search index (dialog "Index wählen"). */
		addressIndex: () => request<string[]>('/api/v1/master-data/address-index'),
		/** Warehouses for the "Lager" selection (package slip, stock). */
		warehouses: () =>
			request<{ id: number; name: string | null; is_default: boolean; zugriff: boolean }[]>(
				'/api/v1/master-data/warehouses'
			),
		/** Bin locations of one warehouse — suggestions of the stock forms. */
		binLocations: (warehouse: number) =>
			request<string[]>(`/api/v1/master-data/bin-locations${query({ warehouse })}`),

		articleGroupCreate: (body: ArticleGroupInput) =>
			request<ArticleGroup>('/api/v1/master-data/article-groups', {
				method: 'POST',
				body: JSON.stringify(body)
			}),
		articleGroupUpdate: (id: number, body: ArticleGroupInput) =>
			request<ArticleGroup>(`/api/v1/master-data/article-groups/${id}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		articleGroupDelete: (id: number) =>
			request<{ deleted: boolean }>(`/api/v1/master-data/article-groups/${id}`, {
				method: 'DELETE'
			}),

		numberSeries: () => request<NumberSeries[]>('/api/v1/master-data/number-series'),
		numberSeriesEntry: (key: string) =>
			request<NumberSeries>(`/api/v1/master-data/number-series/${key}`),
		numberSeriesUpdate: (key: string, body: NumberSeriesInput) =>
			request<NumberSeries>(`/api/v1/master-data/number-series/${key}`, {
				method: 'PUT',
				body: JSON.stringify(body)
			}),
		numberSeriesReset: (key: string) =>
			request<NumberSeries>(`/api/v1/master-data/number-series/${key}/reset`, {
				method: 'POST'
			}),
		searchIndex: () => request<SearchIndexNode[]>('/api/v1/master-data/search-index'),

		/** Factor scales of the rental (`for_sale = false`) or quantity scales of the sale. */
		scales: (forSale = false) =>
			request<ScaleCategory[]>(
				`/api/v1/master-data/scales${query({ for_sale: String(forSale) })}`
			),
		scaleSteps: (id: number) =>
			request<ScaleStep[]>(`/api/v1/master-data/scales/${id}/steps`)
	}
};
