//! Tenants of the installation (`img/Admin Menü/Mandanten`).
//!
//! **Every tenant has its own database.** The legacy schema knows no tenant column on the
//! business tables, so nothing here filters by tenant: two tenants with separate data are
//! two databases, each with its own users and rights. The rows of `"Mandanten"` describe
//! the companies of this installation and carry the stammdaten sync (`SyncID`) with the
//! other databases.
//!
//! One row per client company in the legacy table `"Mandanten"`. The window
//! `Einstellungen Mandanten` shows the tenants in a list (`Abk. | Firmename |
//! Stammdaten-Sync`) and edits the selected one on two tabs, `Einstellungen` and
//! `Adresse`. The two blocks of options below the list (the stammdaten sync and the
//! separations that apply to all tenants) are installation-wide and live in
//! `"where_settings"`, like the other options of the masks.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::DomainError;

/// One tenant of the installation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Tenant {
    pub id: i64,
    /// `Shortname` — the `Abk.` column of the list.
    pub short_name: Option<String>,
    /// `Mandanten_Name` — the `Firmename`.
    pub name: String,
    /// `Mandanten_ID` — the `Ident.-Nr.` of the mask.
    pub tenant_id: i32,
    /// `SyncID` — the `Stammdaten-Sync` column; `0` means "not set up".
    pub sync_id: i32,
    /// `Parent` — the superior tenant.
    pub parent: Option<i32>,
    /// `web_identifier` — the name in the web portal.
    pub web_identifier: Option<String>,
    pub currency: Option<String>,
    pub currency_id: Option<i32>,
    /// `Drucksprache` — e.g. `Deutsch [deu]`.
    pub print_language: Option<String>,
    pub print_language_id: Option<i32>,
    /// `HideAktions` — "Vor Anderen verstecken".
    pub hide_actions: bool,
    /// `Color` and `Color_ON` — the toolbar colour of the tenant.
    pub color: i32,
    pub color_on: bool,
    /// `adr_adressZusatz`
    pub address_extra: Option<String>,
    /// `adr_adressStrasse` — `Straße / Postfach`.
    pub street: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    /// `adr_adressLand_ISO3` — `DEU`.
    pub country_iso3: Option<String>,
    /// `adr_adressLand` — `GERMANY`.
    pub country: Option<String>,
    /// `adr_adressBundesland`
    pub state: Option<String>,
    pub phone: Option<String>,
    pub fax: Option<String>,
    pub email: Option<String>,
    /// `adr_comEmailBuchhaltung`
    pub email_accounting: Option<String>,
    pub website: Option<String>,
    /// `Ustid` — `UST-Ident-Nr.`
    pub vat_id: Option<String>,
    /// `FANr` — `Steuernummer`.
    pub tax_number: Option<String>,
    /// `HRB` — `Register-Eintrag`.
    pub register_entry: Option<String>,
    /// `HRB_Sitz` — `Register-Sitz`.
    pub register_place: Option<String>,
    /// `adr_LinkAddressPK` — the address behind `Verlinkte Adresse`.
    pub linked_address: Option<i32>,
    /// Its `"Adresse"."Name1"`, for the `+ Keine` line of the mask.
    pub linked_address_name: Option<String>,
}

/// What the mask sends for a new or changed tenant.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct TenantInput {
    pub name: String,
    #[serde(default)]
    pub short_name: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<i32>,
    #[serde(default)]
    pub sync_id: Option<i32>,
    #[serde(default)]
    pub parent: Option<i32>,
    #[serde(default)]
    pub web_identifier: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub currency_id: Option<i32>,
    #[serde(default)]
    pub print_language: Option<String>,
    #[serde(default)]
    pub print_language_id: Option<i32>,
    #[serde(default)]
    pub hide_actions: bool,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub color_on: bool,
    #[serde(default)]
    pub address_extra: Option<String>,
    #[serde(default)]
    pub street: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub country_iso3: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub fax: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_accounting: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub vat_id: Option<String>,
    #[serde(default)]
    pub tax_number: Option<String>,
    #[serde(default)]
    pub register_entry: Option<String>,
    #[serde(default)]
    pub register_place: Option<String>,
    #[serde(default)]
    pub linked_address: Option<i32>,
}

/// The print languages the mask offers, with the key the legacy table keeps.
pub const PRINT_LANGUAGES: [(&str, i32); 5] = [
    ("Deutsch [deu]", 1),
    ("English [eng]", 2),
    ("Français [fra]", 3),
    ("Italiano [ita]", 4),
    ("Nederlands [nld]", 5),
];

/// The long name the mask shows next to a country code.
///
/// The legacy table keeps both (`adr_adressLand_ISO3` and `adr_adressLand`); the
/// list holds the countries of the German-speaking rental business plus their
/// neighbours. An unknown code is kept as it is.
pub const COUNTRIES: [(&str, &str); 27] = [
    ("DEU", "GERMANY"),
    ("AUT", "AUSTRIA"),
    ("CHE", "SWITZERLAND"),
    ("LIE", "LIECHTENSTEIN"),
    ("FRA", "FRANCE"),
    ("ITA", "ITALY"),
    ("NLD", "NETHERLANDS"),
    ("BEL", "BELGIUM"),
    ("LUX", "LUXEMBOURG"),
    ("DNK", "DENMARK"),
    ("SWE", "SWEDEN"),
    ("NOR", "NORWAY"),
    ("FIN", "FINLAND"),
    ("POL", "POLAND"),
    ("CZE", "CZECH REPUBLIC"),
    ("SVK", "SLOVAKIA"),
    ("HUN", "HUNGARY"),
    ("SVN", "SLOVENIA"),
    ("HRV", "CROATIA"),
    ("ESP", "SPAIN"),
    ("PRT", "PORTUGAL"),
    ("GRC", "GREECE"),
    ("GBR", "UNITED KINGDOM"),
    ("IRL", "IRELAND"),
    ("TUR", "TURKEY"),
    ("USA", "UNITED STATES"),
    ("CAN", "CANADA"),
];

/// The long name of a code; an unknown code comes back unchanged.
pub fn country_name(iso3: &str) -> &str {
    COUNTRIES
        .iter()
        .find(|(code, _)| *code == iso3)
        .map(|(_, name)| *name)
        .unwrap_or(iso3)
}

/// One entry of the country list of the mask.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CountryEntry {
    pub iso3: String,
    pub name: String,
}

/// One entry of the `Drucksprache` list of the mask.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrintLanguageEntry {
    pub label: String,
    pub id: i32,
}

/// The two selection lists the tenant mask shows: countries and print languages.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantReference {
    pub countries: Vec<CountryEntry>,
    pub print_languages: Vec<PrintLanguageEntry>,
}

impl TenantReference {
    pub fn new() -> Self {
        Self {
            countries: COUNTRIES
                .iter()
                .map(|(iso3, name)| CountryEntry {
                    iso3: (*iso3).to_string(),
                    name: (*name).to_string(),
                })
                .collect(),
            print_languages: PRINT_LANGUAGES
                .iter()
                .map(|(label, id)| PrintLanguageEntry {
                    label: (*label).to_string(),
                    id: *id,
                })
                .collect(),
        }
    }
}

impl Default for TenantReference {
    fn default() -> Self {
        Self::new()
    }
}

/// Trimmed value; an empty text is no value.
fn optional(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

impl TenantInput {
    /// Checks the input and builds the row the repository writes.
    pub fn prepare(self) -> Result<Tenant, DomainError> {
        let name = self.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Für den Mandanten fehlt der Firmenname.".to_string(),
            ));
        }
        let tenant_id = self.tenant_id.unwrap_or(0);
        if tenant_id < 0 {
            return Err(DomainError::Validation(
                "Die Ident.-Nr. darf nicht negativ sein.".to_string(),
            ));
        }
        let country_iso3 = optional(self.country_iso3).map(|text| text.to_uppercase());
        let country = optional(self.country).or_else(|| {
            country_iso3
                .as_deref()
                .map(|iso3| country_name(iso3).to_string())
        });
        let print_language = optional(self.print_language);
        let print_language_id = self.print_language_id.or_else(|| {
            print_language.as_deref().and_then(|label| {
                PRINT_LANGUAGES
                    .iter()
                    .find(|(known, _)| *known == label)
                    .map(|(_, id)| *id)
            })
        });
        let linked_address = self.linked_address.filter(|id| *id > 0);
        Ok(Tenant {
            id: 0,
            short_name: optional(self.short_name),
            name,
            tenant_id,
            sync_id: self.sync_id.unwrap_or(0),
            parent: self.parent.filter(|id| *id > 0),
            web_identifier: optional(self.web_identifier),
            currency: optional(self.currency),
            currency_id: self.currency_id,
            print_language,
            print_language_id,
            hide_actions: self.hide_actions,
            color: self.color.unwrap_or(0),
            color_on: self.color_on,
            address_extra: optional(self.address_extra),
            street: optional(self.street),
            postal_code: optional(self.postal_code),
            city: optional(self.city),
            country_iso3,
            country,
            state: optional(self.state),
            phone: optional(self.phone),
            fax: optional(self.fax),
            email: optional(self.email),
            email_accounting: optional(self.email_accounting),
            website: optional(self.website),
            vat_id: optional(self.vat_id),
            tax_number: optional(self.tax_number),
            register_entry: optional(self.register_entry),
            register_place: optional(self.register_place),
            linked_address,
            linked_address_name: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> TenantInput {
        TenantInput {
            name: "  Theater Ulm  ".into(),
            short_name: Some(" TH ".into()),
            tenant_id: Some(0),
            sync_id: None,
            parent: Some(0),
            web_identifier: Some("  ".into()),
            currency: None,
            currency_id: None,
            print_language: Some("Deutsch [deu]".into()),
            print_language_id: None,
            hide_actions: false,
            color: None,
            color_on: true,
            address_extra: None,
            street: Some("Herbert-von-Karajan-Platz 1".into()),
            postal_code: Some("89073".into()),
            city: Some("Ulm".into()),
            country_iso3: Some("deu".into()),
            country: None,
            state: None,
            phone: None,
            fax: None,
            email: None,
            email_accounting: None,
            website: None,
            vat_id: None,
            tax_number: None,
            register_entry: None,
            register_place: None,
            linked_address: Some(0),
        }
    }

    #[test]
    fn a_tenant_needs_a_company_name() {
        let tenant = input().prepare().unwrap();
        assert_eq!(tenant.name, "Theater Ulm");
        assert_eq!(tenant.short_name.as_deref(), Some("TH"));
        assert_eq!(tenant.web_identifier, None);
        assert_eq!(tenant.parent, None);
        assert_eq!(tenant.linked_address, None);
        assert_eq!(tenant.country_iso3.as_deref(), Some("DEU"));
        assert_eq!(tenant.country.as_deref(), Some("GERMANY"));
        assert_eq!(tenant.print_language_id, Some(1));

        let mut ohne_name = input();
        ohne_name.name = "   ".into();
        assert!(ohne_name.prepare().is_err());

        let mut negativ = input();
        negativ.tenant_id = Some(-1);
        assert!(negativ.prepare().is_err());
    }

    #[test]
    fn a_given_country_name_wins_and_unknown_codes_stay() {
        let mut eigene = input();
        eigene.country = Some("Deutschland".into());
        assert_eq!(
            eigene.prepare().unwrap().country.as_deref(),
            Some("Deutschland")
        );

        let mut unbekannt = input();
        unbekannt.country_iso3 = Some("XYZ".into());
        assert_eq!(unbekannt.prepare().unwrap().country.as_deref(), Some("XYZ"));
        assert_eq!(country_name("CHE"), "SWITZERLAND");
    }
}
