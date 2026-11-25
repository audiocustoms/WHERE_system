//! Row types mirroring the reconstructed legacy columns.
//!
//! 4D column names are German and historically inconsistent (for example
//! `"Adresse"."Primary_key"` vs `"Artikel"."Primary_Key"`). They are mapped here,
//! once, so the rest of the codebase uses consistent English names.
//!
//! Every rename is explicit because `sqlx::FromRow` matches column names
//! case-sensitively against what PostgreSQL reports.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AddressRow {
    #[sqlx(rename = "Primary_key")]
    pub id: i32,
    #[sqlx(rename = "KundenNr")]
    pub customer_no: Option<i32>,
    #[sqlx(rename = "Titel")]
    pub title: Option<String>,
    #[sqlx(rename = "Name1")]
    pub name1: Option<String>,
    #[sqlx(rename = "Name2")]
    pub name2: Option<String>,
    #[sqlx(rename = "Name3")]
    pub name3: Option<String>,
    #[sqlx(rename = "Anrede")]
    pub salutation: Option<String>,
    #[sqlx(rename = "import_Strasse")]
    pub street: Option<String>,
    #[sqlx(rename = "import_Land")]
    pub country: Option<String>,
    #[sqlx(rename = "import_Ort")]
    pub city: Option<String>,
    #[sqlx(rename = "USTID")]
    pub vat_id: Option<String>,
    #[sqlx(rename = "knd_Debitor")]
    pub debtor: Option<String>,
    #[sqlx(rename = "import_Telefon")]
    pub phone: Option<String>,
    #[sqlx(rename = "import_Email")]
    pub email: Option<String>,
    #[sqlx(rename = "GruppenNameAdresse")]
    pub group: Option<String>,
    #[sqlx(rename = "Type_Kunde")]
    pub is_customer: bool,
    #[sqlx(rename = "Type_Lieferant")]
    pub is_supplier: bool,
    #[sqlx(rename = "Auftragssperre")]
    pub auftragssperre: bool,
    #[sqlx(rename = "Mahnsperre")]
    pub mahnsperre: bool,

    #[sqlx(rename = "active")]
    pub active: bool,
    #[sqlx(rename = "adr_Einzelperson")]
    pub is_person: bool,
    #[sqlx(rename = "Person_Geburtstag")]
    pub birthday: Option<NaiveDate>,
    #[sqlx(rename = "import_Adresszusatz")]
    pub address_addition: Option<String>,
    #[sqlx(rename = "import_PLZ")]
    pub postal_code: Option<String>,
    #[sqlx(rename = "import_Bundesland")]
    pub state: Option<String>,
    #[sqlx(rename = "import_Fax")]
    pub fax: Option<String>,
    #[sqlx(rename = "import_MobilTel")]
    pub mobile: Option<String>,
    #[sqlx(rename = "import_URL")]
    pub website: Option<String>,
    #[sqlx(rename = "Type_Interessent")]
    pub is_interested: bool,
    #[sqlx(rename = "Type_Location")]
    pub is_location: bool,
    #[sqlx(rename = "Type_Personal")]
    pub is_employee: bool,
    #[sqlx(rename = "Type_VIP")]
    pub is_vip: bool,

    #[sqlx(rename = "Adr_EORINr")]
    pub eori_no: Option<String>,
    #[sqlx(rename = "Knd_Steuernummer")]
    pub tax_number: Option<String>,
    #[sqlx(rename = "HRB")]
    pub register_entry: Option<String>,
    #[sqlx(rename = "HRB_Sitz")]
    pub register_seat: Option<String>,
    #[sqlx(rename = "KreditLimit")]
    pub credit_limit: Option<Decimal>,
    #[sqlx(rename = "import_knd_RabattMiete")]
    pub discount_rental: Option<Decimal>,
    #[sqlx(rename = "import_knd_RabattVerkauf")]
    pub discount_sale: Option<Decimal>,
    #[sqlx(rename = "knd_Bank")]
    pub bank: Option<String>,
    #[sqlx(rename = "knd_Konto")]
    pub bank_account: Option<String>,
    #[sqlx(rename = "knd_BLZ")]
    pub bank_code: Option<String>,
    #[sqlx(rename = "knd_IBAN")]
    pub iban: Option<String>,
    #[sqlx(rename = "knd_BIC")]
    pub bic: Option<String>,

    #[sqlx(rename = "lief_LieferantenNr")]
    pub supplier_no: Option<i32>,
    #[sqlx(rename = "lief_Kreditor")]
    pub creditor: Option<String>,
    #[sqlx(rename = "lief_Umsatz")]
    pub supplier_revenue: Option<Decimal>,
    #[sqlx(rename = "import_lief_RabattMiete")]
    pub supplier_discount_rental: Option<Decimal>,
    #[sqlx(rename = "import_lief_RabattVerkauf")]
    pub supplier_discount_purchase: Option<Decimal>,
    #[sqlx(rename = "lief_Bank")]
    pub supplier_bank: Option<String>,
    #[sqlx(rename = "lief_Konto")]
    pub supplier_bank_account: Option<String>,
    #[sqlx(rename = "lief_BLZ")]
    pub supplier_bank_code: Option<String>,
    #[sqlx(rename = "lief_IBAN")]
    pub supplier_iban: Option<String>,
    #[sqlx(rename = "lief_BIC")]
    pub supplier_bic: Option<String>,
}

impl From<AddressRow> for where_domain::Address {
    fn from(r: AddressRow) -> Self {
        Self {
            id: r.id,
            customer_no: r.customer_no,
            title: r.title,
            name1: r.name1,
            name2: r.name2,
            name3: r.name3,
            salutation: r.salutation,
            street: r.street,
            country: r.country,
            city: r.city,
            vat_id: r.vat_id,
            debtor: r.debtor,
            phone: r.phone,
            email: r.email,
            group: r.group,
            is_customer: r.is_customer,
            is_supplier: r.is_supplier,
            auftragssperre: r.auftragssperre,
            mahnsperre: r.mahnsperre,
            active: r.active,
            is_person: r.is_person,
            birthday: r.birthday,
            address_addition: r.address_addition,
            postal_code: r.postal_code,
            state: r.state,
            fax: r.fax,
            mobile: r.mobile,
            website: r.website,
            is_interested: r.is_interested,
            is_location: r.is_location,
            is_employee: r.is_employee,
            is_vip: r.is_vip,
            eori_no: r.eori_no,
            tax_number: r.tax_number,
            register_entry: r.register_entry,
            register_seat: r.register_seat,
            credit_limit: r.credit_limit,
            discount_rental: r.discount_rental,
            discount_sale: r.discount_sale,
            bank: r.bank,
            bank_account: r.bank_account,
            bank_code: r.bank_code,
            iban: r.iban,
            bic: r.bic,
            supplier_no: r.supplier_no,
            creditor: r.creditor,
            supplier_revenue: r.supplier_revenue,
            supplier_discount_rental: r.supplier_discount_rental,
            supplier_discount_purchase: r.supplier_discount_purchase,
            supplier_bank: r.supplier_bank,
            supplier_bank_account: r.supplier_bank_account,
            supplier_bank_code: r.supplier_bank_code,
            supplier_iban: r.supplier_iban,
            supplier_bic: r.supplier_bic,
        }
    }
}

// ------------------------------------------------------------------ Article

/// Complete row of the article form (`img/articles_01..05.PNG`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticleRow {
    #[sqlx(rename = "Primary_Key")]
    pub id: i32,
    #[sqlx(rename = "ArtikelNr")]
    pub article_no: i32,
    #[sqlx(rename = "InventarNr")]
    pub inventory_no: Option<String>,
    #[sqlx(rename = "Name")]
    pub name: Option<String>,
    #[sqlx(rename = "Import_kurzname")]
    pub short_name: Option<String>,
    #[sqlx(rename = "Matchcode")]
    pub match_code: Option<String>,
    #[sqlx(rename = "Import_Einheit")]
    pub unit: Option<String>,

    #[sqlx(rename = "Aktiv")]
    pub active: Option<bool>,
    #[sqlx(rename = "Inventar_Flag")]
    pub inventory: Option<bool>,
    #[sqlx(rename = "ZurMiete_Flag")]
    pub rental: Option<bool>,
    #[sqlx(rename = "ZumVerkauf_Flag")]
    pub sale: Option<bool>,
    #[sqlx(rename = "Textsync_Flag")]
    pub text_sync: Option<bool>,

    #[sqlx(rename = "Bestandskontrolle")]
    pub stock_tracking: Option<bool>,
    #[sqlx(rename = "OhneEigenbestand")]
    pub no_own_stock: Option<bool>,
    #[sqlx(rename = "SerienNr_Flag")]
    pub individual_items: Option<bool>,
    #[sqlx(rename = "Dispokontrolle")]
    pub allocation_control: Option<bool>,
    #[sqlx(rename = "virtuellerArtikel")]
    pub virtual_article: Option<bool>,

    #[sqlx(rename = "EANCode")]
    pub ean_code: Option<String>,
    #[sqlx(rename = "HerstellerArtNr")]
    pub manufacturer_no: Option<String>,
    #[sqlx(rename = "Hersteller")]
    pub manufacturer: Option<String>,
    #[sqlx(rename = "Herstellerland")]
    pub manufacturer_country: Option<String>,
    #[sqlx(rename = "Import_VKPreisEmpfehlung")]
    pub retail_price: Option<Decimal>,
    #[sqlx(rename = "Zolltarifnummer_export")]
    pub customs_tariff_export: Option<String>,
    #[sqlx(rename = "Zolltarifnummer_import")]
    pub customs_tariff_import: Option<String>,

    #[sqlx(rename = "Import_VersicherungswertNetto")]
    pub insurance_value_net: Option<Decimal>,
    #[sqlx(rename = "Import_VersicherungswertBrutto")]
    pub insurance_value_gross: Option<Decimal>,
    #[sqlx(rename = "TeamAgenda_Gruppe")]
    pub team_group: Option<String>,
    #[sqlx(rename = "Proviklasse")]
    pub commission_class: Option<String>,

    #[sqlx(rename = "Breite_Netto")]
    pub width_net: Option<Decimal>,
    #[sqlx(rename = "Hoehe_Netto")]
    pub height_net: Option<Decimal>,
    #[sqlx(rename = "Tiefe_Netto")]
    pub depth_net: Option<Decimal>,
    #[sqlx(rename = "Volumen_Netto")]
    pub volume_net: Option<Decimal>,
    #[sqlx(rename = "Gewicht_Netto")]
    pub weight_net: Option<Decimal>,

    #[sqlx(rename = "Breite_Brutto")]
    pub width_gross: Option<Decimal>,
    #[sqlx(rename = "Hoehe_Brutto")]
    pub height_gross: Option<Decimal>,
    #[sqlx(rename = "Tiefe_Brutto")]
    pub depth_gross: Option<Decimal>,
    #[sqlx(rename = "Volumen_Brutto")]
    pub volume_gross: Option<Decimal>,
    #[sqlx(rename = "Gewicht_Brutto")]
    pub weight_gross: Option<Decimal>,

    #[sqlx(rename = "BetriebsspannungVon")]
    pub voltage_from: Option<Decimal>,
    #[sqlx(rename = "BetriebsspannungBis")]
    pub voltage_to: Option<Decimal>,
    #[sqlx(rename = "Stromaufnahme")]
    pub current_draw: Option<Decimal>,
    #[sqlx(rename = "Leistung")]
    pub power: Option<Decimal>,
    #[sqlx(rename = "Aufbauzeit")]
    pub setup_minutes: Option<i32>,
    #[sqlx(rename = "Abbauzeit")]
    pub teardown_minutes: Option<i32>,
    #[sqlx(rename = "CheckOut_time")]
    pub check_out_seconds: Option<i32>,
    #[sqlx(rename = "CheckIn_time")]
    pub check_in_seconds: Option<i32>,

    #[sqlx(rename = "VDE_Schutzklasse")]
    pub inspection_class: Option<i32>,
    #[sqlx(rename = "VDECheck_Intervall")]
    pub inspection_interval_months: Option<i32>,
    #[sqlx(rename = "VDE_test_time")]
    pub inspection_minutes: Option<i32>,
    #[sqlx(rename = "WM_FlagEinsatztage")]
    pub workshop_by_usage_days: Option<bool>,
    #[sqlx(rename = "WM_maxEinsatztage")]
    pub workshop_max_usage_days: Option<Decimal>,
    #[sqlx(rename = "WM_FlagWartung")]
    pub workshop_fixed_intervals: Option<bool>,
    #[sqlx(rename = "WM_FlagTage")]
    pub workshop_by_days: Option<bool>,
    #[sqlx(rename = "WM_FlagMonate")]
    pub workshop_by_months: Option<bool>,
    #[sqlx(rename = "WM_AnzahlTage")]
    pub workshop_days: Option<i32>,
    #[sqlx(rename = "WM_AnzahlMonate")]
    pub workshop_months: Option<i32>,

    // ---- Tab "Miete"
    #[sqlx(rename = "RolltagePreise_aktiv")]
    pub rental_roll_days_active: Option<bool>,
    #[sqlx(rename = "OffDayPreise_aktiv")]
    pub rental_off_days_active: Option<bool>,
    #[sqlx(rename = "WEPreis_aktiv")]
    pub rental_weekend_active: Option<bool>,
    #[sqlx(rename = "Verpackungseinheit")]
    pub packaging_release: Option<bool>,
    #[sqlx(rename = "RabattMOFF")]
    pub rental_no_discount: Option<bool>,
    #[sqlx(rename = "VersicherungMOFF")]
    pub rental_not_insurable: Option<bool>,
    #[sqlx(rename = "NurMaterial")]
    pub material_only: Option<bool>,
    #[sqlx(rename = "Unikatartikel")]
    pub unique_article: Option<bool>,
    #[sqlx(rename = "MieteForceSNScan")]
    pub checkout_requires_serial: Option<bool>,
    #[sqlx(rename = "MieteWiredBundle")]
    pub fixed_installation: Option<bool>,
    #[sqlx(rename = "Sichtpruefung")]
    pub inspection_on_checkin: Option<bool>,
    #[sqlx(rename = "Funktionspruefung")]
    pub function_check_on_checkin: Option<bool>,
    #[sqlx(rename = "Verbrauchserfassung")]
    pub consumption_recording: Option<bool>,

    // ---- Tab "Verkauf"
    #[sqlx(rename = "RabattVOFF")]
    pub sale_no_discount: Option<bool>,
    #[sqlx(rename = "KolliVielfaches")]
    pub sell_full_packages: Option<bool>,
    #[sqlx(rename = "KolliEinheit")]
    pub package_unit: Option<String>,
    #[sqlx(rename = "KolliGroesse")]
    pub package_size: Option<Decimal>,
    #[sqlx(rename = "VerkaufMindBestand")]
    pub reorder_level: Option<Decimal>,

    // ---- Tab "Pyro" (`pyro_*` of `"Artikel"`)
    #[sqlx(rename = "pyro_UN_number")]
    pub pyro_un_number: Option<String>,
    #[sqlx(rename = "pyro_UN_description_o")]
    pub pyro_un_description: Option<String>,
    #[sqlx(rename = "pyro_hazard_class_o")]
    pub pyro_hazard_class: Option<String>,
    #[sqlx(rename = "pyro_caliber")]
    pub pyro_caliber: Option<i32>,
    #[sqlx(rename = "pyro_lateral_safety_distance")]
    pub pyro_lateral_safety_distance: Option<Decimal>,
    #[sqlx(rename = "pyro_vertical_safety_distance")]
    pub pyro_vertical_safety_distance: Option<Decimal>,
    #[sqlx(rename = "pyro_ascending_height")]
    pub pyro_ascending_height: Option<Decimal>,
    #[sqlx(rename = "pyro_sound_pressure_level")]
    pub pyro_sound_pressure_level: Option<Decimal>,
    #[sqlx(rename = "pyro_sound_pressure_range")]
    pub pyro_sound_pressure_range: Option<Decimal>,
    #[sqlx(rename = "pyro_tunnel_delimitation_code")]
    pub pyro_tunnel_restriction_code: Option<String>,
    #[sqlx(rename = "pyro_delay_time")]
    pub pyro_delay_time: Option<Decimal>,
    #[sqlx(rename = "pyro_duration_time")]
    pub pyro_effect_duration: Option<Decimal>,
    #[sqlx(rename = "pyro_category")]
    pub pyro_category: Option<String>,
    #[sqlx(rename = "pyro_bam_id")]
    pub pyro_bam_id: Option<String>,
    #[sqlx(rename = "pyro_categorisation")]
    pub pyro_classification: Option<String>,
    #[sqlx(rename = "pyro_packing_group")]
    pub pyro_packing_group: Option<String>,
    #[sqlx(rename = "pyro_approval_number")]
    pub pyro_approval_number: Option<String>,
    #[sqlx(rename = "pyro_neq")]
    pub pyro_neq: Option<Decimal>,
    #[sqlx(rename = "pyro_neq_einheit")]
    pub pyro_neq_unit: Option<String>,
    #[sqlx(rename = "pyro_UN_points")]
    pub pyro_un_points: Option<Decimal>,
    #[sqlx(rename = "pyro_transport_category")]
    pub pyro_transport_category: Option<String>,

    pub audit_created_at: Option<DateTime<Utc>>,
    pub audit_changed_at: Option<DateTime<Utc>>,
}

/// Row of `"Artikel_Lieferanten"` joined with the supplier address.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticleSupplierRow {
    pub supplier_id: i32,
    pub supplier_no: Option<i32>,
    pub name: Option<String>,
    pub position: Option<i32>,
    pub purchase_price: Option<Decimal>,
    pub delivery_days: Option<i32>,
    pub favorite: Option<bool>,
    pub sales_article: Option<bool>,
    pub supplier_article_no: Option<String>,
}

impl From<ArticleSupplierRow> for where_domain::ArticleSupplier {
    fn from(r: ArticleSupplierRow) -> Self {
        Self {
            supplier_id: r.supplier_id,
            supplier_no: r.supplier_no,
            name: r.name,
            position: r.position,
            purchase_price: r.purchase_price,
            delivery_days: r.delivery_days,
            favorite: flag(r.favorite),
            sales_article: flag(r.sales_article),
            supplier_article_no: r.supplier_article_no,
        }
    }
}

/// Row of `"Art_MandantDaten"` — prices, accounts and scales of one article.
/// WHERE works with a single client, so the lowest client row is used.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticlePriceRow {
    #[sqlx(rename = "Mietpreis")]
    pub rental_price_per_day: Option<Decimal>,
    #[sqlx(rename = "BasisMietpreis")]
    pub rental_base_price: Option<Decimal>,
    #[sqlx(rename = "Kautionsbetrag")]
    pub deposit: Option<Decimal>,
    #[sqlx(rename = "Mietpreis_Rolltage")]
    pub rental_price_per_day_roll_days: Option<Decimal>,
    #[sqlx(rename = "BasisPreis_Rolltage")]
    pub rental_base_price_roll_days: Option<Decimal>,
    #[sqlx(rename = "Mietpreis_OffDays")]
    pub rental_price_per_day_off_days: Option<Decimal>,
    #[sqlx(rename = "BasisPreis_OffDays")]
    pub rental_base_price_off_days: Option<Decimal>,
    #[sqlx(rename = "Mietpreis_WE")]
    pub rental_price_weekend: Option<Decimal>,
    #[sqlx(rename = "VKPreisNetto")]
    pub sale_price_net: Option<Decimal>,
    #[sqlx(rename = "VKPreisBrutto")]
    pub sale_price_gross: Option<Decimal>,
    #[sqlx(rename = "VKPreisEmpfehlung")]
    pub sale_retail_price: Option<Decimal>,
    #[sqlx(rename = "Kapitalbindung")]
    pub capital_binding: Option<Decimal>,
    #[sqlx(rename = "VerkaufEKNEtto")]
    pub sale_cost_price_net: Option<Decimal>,
    #[sqlx(rename = "EKNetto")]
    pub rental_cost_price: Option<Decimal>,
    #[sqlx(rename = "EKNetto_Rolltage")]
    pub rental_cost_price_roll_days: Option<Decimal>,
    #[sqlx(rename = "EKNetto_OffDays")]
    pub rental_cost_price_off_days: Option<Decimal>,
    #[sqlx(rename = "EKNetto_WE")]
    pub rental_cost_price_weekend: Option<Decimal>,
    #[sqlx(rename = "MwStMietArtikel")]
    pub rental_vat_rate: Option<i32>,
    #[sqlx(rename = "MwStVKArtikel")]
    pub sale_vat_rate: Option<i32>,
    #[sqlx(rename = "VerkaufEKzuVKAufschlag")]
    pub sale_markup_percent: Option<Decimal>,
    #[sqlx(rename = "FiBu_KtoErtragMiete")]
    pub rental_revenue_account: Option<String>,
    #[sqlx(rename = "FiBu_KtoErtragVerkauf")]
    pub sale_revenue_account: Option<String>,
    #[sqlx(rename = "FiBu_KST_Miete")]
    pub rental_cost_center: Option<String>,
    #[sqlx(rename = "FiBu_KST_Verkauf")]
    pub sale_cost_center: Option<String>,
    #[sqlx(rename = "ZRabStaffel_PK")]
    pub factor_scale: Option<i32>,
    #[sqlx(rename = "ZRabStaffelRollD_PK")]
    pub factor_scale_roll_days: Option<i32>,
    #[sqlx(rename = "ZRabStaffelOffD_PK")]
    pub factor_scale_off_days: Option<i32>,
    #[sqlx(rename = "MengenStaffel_PK")]
    pub quantity_scale: Option<i32>,
    #[sqlx(rename = "Gebrauchtpreis")]
    pub used_sale_price: Option<Decimal>,
    #[sqlx(rename = "Lagerplatz")]
    pub approximate_location: Option<String>,
}

/// Scale of days of use or quantities — `"StaffelKategorie"`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScaleCategoryRow {
    #[sqlx(rename = "Primary_Key")]
    pub id: i32,
    #[sqlx(rename = "Name")]
    pub name: Option<String>,
    #[sqlx(rename = "Verkauf")]
    pub for_sale: Option<bool>,
    #[sqlx(rename = "Standard")]
    pub is_default: Option<bool>,
}

/// One step of a scale — `"Staffelrabatt"`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScaleStepRow {
    #[sqlx(rename = "Miettage")]
    pub from_value: Option<Decimal>,
    #[sqlx(rename = "Faktor")]
    pub factor: Option<Decimal>,
    #[sqlx(rename = "Rabatt")]
    pub discount: Option<Decimal>,
}

impl From<ScaleCategoryRow> for where_domain::ScaleCategory {
    fn from(r: ScaleCategoryRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            for_sale: r.for_sale.unwrap_or(false),
            is_default: r.is_default.unwrap_or(false),
        }
    }
}

impl From<ScaleStepRow> for where_domain::ScaleStep {
    fn from(r: ScaleStepRow) -> Self {
        Self {
            from_value: r.from_value,
            factor: r.factor,
            discount: r.discount,
        }
    }
}

impl ArticlePriceRow {
    /// Fills the price part of [`where_domain::ArticlePricing`]; the flags stay untouched
    /// because they live in `"Artikel"`.
    pub fn apply(self, p: &mut where_domain::ArticlePricing) {
        p.rental_price_per_day = self.rental_price_per_day;
        p.rental_base_price = self.rental_base_price;
        p.rental_cost_price = self.rental_cost_price;
        p.rental_price_per_day_roll_days = self.rental_price_per_day_roll_days;
        p.rental_base_price_roll_days = self.rental_base_price_roll_days;
        p.rental_cost_price_roll_days = self.rental_cost_price_roll_days;
        p.rental_price_per_day_off_days = self.rental_price_per_day_off_days;
        p.rental_base_price_off_days = self.rental_base_price_off_days;
        p.rental_cost_price_off_days = self.rental_cost_price_off_days;
        p.rental_price_weekend = self.rental_price_weekend;
        p.rental_cost_price_weekend = self.rental_cost_price_weekend;
        p.deposit = self.deposit;
        p.used_sale_price = self.used_sale_price;
        p.rental_vat_rate = self.rental_vat_rate;
        p.rental_revenue_account = self.rental_revenue_account;
        p.rental_cost_center = self.rental_cost_center;
        p.capital_binding = self.capital_binding;
        p.approximate_location = self.approximate_location;
        p.factor_scale = self.factor_scale;
        p.factor_scale_roll_days = self.factor_scale_roll_days;
        p.factor_scale_off_days = self.factor_scale_off_days;
        p.sale_price_net = self.sale_price_net;
        p.sale_price_gross = self.sale_price_gross;
        p.sale_retail_price = self.sale_retail_price;
        p.sale_cost_price_net = self.sale_cost_price_net;
        p.sale_markup_percent = self.sale_markup_percent;
        p.sale_vat_rate = self.sale_vat_rate;
        p.sale_revenue_account = self.sale_revenue_account;
        p.sale_cost_center = self.sale_cost_center;
        p.quantity_scale = self.quantity_scale;
    }
}

fn flag(v: Option<bool>) -> bool {
    v.unwrap_or(false)
}

impl From<ArticleRow> for where_domain::Article {
    fn from(r: ArticleRow) -> Self {
        Self {
            id: Some(r.id),
            article_no: r.article_no,
            inventory_no: r.inventory_no,
            name: r.name.unwrap_or_default(),
            short_name: r.short_name,
            match_code: r.match_code,
            unit: r.unit,
            active: flag(r.active),
            inventory: flag(r.inventory),
            rental: flag(r.rental),
            sale: flag(r.sale),
            text_sync: flag(r.text_sync),
            stock_tracking: flag(r.stock_tracking),
            no_own_stock: flag(r.no_own_stock),
            individual_items: flag(r.individual_items),
            allocation_control: flag(r.allocation_control),
            virtual_article: flag(r.virtual_article),
            ean_code: r.ean_code,
            manufacturer_no: r.manufacturer_no,
            manufacturer: r.manufacturer,
            manufacturer_country: r.manufacturer_country,
            retail_price: r.retail_price,
            customs_tariff_export: r.customs_tariff_export,
            customs_tariff_import: r.customs_tariff_import,
            insurance_value_net: r.insurance_value_net,
            insurance_value_gross: r.insurance_value_gross,
            team_group: r.team_group,
            commission_class: r.commission_class,
            width_net: r.width_net,
            height_net: r.height_net,
            depth_net: r.depth_net,
            volume_net: r.volume_net,
            weight_net: r.weight_net,
            width_gross: r.width_gross,
            height_gross: r.height_gross,
            depth_gross: r.depth_gross,
            volume_gross: r.volume_gross,
            weight_gross: r.weight_gross,
            voltage_from: r.voltage_from,
            voltage_to: r.voltage_to,
            current_draw: r.current_draw,
            power: r.power,
            setup_minutes: r.setup_minutes,
            teardown_minutes: r.teardown_minutes,
            check_out_seconds: r.check_out_seconds,
            check_in_seconds: r.check_in_seconds,
            inspection_class: r.inspection_class,
            inspection_interval_months: r.inspection_interval_months,
            inspection_minutes: r.inspection_minutes,
            workshop_by_usage_days: flag(r.workshop_by_usage_days),
            workshop_max_usage_days: r.workshop_max_usage_days,
            workshop_fixed_intervals: flag(r.workshop_fixed_intervals),
            workshop_by_days: flag(r.workshop_by_days),
            workshop_by_months: flag(r.workshop_by_months),
            workshop_days: r.workshop_days,
            workshop_months: r.workshop_months,
            pricing: where_domain::ArticlePricing {
                rental_roll_days_active: flag(r.rental_roll_days_active),
                rental_off_days_active: flag(r.rental_off_days_active),
                rental_weekend_active: flag(r.rental_weekend_active),
                packaging_release: flag(r.packaging_release),
                rental_no_discount: flag(r.rental_no_discount),
                rental_not_insurable: flag(r.rental_not_insurable),
                material_only: flag(r.material_only),
                unique_article: flag(r.unique_article),
                checkout_requires_serial: flag(r.checkout_requires_serial),
                fixed_installation: flag(r.fixed_installation),
                inspection_on_checkin: flag(r.inspection_on_checkin),
                function_check_on_checkin: flag(r.function_check_on_checkin),
                consumption_recording: flag(r.consumption_recording),
                sale_no_discount: flag(r.sale_no_discount),
                sell_full_packages: flag(r.sell_full_packages),
                package_unit: r.package_unit,
                package_size: r.package_size,
                reorder_level: r.reorder_level,
                ..Default::default()
            },
            texts: Default::default(),
            article_groups: Vec::new(),
            suppliers: Vec::new(),
            pyro: where_domain::ArticlePyro {
                un_number: r.pyro_un_number,
                un_description: r.pyro_un_description,
                hazard_class: r.pyro_hazard_class,
                caliber: r.pyro_caliber,
                lateral_safety_distance: r.pyro_lateral_safety_distance,
                vertical_safety_distance: r.pyro_vertical_safety_distance,
                ascending_height: r.pyro_ascending_height,
                sound_pressure_level: r.pyro_sound_pressure_level,
                sound_pressure_range: r.pyro_sound_pressure_range,
                tunnel_restriction_code: r.pyro_tunnel_restriction_code,
                delay_time: r.pyro_delay_time,
                effect_duration: r.pyro_effect_duration,
                category: r.pyro_category,
                bam_id: r.pyro_bam_id,
                classification: r.pyro_classification,
                packing_group: r.pyro_packing_group,
                approval_number: r.pyro_approval_number,
                neq: r.pyro_neq,
                neq_unit: r.pyro_neq_unit,
                un_points: r.pyro_un_points,
                transport_category: r.pyro_transport_category,
            },
            created_at: r.audit_created_at,
            changed_at: r.audit_changed_at,
        }
    }
}

/// Row of the article list — deliberately slim so long lists stay cheap.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticleSummaryRow {
    #[sqlx(rename = "Primary_Key")]
    pub id: i32,
    #[sqlx(rename = "ArtikelNr")]
    pub article_no: i32,
    #[sqlx(rename = "Name")]
    pub name: Option<String>,
    #[sqlx(rename = "Hersteller")]
    pub manufacturer: Option<String>,
    #[sqlx(rename = "InventarNr")]
    pub inventory_no: Option<String>,
    #[sqlx(rename = "Aktiv")]
    pub active: Option<bool>,
    #[sqlx(rename = "ZurMiete_Flag")]
    pub rental: Option<bool>,
    #[sqlx(rename = "ZumVerkauf_Flag")]
    pub sale: Option<bool>,
    /// Sum across all warehouses; `None` = no booking yet.
    pub stock_on_hand: Option<rust_decimal::Decimal>,
    /// Stock minus reservations and withdrawals.
    pub available_stock: Option<rust_decimal::Decimal>,
    /// `"Bestand_GesamtMiete"` summed over all warehouses (column "Bestand Miete").
    pub rental_stock: Option<rust_decimal::Decimal>,
    /// `"Bestand_GesamtNVerkauf"` + `"Bestand_GesamtGVerkauf"` (column "Bestand Verkauf").
    pub sale_stock: Option<rust_decimal::Decimal>,
    /// `"Art_MandantDaten"."Mietpreis"` (column "Preis Miete").
    pub rental_price: Option<rust_decimal::Decimal>,
    /// `"Art_MandantDaten"."VKPreisNetto"` (column "Preis Verkauf").
    pub sale_price: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticleTextRow {
    #[sqlx(rename = "Beschreibung_short")]
    pub short: Option<String>,
    #[sqlx(rename = "Beschreibung_long")]
    pub long: Option<String>,
    #[sqlx(rename = "Beschreibung_tech")]
    pub technical_details: Option<String>,
    #[sqlx(rename = "Beschreibung_delivery")]
    pub scope_of_delivery: Option<String>,
    #[sqlx(rename = "Beschreibung_package")]
    pub packaging_info: Option<String>,
    #[sqlx(rename = "Beschreibung_webshop")]
    pub webshop: Option<String>,
}

impl From<ArticleTextRow> for where_domain::ArticleTexts {
    fn from(r: ArticleTextRow) -> Self {
        Self {
            short: r.short,
            long: r.long,
            technical_details: r.technical_details,
            scope_of_delivery: r.scope_of_delivery,
            packaging_info: r.packaging_info,
            webshop: r.webshop,
        }
    }
}

// ------------------------------------------------------------------ File storage

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileRow {
    pub id: i32,
    pub article_id: i32,
    pub folder: String,
    pub name: String,
    pub is_link: bool,
    pub link_target: Option<String>,
    pub mime: Option<String>,
    pub size: Option<i64>,
    pub sha256: Option<String>,
    pub version: i32,
    pub is_current: bool,
    pub uploaded_by: Option<String>,
    pub uploaded_at: Option<DateTime<Utc>>,
}

impl From<FileRow> for where_domain::ArticleFile {
    fn from(r: FileRow) -> Self {
        Self {
            id: r.id,
            article_id: r.article_id,
            folder: r.folder,
            name: r.name,
            is_link: r.is_link,
            link_target: r.link_target,
            mime: r.mime,
            size: r.size,
            sha256: r.sha256,
            version: r.version,
            is_current: r.is_current,
            uploaded_by: r.uploaded_by,
            uploaded_at: r.uploaded_at,
        }
    }
}

// ------------------------------------------------------------------ Article search

/// Hit row of the search form: the same columns as the list plus the total count
/// (as a window function so list and counter come from one query).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticleSearchRow {
    #[sqlx(rename = "Primary_Key")]
    pub id: i32,
    #[sqlx(rename = "ArtikelNr")]
    pub article_no: i32,
    #[sqlx(rename = "Name")]
    pub name: Option<String>,
    #[sqlx(rename = "Hersteller")]
    pub manufacturer: Option<String>,
    #[sqlx(rename = "InventarNr")]
    pub inventory_no: Option<String>,
    #[sqlx(rename = "Aktiv")]
    pub active: Option<bool>,
    #[sqlx(rename = "ZurMiete_Flag")]
    pub rental: Option<bool>,
    #[sqlx(rename = "ZumVerkauf_Flag")]
    pub sale: Option<bool>,
    pub stock_on_hand: Option<Decimal>,
    pub available_stock: Option<Decimal>,
    pub rental_stock: Option<Decimal>,
    pub sale_stock: Option<Decimal>,
    pub rental_price: Option<Decimal>,
    pub sale_price: Option<Decimal>,
    /// `count(*) OVER ()` — total number of hits before `LIMIT`.
    pub total: i64,
}

impl From<ArticleSearchRow> for ArticleSummaryRow {
    fn from(r: ArticleSearchRow) -> Self {
        Self {
            id: r.id,
            article_no: r.article_no,
            name: r.name,
            manufacturer: r.manufacturer,
            inventory_no: r.inventory_no,
            active: r.active,
            rental: r.rental,
            sale: r.sale,
            stock_on_hand: r.stock_on_hand,
            available_stock: r.available_stock,
            rental_stock: r.rental_stock,
            sale_stock: r.sale_stock,
            rental_price: r.rental_price,
            sale_price: r.sale_price,
        }
    }
}

/// Saved search; `criteria` is the JSON body of the search.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SavedSearch {
    pub id: i32,
    pub name: String,
    pub criteria: String,
    pub created_at: DateTime<Utc>,
    pub changed_at: DateTime<Utc>,
}

/// Node of the search index tree.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticleSearchIndexNode {
    pub id: i32,
    pub name: Option<String>,
    pub parent_id: Option<i32>,
}

// ------------------------------------------------------------------ Master data

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LanguageRow {
    #[sqlx(rename = "Primary_Key")]
    pub id: i32,
    #[sqlx(rename = "Sprache")]
    pub language: Option<String>,
    #[sqlx(rename = "Muttersprache")]
    pub native_language: Option<bool>,
    #[sqlx(rename = "ISO_639_1")]
    pub iso_639_1: Option<String>,
}
