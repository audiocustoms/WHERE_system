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
	{ key: 'customer_no', label: 'Kunden-Nr.', kind: 'N', group: 'Adresse' },
	{ key: 'debtor', label: 'Debitor-Nr.', kind: 'T', group: 'Adresse' },
	{ key: 'supplier_no', label: 'Lieferanten-Nr.', kind: 'N', group: 'Adresse' },
	{ key: 'creditor', label: 'Kreditor-Nr.', kind: 'T', group: 'Adresse' },
	{ key: 'salutation', label: 'Anrede', kind: 'T', group: 'Adresse' },
	{ key: 'title', label: 'Titel', kind: 'T', group: 'Adresse' },
	{ key: 'name1', label: 'Firma / Nachname', kind: 'T', group: 'Adresse' },
	{ key: 'name2', label: 'Zusatz 1 / Vorname', kind: 'T', group: 'Adresse' },
	{ key: 'name3', label: 'Zusatz 2 / Spitzname', kind: 'T', group: 'Adresse' },
	{ key: 'group', label: 'Adressgruppe', kind: 'T', group: 'Adresse' },
	{ key: 'address_addition', label: 'Adresszusatz', kind: 'T', group: 'Adresse' },
	{ key: 'street', label: 'Straße', kind: 'T', group: 'Adresse' },
	{ key: 'postal_code', label: 'PLZ', kind: 'T', group: 'Adresse' },
	{ key: 'city', label: 'Ort', kind: 'T', group: 'Adresse' },
	{ key: 'state', label: 'Bundesland / Kanton', kind: 'T', group: 'Adresse' },
	{ key: 'country', label: 'Land (ausgeschrieben)', kind: 'T', group: 'Adresse' },
	{ key: 'birthday', label: 'Geburtstag', kind: 'D', group: 'Adresse' },

	// ---- Communication
	{ key: 'phone', label: 'Telefon', kind: 'T', group: 'Kommunikation' },
	{ key: 'fax', label: 'Fax', kind: 'T', group: 'Kommunikation' },
	{ key: 'mobile', label: 'Mobiltelefon', kind: 'T', group: 'Kommunikation' },
	{ key: 'email', label: 'eMail', kind: 'T', group: 'Kommunikation' },
	{ key: 'website', label: 'Webseite', kind: 'T', group: 'Kommunikation' },

	// ---- Types
	{ key: 'is_customer', label: 'Typ Kunde', kind: 'B', group: 'Typen' },
	{ key: 'is_supplier', label: 'Typ Lieferant', kind: 'B', group: 'Typen' },
	{ key: 'is_interested', label: 'Typ Interessent', kind: 'B', group: 'Typen' },
	{ key: 'is_location', label: 'Typ Location', kind: 'B', group: 'Typen' },
	{ key: 'is_employee', label: 'Typ Mitarbeiter', kind: 'B', group: 'Typen' },
	{ key: 'is_vip', label: 'Typ VIP', kind: 'B', group: 'Typen' },
	{ key: 'is_person', label: 'Einzelperson', kind: 'B', group: 'Typen' },
	{ key: 'active', label: 'aktiv', kind: 'B', group: 'Typen' },
	{ key: 'auftragssperre', label: 'Auftragssperre', kind: 'B', group: 'Typen', module: 'go' },
	{ key: 'mahnsperre', label: 'Mahnsperre', kind: 'B', group: 'Typen', module: 'money' },

	// ---- Customer
	{ key: 'vat_id', label: 'UST-ID-Nr.', kind: 'T', group: 'Kunde' },
	{ key: 'tax_number', label: 'Steuer-Nr.', kind: 'T', group: 'Kunde' },
	{ key: 'eori_no', label: 'EORI-Nr.', kind: 'T', group: 'Kunde' },
	{ key: 'register_entry', label: 'Register-Eintrag', kind: 'T', group: 'Kunde' },
	{ key: 'register_seat', label: 'Register-Sitz', kind: 'T', group: 'Kunde' },
	{ key: 'credit_limit', label: 'Kreditlimit', kind: 'Z', group: 'Kunde' },
	{ key: 'discount_rental', label: 'Kunde Rabatt Miete', kind: 'Z', group: 'Kunde', module: 'go' },
	{ key: 'discount_sale', label: 'Kunde Rabatt Verkauf', kind: 'Z', group: 'Kunde', module: 'go' },
	{ key: 'bank', label: 'Kunde Bank', kind: 'T', group: 'Kunde', module: 'money' },
	{ key: 'bank_account', label: 'Kunde Konto', kind: 'T', group: 'Kunde', module: 'money' },
	{ key: 'bank_code', label: 'Kunde BLZ', kind: 'T', group: 'Kunde', module: 'money' },
	{ key: 'iban', label: 'Kunde IBAN', kind: 'T', group: 'Kunde', module: 'money' },
	{ key: 'bic', label: 'Kunde BIC', kind: 'T', group: 'Kunde', module: 'money' },

	// ---- Supplier
	{ key: 'supplier_revenue', label: 'Lieferanten-Umsatz', kind: 'Z', group: 'Lieferant' },
	{ key: 'supplier_discount_rental', label: 'Lief. Rabatt Miete', kind: 'Z', group: 'Lieferant', module: 'go' },
	{ key: 'supplier_discount_purchase', label: 'Lief. Rabatt Einkauf', kind: 'Z', group: 'Lieferant' },
	{ key: 'supplier_bank', label: 'Lief. Bank', kind: 'T', group: 'Lieferant', module: 'money' },
	{ key: 'supplier_bank_account', label: 'Lief. Konto', kind: 'T', group: 'Lieferant', module: 'money' },
	{ key: 'supplier_bank_code', label: 'Lief. BLZ', kind: 'T', group: 'Lieferant', module: 'money' },
	{ key: 'supplier_iban', label: 'Lief. IBAN', kind: 'T', group: 'Lieferant', module: 'money' },
	{ key: 'supplier_bic', label: 'Lief. BIC', kind: 'T', group: 'Lieferant', module: 'money' }
];

/** The fields of the article mask (`img/articles_01..05.PNG`). */
export const ARTICLE_FIELDS: ImportField[] = [
	// ---- Master data
	{ key: 'article_no', label: 'Artikel-Nr.', kind: 'N', group: 'Artikel' },
	{ key: 'inventory_no', label: 'Inventar-Nr.', kind: 'T', group: 'Artikel' },
	{ key: 'name', label: 'Name', kind: 'T', group: 'Artikel' },
	{ key: 'short_name', label: 'Kurzname', kind: 'T', group: 'Artikel' },
	{ key: 'match_code', label: 'Matchcode', kind: 'T', group: 'Artikel' },
	{ key: 'unit', label: 'Einheit', kind: 'T', group: 'Artikel' },
	{ key: 'manufacturer', label: 'Hersteller', kind: 'T', group: 'Artikel' },
	{ key: 'manufacturer_no', label: 'Hersteller-Art.-Nr.', kind: 'T', group: 'Artikel' },
	{ key: 'manufacturer_country', label: 'Herstellerland', kind: 'T', group: 'Artikel' },
	{ key: 'ean_code', label: 'Hersteller EAN-Code', kind: 'T', group: 'Artikel' },
	{ key: 'retail_price', label: 'UVP / EVP brutto', kind: 'Z', group: 'Artikel' },
	{ key: 'customs_tariff_export', label: 'Zolltarif-Nr. Export', kind: 'T', group: 'Artikel' },
	{ key: 'customs_tariff_import', label: 'Zolltarif-Nr. Import', kind: 'T', group: 'Artikel' },
	{ key: 'insurance_value_net', label: 'Versicherungswert netto', kind: 'Z', group: 'Artikel' },
	{ key: 'insurance_value_gross', label: 'Versicherungswert brutto', kind: 'Z', group: 'Artikel' },
	{ key: 'team_group', label: 'Mannschaft', kind: 'T', group: 'Artikel' },
	{ key: 'commission_class', label: 'Provisionsklasse', kind: 'T', group: 'Artikel' },

	// ---- Category
	{ key: 'warengruppe', label: 'Warengruppe', kind: 'T', group: 'Kategorie' },
	{ key: 'active', label: 'Artikel aktiv', kind: 'B', group: 'Kategorie' },
	{ key: 'inventory', label: 'Kategorie „Inventar“', kind: 'B', group: 'Kategorie' },
	{ key: 'rental', label: 'Kategorie „zur Miete“', kind: 'B', group: 'Kategorie', module: 'go' },
	{ key: 'sale', label: 'Kategorie „zum Verkauf“', kind: 'B', group: 'Kategorie', module: 'go' },
	{ key: 'text_sync', label: 'Textsync', kind: 'B', group: 'Kategorie' },
	{ key: 'stock_tracking', label: 'Bestandsführung', kind: 'B', group: 'Kategorie' },
	{ key: 'no_own_stock', label: 'ohne Eigenbestand', kind: 'B', group: 'Kategorie' },
	{ key: 'individual_items', label: 'mit Einzelerfassung', kind: 'B', group: 'Kategorie' },
	{ key: 'allocation_control', label: 'Dispositionskontrolle', kind: 'B', group: 'Kategorie' },
	{ key: 'virtual_article', label: 'virtueller Artikel', kind: 'B', group: 'Kategorie' },

	// ---- Dimensions
	{ key: 'width_net', label: 'Breite netto (cm)', kind: 'Z', group: 'Maße netto' },
	{ key: 'height_net', label: 'Höhe netto (cm)', kind: 'Z', group: 'Maße netto' },
	{ key: 'depth_net', label: 'Tiefe netto (cm)', kind: 'Z', group: 'Maße netto' },
	{ key: 'volume_net', label: 'Volumen netto (m³)', kind: 'Z', group: 'Maße netto' },
	{ key: 'weight_net', label: 'Gewicht netto (kg)', kind: 'Z', group: 'Maße netto' },
	{ key: 'width_gross', label: 'Breite brutto (cm)', kind: 'Z', group: 'Maße brutto' },
	{ key: 'height_gross', label: 'Höhe brutto (cm)', kind: 'Z', group: 'Maße brutto' },
	{ key: 'depth_gross', label: 'Tiefe brutto (cm)', kind: 'Z', group: 'Maße brutto' },
	{ key: 'volume_gross', label: 'Volumen brutto (m³)', kind: 'Z', group: 'Maße brutto' },
	{ key: 'weight_gross', label: 'Gewicht brutto (kg)', kind: 'Z', group: 'Maße brutto' },

	// ---- Operation
	{ key: 'voltage_from', label: 'Betriebsspannung (V) von', kind: 'Z', group: 'Betrieb' },
	{ key: 'voltage_to', label: 'Betriebsspannung (V) bis', kind: 'Z', group: 'Betrieb' },
	{ key: 'current_draw', label: 'Stromaufnahme (A)', kind: 'Z', group: 'Betrieb' },
	{ key: 'power', label: 'elektr. Leistung (kW)', kind: 'Z', group: 'Betrieb' },
	{ key: 'setup_minutes', label: 'Aufbauzeit (min)', kind: 'N', group: 'Betrieb' },
	{ key: 'teardown_minutes', label: 'Abbauzeit (min)', kind: 'N', group: 'Betrieb' },
	{ key: 'check_out_seconds', label: 'Check-Out-Zeit (sek)', kind: 'N', group: 'Betrieb' },
	{ key: 'check_in_seconds', label: 'Check-In-Zeit (sek)', kind: 'N', group: 'Betrieb' },

	// ---- DGUV
	{ key: 'inspection_class', label: 'DGUV-Schutzklasse', kind: 'N', group: 'DGUV' },
	{ key: 'inspection_interval_months', label: 'DGUV-Prüfintervall (Monate)', kind: 'N', group: 'DGUV' },
	{ key: 'inspection_minutes', label: 'DGUV-Prüfzeit (min)', kind: 'N', group: 'DGUV' },
	{ key: 'workshop_max_usage_days', label: 'Werkstatt max. Einsatztage', kind: 'Z', group: 'DGUV' },
	{ key: 'workshop_days', label: 'Werkstatt Intervall (Tage)', kind: 'N', group: 'DGUV' },
	{ key: 'workshop_months', label: 'Werkstatt Intervall (Monate)', kind: 'N', group: 'DGUV' },

	// ---- Rental
	{ key: 'pricing.rental_price_per_day', label: 'Mietpreis pro Tag', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_base_price', label: 'Basismietpreis', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_cost_price', label: 'kalk. EK-Preis pro Tag', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_price_per_day_roll_days', label: 'Mietpreis pro Rolltag', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_base_price_roll_days', label: 'Basismietpreis Rolltage', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_cost_price_roll_days', label: 'kalk. EK-Preis pro Rolltag', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_price_per_day_off_days', label: 'Mietpreis pro Offday', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_base_price_off_days', label: 'Basismietpreis Offdays', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_cost_price_off_days', label: 'kalk. EK-Preis pro Offday', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_price_weekend', label: 'Mietpreis Wochenende', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_cost_price_weekend', label: 'kalk. EK-Preis pro Wochenende', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.deposit', label: 'Kaution', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.used_sale_price', label: 'Gebrauchtpreis', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_vat_rate', label: 'MwSt Miete (%)', kind: 'N', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_revenue_account', label: 'Ertragskonto Miete', kind: 'T', group: 'Miete', module: 'go' },
	{ key: 'pricing.rental_cost_center', label: 'Kostenstelle Miete', kind: 'T', group: 'Miete', module: 'go' },
	{ key: 'pricing.capital_binding', label: 'Kapitalbindung netto', kind: 'Z', group: 'Miete', module: 'go' },
	{ key: 'pricing.approximate_location', label: 'Annähernder Lagerplatz', kind: 'T', group: 'Miete', module: 'go' },

	// ---- Sale
	{ key: 'pricing.sale_price_net', label: 'Verkaufspreis netto', kind: 'Z', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.sale_price_gross', label: 'Verkaufspreis brutto', kind: 'Z', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.sale_retail_price', label: 'Verkaufs-UVP', kind: 'Z', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.sale_cost_price_net', label: 'Verkaufs-EK-Preis netto', kind: 'Z', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.sale_markup_percent', label: 'Aufschlag (%)', kind: 'Z', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.sale_vat_rate', label: 'MwSt Verkauf (%)', kind: 'N', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.sale_revenue_account', label: 'Ertragskonto Verkauf', kind: 'T', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.sale_cost_center', label: 'Kostenstelle Verkauf', kind: 'T', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.package_unit', label: 'Kolli-Einheit', kind: 'T', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.package_size', label: 'Kolli-Größe', kind: 'Z', group: 'Verkauf', module: 'go' },
	{ key: 'pricing.reorder_level', label: 'Meldebestand', kind: 'Z', group: 'Verkauf', module: 'go' },

	// ---- Text blocks
	{ key: 'texts.short', label: 'Beschreibung', kind: 'T', group: 'Texte' },
	{ key: 'texts.long', label: 'Beschreibung lang', kind: 'T', group: 'Texte' },
	{ key: 'texts.technical_details', label: 'Technische Beschreibung', kind: 'T', group: 'Texte' },
	{ key: 'texts.scope_of_delivery', label: 'Beschreibung Lieferumfang', kind: 'T', group: 'Texte' },
	{ key: 'texts.packaging_info', label: 'Beschreibung Packinfo', kind: 'T', group: 'Texte' },
	{ key: 'texts.webshop', label: 'Beschreibung für Webshop', kind: 'T', group: 'Texte' },

	// ---- Pyro register
	{ key: 'pyro.un_number', label: 'Pyro: UN-Nr.', kind: 'T', group: 'Pyro' },
	{ key: 'pyro.hazard_class', label: 'Pyro: Gefahrklasse', kind: 'T', group: 'Pyro' },
	{ key: 'pyro.neq', label: 'Pyro: Netto-Explosivmasse (kg)', kind: 'Z', group: 'Pyro' },
	{ key: 'pyro.category', label: 'Pyro: Kategorie', kind: 'T', group: 'Pyro' }
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
