//! Tenants of the installation (`img/Admin Menü/Mandanten`).
//!
//! One row per client company in the legacy table `"Mandanten"`; the linked address
//! of the `Verlinkte Adresse` line is read from `"Adresse"`.
//!
//! Each tenant is a **database of its own** — the legacy schema carries no tenant column
//! in `"Adresse"`, `"Artikel"`, … and this repository filters nothing by tenant. The rows
//! describe the companies of this installation (own company, sister houses, the partners
//! of the stammdaten sync) and the assignment of the users (`"User_Mandanten"`).

use sqlx::{PgPool, Row};
use where_domain::{DomainError, Tenant, TenantInput};

use crate::error::db_error;

const COLUMNS: &str = "m.\"Primary_Key\"::bigint AS id, m.\"Shortname\" AS short_name, \
     COALESCE(m.\"Mandanten_Name\", '') AS name, COALESCE(m.\"Mandanten_ID\", 0) AS tenant_id, \
     COALESCE(m.\"SyncID\", 0) AS sync_id, m.\"Parent\" AS parent, \
     m.\"web_identifier\" AS web_identifier, m.\"Waehrung\" AS currency, \
     m.\"Waehrung_PK\" AS currency_id, m.\"Drucksprache\" AS print_language, \
     m.\"Drucksprache_PK\" AS print_language_id, COALESCE(m.\"HideAktions\", false) AS hide_actions, \
     COALESCE(m.\"Color\", 0) AS color, COALESCE(m.\"Color_ON\", false) AS color_on, \
     m.\"adr_adressZusatz\" AS address_extra, m.\"adr_adressStrasse\" AS street, \
     m.\"adr_adressPLZ\" AS postal_code, m.\"adr_adressOrt\" AS city, \
     m.\"adr_adressLand_ISO3\" AS country_iso3, m.\"adr_adressLand\" AS country, \
     m.\"adr_adressBundesland\" AS state, m.\"adr_comTelefon\" AS phone, m.\"adr_comFax\" AS fax, \
     m.\"adr_comEmail\" AS email, m.\"adr_comEmailBuchhaltung\" AS email_accounting, \
     m.\"adr_comInternet\" AS website, m.\"Ustid\" AS vat_id, m.\"FANr\" AS tax_number, \
     m.\"HRB\" AS register_entry, m.\"HRB_Sitz\" AS register_place, \
     NULLIF(m.\"adr_LinkAddressPK\", 0) AS linked_address, a.\"Name1\" AS linked_address_name";

const FROM: &str =
    "\"Mandanten\" m LEFT JOIN \"Adresse\" a ON a.\"Primary_key\" = m.\"adr_LinkAddressPK\"";

/// The tenant a fresh installation brings along.
pub const INITIAL_TENANT_NAME: &str = "Standard";

/// Creates the first tenant when the installation has none.
///
/// The login mask picks a tenant before it lists accounts, and every working page
/// needs one, so an installation without a tenant cannot be entered. Together with
/// the initial administrator this is the bootstrap of a new database; the
/// administrator renames the tenant under `Admin → Mandanten`.
///
/// Returns `true` when the tenant was created.
pub async fn ensure_initial_tenant(pool: &PgPool) -> Result<bool, DomainError> {
    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM \"Mandanten\"")
        .fetch_one(pool)
        .await
        .map_err(db_error)?;
    if existing > 0 {
        return Ok(false);
    }
    TenantRepository::new(pool.clone())
        .create(TenantInput {
            name: INITIAL_TENANT_NAME.to_string(),
            ..TenantInput::default()
        })
        .await?;
    Ok(true)
}

fn row_to_tenant(row: &sqlx::postgres::PgRow) -> Tenant {
    Tenant {
        id: row.get("id"),
        short_name: row.get("short_name"),
        name: row.get("name"),
        tenant_id: row.get("tenant_id"),
        sync_id: row.get("sync_id"),
        parent: row.get("parent"),
        web_identifier: row.get("web_identifier"),
        currency: row.get("currency"),
        currency_id: row.get("currency_id"),
        print_language: row.get("print_language"),
        print_language_id: row.get("print_language_id"),
        hide_actions: row.get("hide_actions"),
        color: row.get("color"),
        color_on: row.get("color_on"),
        address_extra: row.get("address_extra"),
        street: row.get("street"),
        postal_code: row.get("postal_code"),
        city: row.get("city"),
        country_iso3: row.get("country_iso3"),
        country: row.get("country"),
        state: row.get("state"),
        phone: row.get("phone"),
        fax: row.get("fax"),
        email: row.get("email"),
        email_accounting: row.get("email_accounting"),
        website: row.get("website"),
        vat_id: row.get("vat_id"),
        tax_number: row.get("tax_number"),
        register_entry: row.get("register_entry"),
        register_place: row.get("register_place"),
        linked_address: row.get("linked_address"),
        linked_address_name: row.get("linked_address_name"),
    }
}

/// Reads and writes the tenants of the installation.
#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// All tenants, by company name.
    pub async fn list(&self) -> Result<Vec<Tenant>, DomainError> {
        let sql = format!("SELECT {COLUMNS} FROM {FROM} ORDER BY m.\"Mandanten_Name\" NULLS LAST");
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.iter().map(row_to_tenant).collect())
    }

    pub async fn get(&self, id: i64) -> Result<Tenant, DomainError> {
        let sql = format!("SELECT {COLUMNS} FROM {FROM} WHERE m.\"Primary_Key\" = $1");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        row.as_ref()
            .map(row_to_tenant)
            .ok_or(DomainError::NotFound {
                entity: "Mandant",
                id: id as i32,
            })
    }

    pub async fn create(&self, input: TenantInput) -> Result<Tenant, DomainError> {
        let tenant = input.prepare()?;
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO \"Mandanten\" (\"Mandanten_Name\", \"Shortname\", \"Mandanten_ID\", \
             \"SyncID\", \"Parent\", \"web_identifier\", \"Waehrung\", \"Waehrung_PK\", \
             \"Drucksprache\", \"Drucksprache_PK\", \"HideAktions\", \"Color\", \"Color_ON\", \
             \"adr_adressZusatz\", \"adr_adressStrasse\", \"adr_adressPLZ\", \"adr_adressOrt\", \
             \"adr_adressLand_ISO3\", \"adr_adressLand\", \"adr_adressBundesland\", \
             \"adr_comTelefon\", \"adr_comFax\", \"adr_comEmail\", \"adr_comEmailBuchhaltung\", \
             \"adr_comInternet\", \"Ustid\", \"FANr\", \"HRB\", \"HRB_Sitz\", \"adr_LinkAddressPK\") \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
                     $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30) \
             RETURNING \"Primary_Key\"::bigint",
        )
        .bind(&tenant.name)
        .bind(&tenant.short_name)
        .bind(tenant.tenant_id)
        .bind(tenant.sync_id)
        .bind(tenant.parent)
        .bind(&tenant.web_identifier)
        .bind(&tenant.currency)
        .bind(tenant.currency_id)
        .bind(&tenant.print_language)
        .bind(tenant.print_language_id)
        .bind(tenant.hide_actions)
        .bind(tenant.color)
        .bind(tenant.color_on)
        .bind(&tenant.address_extra)
        .bind(&tenant.street)
        .bind(&tenant.postal_code)
        .bind(&tenant.city)
        .bind(&tenant.country_iso3)
        .bind(&tenant.country)
        .bind(&tenant.state)
        .bind(&tenant.phone)
        .bind(&tenant.fax)
        .bind(&tenant.email)
        .bind(&tenant.email_accounting)
        .bind(&tenant.website)
        .bind(&tenant.vat_id)
        .bind(&tenant.tax_number)
        .bind(&tenant.register_entry)
        .bind(&tenant.register_place)
        .bind(tenant.linked_address)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(&self, id: i64, input: TenantInput) -> Result<Tenant, DomainError> {
        let mut tenant = input.prepare()?;
        if tenant.parent == Some(id as i32) {
            tenant.parent = None;
        }
        let changed = sqlx::query(
            "UPDATE \"Mandanten\" SET \"Mandanten_Name\" = $2, \"Shortname\" = $3, \
             \"Mandanten_ID\" = $4, \"SyncID\" = $5, \"Parent\" = $6, \"web_identifier\" = $7, \
             \"Waehrung\" = $8, \"Waehrung_PK\" = $9, \"Drucksprache\" = $10, \
             \"Drucksprache_PK\" = $11, \"HideAktions\" = $12, \"Color\" = $13, \
             \"Color_ON\" = $14, \"adr_adressZusatz\" = $15, \"adr_adressStrasse\" = $16, \
             \"adr_adressPLZ\" = $17, \"adr_adressOrt\" = $18, \"adr_adressLand_ISO3\" = $19, \
             \"adr_adressLand\" = $20, \"adr_adressBundesland\" = $21, \"adr_comTelefon\" = $22, \
             \"adr_comFax\" = $23, \"adr_comEmail\" = $24, \"adr_comEmailBuchhaltung\" = $25, \
             \"adr_comInternet\" = $26, \"Ustid\" = $27, \"FANr\" = $28, \"HRB\" = $29, \
             \"HRB_Sitz\" = $30, \"adr_LinkAddressPK\" = $31 WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .bind(&tenant.name)
        .bind(&tenant.short_name)
        .bind(tenant.tenant_id)
        .bind(tenant.sync_id)
        .bind(tenant.parent)
        .bind(&tenant.web_identifier)
        .bind(&tenant.currency)
        .bind(tenant.currency_id)
        .bind(&tenant.print_language)
        .bind(tenant.print_language_id)
        .bind(tenant.hide_actions)
        .bind(tenant.color)
        .bind(tenant.color_on)
        .bind(&tenant.address_extra)
        .bind(&tenant.street)
        .bind(&tenant.postal_code)
        .bind(&tenant.city)
        .bind(&tenant.country_iso3)
        .bind(&tenant.country)
        .bind(&tenant.state)
        .bind(&tenant.phone)
        .bind(&tenant.fax)
        .bind(&tenant.email)
        .bind(&tenant.email_accounting)
        .bind(&tenant.website)
        .bind(&tenant.vat_id)
        .bind(&tenant.tax_number)
        .bind(&tenant.register_entry)
        .bind(&tenant.register_place)
        .bind(tenant.linked_address)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Mandant",
                id: id as i32,
            });
        }
        self.get(id).await
    }

    pub async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let changed = sqlx::query("DELETE FROM \"Mandanten\" WHERE \"Primary_Key\" = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Mandant",
                id: id as i32,
            });
        }
        Ok(())
    }
}
