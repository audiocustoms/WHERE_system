//! Bank accounts of the company (`img/Admin Menü/Bankverbindungen`).
//!
//! The legacy table `"Bankverbindungen"` keeps one row per account with three groups of
//! columns: the general account data, the SEPA creditor data and the Swiss address of the
//! holder. WHERE shows them as the three tabs of the window and computes the IBAN out of
//! sort code and account number, which the original offers behind a button.

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use where_domain::DomainError;

/// One account of the company.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BankAccount {
    pub id: i64,
    /// `Bezeichnung` — the name the list shows.
    pub label: String,
    pub holder: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
    /// `Institut` — the bank behind the account.
    pub institute: Option<String>,
    pub account_number: Option<String>,
    pub blz: Option<String>,
    pub bank_address: Option<String>,
    pub standard: bool,
    pub fibu_account: Option<String>,
    pub sepa_creditor_id: Option<String>,
    pub sepa_direct_debit: bool,
    /// Working days before the first direct debit.
    pub sepa_lead_first: Option<i32>,
    /// Working days before a repeated direct debit.
    pub sepa_lead_follow: Option<i32>,
    /// Latest time of day the file goes to the bank.
    pub sepa_submit_time: Option<String>,
    pub ch_street: Option<String>,
    pub ch_house_number: Option<String>,
    pub ch_postal_code: Option<String>,
    pub ch_city: Option<String>,
    pub ch_country: Option<String>,
    pub ch_holder_address: Option<String>,
    /// The electronic-banking container is set — the `EB` column of the list.
    pub electronic_banking: bool,
}

/// What the mask sends for a new or changed account.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BankAccountInput {
    pub label: String,
    #[serde(default)]
    pub holder: Option<String>,
    #[serde(default)]
    pub iban: Option<String>,
    #[serde(default)]
    pub bic: Option<String>,
    #[serde(default)]
    pub institute: Option<String>,
    #[serde(default)]
    pub account_number: Option<String>,
    #[serde(default)]
    pub blz: Option<String>,
    #[serde(default)]
    pub bank_address: Option<String>,
    #[serde(default)]
    pub standard: bool,
    #[serde(default)]
    pub fibu_account: Option<String>,
    #[serde(default)]
    pub sepa_creditor_id: Option<String>,
    #[serde(default)]
    pub sepa_direct_debit: bool,
    #[serde(default)]
    pub sepa_lead_first: Option<i32>,
    #[serde(default)]
    pub sepa_lead_follow: Option<i32>,
    #[serde(default)]
    pub sepa_submit_time: Option<String>,
    #[serde(default)]
    pub ch_street: Option<String>,
    #[serde(default)]
    pub ch_house_number: Option<String>,
    #[serde(default)]
    pub ch_postal_code: Option<String>,
    #[serde(default)]
    pub ch_city: Option<String>,
    #[serde(default)]
    pub ch_country: Option<String>,
    #[serde(default)]
    pub ch_holder_address: Option<String>,
}

/// Trimmed value; an empty text is no value.
fn optional(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// "09:30" and "09:30:00" both mean the same time.
fn parse_time(text: Option<String>) -> Result<Option<NaiveTime>, DomainError> {
    let Some(text) = optional(text) else {
        return Ok(None);
    };
    for format in ["%H:%M", "%H:%M:%S"] {
        if let Ok(time) = NaiveTime::parse_from_str(&text, format) {
            return Ok(Some(time));
        }
    }
    Err(DomainError::Validation(format!(
        "„{text}\u{201c} ist keine Uhrzeit (SS:MM)."
    )))
}

impl BankAccountInput {
    /// Checks the input and builds the row the repository writes.
    pub fn prepare(self) -> Result<BankAccount, DomainError> {
        let label = self.label.trim().to_string();
        if label.is_empty() {
            return Err(DomainError::Validation(
                "Für die Bankverbindung fehlt die Bezeichnung.".to_string(),
            ));
        }
        let iban = optional(self.iban).map(|text| text.replace(' ', "").to_uppercase());
        if let Some(iban) = iban.as_deref()
            && (!(15..=34).contains(&iban.chars().count())
                || !iban.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err(DomainError::Validation(format!(
                "„{iban}\u{201c} ist keine IBAN (15 bis 34 Zeichen, nur Buchstaben und Ziffern)."
            )));
        }
        let bic = optional(self.bic).map(|text| text.to_uppercase());
        if let Some(bic) = bic.as_deref()
            && (!(8..=11).contains(&bic.chars().count())
                || !bic.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err(DomainError::Validation(format!(
                "„{bic}\u{201c} ist kein BIC (8 oder 11 Zeichen)."
            )));
        }
        let country = optional(self.ch_country).map(|text| text.to_uppercase());
        if let Some(country) = country.as_deref()
            && country != "CH"
            && country != "LI"
        {
            return Err(DomainError::Validation(
                "Das Land der Kontoinhaber-Anschrift ist CH oder LI.".to_string(),
            ));
        }
        Ok(BankAccount {
            id: 0,
            label,
            holder: optional(self.holder),
            iban,
            bic,
            institute: optional(self.institute),
            account_number: optional(self.account_number),
            blz: optional(self.blz),
            bank_address: optional(self.bank_address),
            standard: self.standard,
            fibu_account: optional(self.fibu_account),
            sepa_creditor_id: optional(self.sepa_creditor_id),
            sepa_direct_debit: self.sepa_direct_debit,
            sepa_lead_first: self.sepa_lead_first,
            sepa_lead_follow: self.sepa_lead_follow,
            sepa_submit_time: parse_time(self.sepa_submit_time)?
                .map(|time| time.format("%H:%M:%S").to_string()),
            ch_street: optional(self.ch_street),
            ch_house_number: optional(self.ch_house_number),
            ch_postal_code: optional(self.ch_postal_code),
            ch_city: optional(self.ch_city),
            ch_country: country,
            ch_holder_address: optional(self.ch_holder_address),
            electronic_banking: false,
        })
    }
}

/// What the „Kto & BLZ in IBAN & BIC umwandeln“ button sends.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ConversionRequest {
    /// Sort code, eight digits.
    pub blz: String,
    /// Account number, up to ten digits.
    pub account_number: String,
}

/// The IBAN built from sort code and account number, plus what the bank list knows.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConversionResult {
    pub iban: String,
    pub bic: Option<String>,
    pub institute: Option<String>,
    pub bank_address: Option<String>,
}

/// German IBAN: `DE` + check digits + sort code (8) + account number (10).
pub fn iban_de(blz: &str, account_number: &str) -> Result<String, DomainError> {
    let blz = blz.trim();
    let konto = account_number.trim();
    if blz.chars().count() != 8 || !blz.chars().all(|c| c.is_ascii_digit()) {
        return Err(DomainError::Validation(
            "Die Bankleitzahl muss aus acht Ziffern bestehen.".to_string(),
        ));
    }
    if konto.is_empty() || konto.chars().count() > 10 || !konto.chars().all(|c| c.is_ascii_digit())
    {
        return Err(DomainError::Validation(
            "Die Kontonummer besteht aus höchstens zehn Ziffern.".to_string(),
        ));
    }
    let bban = format!("{blz}{konto:0>10}");
    // Check digits: 98 - (BBAN + "1314" + "00") mod 97.
    let check = 98 - mod97(&format!("{bban}131400"));
    Ok(format!("DE{check:02}{bban}"))
}

/// Remainder of a long numeric string divided by 97, in chunks that fit an `u32`.
fn mod97(number: &str) -> u32 {
    let mut remainder: u32 = 0;
    for digit in number.chars() {
        let value = digit.to_digit(10).unwrap_or(0);
        remainder = (remainder * 10 + value) % 97;
    }
    remainder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_iban_is_built_from_sort_code_and_account() {
        // Known example of the Bundesbank.
        assert_eq!(
            iban_de("37040044", "532013000").unwrap(),
            "DE89370400440532013000"
        );
        assert_eq!(
            iban_de("10000000", "1234567890").unwrap(),
            "DE23100000001234567890"
        );
        assert!(iban_de("1234", "1").is_err());
        assert!(iban_de("12345678", "abcdef").is_err());
        assert!(iban_de("12345678", "12345678901").is_err());
    }

    #[test]
    fn an_account_needs_a_label_and_a_clean_iban() {
        let input = BankAccountInput {
            label: "  Hausbank ".into(),
            holder: Some(" Theater GmbH ".into()),
            iban: Some("de89 3704 0044 0532 0130 00".into()),
            bic: Some("cobadeffxxx".into()),
            institute: None,
            account_number: None,
            blz: None,
            bank_address: None,
            standard: true,
            fibu_account: Some(" 1200 ".into()),
            sepa_creditor_id: None,
            sepa_direct_debit: true,
            sepa_lead_first: Some(5),
            sepa_lead_follow: Some(2),
            sepa_submit_time: Some("09:30".into()),
            ch_street: None,
            ch_house_number: None,
            ch_postal_code: None,
            ch_city: None,
            ch_country: Some("ch".into()),
            ch_holder_address: None,
        };
        let account = input.prepare().unwrap();
        assert_eq!(account.label, "Hausbank");
        assert_eq!(account.iban.as_deref(), Some("DE89370400440532013000"));
        assert_eq!(account.bic.as_deref(), Some("COBADEFFXXX"));
        assert_eq!(account.fibu_account.as_deref(), Some("1200"));
        assert_eq!(account.sepa_submit_time.as_deref(), Some("09:30:00"));
        assert_eq!(account.ch_country.as_deref(), Some("CH"));

        let ohne_label = BankAccountInput {
            label: "   ".into(),
            holder: None,
            iban: None,
            bic: None,
            institute: None,
            account_number: None,
            blz: None,
            bank_address: None,
            standard: false,
            fibu_account: None,
            sepa_creditor_id: None,
            sepa_direct_debit: false,
            sepa_lead_first: None,
            sepa_lead_follow: None,
            sepa_submit_time: None,
            ch_street: None,
            ch_house_number: None,
            ch_postal_code: None,
            ch_city: None,
            ch_country: None,
            ch_holder_address: None,
        };
        assert!(ohne_label.prepare().is_err());
    }

    #[test]
    fn times_and_countries_are_checked() {
        let mut input = BankAccountInput {
            label: "X".into(),
            holder: None,
            iban: None,
            bic: Some("COBADEFF".into()),
            institute: None,
            account_number: None,
            blz: None,
            bank_address: None,
            standard: false,
            fibu_account: None,
            sepa_creditor_id: None,
            sepa_direct_debit: false,
            sepa_lead_first: None,
            sepa_lead_follow: None,
            sepa_submit_time: Some("25:99".into()),
            ch_street: None,
            ch_house_number: None,
            ch_postal_code: None,
            ch_city: None,
            ch_country: None,
            ch_holder_address: None,
        };
        assert!(input.clone().prepare().is_err());
        input.sepa_submit_time = Some("06:15".into());
        assert_eq!(
            input.clone().prepare().unwrap().sepa_submit_time.as_deref(),
            Some("06:15:00")
        );

        input.ch_country = Some("AT".into());
        assert!(input.prepare().is_err());
    }
}
