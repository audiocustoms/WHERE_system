use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Article master record — matches the form in `img/articles_01..05.PNG`.
///
/// One struct for reading and writing: `id` and the audit fields are `None` on
/// creation. The text blocks live in `Artikel_Beschreibung` (one row per language),
/// the article groups in `Artikel_Warengruppe`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Article {
    /// `Primary_Key`; `None` when creating.
    pub id: Option<i32>,
    /// `ArtikelNr`; `0` on creation means "assign the next free number".
    pub article_no: i32,
    pub inventory_no: Option<String>,
    /// `Name`; required.
    pub name: String,
    pub short_name: Option<String>,
    pub match_code: Option<String>,
    pub unit: Option<String>,

    // ---- Category
    pub active: bool,
    pub inventory: bool,
    pub rental: bool,
    pub sale: bool,
    pub text_sync: bool,

    // ---- Stock handling
    pub stock_tracking: bool,
    pub no_own_stock: bool,
    pub individual_items: bool,
    pub allocation_control: bool,
    pub virtual_article: bool,

    // ---- Manufacturer data
    pub ean_code: Option<String>,
    pub manufacturer_no: Option<String>,
    pub manufacturer: Option<String>,
    pub manufacturer_country: Option<String>,
    /// Label of the form: "retail price (gross)".
    pub retail_price: Option<Decimal>,
    pub customs_tariff_export: Option<String>,
    pub customs_tariff_import: Option<String>,

    // ---- Further information
    pub insurance_value_net: Option<Decimal>,
    pub insurance_value_gross: Option<Decimal>,
    pub team_group: Option<String>,
    pub commission_class: Option<String>,

    // ---- Dimensions and values, net
    pub width_net: Option<Decimal>,
    pub height_net: Option<Decimal>,
    pub depth_net: Option<Decimal>,
    pub volume_net: Option<Decimal>,
    pub weight_net: Option<Decimal>,

    // ---- Dimensions and values, gross
    pub width_gross: Option<Decimal>,
    pub height_gross: Option<Decimal>,
    pub depth_gross: Option<Decimal>,
    pub volume_gross: Option<Decimal>,
    pub weight_gross: Option<Decimal>,

    // ---- Operating values
    pub voltage_from: Option<Decimal>,
    pub voltage_to: Option<Decimal>,
    pub current_draw: Option<Decimal>,
    pub power: Option<Decimal>,
    pub setup_minutes: Option<i32>,
    pub teardown_minutes: Option<i32>,
    pub check_out_seconds: Option<i32>,
    pub check_in_seconds: Option<i32>,

    // ---- Inspection (DGUV)
    pub inspection_class: Option<i32>,
    pub inspection_interval_months: Option<i32>,
    pub inspection_minutes: Option<i32>,
    pub workshop_by_usage_days: bool,
    pub workshop_max_usage_days: Option<Decimal>,
    pub workshop_fixed_intervals: bool,
    pub workshop_by_days: bool,
    pub workshop_by_months: bool,
    pub workshop_days: Option<i32>,
    pub workshop_months: Option<i32>,

    // ---- Tabs "Miete" and "Verkauf" (`img/miete.PNG`, `img/verkauf.PNG`)
    #[serde(default)]
    pub pricing: ArticlePricing,

    // ---- Text blocks (short/long/technical details/scope of delivery/packaging/webshop)
    #[serde(default)]
    pub texts: ArticleTexts,
    /// Ids from `Storage_Warengruppe`.
    #[serde(default)]
    pub article_groups: Vec<i32>,
    /// "Lieferanten" register (`"Artikel_Lieferanten"`).
    #[serde(default)]
    pub suppliers: Vec<ArticleSupplier>,
    /// Register "Pyro" (`pyro_*` of `"Artikel"`).
    #[serde(default)]
    pub pyro: ArticlePyro,

    // ---- Audit
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub changed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Pyrotechnic data of an article — the tab `Pyro` of the form and the pyro block of
/// the search mask (`pyro_*` of `"Artikel"`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArticlePyro {
    /// UN number of the substance (`"pyro_UN_number"`).
    pub un_number: Option<String>,
    /// Official description of the UN number (`"pyro_UN_description_o"`).
    pub un_description: Option<String>,
    /// Hazard class (`"pyro_hazard_class_o"`), e.g. `1.4G`.
    pub hazard_class: Option<String>,
    /// Caliber in millimetres (`"pyro_caliber"`).
    pub caliber: Option<i32>,
    /// Lateral safety distance in metres to the audience.
    pub lateral_safety_distance: Option<Decimal>,
    /// Vertical safety distance in metres.
    pub vertical_safety_distance: Option<Decimal>,
    /// Ascending height in metres (`"pyro_ascending_height"`).
    pub ascending_height: Option<Decimal>,
    /// Sound pressure level in dB.
    pub sound_pressure_level: Option<Decimal>,
    /// Sound pressure range in metres.
    pub sound_pressure_range: Option<Decimal>,
    /// Tunnel restriction code (`"pyro_tunnel_delimitation_code"`).
    pub tunnel_restriction_code: Option<String>,
    /// Delay time in seconds.
    pub delay_time: Option<Decimal>,
    /// Effect duration in seconds.
    pub effect_duration: Option<Decimal>,
    /// Category (`"pyro_category"`).
    pub category: Option<String>,
    /// BAM identification (`"pyro_bam_id"`).
    pub bam_id: Option<String>,
    /// Classification (`"pyro_categorisation"`).
    pub classification: Option<String>,
    /// Packing group (`"pyro_packing_group"`).
    pub packing_group: Option<String>,
    /// Approval number (`"pyro_approval_number"`).
    pub approval_number: Option<String>,
    /// Net explosive quantity in kg (`"pyro_neq"`).
    pub neq: Option<Decimal>,
    /// Unit of the NEQ (`"pyro_neq_einheit"`), e.g. `kg`.
    pub neq_unit: Option<String>,
    /// UN points (`"pyro_UN_points"`).
    pub un_points: Option<Decimal>,
    /// Transport category (`"pyro_transport_category"`).
    pub transport_category: Option<String>,
}

/// Supplier of an article — register "Lieferanten" (`"Artikel_Lieferanten"`).
///
/// The supplier is an address with `"Type_Lieferant"`; the name and number are read
/// from `"Adresse"` so the form can show them without a second request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArticleSupplier {
    /// `"Lieferant_Primkey"` — primary key of the address.
    pub supplier_id: i32,
    /// `"lief_LieferantenNr"` of the address.
    pub supplier_no: Option<i32>,
    /// `"Name1"` of the address.
    pub name: Option<String>,
    /// `"lfd_Pos"` — order in the list.
    pub position: Option<i32>,
    /// `"EK_Preis"` — purchase price at this supplier.
    pub purchase_price: Option<Decimal>,
    /// `"Lieferzeit_Tage"`.
    pub delivery_days: Option<i32>,
    /// `"Favorit"` — preferred supplier.
    pub favorite: bool,
    /// `"Verkaufsartikel"` — this supplier sells the article to end customers.
    pub sales_article: bool,
    /// `"ArtLiefNr"` — the supplier's own article number.
    pub supplier_article_no: Option<String>,
}

/// Rental and sale data — the tabs `Miete` and `Verkauf` of the form
/// (`img/miete.PNG`, `img/verkauf.PNG`).
///
/// The 4D form reads the prices from `"Art_MandantDaten"` (one row per article and
/// client) and the flags from `"Artikel"`. Both are kept apart here as well, but exposed
/// as one block because they always appear together in the form.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArticlePricing {
    // ---------------------------------------------------------------- Rental: calculation
    /// Price set for rolling days is in use (`"RolltagePreise_aktiv"`).
    pub rental_roll_days_active: bool,
    /// Price set for off days is in use (`"OffDayPreise_aktiv"`).
    pub rental_off_days_active: bool,
    /// Flat weekend price is in use (`"WEPreis_aktiv"`).
    pub rental_weekend_active: bool,

    // ---------------------------------------------------------------- Rental: prices
    pub rental_price_per_day: Option<Decimal>,
    pub rental_base_price: Option<Decimal>,
    pub rental_cost_price: Option<Decimal>,
    pub rental_price_per_day_roll_days: Option<Decimal>,
    pub rental_base_price_roll_days: Option<Decimal>,
    pub rental_cost_price_roll_days: Option<Decimal>,
    pub rental_price_per_day_off_days: Option<Decimal>,
    pub rental_base_price_off_days: Option<Decimal>,
    pub rental_cost_price_off_days: Option<Decimal>,
    pub rental_price_weekend: Option<Decimal>,
    pub rental_cost_price_weekend: Option<Decimal>,
    pub deposit: Option<Decimal>,
    /// Price for a used unit of this article (`"Gebrauchtpreis"`).
    pub used_sale_price: Option<Decimal>,
    /// VAT rate in percent (`"MwStMietArtikel"`).
    pub rental_vat_rate: Option<i32>,
    pub rental_revenue_account: Option<String>,
    pub rental_cost_center: Option<String>,
    pub capital_binding: Option<Decimal>,
    /// Approximate storage location shown at check-out (`"Lagerplatz"`).
    pub approximate_location: Option<String>,
    /// Factor scale of the days of use (`"ZRabStaffel_PK"` → [`ScaleCategory`]).
    pub factor_scale: Option<i32>,
    pub factor_scale_roll_days: Option<i32>,
    pub factor_scale_off_days: Option<i32>,

    // ---------------------------------------------------------------- Rental: properties
    /// May be part of a packaging unit (`"Verpackungseinheit"`).
    pub packaging_release: bool,
    /// Not discountable in rental (`"RabattMOFF"`).
    pub rental_no_discount: bool,
    /// Not insurable in rental (`"VersicherungMOFF"`).
    pub rental_not_insurable: bool,
    /// Only usable as material (`"NurMaterial"`).
    pub material_only: bool,
    /// Unique article — refuses an overbooked schedule (`"Unikatartikel"`).
    pub unique_article: bool,
    /// Check-out/check-in only with serial number (`"MieteForceSNScan"`).
    pub checkout_requires_serial: bool,
    /// Not divisible bundle (`"MieteWiredBundle"`).
    pub fixed_installation: bool,
    pub inspection_on_checkin: bool,
    pub function_check_on_checkin: bool,
    pub consumption_recording: bool,

    // ---------------------------------------------------------------- Sale: prices
    pub sale_price_net: Option<Decimal>,
    pub sale_price_gross: Option<Decimal>,
    pub sale_retail_price: Option<Decimal>,
    pub sale_cost_price_net: Option<Decimal>,
    /// Surcharge purchase → sale in percent (`"VerkaufEKzuVKAufschlag"`).
    pub sale_markup_percent: Option<Decimal>,
    /// VAT rate in percent (`"MwStVKArtikel"`).
    pub sale_vat_rate: Option<i32>,
    pub sale_revenue_account: Option<String>,
    pub sale_cost_center: Option<String>,
    /// Quantity scale (`"MengenStaffel_PK"` → [`ScaleCategory`]).
    pub quantity_scale: Option<i32>,

    // ---------------------------------------------------------------- Sale: properties
    /// Not discountable in sale (`"RabattVOFF"`).
    pub sale_no_discount: bool,
    /// Only sell multiples of the packaging unit (`"KolliVielfaches"`).
    pub sell_full_packages: bool,
    /// Packaging unit (`"KolliEinheit"`).
    pub package_unit: Option<String>,
    /// Size of one package (`"KolliGroesse"`).
    pub package_size: Option<Decimal>,
    /// Reorder level (`"VerkaufMindBestand"`).
    pub reorder_level: Option<Decimal>,
}

impl ArticlePricing {
    /// `true` when the block still carries no price and no flag.
    ///
    /// The core without `WHERE - does my stuff go?` uses this to tell an
    /// untouched pricing block from one the client tried to fill.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Scale of days of use (rental) or quantities (sale) — `"StaffelKategorie"`.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScaleCategory {
    pub id: i32,
    pub name: Option<String>,
    /// `true` = quantity scale of the sale, `false` = factor scale of the rental.
    pub for_sale: bool,
    /// Marked as the standard scale of its kind.
    pub is_default: bool,
}

/// One step of a [`ScaleCategory`] — `"Staffelrabatt"`.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScaleStep {
    /// Days of use (rental) or quantity from which the step applies (sale).
    pub from_value: Option<Decimal>,
    /// Factor for the days of use (rental).
    pub factor: Option<Decimal>,
    /// Discount in percent (sale).
    pub discount: Option<Decimal>,
}

/// The six text blocks of the form, one row per language in `Artikel_Beschreibung`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArticleTexts {
    pub short: Option<String>,
    pub long: Option<String>,
    pub technical_details: Option<String>,
    pub scope_of_delivery: Option<String>,
    pub packaging_info: Option<String>,
    pub webshop: Option<String>,
}

impl Article {
    pub fn validate(&self) -> Result<(), crate::DomainError> {
        if self.name.trim().is_empty() {
            return Err(crate::DomainError::Validation(
                "Name is a required field".into(),
            ));
        }
        if self.article_no < 0 {
            return Err(crate::DomainError::Validation(
                "Article number must not be negative".into(),
            ));
        }
        for (label, value) in [
            ("Retail price (gross)", self.retail_price),
            ("Insurance value net", self.insurance_value_net),
            ("Insurance value gross", self.insurance_value_gross),
            ("Width net", self.width_net),
            ("Height net", self.height_net),
            ("Depth net", self.depth_net),
            ("Weight net", self.weight_net),
            ("Volume net", self.volume_net),
            ("Width gross", self.width_gross),
            ("Height gross", self.height_gross),
            ("Depth gross", self.depth_gross),
            ("Weight gross", self.weight_gross),
            ("Volume gross", self.volume_gross),
            ("Rental price per day", self.pricing.rental_price_per_day),
            ("Rental base price", self.pricing.rental_base_price),
            ("Rental cost price", self.pricing.rental_cost_price),
            ("Deposit", self.pricing.deposit),
            ("Used sale price", self.pricing.used_sale_price),
            ("Capital binding", self.pricing.capital_binding),
            ("Sale price net", self.pricing.sale_price_net),
            ("Sale price gross", self.pricing.sale_price_gross),
            ("Sale retail price", self.pricing.sale_retail_price),
            ("Sale cost price net", self.pricing.sale_cost_price_net),
            ("Package size", self.pricing.package_size),
            ("Reorder level", self.pricing.reorder_level),
        ] {
            if let Some(v) = value
                && v.is_sign_negative()
            {
                return Err(crate::DomainError::Validation(format!(
                    "{label} must not be negative"
                )));
            }
        }
        if self.rental && !self.stock_tracking && !self.no_own_stock && !self.virtual_article {
            return Err(crate::DomainError::Validation(
                "A rental article needs stock tracking, 'no own stock' or 'virtual article'".into(),
            ));
        }
        for (label, rate) in [
            ("Rental VAT rate", self.pricing.rental_vat_rate),
            ("Sale VAT rate", self.pricing.sale_vat_rate),
        ] {
            if let Some(v) = rate
                && !(0..=100).contains(&v)
            {
                return Err(crate::DomainError::Validation(format!(
                    "{label} must be between 0 and 100"
                )));
            }
        }
        for (label, account) in [
            (
                "Rental revenue account",
                &self.pricing.rental_revenue_account,
            ),
            ("Sale revenue account", &self.pricing.sale_revenue_account),
            ("Rental cost center", &self.pricing.rental_cost_center),
            ("Sale cost center", &self.pricing.sale_cost_center),
        ] {
            if let Some(text) = account
                && text.chars().count() > 10
            {
                return Err(crate::DomainError::Validation(format!(
                    "{label} must not be longer than 10 characters"
                )));
            }
        }
        // Register "Lieferanten": one entry per supplier, ids of existing addresses.
        let mut seen = std::collections::HashSet::new();
        for s in &self.suppliers {
            if s.supplier_id <= 0 {
                return Err(crate::DomainError::Validation(
                    "A supplier needs the id of an address".into(),
                ));
            }
            if !seen.insert(s.supplier_id) {
                return Err(crate::DomainError::Validation(
                    "A supplier may only be listed once".into(),
                ));
            }
        }
        // Register "Pyro": distances and quantities must not be negative.
        for (label, value) in [
            ("Lateral safety distance", self.pyro.lateral_safety_distance),
            (
                "Vertical safety distance",
                self.pyro.vertical_safety_distance,
            ),
            ("Ascending height", self.pyro.ascending_height),
            ("Sound pressure level", self.pyro.sound_pressure_level),
            ("Sound pressure range", self.pyro.sound_pressure_range),
            ("Delay time", self.pyro.delay_time),
            ("Effect duration", self.pyro.effect_duration),
            ("Net explosive quantity", self.pyro.neq),
            ("UN points", self.pyro.un_points),
        ] {
            if let Some(v) = value
                && v.is_sign_negative()
            {
                return Err(crate::DomainError::Validation(format!(
                    "{label} must not be negative"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article() -> Article {
        Article {
            name: "Scheinwerfer".into(),
            stock_tracking: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_plain_article_is_valid() {
        assert!(article().validate().is_ok());
    }

    #[test]
    fn rental_prices_must_not_be_negative() {
        let mut a = article();
        a.pricing.rental_price_per_day = Some(Decimal::from(-1));
        assert!(a.validate().is_err());

        a.pricing.rental_price_per_day = Some(Decimal::ZERO);
        assert!(a.validate().is_ok());
    }

    #[test]
    fn sale_prices_must_not_be_negative() {
        let mut a = article();
        a.pricing.sale_price_net = Some(Decimal::from(-5));
        let error = a.validate().unwrap_err().to_string();
        assert!(error.contains("Sale price net"), "{error}");
    }

    #[test]
    fn vat_rate_stays_between_0_and_100() {
        let mut a = article();
        a.pricing.rental_vat_rate = Some(101);
        assert!(a.validate().is_err());

        a.pricing.rental_vat_rate = Some(0);
        a.pricing.sale_vat_rate = Some(7);
        assert!(a.validate().is_ok());
    }

    #[test]
    fn accounts_are_limited_to_the_column_width() {
        let mut a = article();
        a.pricing.sale_cost_center = Some("KST-VERKAUF".into());
        let error = a.validate().unwrap_err().to_string();
        assert!(error.contains("Sale cost center"), "{error}");

        a.pricing.sale_cost_center = Some("KST-VK".into());
        assert!(a.validate().is_ok());
    }

    #[test]
    fn rental_needs_a_stock_model() {
        let mut a = article();
        a.rental = true;
        a.stock_tracking = false;
        assert!(a.validate().is_err());

        a.virtual_article = true;
        assert!(a.validate().is_ok());
    }
}
