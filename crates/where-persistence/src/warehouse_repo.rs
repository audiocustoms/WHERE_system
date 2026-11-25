//! Warehouses and bin locations (`img/Admin Menü/Lager und Lagerplätze`).
//!
//! Three tabs, three tables:
//!
//! * `Lager` — `"Storage_Lager"`, a warehouse belongs to a tenant through
//!   `"Mandanten_ID"`. The seven switches of the mask live in `where_*` columns
//!   (migration `1860_warehouses.sql`).
//! * `Lagerplatz (Symetric)` — `"Lagerortung"`: the location based bin locations of a
//!   warehouse with short code and range of values.
//! * `Lagerplatz (Self defined)` — `"Lagerortung2"`: freely named bin locations of a
//!   warehouse with path and optional parent node.

use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;

/// One warehouse with the switches of the mask.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WarehouseAdminRow {
    pub id: i32,
    pub name: Option<String>,
    pub mandant_id: Option<i32>,
    pub is_default: bool,
    pub zugriff: bool,
    pub prioritaet: bool,
    pub such_std: bool,
    pub entnahme: bool,
    pub anmelden: bool,
    pub fremd: bool,
    pub konzern: bool,
}

/// The seven switches, as the mask writes them.
#[derive(Debug, Clone, Default)]
pub struct WarehouseFlags {
    pub zugriff: bool,
    pub prioritaet: bool,
    pub such_std: bool,
    pub entnahme: bool,
    pub anmelden: bool,
    pub fremd: bool,
    pub konzern: bool,
}

/// A "symmetric" bin location (`"Lagerortung"`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BinRow {
    pub id: i32,
    pub warehouse_id: i32,
    pub name: Option<String>,
    pub prefix: Option<String>,
    pub values: Option<String>,
    pub position: Option<i32>,
}

/// A "self defined" bin location (`"Lagerortung2"`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubBinRow {
    pub id: i32,
    pub warehouse_id: i32,
    pub label: Option<String>,
    pub path: Option<String>,
    pub parent_id: Option<i32>,
}

const WAREHOUSE_LIST_SQL: &str = r#"
SELECT "Primary_Key" AS id, "Name" AS name,
       CASE WHEN COALESCE("Mandanten_ID", 0) = 0 THEN NULL ELSE "Mandanten_ID" END AS mandant_id,
       COALESCE("__DefaultLager", false) AS is_default,
       COALESCE("where_zugriff", false) AS zugriff,
       COALESCE("where_prioritaet", false) AS prioritaet,
       COALESCE("where_such_std", false) AS such_std,
       COALESCE("where_entnahme", false) AS entnahme,
       COALESCE("where_anmelden", false) AS anmelden,
       COALESCE("where_fremd", false) AS fremd,
       COALESCE("where_konzern", false) AS konzern
  FROM "Storage_Lager"
 WHERE "Mandanten_ID" = $1 OR COALESCE("Mandanten_ID", 0) = 0
 ORDER BY COALESCE("__DefaultLager", false) DESC, "Name", "Primary_Key""#;

const WAREHOUSE_GET_SQL: &str = r#"
SELECT "Primary_Key" AS id, "Name" AS name,
       CASE WHEN COALESCE("Mandanten_ID", 0) = 0 THEN NULL ELSE "Mandanten_ID" END AS mandant_id,
       COALESCE("__DefaultLager", false) AS is_default,
       COALESCE("where_zugriff", false) AS zugriff,
       COALESCE("where_prioritaet", false) AS prioritaet,
       COALESCE("where_such_std", false) AS such_std,
       COALESCE("where_entnahme", false) AS entnahme,
       COALESCE("where_anmelden", false) AS anmelden,
       COALESCE("where_fremd", false) AS fremd,
       COALESCE("where_konzern", false) AS konzern
  FROM "Storage_Lager"
 WHERE "Primary_Key" = $1"#;

const BIN_LIST_SQL: &str = r#"
SELECT "Primary_Key" AS id, COALESCE("Lager_Primkey", 0) AS warehouse_id,
       "Name" AS name, "Prefix" AS prefix, "Wertebereich" AS values, "Position" AS position
  FROM "Lagerortung"
 WHERE "Lager_Primkey" = $1
 ORDER BY "Position" NULLS LAST, "Primary_Key""#;

const BIN_GET_SQL: &str = r#"
SELECT "Primary_Key" AS id, COALESCE("Lager_Primkey", 0) AS warehouse_id,
       "Name" AS name, "Prefix" AS prefix, "Wertebereich" AS values, "Position" AS position
  FROM "Lagerortung"
 WHERE "Primary_Key" = $1"#;

const SUB_BIN_LIST_SQL: &str = r#"
SELECT "Primary_Key" AS id, COALESCE("Lager_Primkey", 0) AS warehouse_id,
       "Index" AS label, "Pfad" AS path,
       CASE WHEN COALESCE("Parent_ID", 0) = 0 THEN NULL ELSE "Parent_ID" END AS parent_id
  FROM "Lagerortung2"
 WHERE "Lager_Primkey" = $1
 ORDER BY "Index", "Primary_Key""#;

const SUB_BIN_GET_SQL: &str = r#"
SELECT "Primary_Key" AS id, COALESCE("Lager_Primkey", 0) AS warehouse_id,
       "Index" AS label, "Pfad" AS path,
       CASE WHEN COALESCE("Parent_ID", 0) = 0 THEN NULL ELSE "Parent_ID" END AS parent_id
  FROM "Lagerortung2"
 WHERE "Primary_Key" = $1"#;

/// Maintains warehouses and their bin locations.
#[derive(Clone)]
pub struct WarehouseRepository {
    pool: PgPool,
}

impl WarehouseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Warehouses of one tenant — plus those not assigned yet (`"Mandanten_ID" IS
    /// NULL`), so they can be assigned in the mask. Without a tenant there is
    /// no list.
    pub async fn list(&self, mandant: i64) -> Result<Vec<WarehouseAdminRow>, DomainError> {
        sqlx::query_as::<_, WarehouseAdminRow>(WAREHOUSE_LIST_SQL)
            .bind(mandant as i32)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn get(&self, id: i32) -> Result<WarehouseAdminRow, DomainError> {
        sqlx::query_as::<_, WarehouseAdminRow>(WAREHOUSE_GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Warehouse",
                id,
            })
    }

    /// Creates a warehouse for the tenant. The first warehouse of a tenant becomes
    /// its default.
    pub async fn create(&self, mandant: i64, name: &str) -> Result<WarehouseAdminRow, DomainError> {
        let name = Self::validate_name(name)?;
        let (id,): (i32,) = sqlx::query_as(
            r#"INSERT INTO "Storage_Lager" ("Name", "Mandanten_ID", "__DefaultLager")
               VALUES ($1, $2,
                       NOT EXISTS (SELECT 1 FROM "Storage_Lager" WHERE "Mandanten_ID" = $2))
               RETURNING "Primary_Key""#,
        )
        .bind(&name)
        .bind(mandant as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(
        &self,
        id: i32,
        name: &str,
        flags: &WarehouseFlags,
    ) -> Result<WarehouseAdminRow, DomainError> {
        self.save(id, name, flags, None).await
    }

    /// Assigns a warehouse to a tenant (the legacy warehouses carry no
    /// `"Mandanten_ID"`); the switches stay as they are.
    pub async fn assign(&self, id: i32, mandant: i64) -> Result<WarehouseAdminRow, DomainError> {
        sqlx::query(r#"UPDATE "Storage_Lager" SET "Mandanten_ID" = $2 WHERE "Primary_Key" = $1"#)
            .bind(id)
            .bind(mandant as i32)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        self.get(id).await
    }

    async fn save(
        &self,
        id: i32,
        name: &str,
        flags: &WarehouseFlags,
        mandant: Option<i64>,
    ) -> Result<WarehouseAdminRow, DomainError> {
        let name = Self::validate_name(name)?;
        let row = sqlx::query_as::<_, (i32,)>(
            r#"UPDATE "Storage_Lager"
                  SET "Name" = $2,
                      "where_zugriff" = $3,
                      "where_prioritaet" = $4,
                      "where_such_std" = $5,
                      "where_entnahme" = $6,
                      "where_anmelden" = $7,
                      "where_fremd" = $8,
                      "where_konzern" = $9,
                      "Mandanten_ID" = COALESCE($10, "Mandanten_ID")
                WHERE "Primary_Key" = $1
            RETURNING "Primary_Key""#,
        )
        .bind(id)
        .bind(&name)
        .bind(flags.zugriff)
        .bind(flags.prioritaet)
        .bind(flags.such_std)
        .bind(flags.entnahme)
        .bind(flags.anmelden)
        .bind(flags.fremd)
        .bind(flags.konzern)
        .bind(mandant.map(|m| m as i32))
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        if row.is_none() {
            return Err(DomainError::NotFound {
                entity: "Warehouse",
                id,
            });
        }
        self.get(id).await
    }

    /// Deletes a warehouse only when no devices and no bin locations hang on it —
    /// the message names the reason.
    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        self.get(id).await?;
        let (geraete,): (i64,) =
            sqlx::query_as(r#"SELECT count(*) FROM "Seriennummern" WHERE "Lager_Primkey" = $1"#)
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?;
        if geraete > 0 {
            return Err(DomainError::Conflict(format!(
                "Das Lager enthält {geraete} Gerät(e) und kann nicht gelöscht werden"
            )));
        }
        let (plaetze,): (i64,) = sqlx::query_as(
            r#"SELECT (SELECT count(*) FROM "Lagerortung" WHERE "Lager_Primkey" = $1)
                    + (SELECT count(*) FROM "Lagerortung2" WHERE "Lager_Primkey" = $1)"#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        if plaetze > 0 {
            return Err(DomainError::Conflict(format!(
                "Am Lager hängen {plaetze} Lagerplätze — bitte zuerst entfernen"
            )));
        }
        sqlx::query(r#"DELETE FROM "Storage_Lager" WHERE "Primary_Key" = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    // ---------------------------------------------------------- Bin locations

    pub async fn bins(&self, warehouse: i32) -> Result<Vec<BinRow>, DomainError> {
        sqlx::query_as::<_, BinRow>(BIN_LIST_SQL)
            .bind(warehouse)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn bin(&self, id: i32) -> Result<BinRow, DomainError> {
        sqlx::query_as::<_, BinRow>(BIN_GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Bin location",
                id,
            })
    }

    /// Creates a bin location; it is appended behind the last one of its warehouse.
    pub async fn bin_create(&self, warehouse: i32, name: &str) -> Result<BinRow, DomainError> {
        self.get(warehouse).await?;
        let position: i32 = sqlx::query_scalar(
            r#"SELECT COALESCE(max("Position"), 0) + 1 FROM "Lagerortung" WHERE "Lager_Primkey" = $1"#,
        )
        .bind(warehouse)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        let (id,): (i32,) = sqlx::query_as(
            r#"INSERT INTO "Lagerortung" ("Lager_Primkey", "Position", "Name", "Prefix", "Wertebereich")
               VALUES ($1, $2, $3, '', '')
               RETURNING "Primary_Key""#,
        )
        .bind(warehouse)
        .bind(position)
        .bind(name.trim())
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        self.bin(id).await
    }

    pub async fn bin_update(
        &self,
        id: i32,
        name: &str,
        prefix: &str,
        values: &str,
    ) -> Result<BinRow, DomainError> {
        let row = sqlx::query_as::<_, (i32,)>(
            r#"UPDATE "Lagerortung"
                  SET "Name" = $2, "Prefix" = $3, "Wertebereich" = $4
                WHERE "Primary_Key" = $1
            RETURNING "Primary_Key""#,
        )
        .bind(id)
        .bind(name.trim())
        .bind(prefix.trim())
        .bind(values.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        if row.is_none() {
            return Err(DomainError::NotFound {
                entity: "Bin location",
                id,
            });
        }
        self.bin(id).await
    }

    pub async fn bin_delete(&self, id: i32) -> Result<(), DomainError> {
        let result = sqlx::query(r#"DELETE FROM "Lagerortung" WHERE "Primary_Key" = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "Bin location",
                id,
            });
        }
        Ok(())
    }

    // ------------------------------------------- Bin locations (freely named)

    pub async fn sub_bins(&self, warehouse: i32) -> Result<Vec<SubBinRow>, DomainError> {
        sqlx::query_as::<_, SubBinRow>(SUB_BIN_LIST_SQL)
            .bind(warehouse)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn sub_bin(&self, id: i32) -> Result<SubBinRow, DomainError> {
        sqlx::query_as::<_, SubBinRow>(SUB_BIN_GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Bin location",
                id,
            })
    }

    pub async fn sub_bin_create(
        &self,
        warehouse: i32,
        label: &str,
    ) -> Result<SubBinRow, DomainError> {
        self.get(warehouse).await?;
        let label = if label.trim().is_empty() {
            "Neuer Lagerplatz".to_string()
        } else {
            label.trim().to_string()
        };
        let (id,): (i32,) = sqlx::query_as(
            r#"INSERT INTO "Lagerortung2" ("Lager_Primkey", "Index", "Pfad", "Parent_ID", "Child_ID")
               VALUES ($1, $2, $2, 0, 0)
               RETURNING "Primary_Key""#,
        )
        .bind(warehouse)
        .bind(&label)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        self.sub_bin(id).await
    }

    pub async fn sub_bin_update(&self, id: i32, label: &str) -> Result<SubBinRow, DomainError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(DomainError::Validation(
                "Der Name des Lagerplatzes darf nicht leer sein".into(),
            ));
        }
        let row = sqlx::query_as::<_, (i32,)>(
            r#"UPDATE "Lagerortung2"
                  SET "Index" = $2, "Pfad" = $2
                WHERE "Primary_Key" = $1
            RETURNING "Primary_Key""#,
        )
        .bind(id)
        .bind(label)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        if row.is_none() {
            return Err(DomainError::NotFound {
                entity: "Bin location",
                id,
            });
        }
        self.sub_bin(id).await
    }

    pub async fn sub_bin_delete(&self, id: i32) -> Result<(), DomainError> {
        let (kinder,): (i64,) =
            sqlx::query_as(r#"SELECT count(*) FROM "Lagerortung2" WHERE "Parent_ID" = $1"#)
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?;
        if kinder > 0 {
            return Err(DomainError::Conflict(format!(
                "Der Lagerplatz hat {kinder} Unterlagerplätze"
            )));
        }
        let result = sqlx::query(r#"DELETE FROM "Lagerortung2" WHERE "Primary_Key" = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "Bin location",
                id,
            });
        }
        Ok(())
    }

    fn validate_name(name: &str) -> Result<String, DomainError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Der Name des Lagers darf nicht leer sein".into(),
            ));
        }
        if name.chars().count() > 80 {
            return Err(DomainError::Validation(
                "Der Name des Lagers darf höchstens 80 Zeichen haben".into(),
            ));
        }
        Ok(name.to_string())
    }
}
