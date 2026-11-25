use serde::{Deserialize, Serialize};

/// Address master record (`"Adresse"`, 143 columns in the original).
///
/// Only the subset the current API surface exposes is modelled; the remaining
/// columns exist in the migrations and can be added incrementally.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Address {
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
    /// Free-text address group (`"Adresse"."GruppenNameAdresse"`).
    pub group: Option<String>,
    pub is_customer: bool,
    pub is_supplier: bool,
    pub auftragssperre: bool,
    pub mahnsperre: bool,

    // ---- Basic data (`img/adressen/Adressen_Basisdaten_*.PNG`)
    /// `"Adresse"."aktiv"`.
    pub active: bool,
    /// `"Adresse"."adr_Einzelperson"` — the mask switches between "Firma" and "Einzelperson".
    pub is_person: bool,
    /// `"Adresse"."Person_Geburtstag"` (single person only).
    pub birthday: Option<chrono::NaiveDate>,
    /// `"Adresse"."import_Adresszusatz"` — second address line.
    pub address_addition: Option<String>,
    pub postal_code: Option<String>,
    pub state: Option<String>,
    pub fax: Option<String>,
    pub mobile: Option<String>,
    pub website: Option<String>,
    /// `"Adresse"."Type_Interessent"`.
    pub is_interested: bool,
    /// `"Adresse"."Type_Location"`.
    pub is_location: bool,
    /// `"Adresse"."Type_Personal"` — "Mitarbeiter" of the mask.
    pub is_employee: bool,
    /// `"Adresse"."Type_VIP"`.
    pub is_vip: bool,

    // ---- Prospect / customer
    /// `"Adresse"."Adr_EORINr"`.
    pub eori_no: Option<String>,
    /// `"Adresse"."Knd_Steuernummer"`.
    pub tax_number: Option<String>,
    /// `"Adresse"."HRB"` — "Register-Eintrag".
    pub register_entry: Option<String>,
    /// `"Adresse"."HRB_Sitz"` — "Register-Sitz".
    pub register_seat: Option<String>,
    /// `"Adresse"."KreditLimit"`.
    pub credit_limit: Option<rust_decimal::Decimal>,
    /// `"Adresse"."import_knd_RabattMiete"`.
    pub discount_rental: Option<rust_decimal::Decimal>,
    /// `"Adresse"."import_knd_RabattVerkauf"`.
    pub discount_sale: Option<rust_decimal::Decimal>,
    /// `"Adresse"."knd_Bank"`.
    pub bank: Option<String>,
    /// `"Adresse"."knd_Konto"`.
    pub bank_account: Option<String>,
    /// `"Adresse"."knd_BLZ"`.
    pub bank_code: Option<String>,
    /// `"Adresse"."knd_IBAN"`.
    pub iban: Option<String>,
    /// `"Adresse"."knd_BIC"`.
    pub bic: Option<String>,

    // ---- Supplier
    /// `"Adresse"."lief_LieferantenNr"` — the supplier number of the supplier tab.
    pub supplier_no: Option<i32>,
    /// `"Adresse"."lief_Kreditor"`.
    pub creditor: Option<String>,
    /// `"Adresse"."lief_Umsatz"`.
    pub supplier_revenue: Option<rust_decimal::Decimal>,
    /// `"Adresse"."import_lief_RabattMiete"`.
    pub supplier_discount_rental: Option<rust_decimal::Decimal>,
    /// `"Adresse"."import_lief_RabattVerkauf"`.
    pub supplier_discount_purchase: Option<rust_decimal::Decimal>,
    /// `"Adresse"."lief_Bank"`.
    pub supplier_bank: Option<String>,
    /// `"Adresse"."lief_Konto"`.
    pub supplier_bank_account: Option<String>,
    /// `"Adresse"."lief_BLZ"`.
    pub supplier_bank_code: Option<String>,
    /// `"Adresse"."lief_IBAN"`.
    pub supplier_iban: Option<String>,
    /// `"Adresse"."lief_BIC"`.
    pub supplier_bic: Option<String>,
}

/// Create/replace payload for an [`Adresse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInput {
    #[serde(default)]
    pub customer_no: Option<i32>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub name1: Option<String>,
    #[serde(default)]
    pub name2: Option<String>,
    #[serde(default)]
    pub name3: Option<String>,
    #[serde(default)]
    pub salutation: Option<String>,
    #[serde(default)]
    pub street: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub vat_id: Option<String>,
    #[serde(default)]
    pub debtor: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub is_customer: bool,
    #[serde(default)]
    pub is_supplier: bool,
    #[serde(default)]
    pub auftragssperre: bool,
    #[serde(default)]
    pub mahnsperre: bool,

    #[serde(default = "aktiv")]
    pub active: bool,
    #[serde(default)]
    pub is_person: bool,
    #[serde(default)]
    pub birthday: Option<chrono::NaiveDate>,
    #[serde(default)]
    pub address_addition: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub fax: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub is_interested: bool,
    #[serde(default)]
    pub is_location: bool,
    #[serde(default)]
    pub is_employee: bool,
    #[serde(default)]
    pub is_vip: bool,

    #[serde(default)]
    pub eori_no: Option<String>,
    #[serde(default)]
    pub tax_number: Option<String>,
    #[serde(default)]
    pub register_entry: Option<String>,
    #[serde(default)]
    pub register_seat: Option<String>,
    #[serde(default)]
    pub credit_limit: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub discount_rental: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub discount_sale: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub bank: Option<String>,
    #[serde(default)]
    pub bank_account: Option<String>,
    #[serde(default)]
    pub bank_code: Option<String>,
    #[serde(default)]
    pub iban: Option<String>,
    #[serde(default)]
    pub bic: Option<String>,

    #[serde(default)]
    pub supplier_no: Option<i32>,
    #[serde(default)]
    pub creditor: Option<String>,
    #[serde(default)]
    pub supplier_revenue: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub supplier_discount_rental: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub supplier_discount_purchase: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub supplier_bank: Option<String>,
    #[serde(default)]
    pub supplier_bank_account: Option<String>,
    #[serde(default)]
    pub supplier_bank_code: Option<String>,
    #[serde(default)]
    pub supplier_iban: Option<String>,
    #[serde(default)]
    pub supplier_bic: Option<String>,
}

/// `aktiv` is set unless the caller says otherwise.
fn aktiv() -> bool {
    true
}

impl Default for AddressInput {
    fn default() -> Self {
        Self {
            customer_no: None,
            title: None,
            name1: None,
            name2: None,
            name3: None,
            salutation: None,
            street: None,
            country: None,
            city: None,
            vat_id: None,
            debtor: None,
            phone: None,
            email: None,
            group: None,
            is_customer: false,
            is_supplier: false,
            auftragssperre: false,
            mahnsperre: false,
            active: true,
            is_person: false,
            birthday: None,
            address_addition: None,
            postal_code: None,
            state: None,
            fax: None,
            mobile: None,
            website: None,
            is_interested: false,
            is_location: false,
            is_employee: false,
            is_vip: false,
            eori_no: None,
            tax_number: None,
            register_entry: None,
            register_seat: None,
            credit_limit: None,
            discount_rental: None,
            discount_sale: None,
            bank: None,
            bank_account: None,
            bank_code: None,
            iban: None,
            bic: None,
            supplier_no: None,
            creditor: None,
            supplier_revenue: None,
            supplier_discount_rental: None,
            supplier_discount_purchase: None,
            supplier_bank: None,
            supplier_bank_account: None,
            supplier_bank_code: None,
            supplier_iban: None,
            supplier_bic: None,
        }
    }
}

impl AddressInput {
    /// "Kunde" and "Interessent" exclude each other; everything else may be combined.
    pub fn check_roles(&self) -> Result<(), crate::DomainError> {
        if self.is_customer && self.is_interested {
            return Err(crate::DomainError::Validation(
                "an address cannot be customer and interested party at the same time".into(),
            ));
        }
        Ok(())
    }

    /// Rejects records that cannot be identified in the UI.
    ///
    /// The legacy application required neither name nor number, but an address without any
    /// name is not usable in rental contracts.
    pub fn validate(&self) -> Result<(), crate::DomainError> {
        let has_name = [&self.name1, &self.name2, &self.name3]
            .into_iter()
            .flatten()
            .any(|s| !s.trim().is_empty());
        if !has_name {
            return Err(crate::DomainError::Validation(
                "at least one of name1, name2 or name3 must be set".into(),
            ));
        }
        if !self.is_customer && !self.is_supplier && !self.is_interested {
            return Err(crate::DomainError::Validation(
                "an address must be customer, supplier or interested party".into(),
            ));
        }
        self.check_roles()?;
        // The mask asks for the number before any other field; the same rule is
        // enforced here so the API cannot create half-made masters.
        if (self.is_customer || self.is_interested) && self.customer_no.is_none() {
            return Err(crate::DomainError::Validation(
                "a customer or interested party needs a customer number".into(),
            ));
        }
        if self.is_supplier && self.supplier_no.is_none() {
            return Err(crate::DomainError::Validation(
                "a supplier needs a supplier number".into(),
            ));
        }
        if self.customer_no.is_some_and(|n| n <= 0) {
            return Err(crate::DomainError::Validation(
                "the customer number must be positive".into(),
            ));
        }
        if self.supplier_no.is_some_and(|n| n <= 0) {
            return Err(crate::DomainError::Validation(
                "the supplier number must be positive".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> AddressInput {
        AddressInput {
            name1: Some("Muster GmbH".into()),
            is_customer: true,
            customer_no: Some(4711),
            ..AddressInput::default()
        }
    }

    #[test]
    fn accepts_a_customer_with_number() {
        assert!(base().validate().is_ok());
    }

    #[test]
    fn a_customer_needs_a_number() {
        let mut a = base();
        a.customer_no = None;
        assert!(a.validate().is_err());
    }

    #[test]
    fn a_supplier_needs_a_supplier_number() {
        let mut a = base();
        a.is_supplier = true;
        assert!(a.validate().is_err());
        a.supplier_no = Some(7);
        assert!(a.validate().is_ok());
    }

    #[test]
    fn an_interested_party_needs_a_customer_number() {
        let mut a = base();
        a.is_customer = false;
        a.is_interested = true;
        assert!(a.validate().is_ok(), "the customer number is reused");
        a.customer_no = None;
        assert!(a.validate().is_err());
    }

    #[test]
    fn customer_and_interested_exclude_each_other() {
        let mut a = base();
        a.is_interested = true;
        let error = a.check_roles().unwrap_err().to_string();
        assert!(error.contains("customer and interested"), "{error}");
    }

    #[test]
    fn supplier_combines_with_customer() {
        let mut a = base();
        a.is_supplier = true;
        a.supplier_no = Some(2001);
        assert!(a.validate().is_ok());
    }

    #[test]
    fn a_role_is_required() {
        let mut a = base();
        a.is_customer = false;
        assert!(a.validate().is_err());
    }

    #[test]
    fn active_defaults_to_true() {
        assert!(AddressInput::default().active);
    }
}
