//! Storage of the disposition flags (`where_people_flags`) and their staffing
//! (`where_people_flag_staffing`).

use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::flags::{CleanFlag, Flag, FlagPresetReport, FlagStaffing, PRESETS};

const SELECT_FLAGS: &str = "SELECT f.\"id\", f.\"code\", f.\"name\", f.\"sort_order\", \
     s.\"location_id\", s.\"position\", s.\"needed\" \
     FROM \"where_people_flags\" f \
     LEFT JOIN \"where_people_flag_staffing\" s ON s.\"flag_id\" = f.\"id\" \
     ORDER BY f.\"sort_order\", f.\"id\", s.\"sort_order\", s.\"id\"";

const SELECT_FLAG: &str = "SELECT \"id\", \"code\", \"name\", \"sort_order\" \
     FROM \"where_people_flags\" WHERE \"id\" = $1";

const INSERT_FLAG: &str = "INSERT INTO \"where_people_flags\" (\"code\", \"name\", \"sort_order\") \
     SELECT $1, $2, coalesce(max(\"sort_order\") + 1, 0) FROM \"where_people_flags\" \
     RETURNING \"id\", \"code\", \"name\", \"sort_order\"";

const RENAME_FLAG: &str = "UPDATE \"where_people_flags\" \
     SET \"code\" = $2, \"name\" = $3, \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"code\", \"name\", \"sort_order\"";

const DELETE_FLAG: &str = "DELETE FROM \"where_people_flags\" WHERE \"id\" = $1";

const CLEAR_STAFFING: &str = "DELETE FROM \"where_people_flag_staffing\" \
     WHERE \"flag_id\" = $1 AND \"location_id\" = $2";

const INSERT_STAFFING: &str = "INSERT INTO \"where_people_flag_staffing\" \
     (\"flag_id\", \"location_id\", \"position\", \"needed\", \"sort_order\") \
     VALUES ($1, $2, $3, $4, $5)";

const SELECT_STAFFING: &str = "SELECT \"location_id\", \"position\", \"needed\" \
     FROM \"where_people_flag_staffing\" WHERE \"flag_id\" = $1 \
     ORDER BY \"location_id\", \"sort_order\", \"id\"";

const SELECT_LOCATIONS: &str = "SELECT \"id\" FROM \"where_people_locations\" \
     ORDER BY \"sort_order\", \"id\"";

/// Houses nobody has given a crew yet — the ones an import may equip from the legend.
const SELECT_UNSTAFFED_LOCATIONS: &str = "SELECT l.\"id\" FROM \"where_people_locations\" l \
     WHERE NOT EXISTS (SELECT 1 FROM \"where_people_flag_staffing\" s \
     WHERE s.\"location_id\" = l.\"id\") ORDER BY l.\"sort_order\", l.\"id\"";

const SELECT_LOCATION_EXISTS: &str = "SELECT 1 FROM \"where_people_locations\" WHERE \"id\" = $1";

const SELECT_CODES: &str = "SELECT \"code\" FROM \"where_people_flags\" \
     ORDER BY \"sort_order\", \"id\"";

const SELECT_FLAG_BY_CODE: &str = "SELECT \"id\" FROM \"where_people_flags\" \
     WHERE lower(btrim(\"code\")) = lower(btrim($1))";

const SELECT_EXCLUSIONS: &str = "SELECT \"flag_id\", \"position\" \
     FROM \"where_people_flag_excluded_positions\" ORDER BY \"flag_id\", \"sort_order\", \"id\"";

const SELECT_EXCLUSIONS_OF: &str = "SELECT \"position\" \
     FROM \"where_people_flag_excluded_positions\" WHERE \"flag_id\" = $1 \
     ORDER BY \"sort_order\", \"id\"";

const CLEAR_EXCLUSIONS: &str =
    "DELETE FROM \"where_people_flag_excluded_positions\" WHERE \"flag_id\" = $1";

const INSERT_EXCLUSION: &str = "INSERT INTO \"where_people_flag_excluded_positions\" \
     (\"flag_id\", \"position\", \"sort_order\") VALUES ($1, $2, $3)";

/// Reads and writes the flags of the disposition.
#[derive(Clone)]
pub struct FlagRepository {
    pool: PgPool,
}

impl FlagRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Every flag with its staffing, in the maintained order.
    pub async fn list(&self) -> Result<Vec<Flag>, DomainError> {
        let rows = sqlx::query(SELECT_FLAGS)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut flags: Vec<Flag> = Vec::new();
        for row in rows.iter() {
            let id: i64 = row.get("id");
            let location_id: Option<i64> = row.get("location_id");
            let position: Option<String> = row.get("position");
            let needed: Option<i32> = row.get("needed");
            match flags.last_mut() {
                Some(last) if last.id == id => {}
                _ => flags.push(Flag {
                    id,
                    code: row.get("code"),
                    name: row.get("name"),
                    sort_order: row.get("sort_order"),
                    staffing: Vec::new(),
                    excluded_positions: Vec::new(),
                }),
            }
            if let (Some(location_id), Some(position), Some(needed)) =
                (location_id, position, needed)
            {
                if let Some(last) = flags.last_mut() {
                    last.staffing.push(FlagStaffing {
                        location_id,
                        position,
                        needed,
                    });
                }
            }
        }
        let exclusions = sqlx::query(SELECT_EXCLUSIONS)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        for row in exclusions.iter() {
            let flag_id: i64 = row.get("flag_id");
            if let Some(flag) = flags.iter_mut().find(|flag| flag.id == flag_id) {
                flag.excluded_positions.push(row.get("position"));
            }
        }
        Ok(flags)
    }

    /// The codes of all flags — what the import recognises in the event texts.
    pub async fn codes(&self) -> Result<Vec<String>, DomainError> {
        sqlx::query_scalar(SELECT_CODES)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// The id of the flag with that code, ignoring case.
    pub async fn id_by_code(&self, code: &str) -> Result<Option<i64>, DomainError> {
        sqlx::query_scalar(SELECT_FLAG_BY_CODE)
            .bind(code)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Creates a flag without staffing.
    pub async fn create(&self, flag: &CleanFlag) -> Result<Flag, DomainError> {
        let row = sqlx::query(INSERT_FLAG)
            .bind(&flag.code)
            .bind(&flag.name)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(Flag {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            sort_order: row.get("sort_order"),
            staffing: Vec::new(),
            excluded_positions: Vec::new(),
        })
    }

    /// Renames a flag; `None` when the id is unknown.
    pub async fn rename(&self, id: i64, flag: &CleanFlag) -> Result<Option<Flag>, DomainError> {
        let row = sqlx::query(RENAME_FLAG)
            .bind(id)
            .bind(&flag.code)
            .bind(&flag.name)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(Flag {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            sort_order: row.get("sort_order"),
            staffing: self.staffing(id).await?,
            excluded_positions: self.exclusions(id).await?,
        }))
    }

    /// The positions this flag never uses.
    pub async fn exclusions(&self, id: i64) -> Result<Vec<String>, DomainError> {
        sqlx::query_scalar(SELECT_EXCLUSIONS_OF)
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Replaces the excluded positions of a flag; `None` when the id is unknown.
    pub async fn set_exclusions(
        &self,
        id: i64,
        positions: &[String],
    ) -> Result<Option<Vec<String>>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let known = sqlx::query(SELECT_FLAG)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        if known.is_none() {
            return Ok(None);
        }
        sqlx::query(CLEAR_EXCLUSIONS)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for (order, position) in positions.iter().enumerate() {
            sqlx::query(INSERT_EXCLUSION)
                .bind(id)
                .bind(position)
                .bind(order as i32)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        tx.commit().await.map_err(db_error)?;
        Ok(Some(self.exclusions(id).await?))
    }

    /// Deletes a flag; the entries that carried it lose it.
    pub async fn delete(&self, id: i64) -> Result<bool, DomainError> {
        let done = sqlx::query(DELETE_FLAG)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(done.rows_affected() > 0)
    }

    /// The staffing of one flag.
    pub async fn staffing(&self, id: i64) -> Result<Vec<FlagStaffing>, DomainError> {
        let rows = sqlx::query(SELECT_STAFFING)
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .iter()
            .map(|row| FlagStaffing {
                location_id: row.get("location_id"),
                position: row.get("position"),
                needed: row.get("needed"),
            })
            .collect())
    }

    /// Replaces the staffing of a flag for one location and answers with all its rows
    /// — every house — `None` when the flag is unknown.
    pub async fn replace_staffing(
        &self,
        id: i64,
        location_id: i64,
        rows: &[FlagStaffing],
    ) -> Result<Option<Vec<FlagStaffing>>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let known = sqlx::query(SELECT_FLAG)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        if known.is_none() {
            return Ok(None);
        }
        let house = sqlx::query(SELECT_LOCATION_EXISTS)
            .bind(location_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        if house.is_none() {
            return Err(DomainError::Validation("Unknown location.".to_string()));
        }
        sqlx::query(CLEAR_STAFFING)
            .bind(id)
            .bind(location_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for (order, row) in rows.iter().enumerate() {
            sqlx::query(INSERT_STAFFING)
                .bind(id)
                .bind(location_id)
                .bind(&row.position)
                .bind(row.needed)
                .bind(order as i32)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        tx.commit().await.map_err(db_error)?;
        Ok(Some(self.staffing(id).await?))
    }

    /// Creates every flag of [`PRESETS`] that does not exist yet, together with its
    /// staffing in every location. Known flags keep what they have.
    pub async fn create_presets(&self) -> Result<FlagPresetReport, DomainError> {
        let codes: Vec<String> = PRESETS
            .iter()
            .map(|preset| preset.code.to_string())
            .collect();
        self.create_presets_of(&codes).await
    }

    /// The same for the given codes, so an import can bring in only the flags its
    /// disposition uses. Codes the legend does not know are skipped.
    pub async fn create_presets_of(
        &self,
        codes: &[String],
    ) -> Result<FlagPresetReport, DomainError> {
        let locations: Vec<i64> = sqlx::query_scalar(SELECT_LOCATIONS)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut created = 0;
        let mut kept = 0;
        for code in codes {
            let Some(preset) = crate::flags::preset(code) else {
                continue;
            };
            if self.id_by_code(preset.code).await?.is_some() {
                kept += 1;
                continue;
            }
            let flag = self
                .create(&CleanFlag {
                    code: preset.code.to_string(),
                    name: preset.name.to_string(),
                })
                .await?;
            for location_id in &locations {
                let staffing: Vec<FlagStaffing> = preset
                    .staffing
                    .iter()
                    .map(|(position, needed)| FlagStaffing {
                        location_id: *location_id,
                        position: (*position).to_string(),
                        needed: *needed,
                    })
                    .collect();
                self.replace_staffing(flag.id, *location_id, &staffing)
                    .await?;
            }
            created += 1;
        }
        Ok(FlagPresetReport { created, kept })
    }

    /// Gives every house without any crew the staff of the legend for these flags. An
    /// import calls this after it has created the houses of its disposition; a house
    /// that was configured once keeps what it has.
    pub async fn staff_presets_of(&self, codes: &[String]) -> Result<(), DomainError> {
        let locations: Vec<i64> = sqlx::query_scalar(SELECT_UNSTAFFED_LOCATIONS)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        if locations.is_empty() {
            return Ok(());
        }
        for code in codes {
            let Some(preset) = crate::flags::preset(code) else {
                continue;
            };
            let Some(id) = self.id_by_code(preset.code).await? else {
                continue;
            };
            for location_id in &locations {
                let staffing: Vec<FlagStaffing> = preset
                    .staffing
                    .iter()
                    .map(|(position, needed)| FlagStaffing {
                        location_id: *location_id,
                        position: (*position).to_string(),
                        needed: *needed,
                    })
                    .collect();
                self.replace_staffing(id, *location_id, &staffing).await?;
            }
        }
        Ok(())
    }
}
