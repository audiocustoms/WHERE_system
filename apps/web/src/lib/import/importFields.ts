import { t } from '../i18n.svelte';

/**
 * The fields the import manager can put a column onto.
 *
 * Per target (addresses, articles) the same fields the mask writes: `key` is the
 * path in the JSON the API expects (`pricing.rental_price_per_day` reaches into
 * the `Miete` register). `typ` drives the conversion of the cell content
 * (`values.ts`), `group` groups the list like the registers of the mask.
 */

/** `T` text, `N` whole number, `Z` decimal, `B` check mark, `D` date. */
export type FieldType = 'T' | 'N' | 'Z' | 'B' | 'D';

export interface ImportField {
	key: string;
	label: string;
	kind: FieldType;
	group: string;
	/** Module without which the field is not offered (e.g. rental prices). */
	module?: 'go' | 'money' | 'people';
}

/** Targets of the import. */
export type ImportTarget = 'addresses' | 'articles';

/** The fields of the address mask (`Adresse +`, register "Kunde"/"Lieferant"). */
export const ADDRESS_FIELDS: ImportField[] = [
	// ---- Basic data
	{ key: 'customer_no', label: t('Kunden-Nr.'), kind: 'N', group: t('Adresse') },
	{ key: 'debtor', label: t('Debitor-Nr.'), kind: 'T', group: t('Adresse') },
	{ key: 'supplier_no', label: t('Lieferanten-Nr.'), kind: 'N', group: t('Adresse') },
	{ key: 'creditor', label: t('Kreditor-Nr.'), kind: 'T', group: t('Adresse') },
	{ key: 'salutation', label: t('Anrede'), kind: 'T', group: t('Adresse') },
	{ key: 'title', label: t('Titel'), kind: 'T', group: t('Adresse') },
	{ key: 'name1', label: t('Firma / Nachname'), kind: 'T', group: t('Adresse') },
	{ key: 'name2', label: t('Zusatz 1 / Vorname'), kind: 'T', group: t('Adresse') },
	{ key: 'name3', label: t('Zusatz 2 / Spitzname'), kind: 'T', group: t('Adresse') },
	{ key: 'group', label: t('Adressgruppe'), kind: 'T', group: t('Adresse') },
	{ key: 'address_addition', label: t('Adresszusatz'), kind: 'T', group: t('Adresse') },
	{ key: 'street', label: t('Straße'), kind: 'T', group: t('Adresse') },
	{ key: 'postal_code', label: t('PLZ'), kind: 'T', group: t('Adresse') },
	{ key: 'city', label: t('Ort'), kind: 'T', group: t('Adresse') },
	{ key: 'state', label: t('Bundesland / Kanton'), kind: 'T', group: t('Adresse') },
	{ key: 'country', label: t('Land (ausgeschrieben)'), kind: 'T', group: t('Adresse') },
	{ key: 'birthday', label: t('Geburtstag'), kind: 'D', group: t('Adresse') },

	// ---- Communication
	{ key: 'phone', label: t('Telefon'), kind: 'T', group: t('Kommunikation') },
	{ key: 'fax', label: t('Fax'), kind: 'T', group: t('Kommunikation') },
	{ key: 'mobile', label: t('Mobiltelefon'), kind: 'T', group: t('Kommunikation') },
	{ key: 'email', label: t('eMail'), kind: 'T', group: t('Kommunikation') },
	{ key: 'website', label: t('Webseite'), kind: 'T', group: t('Kommunikation') },

	// ---- Types
	{ key: 'is_customer', label: t('Typ Kunde'), kind: 'B', group: t('Typen') },
	{ key: 'is_supplier', label: t('Typ Lieferant'), kind: 'B', group: t('Typen') },
	{ key: 'is_interested', label: t('Typ Interessent'), kind: 'B', group: t('Typen') },
	{ key: 'is_location', label: t('Typ Location'), kind: 'B', group: t('Typen') },
	{ key: 'is_employee', label: t('Typ Mitarbeiter'), kind: 'B', group: t('Typen') },
	{ key: 'is_vip', label: t('Typ VIP'), kind: 'B', group: t('Typen') },
	{ key: 'is_person', label: t('Einzelperson'), kind: 'B', group: t('Typen') },
	{ key: 'active', label: t('aktiv'), kind: 'B', group: t('Typen') },
	{ key: 'auftragssperre', label: t('Auftragssperre'), kind: 'B', group: t('Typen'), module: 'go' },
	{ key: 'mahnsperre', label: t('Mahnsperre'), kind: 'B', group: t('Typen'), module: 'money' },

	// ---- Customer
	{ key: 'vat_id', label: t('UST-ID-Nr.'), kind: 'T', group: t('Kunde') },
	{ key: 'tax_number', label: t('Steuer-Nr.'), kind: 'T', group: t('Kunde') },
	{ key: 'eori_no', label: t('EORI-Nr.'), kind: 'T', group: t('Kunde') },
	{ key: 'register_entry', label: t('Register-Eintrag'), kind: 'T', group: t('Kunde') },
	{ key: 'register_seat', label: t('Register-Sitz'), kind: 'T', group: t('Kunde') },
	{ key: 'credit_limit', label: t('Kreditlimit'), kind: 'Z', group: t('Kunde') },
	{ key: 'discount_rental', label: t('Kunde Rabatt Miete'), kind: 'Z', group: t('Kunde'), module: 'go' },
	{ key: 'discount_sale', label: t('Kunde Rabatt Verkauf'), kind: 'Z', group: t('Kunde'), module: 'go' },
	{ key: 'bank', label: t('Kunde Bank'), kind: 'T', group: t('Kunde'), module: 'money' },
	{ key: 'bank_account', label: t('Kunde Konto'), kind: 'T', group: t('Kunde'), module: 'money' },
	{ key: 'bank_code', label: t('Kunde BLZ'), kind: 'T', group: t('Kunde'), module: 'money' },
	{ key: 'iban', label: t('Kunde IBAN'), kind: 'T', group: t('Kunde'), module: 'money' },
	{ key: 'bic', label: t('Kunde BIC'), kind: 'T', group: t('Kunde'), module: 'money' },

	// ---- Supplier
	{ key: 'supplier_revenue', label: t('Lieferanten-Umsatz'), kind: 'Z', group: t('Lieferant') },
	{ key: 'supplier_discount_rental', label: t('Lief. Rabatt Miete'), kind: 'Z', group: t('Lieferant'), module: 'go' },
	{ key: 'supplier_discount_purchase', label: t('Lief. Rabatt Einkauf'), kind: 'Z', group: t('Lieferant') },
	{ key: 'supplier_bank', label: t('Lief. Bank'), kind: 'T', group: t('Lieferant'), module: 'money' },
	{ key: 'supplier_bank_account', label: t('Lief. Konto'), kind: 'T', group: t('Lieferant'), module: 'money' },
	{ key: 'supplier_bank_code', label: t('Lief. BLZ'), kind: 'T', group: t('Lieferant'), module: 'money' },
	{ key: 'supplier_iban', label: t('Lief. IBAN'), kind: 'T', group: t('Lieferant'), module: 'money' },
	{ key: 'supplier_bic', label: t('Lief. BIC'), kind: 'T', group: t('Lieferant'), module: 'money' }
];

/** The fields of the article mask (`img/articles_01..05.PNG`). */
export const ARTICLE_FIELDS: ImportField[] = [
	// ---- Master data
	{ key: 'article_no', label: t('Artikel-Nr.'), kind: 'N', group: t('Artikel') },
	{ key: 'inventory_no', label: t('Inventar-Nr.'), kind: 'T', group: t('Artikel') },
	{ key: 'name', label: t('Name'), kind: 'T', group: t('Artikel') },
	{ key: 'short_name', label: t('Kurzname'), kind: 'T', group: t('Artikel') },
	{ key: 'match_code', label: t('Matchcode'), kind: 'T', group: t('Artikel') },
	{ key: 'unit', label: t('Einheit'), kind: 'T', group: t('Artikel') },
	{ key: 'manufacturer', label: t('Hersteller'), kind: 'T', group: t('Artikel') },
	{ key: 'manufacturer_no', label: t('Hersteller-Art.-Nr.'), kind: 'T', group: t('Artikel') },
	{ key: 'manufacturer_country', label: t('Herstellerland'), kind: 'T', group: t('Artikel') },
	{ key: 'ean_code', label: t('Hersteller EAN-Code'), kind: 'T', group: t('Artikel') },
	{ key: 'retail_price', label: t('UVP / EVP brutto'), kind: 'Z', group: t('Artikel') },
	{ key: 'customs_tariff_export', label: t('Zolltarif-Nr. Export'), kind: 'T', group: t('Artikel') },
	{ key: 'customs_tariff_import', label: t('Zolltarif-Nr. Import'), kind: 'T', group: t('Artikel') },
	{ key: 'insurance_value_net', label: t('Versicherungswert netto'), kind: 'Z', group: t('Artikel') },
	{ key: 'insurance_value_gross', label: t('Versicherungswert brutto'), kind: 'Z', group: t('Artikel') },
	{ key: 'team_group', label: t('Mannschaft'), kind: 'T', group: t('Artikel') },
	{ key: 'commission_class', label: t('Provisionsklasse'), kind: 'T', group: t('Artikel') },

	// ---- Category
	{ key: 'warengruppe', label: t('Warengruppe'), kind: 'T', group: t('Kategorie') },
	{ key: 'active', label: t('Artikel aktiv'), kind: 'B', group: t('Kategorie') },
	{ key: 'inventory', label: t('Kategorie „Inventar“'), kind: 'B', group: t('Kategorie') },
	{ key: 'rental', label: t('Kategorie „zur Miete“'), kind: 'B', group: t('Kategorie'), module: 'go' },
	{ key: 'sale', label: t('Kategorie „zum Verkauf“'), kind: 'B', group: t('Kategorie'), module: 'go' },
	{ key: 'text_sync', label: t('Textsync'), kind: 'B', group: t('Kategorie') },
	{ key: 'stock_tracking', label: t('Bestandsführung'), kind: 'B', group: t('Kategorie') },
	{ key: 'no_own_stock', label: t('ohne Eigenbestand'), kind: 'B', group: t('Kategorie') },
	{ key: 'individual_items', label: t('mit Einzelerfassung'), kind: 'B', group: t('Kategorie') },
	{ key: 'allocation_control', label: t('Dispositionskontrolle'), kind: 'B', group: t('Kategorie') },
	{ key: 'virtual_article', label: t('virtueller Artikel'), kind: 'B', group: t('Kategorie') },

	// ---- Dimensions
	{ key: 'width_net', label: t('Breite netto (cm)'), kind: 'Z', group: t('Maße netto') },
	{ key: 'height_net', label: t('Höhe netto (cm)'), kind: 'Z', group: t('Maße netto') },
	{ key: 'depth_net', label: t('Tiefe netto (cm)'), kind: 'Z', group: t('Maße netto') },
	{ key: 'volume_net', label: t('Volumen netto (m³)'), kind: 'Z', group: t('Maße netto') },
	{ key: 'weight_net', label: t('Gewicht netto (kg)'), kind: 'Z', group: t('Maße netto') },
	{ key: 'width_gross', label: t('Breite brutto (cm)'), kind: 'Z', group: t('Maße brutto') },
	{ key: 'height_gross', label: t('Höhe brutto (cm)'), kind: 'Z', group: t('Maße brutto') },
	{ key: 'depth_gross', label: t('Tiefe brutto (cm)'), kind: 'Z', group: t('Maße brutto') },
	{ key: 'volume_gross', label: t('Volumen brutto (m³)'), kind: 'Z', group: t('Maße brutto') },
	{ key: 'weight_gross', label: t('Gewicht brutto (kg)'), kind: 'Z', group: t('Maße brutto') },

	// ---- Operation
	{ key: 'voltage_from', label: t('Betriebsspannung (V) von'), kind: 'Z', group: t('Betrieb') },
	{ key: 'voltage_to', label: t('Betriebsspannung (V) bis'), kind: 'Z', group: t('Betrieb') },
	{ key: 'current_draw', label: t('Stromaufnahme (A)'), kind: 'Z', group: t('Betrieb') },
	{ key: 'power', label: t('elektr. Leistung (kW)'), kind: 'Z', group: t('Betrieb') },
	{ key: 'setup_minutes', label: t('Aufbauzeit (min)'), kind: 'N', group: t('Betrieb') },
	{ key: 'teardown_minutes', label: t('Abbauzeit (min)'), kind: 'N', group: t('Betrieb') },
	{ key: 'check_out_seconds', label: t('Check-Out-Zeit (sek)'), kind: 'N', group: t('Betrieb') },
	{ key: 'check_in_seconds', label: t('Check-In-Zeit (sek)'), kind: 'N', group: t('Betrieb') },

	// ---- DGUV
	{ key: 'inspection_class', label: t('DGUV-Schutzklasse'), kind: 'N', group: t('DGUV') },
	{ key: 'inspection_interval_months', label: t('DGUV-Prüfintervall (Monate)'), kind: 'N', group: t('DGUV') },
	{ key: 'inspection_minutes', label: t('DGUV-Prüfzeit (min)'), kind: 'N', group: t('DGUV') },
	{ key: 'workshop_max_usage_days', label: t('Werkstatt max. Einsatztage'), kind: 'Z', group: t('DGUV') },
	{ key: 'workshop_days', label: t('Werkstatt Intervall (Tage)'), kind: 'N', group: t('DGUV') },
	{ key: 'workshop_months', label: t('Werkstatt Intervall (Monate)'), kind: 'N', group: t('DGUV') },

	// ---- Rental
	{ key: 'pricing.rental_price_per_day', label: t('Mietpreis pro Tag'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_base_price', label: t('Basismietpreis'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_cost_price', label: t('kalk. EK-Preis pro Tag'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_price_per_day_roll_days', label: t('Mietpreis pro Rolltag'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_base_price_roll_days', label: t('Basismietpreis Rolltage'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_cost_price_roll_days', label: t('kalk. EK-Preis pro Rolltag'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_price_per_day_off_days', label: t('Mietpreis pro Offday'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_base_price_off_days', label: t('Basismietpreis Offdays'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_cost_price_off_days', label: t('kalk. EK-Preis pro Offday'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_price_weekend', label: t('Mietpreis Wochenende'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_cost_price_weekend', label: t('kalk. EK-Preis pro Wochenende'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.deposit', label: t('Kaution'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.used_sale_price', label: t('Gebrauchtpreis'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_vat_rate', label: t('MwSt Miete (%)'), kind: 'N', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_revenue_account', label: t('Ertragskonto Miete'), kind: 'T', group: t('Miete'), module: 'go' },
	{ key: 'pricing.rental_cost_center', label: t('Kostenstelle Miete'), kind: 'T', group: t('Miete'), module: 'go' },
	{ key: 'pricing.capital_binding', label: t('Kapitalbindung netto'), kind: 'Z', group: t('Miete'), module: 'go' },
	{ key: 'pricing.approximate_location', label: t('Annähernder Lagerplatz'), kind: 'T', group: t('Miete'), module: 'go' },

	// ---- Sale
	{ key: 'pricing.sale_price_net', label: t('Verkaufspreis netto'), kind: 'Z', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.sale_price_gross', label: t('Verkaufspreis brutto'), kind: 'Z', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.sale_retail_price', label: t('Verkaufs-UVP'), kind: 'Z', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.sale_cost_price_net', label: t('Verkaufs-EK-Preis netto'), kind: 'Z', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.sale_markup_percent', label: t('Aufschlag (%)'), kind: 'Z', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.sale_vat_rate', label: t('MwSt Verkauf (%)'), kind: 'N', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.sale_revenue_account', label: t('Ertragskonto Verkauf'), kind: 'T', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.sale_cost_center', label: t('Kostenstelle Verkauf'), kind: 'T', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.package_unit', label: t('Kolli-Einheit'), kind: 'T', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.package_size', label: t('Kolli-Größe'), kind: 'Z', group: t('Verkauf'), module: 'go' },
	{ key: 'pricing.reorder_level', label: t('Meldebestand'), kind: 'Z', group: t('Verkauf'), module: 'go' },

	// ---- Text blocks
	{ key: 'texts.short', label: t('Beschreibung'), kind: 'T', group: t('Texte') },
	{ key: 'texts.long', label: t('Beschreibung lang'), kind: 'T', group: t('Texte') },
	{ key: 'texts.technical_details', label: t('Technische Beschreibung'), kind: 'T', group: t('Texte') },
	{ key: 'texts.scope_of_delivery', label: t('Beschreibung Lieferumfang'), kind: 'T', group: t('Texte') },
	{ key: 'texts.packaging_info', label: t('Beschreibung Packinfo'), kind: 'T', group: t('Texte') },
	{ key: 'texts.webshop', label: t('Beschreibung für Webshop'), kind: 'T', group: t('Texte') },

	// ---- Pyro register
	{ key: 'pyro.un_number', label: t('Pyro: UN-Nr.'), kind: 'T', group: t('Pyro') },
	{ key: 'pyro.hazard_class', label: t('Pyro: Gefahrklasse'), kind: 'T', group: t('Pyro') },
	{ key: 'pyro.neq', label: t('Pyro: Netto-Explosivmasse (kg)'), kind: 'Z', group: t('Pyro') },
	{ key: 'pyro.category', label: t('Pyro: Kategorie'), kind: 'T', group: t('Pyro') }
];

/** The fields of a target. */
export function fieldsFor(target: ImportTarget): ImportField[] {
	return target === 'addresses' ? ADDRESS_FIELDS : ARTICLE_FIELDS;
}

/** The field for a key — `undefined` when the mapping is out of date. */
export function fieldWith(target: ImportTarget, key: string | null): ImportField | undefined {
	if (!key) return undefined;
	return fieldsFor(target).find((field) => field.key === key);
}
