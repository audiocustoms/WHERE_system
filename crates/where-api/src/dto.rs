//! API data transfer objects.
//!
//! Addresses and articles have their own wire types so the API can evolve
//! independently of the domain model. For the article master the form is so close
//! to the domain object that `where_domain::Article` is transferred directly — it
//! carries its own OpenAPI derivation (`utoipa::ToSchema`).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use where_domain::{Address, AddressInput, DomainError};
use where_persistence::rows::ArticleSummaryRow;
use where_persistence::{
    AddressList, AddressSearch, AddressSort, AddressState, AddressType, TextMatch,
};
use where_persistence::{ArticleSearch, ArticleType, PyroSearch, SetOperator};

/// Paging and free-text search for all list endpoints.
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListParams {
    /// Page size, 1..=500 (default 50).
    pub limit: Option<i64>,
    /// Number of rows to skip (default 0).
    pub offset: Option<i64>,
    /// Search across the resource search fields.
    pub q: Option<String>,
}

impl ListParams {
    pub fn window(&self) -> (i64, i64) {
        where_domain::PageRequest {
            limit: self.limit,
            offset: self.offset,
        }
        .normalized()
    }

    pub fn search(&self) -> Option<&str> {
        self.q.as_deref().filter(|s| !s.trim().is_empty())
    }
}

/// Paging, quick search, active filter and sorting of the address list.
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AddressListParams {
    /// Page size, 1..=500 (default 50).
    pub limit: Option<i64>,
    /// Number of rows to skip (default 0).
    pub offset: Option<i64>,
    /// Search across the name lines and the customer number.
    pub q: Option<String>,
    /// `active` (default), `inactive` or `all`.
    pub active: Option<String>,
    /// Sort column: `type`, `number`, `name` (default), `country`, `postal_code`, `city`.
    pub sort: Option<String>,
    /// Sort direction: `asc` (default) or `desc`.
    pub dir: Option<String>,
}

impl AddressListParams {
    /// The wire query as the repository request; unknown values are a 422.
    pub fn into_list(&self) -> Result<AddressList, DomainError> {
        let (limit, offset) = where_domain::PageRequest {
            limit: self.limit,
            offset: self.offset,
        }
        .normalized();

        let state = match self.active.as_deref().map(str::trim) {
            None | Some("") => AddressState::default(),
            Some(key) => AddressState::from_key(key).ok_or_else(|| {
                DomainError::Validation(format!(
                    "unknown active filter '{key}' (active, inactive, all)"
                ))
            })?,
        };
        let sort = match self.sort.as_deref().map(str::trim) {
            None | Some("") => AddressSort::default(),
            Some(key) => AddressSort::from_key(key).ok_or_else(|| {
                DomainError::Validation(format!(
                    "unknown sort column '{key}' (type, number, name, country, postal_code, city)"
                ))
            })?,
        };
        let descending = match self.dir.as_deref().map(str::trim) {
            None | Some("") | Some("asc") => false,
            Some("desc") => true,
            Some(other) => {
                return Err(DomainError::Validation(format!(
                    "unknown sort direction '{other}' (asc, desc)"
                )));
            }
        };

        Ok(AddressList {
            limit,
            offset,
            search: self
                .q
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            state,
            sort,
            descending,
        })
    }
}

// ---------------------------------------------------------------- Address

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddressDto {
    pub id: i32,
    pub customer_no: Option<i32>,
    pub title: Option<String>,
    pub name1: Option<String>,
    pub name2: Option<String>,
    pub name3: Option<String>,
    pub salutation: Option<String>,
    pub street: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub vat_id: Option<String>,
    pub debtor: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    /// Free-text address group.
    pub group: Option<String>,
    pub is_customer: bool,
    pub is_supplier: bool,
    pub auftragssperre: bool,
    pub mahnsperre: bool,

    pub active: bool,
    pub is_person: bool,
    pub birthday: Option<chrono::NaiveDate>,
    pub address_addition: Option<String>,
    pub postal_code: Option<String>,
    pub state: Option<String>,
    pub fax: Option<String>,
    pub mobile: Option<String>,
    pub website: Option<String>,
    pub is_interested: bool,
    pub is_location: bool,
    pub is_employee: bool,
    pub is_vip: bool,

    pub eori_no: Option<String>,
    pub tax_number: Option<String>,
    pub register_entry: Option<String>,
    pub register_seat: Option<String>,
    pub credit_limit: Option<Decimal>,
    pub discount_rental: Option<Decimal>,
    pub discount_sale: Option<Decimal>,
    pub bank: Option<String>,
    pub bank_account: Option<String>,
    pub bank_code: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,

    pub supplier_no: Option<i32>,
    pub creditor: Option<String>,
    pub supplier_revenue: Option<Decimal>,
    pub supplier_discount_rental: Option<Decimal>,
    pub supplier_discount_purchase: Option<Decimal>,
    pub supplier_bank: Option<String>,
    pub supplier_bank_account: Option<String>,
    pub supplier_bank_code: Option<String>,
    pub supplier_iban: Option<String>,
    pub supplier_bic: Option<String>,
}

impl From<Address> for AddressDto {
    fn from(a: Address) -> Self {
        Self {
            id: a.id,
            customer_no: a.customer_no,
            title: a.title,
            name1: a.name1,
            name2: a.name2,
            name3: a.name3,
            salutation: a.salutation,
            street: a.street,
            country: a.country,
            city: a.city,
            vat_id: a.vat_id,
            debtor: a.debtor,
            phone: a.phone,
            email: a.email,
            group: a.group,
            is_customer: a.is_customer,
            is_supplier: a.is_supplier,
            auftragssperre: a.auftragssperre,
            mahnsperre: a.mahnsperre,
            active: a.active,
            is_person: a.is_person,
            birthday: a.birthday,
            address_addition: a.address_addition,
            postal_code: a.postal_code,
            state: a.state,
            fax: a.fax,
            mobile: a.mobile,
            website: a.website,
            is_interested: a.is_interested,
            is_location: a.is_location,
            is_employee: a.is_employee,
            is_vip: a.is_vip,
            eori_no: a.eori_no,
            tax_number: a.tax_number,
            register_entry: a.register_entry,
            register_seat: a.register_seat,
            credit_limit: a.credit_limit,
            discount_rental: a.discount_rental,
            discount_sale: a.discount_sale,
            bank: a.bank,
            bank_account: a.bank_account,
            bank_code: a.bank_code,
            iban: a.iban,
            bic: a.bic,
            supplier_no: a.supplier_no,
            creditor: a.creditor,
            supplier_revenue: a.supplier_revenue,
            supplier_discount_rental: a.supplier_discount_rental,
            supplier_discount_purchase: a.supplier_discount_purchase,
            supplier_bank: a.supplier_bank,
            supplier_bank_account: a.supplier_bank_account,
            supplier_bank_code: a.supplier_bank_code,
            supplier_iban: a.supplier_iban,
            supplier_bic: a.supplier_bic,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct AddressInputDto {
    pub customer_no: Option<i32>,
    pub title: Option<String>,
    pub name1: Option<String>,
    pub name2: Option<String>,
    pub name3: Option<String>,
    pub salutation: Option<String>,
    pub street: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub vat_id: Option<String>,
    pub debtor: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub group: Option<String>,
    pub is_customer: bool,
    pub is_supplier: bool,
    pub auftragssperre: bool,
    pub mahnsperre: bool,

    pub active: Option<bool>,
    pub is_person: bool,
    pub birthday: Option<chrono::NaiveDate>,
    pub address_addition: Option<String>,
    pub postal_code: Option<String>,
    pub state: Option<String>,
    pub fax: Option<String>,
    pub mobile: Option<String>,
    pub website: Option<String>,
    pub is_interested: bool,
    pub is_location: bool,
    pub is_employee: bool,
    pub is_vip: bool,

    pub eori_no: Option<String>,
    pub tax_number: Option<String>,
    pub register_entry: Option<String>,
    pub register_seat: Option<String>,
    pub credit_limit: Option<Decimal>,
    pub discount_rental: Option<Decimal>,
    pub discount_sale: Option<Decimal>,
    pub bank: Option<String>,
    pub bank_account: Option<String>,
    pub bank_code: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,

    pub supplier_no: Option<i32>,
    pub creditor: Option<String>,
    pub supplier_revenue: Option<Decimal>,
    pub supplier_discount_rental: Option<Decimal>,
    pub supplier_discount_purchase: Option<Decimal>,
    pub supplier_bank: Option<String>,
    pub supplier_bank_account: Option<String>,
    pub supplier_bank_code: Option<String>,
    pub supplier_iban: Option<String>,
    pub supplier_bic: Option<String>,
}

impl From<AddressInputDto> for AddressInput {
    fn from(d: AddressInputDto) -> Self {
        // `active` is optional on the wire so older clients keep working; the
        // domain default is "active".
        let mut input = AddressInput {
            customer_no: d.customer_no,
            title: d.title,
            name1: d.name1,
            name2: d.name2,
            name3: d.name3,
            salutation: d.salutation,
            street: d.street,
            country: d.country,
            city: d.city,
            vat_id: d.vat_id,
            debtor: d.debtor,
            phone: d.phone,
            email: d.email,
            group: d.group,
            is_customer: d.is_customer,
            is_supplier: d.is_supplier,
            auftragssperre: d.auftragssperre,
            mahnsperre: d.mahnsperre,
            is_person: d.is_person,
            birthday: d.birthday,
            address_addition: d.address_addition,
            postal_code: d.postal_code,
            state: d.state,
            fax: d.fax,
            mobile: d.mobile,
            website: d.website,
            is_interested: d.is_interested,
            is_location: d.is_location,
            is_employee: d.is_employee,
            is_vip: d.is_vip,
            eori_no: d.eori_no,
            tax_number: d.tax_number,
            register_entry: d.register_entry,
            register_seat: d.register_seat,
            credit_limit: d.credit_limit,
            discount_rental: d.discount_rental,
            discount_sale: d.discount_sale,
            bank: d.bank,
            bank_account: d.bank_account,
            bank_code: d.bank_code,
            iban: d.iban,
            bic: d.bic,
            supplier_no: d.supplier_no,
            creditor: d.creditor,
            supplier_revenue: d.supplier_revenue,
            supplier_discount_rental: d.supplier_discount_rental,
            supplier_discount_purchase: d.supplier_discount_purchase,
            supplier_bank: d.supplier_bank,
            supplier_bank_account: d.supplier_bank_account,
            supplier_bank_code: d.supplier_bank_code,
            supplier_iban: d.supplier_iban,
            supplier_bic: d.supplier_bic,
            ..AddressInput::default()
        };
        input.active = d.active.unwrap_or(true);
        input
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AddressPage {
    pub items: Vec<AddressDto>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Criteria of the address search form (`img/suche/Adresssuche.PNG`).
///
/// All fields are optional; the criteria that are set are combined with AND.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct AddressSearchInput {
    /// "Typ": `all` (default), `customers` or `suppliers`.
    pub address_type: Option<String>,
    /// "Name" — matches the three name lines and, with `include_contacts`, the
    /// contact persons.
    pub name: Option<String>,
    /// Matching mode of the "Suchtext enthält…" control: `contains` (default),
    /// `starts_with` or `equals`.
    pub name_match: Option<String>,
    /// "inklusive Ansprechpartner".
    pub include_contacts: bool,
    /// "Adressgruppe"
    pub group: Option<String>,
    /// "Knd. / Debitor-Nr." — the customer number; 0 or empty means "any".
    pub customer_no: Option<i32>,
    /// "Knd. / Debitor-Nr." — the debtor number as text.
    pub debtor_no: Option<String>,
    /// "Anschrift" — street, postal code, city and country.
    pub address_text: Option<String>,
    /// "Telefon"
    pub phone: Option<String>,
    /// "eMail"
    pub email: Option<String>,
    /// "Suchindex" — values from `Adressindex`.`Adressindex`.
    pub search_index: Vec<String>,
    /// How the search index values are combined: `contains`, `contains_any`,
    /// `contains_all`, `contains_none`.
    pub search_index_op: Option<String>,
    /// Second "inklusive Ansprechpartner": index entries of a contact person
    /// also count.
    pub include_contact_index: bool,
    /// Page size, 1..=500 (default 50).
    pub limit: Option<i64>,
    /// Number of rows to skip (default 0).
    pub offset: Option<i64>,
}

impl AddressSearchInput {
    /// Builds the criteria; unknown keys are rejected with 422.
    pub fn criteria(&self) -> Result<AddressSearch, DomainError> {
        let address_type = match self.address_type.as_deref().map(str::trim) {
            None | Some("") => AddressType::default(),
            Some(value) => AddressType::from_key(value).ok_or_else(|| {
                DomainError::Validation(format!(
                    "Address type '{value}' is unknown (all, customers, suppliers)"
                ))
            })?,
        };
        let name_match = match self.name_match.as_deref().map(str::trim) {
            None | Some("") => TextMatch::default(),
            Some(value) => TextMatch::from_key(value).ok_or_else(|| {
                DomainError::Validation(format!(
                    "Text match '{value}' is unknown (contains, starts_with, equals)"
                ))
            })?,
        };
        let search_index_op = match self.search_index_op.as_deref().map(str::trim) {
            None | Some("") => SetOperator::default(),
            Some(value) => SetOperator::from_key(value).ok_or_else(|| {
                DomainError::Validation(format!(
                    "Operator '{value}' for the search index is unknown \
                     (contains, contains_any, contains_all, contains_none)"
                ))
            })?,
        };
        Ok(AddressSearch {
            address_type,
            name: self.name.clone(),
            name_match,
            include_contacts: self.include_contacts,
            group: self.group.clone(),
            customer_no: self.customer_no,
            debtor_no: self.debtor_no.clone(),
            address_text: self.address_text.clone(),
            phone: self.phone.clone(),
            email: self.email.clone(),
            search_index: self.search_index.clone(),
            search_index_op,
            include_contact_index: self.include_contact_index,
        }
        .normalized())
    }

    pub fn window(&self) -> (i64, i64) {
        where_domain::PageRequest {
            limit: self.limit,
            offset: self.offset,
        }
        .normalized()
    }
}

// ---------------------------------------------------------------- Article

/// List row of the article overview.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ArticleListDto {
    pub id: i32,
    pub article_no: i32,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub inventory_no: Option<String>,
    pub active: bool,
    pub rental: bool,
    pub sale: bool,
    /// Sum across all stock rows; `null` = no booking yet.
    pub stock_on_hand: Option<String>,
    /// Stock minus reservations and withdrawals.
    pub available_stock: Option<String>,
    /// Rental stock (`"Bestand_GesamtMiete"`), column "Bestand Miete".
    pub rental_stock: Option<String>,
    /// Sale stock (new + used), column "Bestand Verkauf".
    pub sale_stock: Option<String>,
    /// `"Mietpreis"`, column "Preis Miete".
    pub rental_price: Option<String>,
    /// `"VKPreisNetto"`, column "Preis Verkauf".
    pub sale_price: Option<String>,
}

impl From<ArticleSummaryRow> for ArticleListDto {
    fn from(a: ArticleSummaryRow) -> Self {
        Self {
            id: a.id,
            article_no: a.article_no,
            name: a.name,
            manufacturer: a.manufacturer,
            inventory_no: a.inventory_no,
            active: a.active.unwrap_or(false),
            rental: a.rental.unwrap_or(false),
            sale: a.sale.unwrap_or(false),
            stock_on_hand: a.stock_on_hand.map(|d| d.to_string()),
            available_stock: a.available_stock.map(|d| d.to_string()),
            rental_stock: a.rental_stock.map(|d| d.to_string()),
            sale_stock: a.sale_stock.map(|d| d.to_string()),
            rental_price: a.rental_price.map(|d| d.to_string()),
            sale_price: a.sale_price.map(|d| d.to_string()),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArticlePage {
    pub items: Vec<ArticleListDto>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Paging, search and group filter of the article list (`img/Artikelliste.PNG`).
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ArticleListParams {
    /// Page size, 1..=500 (default 50).
    pub limit: Option<i64>,
    /// Number of rows to skip (default 0).
    pub offset: Option<i64>,
    /// Search across name, short name, match code, manufacturer, inventory no., EAN and number.
    pub q: Option<String>,
    /// Article group of the left menu; includes its subgroups. Empty = all articles,
    /// `0` = only articles that stand in no group at all.
    pub group: Option<i32>,
    /// `active` = only active articles, `inactive` = only inactive, empty/`all` = both.
    pub active: Option<String>,
}

impl ArticleListParams {
    pub fn window(&self) -> (i64, i64) {
        where_domain::PageRequest {
            limit: self.limit,
            offset: self.offset,
        }
        .normalized()
    }

    pub fn search(&self) -> Option<&str> {
        self.q.as_deref().filter(|s| !s.trim().is_empty())
    }

    /// `0` means "articles without any group"; negative values mean "all articles".
    pub fn group(&self) -> Option<i32> {
        self.group.filter(|id| *id >= 0)
    }

    /// `None` = all articles; an unknown value is a 422.
    pub fn active(&self) -> Result<Option<bool>, DomainError> {
        match self.active.as_deref().map(str::trim) {
            None | Some("") | Some("all") => Ok(None),
            Some("active") => Ok(Some(true)),
            Some("inactive") => Ok(Some(false)),
            Some(other) => Err(DomainError::Validation(format!(
                "unknown active filter '{other}' (active, inactive, all)"
            ))),
        }
    }
}

// ---------------------------------------------------------------- Article search

/// Pyro tab of the search form: texts and numeric ranges.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct PyroSearchInput {
    /// UN number
    pub un_number: Option<String>,
    /// Hazard class
    pub hazard_class: Option<String>,
    /// CE number (legacy column `pyro_bam_id`)
    pub ce_nr: Option<String>,
    /// Klassifizierung
    pub classification: Option<String>,
    /// Kategorie
    pub category: Option<String>,
    /// Approval number
    pub approval_number: Option<String>,
    /// Packing group
    pub packing_group: Option<String>,
    /// Tunnel restriction code
    pub tunnel_code: Option<String>,
    /// Transport category
    pub transport_category: Option<String>,
    /// Hazardous substance mass (NEQ) from/to
    pub neq_from: Option<Decimal>,
    pub neq_to: Option<Decimal>,
    /// UN points factor (exact)
    pub un_points: Option<Decimal>,
    /// Lateral safety distance from/to
    pub lateral_distance_from: Option<Decimal>,
    pub lateral_distance_to: Option<Decimal>,
    /// Vertical safety distance from/to
    pub vertical_distance_from: Option<Decimal>,
    pub vertical_distance_to: Option<Decimal>,
    /// Ascending height from/to
    pub ascending_height_from: Option<Decimal>,
    pub ascending_height_to: Option<Decimal>,
    /// Sound pressure level from/to
    pub sound_pressure_level_from: Option<Decimal>,
    pub sound_pressure_level_to: Option<Decimal>,
    /// Sound pressure range from/to
    pub sound_pressure_range_from: Option<Decimal>,
    pub sound_pressure_range_to: Option<Decimal>,
    /// Delay time from/to
    pub delay_time_from: Option<Decimal>,
    pub delay_time_to: Option<Decimal>,
    /// Effect duration from/to
    pub effect_duration_from: Option<Decimal>,
    pub effect_duration_to: Option<Decimal>,
}

impl TryFrom<PyroSearchInput> for PyroSearch {
    type Error = DomainError;

    fn try_from(p: PyroSearchInput) -> Result<Self, Self::Error> {
        let ranges: [(&str, Option<Decimal>, Option<Decimal>); 8] = [
            ("Hazardous substance mass (NEQ)", p.neq_from, p.neq_to),
            (
                "Lateral safety distance",
                p.lateral_distance_from,
                p.lateral_distance_to,
            ),
            (
                "Vertical safety distance",
                p.vertical_distance_from,
                p.vertical_distance_to,
            ),
            (
                "Ascending height",
                p.ascending_height_from,
                p.ascending_height_to,
            ),
            (
                "Sound pressure level",
                p.sound_pressure_level_from,
                p.sound_pressure_level_to,
            ),
            (
                "Sound pressure range",
                p.sound_pressure_range_from,
                p.sound_pressure_range_to,
            ),
            ("Delay time", p.delay_time_from, p.delay_time_to),
            (
                "Effect duration",
                p.effect_duration_from,
                p.effect_duration_to,
            ),
        ];
        for (field, from, to) in ranges {
            if let (Some(from), Some(to)) = (from, to)
                && from > to
            {
                return Err(DomainError::Validation(format!(
                    "{field}: \"from\" ({from}) is above \"to\" ({to})"
                )));
            }
        }
        Ok(Self {
            un_number: p.un_number,
            hazard_class: p.hazard_class,
            ce_nr: p.ce_nr,
            classification: p.classification,
            category: p.category,
            approval_number: p.approval_number,
            packing_group: p.packing_group,
            tunnel_code: p.tunnel_code,
            transport_category: p.transport_category,
            neq_from: p.neq_from,
            neq_to: p.neq_to,
            un_points: p.un_points,
            lateral_distance_from: p.lateral_distance_from,
            lateral_distance_to: p.lateral_distance_to,
            vertical_distance_from: p.vertical_distance_from,
            vertical_distance_to: p.vertical_distance_to,
            ascending_height_from: p.ascending_height_from,
            ascending_height_to: p.ascending_height_to,
            sound_pressure_level_from: p.sound_pressure_level_from,
            sound_pressure_level_to: p.sound_pressure_level_to,
            sound_pressure_range_from: p.sound_pressure_range_from,
            sound_pressure_range_to: p.sound_pressure_range_to,
            delay_time_from: p.delay_time_from,
            delay_time_to: p.delay_time_to,
            effect_duration_from: p.effect_duration_from,
            effect_duration_to: p.effect_duration_to,
        })
    }
}

/// Criteria of the article search form (`img/article-search.PNG`).
///
/// All fields are optional; the criteria that are set are combined with AND.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct ArticleSearchInput {
    /// "Article type": `active` (default), `inactive` or `all`.
    pub article_type: Option<String>,
    /// "Name" — matches name, short name and match code.
    pub name: Option<String>,
    /// "include article descriptions".
    pub include_descriptions: bool,
    /// "Article no." — 0 or empty means "any".
    pub article_no: Option<i32>,
    /// "Inventory no."
    pub inventory_no: Option<String>,
    /// "Serial no." — articles that have a device with this serial number.
    pub serial_no: Option<String>,
    /// "Bundle material: also show related items".
    pub with_bundle: bool,
    /// "Article group" — ids from `Storage_Warengruppe`.
    pub article_groups: Vec<i32>,
    /// How the article groups are combined: `contains`, `contains_any`, `contains_all`, `contains_none`.
    pub article_groups_op: Option<String>,
    /// "Search index" — values from `Artikel_Searchindex`.`Artikelindex`.
    pub search_index: Vec<String>,
    /// How the search index values are combined (same operators as `article_groups_op`).
    pub search_index_op: Option<String>,
    /// "Supplier" — name, customer number or `Lieferant_o`; several values via "+".
    pub suppliers: Vec<String>,
    /// "Manufacturer"
    pub manufacturer: Option<String>,
    /// "Owner" — via the devices of the article.
    pub owner: Option<String>,
    /// "exclude virtual articles".
    pub exclude_virtual: bool,
    /// "Pyro" register.
    pub pyro: Option<PyroSearchInput>,
    /// Page size, 1..=500 (default 50).
    pub limit: Option<i64>,
    /// Number of rows to skip (default 0).
    pub offset: Option<i64>,
}

impl ArticleSearchInput {
    /// Builds the criteria; unknown keys and inverted ranges are rejected with 422.
    pub fn criteria(&self) -> Result<ArticleSearch, DomainError> {
        let article_type = match &self.article_type {
            None => ArticleType::default(),
            Some(value) => ArticleType::from_key(value.trim()).ok_or_else(|| {
                DomainError::Validation(format!(
                    "Article type '{value}' is unknown (active, inactive, all)"
                ))
            })?,
        };
        let op = |value: &Option<String>, field: &str| -> Result<SetOperator, DomainError> {
            match value.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                None => Ok(SetOperator::default()),
                Some(value) => SetOperator::from_key(value).ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Operator '{value}' for {field} is unknown \
                         (contains, contains_any, contains_all, contains_none)"
                    ))
                }),
            }
        };
        let pyro = match self.pyro.clone() {
            None => PyroSearch::default(),
            Some(p) => PyroSearch::try_from(p)?,
        };
        Ok(ArticleSearch {
            article_type,
            name: self.name.clone(),
            include_descriptions: self.include_descriptions,
            article_no: self.article_no,
            inventory_no: self.inventory_no.clone(),
            serial_no: self.serial_no.clone(),
            with_bundle: self.with_bundle,
            article_groups: self.article_groups.clone(),
            article_groups_op: op(&self.article_groups_op, "Warengruppe")?,
            search_index: self.search_index.clone(),
            search_index_op: op(&self.search_index_op, "search index")?,
            suppliers: self.suppliers.clone(),
            manufacturer: self.manufacturer.clone(),
            owner: self.owner.clone(),
            exclude_virtual: self.exclude_virtual,
            pyro,
        }
        .normalisiert())
    }

    pub fn window(&self) -> (i64, i64) {
        where_domain::PageRequest {
            limit: self.limit,
            offset: self.offset,
        }
        .normalized()
    }
}

/// A saved search (the "process" selector of the form).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SavedSearchDto {
    pub id: i32,
    pub name: String,
    /// Exactly the body that `POST /api/v1/articles/search` expects.
    pub criteria: serde_json::Value,
    pub created_at: String,
    pub changed_at: String,
}

/// Body for saving a search under a name.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchSaveInput {
    pub name: String,
    pub criteria: ArticleSearchInput,
}

/// Node of the search index tree (the "select index" dialog).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchIndexNodeDto {
    pub id: i32,
    pub name: Option<String>,
    pub parent_id: Option<i32>,
}

/// Stock of an article in one warehouse (upper grid of the form).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StockRowDto {
    pub stock_id: i32,
    pub stock_name: String,
    pub is_default: bool,
    /// `null` = no booking exists for this warehouse yet.
    pub total: Option<String>,
    pub rental: Option<String>,
    pub sale: Option<String>,
    pub inventory: Option<String>,
    pub used_sale: Option<String>,
    pub reserved: Option<String>,
    pub withdrawal: Option<String>,
    /// Total minus reservations and withdrawals.
    pub available: Option<String>,
}

/// One device or batch (lower grid, right side = device data).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeviceDto {
    /// "Device id" of the form.
    pub id: i32,
    pub serial_no: Option<String>,
    pub inventory_no: Option<String>,
    pub inventory_no2: Option<String>,
    pub quantity: Option<String>,
    /// Usage areas — several at the same time are allowed.
    pub inventory: bool,
    pub rental: bool,
    pub sale: bool,
    pub used_sale: bool,
    /// e.g. "inventory, rental".
    pub usage_text: String,
    pub stock_id: Option<i32>,
    pub stock_name: Option<String>,
    pub bin_location: Option<String>,
    pub condition: Option<String>,
    pub discarded: bool,
    /// Status column of the form: "in stock" or "discarded".
    pub status: String,
    pub purchase_price: Option<String>,
    pub purchase_date: Option<String>,
    pub supplier_no: Option<i32>,
    pub owner: Option<String>,
    pub warranty_months: Option<i32>,
    pub ext_barcode: Option<String>,
    pub year_built: Option<i32>,
    pub note: Option<String>,
    pub without_serial: bool,
    pub recorded_at: Option<String>,
}

/// Content of the stock tab: warehouses on top, devices/batches below.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StockLevelDto {
    pub stock: Vec<StockRowDto>,
    pub devices: Vec<DeviceDto>,
}

/// Result of a goods receipt.
///
/// With serial numbers one device per unit is created, each with its own serial
/// number; a batch without serial numbers is one row whose `quantity` is the amount
/// booked in.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoodsReceiptResponse {
    /// The devices this receipt created, in the order of their serial numbers.
    pub devices: Vec<DeviceDto>,
    /// Quantity booked in — the sum over all created rows.
    pub quantity: String,
}

/// Input for the goods receipt (`img/goods-receipt.PNG`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GoodsReceiptInput {
    /// Omit or leave empty when `without_serial` is set.
    #[serde(default)]
    pub serial_no: Option<String>,
    #[serde(default)]
    pub without_serial: bool,
    pub stock_id: i32,
    #[serde(default)]
    pub bin_location: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub purchase_price: Option<rust_decimal::Decimal>,
    /// Date `YYYY-MM-DD`.
    #[serde(default)]
    pub purchase_date: Option<String>,
    #[serde(default)]
    pub supplier_no: Option<i32>,
    #[serde(default)]
    pub inventory_no: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    /// Usage areas — at least one, several at the same time are allowed.
    #[serde(default)]
    pub inventory: bool,
    #[serde(default)]
    pub rental: bool,
    #[serde(default)]
    pub sale: bool,
    #[serde(default)]
    pub used_sale: bool,
    /// Number of devices with serial numbers; for a batch the quantity (default 1).
    #[serde(default)]
    pub quantity: Option<rust_decimal::Decimal>,
}

/// Editable device data of the right-hand side.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DeviceInput {
    #[serde(default)]
    pub serial_no: Option<String>,
    #[serde(default)]
    pub inventory_no: Option<String>,
    #[serde(default)]
    pub inventory_no2: Option<String>,
    #[serde(default)]
    pub stock_id: Option<i32>,
    #[serde(default)]
    pub bin_location: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub inventory: bool,
    #[serde(default)]
    pub rental: bool,
    #[serde(default)]
    pub sale: bool,
    #[serde(default)]
    pub used_sale: bool,
    #[serde(default)]
    pub discarded: bool,
    #[serde(default)]
    pub purchase_price: Option<rust_decimal::Decimal>,
    /// Date `YYYY-MM-DD`.
    #[serde(default)]
    pub purchase_date: Option<String>,
    #[serde(default)]
    pub supplier_no: Option<i32>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub warranty_months: Option<i32>,
    #[serde(default)]
    pub ext_barcode: Option<String>,
    #[serde(default)]
    pub year_built: Option<i32>,
    #[serde(default)]
    pub note: Option<String>,
}

// ---------------------------------------------------------------- Master data

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ArticleGroupDto {
    pub id: i32,
    pub name: Option<String>,
    /// Parent group; `null` = top level.
    pub parent_id: Option<i32>,
    /// Number of articles assigned to this group.
    pub article_count: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ArticleGroupInput {
    pub name: String,
    /// Parent group; omit or `null` for the top level.
    #[serde(default)]
    pub parent_id: Option<i32>,
}

/// A number series with its counter and the resulting next number.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NumberSeriesDto {
    /// Technical key, for example `article` or `invoice`.
    pub key: String,
    pub name: String,
    /// Prepended to the number.
    pub prefix: String,
    /// Template with the placeholders `{number}`, `{year}`, `{month}`, `{day}`.
    pub format: String,
    /// Width of `{number}` with leading zeros (0 = none).
    pub digits: i16,
    /// First number of a period.
    pub start_value: i64,
    pub step: i64,
    /// Last issued number; 0 = none in this period yet.
    pub current_value: i64,
    pub yearly_reset: bool,
    /// Year of the last issue (for `yearly_reset`).
    pub year: Option<i32>,
    pub active: bool,
    /// Counterpart in the legacy structure, for example `"KundenNr"."KundenNr"`.
    pub legacy_source: Option<String>,
    pub changed_at: String,
    /// Next value as it would be issued (`null` for an inactive series).
    pub next_value: Option<i64>,
    /// Next number as text, including prefix and date parts.
    pub next_number: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NumberSeriesInput {
    pub name: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub digits: i16,
    #[serde(default = "default_usage")]
    pub start_value: i64,
    #[serde(default = "default_usage")]
    pub step: i64,
    #[serde(default)]
    pub yearly_reset: bool,
    #[serde(default = "default_active")]
    pub active: bool,
    /// Counter value that is overwritten — the next number is this value
    /// plus the step.
    pub current_value: i64,
}

fn default_format() -> String {
    "{number}".to_string()
}

fn default_usage() -> i64 {
    1
}

fn default_active() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LanguageDto {
    pub id: i32,
    pub language: Option<String>,
    pub native_language: bool,
    pub iso_639_1: Option<String>,
}

/// Warehouse of the "Lager" select (`Storage_Lager`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseDto {
    pub id: i32,
    pub name: Option<String>,
    pub is_default: bool,
}

// ---------------------------------------------------------------- File storage

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LinkInput {
    /// Folder inside the storage, empty for the root.
    #[serde(default)]
    pub folder: String,
    pub name: String,
    pub target: String,
    #[serde(default)]
    pub user: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_list_defaults_to_active_sorted_by_name() {
        let list = AddressListParams::default()
            .into_list()
            .expect("the defaults are valid");
        assert_eq!(list.state, AddressState::Active);
        assert_eq!(list.sort, AddressSort::Name);
        assert!(!list.descending);
        assert_eq!((list.limit, list.offset), (50, 0));
        assert_eq!(list.search, None);
    }

    #[test]
    fn address_list_reads_filter_and_sort() {
        let list = AddressListParams {
            limit: Some(10),
            offset: Some(20),
            q: Some("  Theater ".into()),
            active: Some("inactive".into()),
            sort: Some("city".into()),
            dir: Some("desc".into()),
        }
        .into_list()
        .expect("the query is valid");
        assert_eq!(list.state, AddressState::Inactive);
        assert_eq!(list.sort, AddressSort::City);
        assert!(list.descending);
        assert_eq!((list.limit, list.offset), (10, 20));
        assert_eq!(list.search.as_deref(), Some("Theater"));
    }

    #[test]
    fn address_list_rejects_unknown_values() {
        for params in [
            AddressListParams {
                active: Some("vielleicht".into()),
                ..AddressListParams::default()
            },
            AddressListParams {
                sort: Some("vielleicht".into()),
                ..AddressListParams::default()
            },
            AddressListParams {
                dir: Some("vielleicht".into()),
                ..AddressListParams::default()
            },
        ] {
            assert!(matches!(
                params.into_list(),
                Err(DomainError::Validation(_))
            ));
        }
    }
}
