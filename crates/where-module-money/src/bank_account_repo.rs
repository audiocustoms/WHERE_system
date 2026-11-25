//! Bank accounts of the company (`img/Admin Menü/Bankverbindungen`).
//!
//! One row per account in the legacy table `"Bankverbindungen"`; the columns of the three
//! tabs of the window are read and written as they are. The IBAN helper looks its BIC up
//! in the bank list (`"Storage_Banken"`), which the bank window fills.

use chrono::NaiveTime;
use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::bank_account::{
    BankAccount, BankAccountInput, ConversionRequest, ConversionResult, iban_de,
};

const COLUMNS: &str = "\"Primary_Key\"::bigint AS id, \"Bezeichnung\" AS label, \
     \"Kontoinhaber\" AS holder, \"IBAN\" AS iban, \"BIC\" AS bic, \"Institut\" AS institute, \
     \"Kontonummer\" AS account_number, \"BLZ\" AS blz, \"BankAnschrift\" AS bank_address, \
     COALESCE(\"Standard\", false) AS standard, \"FiBu_Konto\" AS fibu_account, \
     \"SEPA_Glaeubiger_ID\" AS creditor_id, COALESCE(\"SEPA_Sammellastschrift\", false) AS direct_debit, \
     \"SEPA_VorlaufErstLS\" AS lead_first, \"SEPA_VorlaufFolgeLS\" AS lead_follow, \
     \"SEPA_Einreichendzeit\" AS submit_time, \"CH_adrStrasse\" AS ch_street, \
     \"CH_adrHausnummer\" AS ch_house_number, \"CH_adrPLZ\" AS ch_postal_code, \
     \"CH_adrPlzOrt\" AS ch_city, \"CH_adrLand\" AS ch_country, \
     \"CH_InhaberAnschrift\" AS ch_holder_address, \
     (\"BankX_Kontenmappe\" IS NOT NULL AND \"BankX_Kontenmappe\" <> '') AS electronic_banking";

fn row_to_account(row: &sqlx::postgres::PgRow) -> BankAccount {
    let time: Option<NaiveTime> = row.get("submit_time");
    let postal_code: Option<String> = row.get("ch_postal_code");
    let place: Option<String> = row.get("ch_city");
    BankAccount {
        id: row.get("id"),
        label: row.get::<Option<String>, _>("label").unwrap_or_default(),
        holder: row.get("holder"),
        iban: row.get("iban"),
        bic: row.get("bic"),
        institute: row.get("institute"),
        account_number: row.get("account_number"),
        blz: row.get("blz"),
        bank_address: row.get("bank_address"),
        standard: row.get("standard"),
        fibu_account: row.get("fibu_account"),
        sepa_creditor_id: row.get("creditor_id"),
        sepa_direct_debit: row.get("direct_debit"),
        sepa_lead_first: row.get("lead_first"),
        sepa_lead_follow: row.get("lead_follow"),
        sepa_submit_time: time.map(|time| time.format("%H:%M:%S").to_string()),
        ch_street: row.get("ch_street"),
        ch_house_number: row.get("ch_house_number"),
        ch_city: city_of(place, &postal_code),
        ch_postal_code: postal_code,
        ch_country: row.get("ch_country"),
        ch_holder_address: row.get("ch_holder_address"),
        electronic_banking: row.get("electronic_banking"),
    }
}

/// The legacy application keeps post code and town both separate and combined.
fn ch_place(postal_code: &Option<String>, city: &Option<String>) -> Option<String> {
    match (postal_code.as_deref(), city.as_deref()) {
        (Some(plz), Some(ort)) => Some(format!("{plz} {ort}")),
        (Some(plz), None) => Some(plz.to_string()),
        (None, Some(ort)) => Some(ort.to_string()),
        (None, None) => None,
    }
}

/// "8001 Zürich" with post code "8001" yields "Zürich".
fn city_of(place: Option<String>, postal_code: &Option<String>) -> Option<String> {
    let place = place?.trim().to_string();
    if let Some(plz) = postal_code.as_deref()
        && let Some(rest) = place.strip_prefix(plz)
    {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }
    Some(place)
}

/// Reads and writes the accounts of the company.
#[derive(Clone)]
pub struct BankAccountRepository {
    pool: PgPool,
}

impl BankAccountRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// All accounts; the standard account first.
    pub async fn list(&self) -> Result<Vec<BankAccount>, DomainError> {
        let sql = format!(
            "SELECT {COLUMNS} FROM \"Bankverbindungen\" \
             ORDER BY COALESCE(\"Standard\", false) DESC, \"Bezeichnung\" NULLS LAST"
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.iter().map(row_to_account).collect())
    }

    pub async fn get(&self, id: i64) -> Result<BankAccount, DomainError> {
        let sql = format!("SELECT {COLUMNS} FROM \"Bankverbindungen\" WHERE \"Primary_Key\" = $1");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        row.as_ref()
            .map(row_to_account)
            .ok_or(DomainError::NotFound {
                entity: "Bankverbindung",
                id: id as i32,
            })
    }

    pub async fn create(&self, input: BankAccountInput) -> Result<BankAccount, DomainError> {
        let account = input.prepare()?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if account.standard {
            sqlx::query(
                "UPDATE \"Bankverbindungen\" SET \"Standard\" = false WHERE \"Standard\" = true",
            )
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO \"Bankverbindungen\" (\"Bezeichnung\", \"Kontoinhaber\", \"IBAN\", \"BIC\", \
             \"Institut\", \"Kontonummer\", \"BLZ\", \"BankAnschrift\", \"Standard\", \"FiBu_Konto\", \
             \"SEPA_Glaeubiger_ID\", \"SEPA_Sammellastschrift\", \"SEPA_VorlaufErstLS\", \
             \"SEPA_VorlaufFolgeLS\", \"SEPA_Einreichendzeit\", \"CH_adrStrasse\", \"CH_adrHausnummer\", \
             \"CH_adrPLZ\", \"CH_adrPlzOrt\", \"CH_adrLand\", \"CH_InhaberAnschrift\") \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, \
                     CAST($15 AS time), $16, $17, $18, $19, $20, $21) RETURNING \"Primary_Key\"::bigint",
        )
        .bind(&account.label)
        .bind(&account.holder)
        .bind(&account.iban)
        .bind(&account.bic)
        .bind(&account.institute)
        .bind(&account.account_number)
        .bind(&account.blz)
        .bind(&account.bank_address)
        .bind(account.standard)
        .bind(&account.fibu_account)
        .bind(&account.sepa_creditor_id)
        .bind(account.sepa_direct_debit)
        .bind(account.sepa_lead_first)
        .bind(account.sepa_lead_follow)
        .bind(&account.sepa_submit_time)
        .bind(&account.ch_street)
        .bind(&account.ch_house_number)
        .bind(&account.ch_postal_code)
        .bind(ch_place(&account.ch_postal_code, &account.ch_city))
        .bind(&account.ch_country)
        .bind(&account.ch_holder_address)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(
        &self,
        id: i64,
        input: BankAccountInput,
    ) -> Result<BankAccount, DomainError> {
        let account = input.prepare()?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if account.standard {
            sqlx::query(
                "UPDATE \"Bankverbindungen\" SET \"Standard\" = false \
                 WHERE \"Primary_Key\" <> $1 AND \"Standard\" = true",
            )
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }
        let changed = sqlx::query(
            "UPDATE \"Bankverbindungen\" SET \"Bezeichnung\" = $2, \"Kontoinhaber\" = $3, \
             \"IBAN\" = $4, \"BIC\" = $5, \"Institut\" = $6, \"Kontonummer\" = $7, \"BLZ\" = $8, \
             \"BankAnschrift\" = $9, \"Standard\" = $10, \"FiBu_Konto\" = $11, \
             \"SEPA_Glaeubiger_ID\" = $12, \"SEPA_Sammellastschrift\" = $13, \
             \"SEPA_VorlaufErstLS\" = $14, \"SEPA_VorlaufFolgeLS\" = $15, \
             \"SEPA_Einreichendzeit\" = CAST($16 AS time), \"CH_adrStrasse\" = $17, \
             \"CH_adrHausnummer\" = $18, \"CH_adrPLZ\" = $19, \"CH_adrPlzOrt\" = $20, \
             \"CH_adrLand\" = $21, \"CH_InhaberAnschrift\" = $22 WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .bind(&account.label)
        .bind(&account.holder)
        .bind(&account.iban)
        .bind(&account.bic)
        .bind(&account.institute)
        .bind(&account.account_number)
        .bind(&account.blz)
        .bind(&account.bank_address)
        .bind(account.standard)
        .bind(&account.fibu_account)
        .bind(&account.sepa_creditor_id)
        .bind(account.sepa_direct_debit)
        .bind(account.sepa_lead_first)
        .bind(account.sepa_lead_follow)
        .bind(&account.sepa_submit_time)
        .bind(&account.ch_street)
        .bind(&account.ch_house_number)
        .bind(&account.ch_postal_code)
        .bind(ch_place(&account.ch_postal_code, &account.ch_city))
        .bind(&account.ch_country)
        .bind(&account.ch_holder_address)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?
        .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Bankverbindung",
                id: id as i32,
            });
        }
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let changed = sqlx::query("DELETE FROM \"Bankverbindungen\" WHERE \"Primary_Key\" = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Bankverbindung",
                id: id as i32,
            });
        }
        Ok(())
    }

    /// Sets or clears the `Standard` flag of one account.
    pub async fn set_standard(&self, id: i64, standard: bool) -> Result<BankAccount, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if standard {
            sqlx::query(
                "UPDATE \"Bankverbindungen\" SET \"Standard\" = false \
                 WHERE \"Primary_Key\" <> $1 AND \"Standard\" = true",
            )
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }
        let changed = sqlx::query(
            "UPDATE \"Bankverbindungen\" SET \"Standard\" = $2 WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .bind(standard)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?
        .rows_affected();
        tx.commit().await.map_err(db_error)?;
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Bankverbindung",
                id: id as i32,
            });
        }
        self.get(id).await
    }

    /// „Kto & BLZ in IBAN & BIC umwandeln“: builds the German IBAN and looks the bank up
    /// in the bank list.
    pub async fn convert(
        &self,
        request: ConversionRequest,
    ) -> Result<ConversionResult, DomainError> {
        let iban = iban_de(&request.blz, &request.account_number)?;
        let row = sqlx::query(
            "SELECT \"BIC\", \"Name\", \"Ort\" FROM \"Storage_Banken\" \
             WHERE \"BLZ\" = $1 AND \"BIC\" IS NOT NULL AND \"BIC\" <> '' \
             ORDER BY \"Name\" LIMIT 1",
        )
        .bind(request.blz.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let (bic, institute, city): (Option<String>, Option<String>, Option<String>) = match row {
            Some(row) => (row.get("BIC"), row.get("Name"), row.get("Ort")),
            None => (None, None, None),
        };
        Ok(ConversionResult {
            iban,
            bank_address: match (institute.clone(), city) {
                (Some(name), Some(city)) => Some(format!("{name}, {city}")),
                _ => None,
            },
            bic,
            institute,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_and_postal_code_are_kept_in_sync() {
        // What gets written is the combined column of the legacy application.
        assert_eq!(
            ch_place(&Some("8001".into()), &Some("Zürich".into())).as_deref(),
            Some("8001 Zürich")
        );
        assert_eq!(
            ch_place(&Some("8001".into()), &None).as_deref(),
            Some("8001")
        );
        assert_eq!(ch_place(&None, &None), None);

        // And when reading, the post code drops out again.
        assert_eq!(
            city_of(Some("8001 Zürich".into()), &Some("8001".into())).as_deref(),
            Some("Zürich")
        );
        assert_eq!(
            city_of(Some("Zürich".into()), &Some("8001".into())).as_deref(),
            Some("Zürich")
        );
        assert_eq!(
            city_of(Some("8001".into()), &Some("8001".into())).as_deref(),
            Some("8001")
        );
        assert_eq!(city_of(None, &Some("8001".into())), None);
    }
}
