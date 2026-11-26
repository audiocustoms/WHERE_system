//! Storage of the personnel lists (`where_people_departments`,
//! `where_people_subdepartments`, `where_people_positions`, `where_people_locations`).

use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::lists::{ListEntry, ListEntryInput, ListKind};

const SELECT_DEPARTMENTS: &str = "SELECT \"id\", \"name\", \"color\" FROM \"where_people_departments\" \
     ORDER BY \"sort_order\", lower(\"name\")";
const SELECT_SUBDEPARTMENTS: &str = "SELECT \"id\", \"name\", \"department\", \"color\" \
     FROM \"where_people_subdepartments\" \
     ORDER BY \"sort_order\", lower(\"name\")";
const SELECT_POSITIONS: &str = "SELECT \"id\", \"name\", \"color\" \
     FROM \"where_people_positions\" \
     ORDER BY \"sort_order\", lower(\"name\")";

const INSERT_DEPARTMENTS: &str = "INSERT INTO \"where_people_departments\" \
     (\"name\", \"sort_order\") \
     SELECT $1, coalesce(max(\"sort_order\") + 1, 0) FROM \"where_people_departments\" \
     RETURNING \"id\", \"name\", \"color\"";
const INSERT_SUBDEPARTMENTS: &str = "INSERT INTO \"where_people_subdepartments\" \
     (\"name\", \"sort_order\") \
     SELECT $1, coalesce(max(\"sort_order\") + 1, 0) FROM \"where_people_subdepartments\" \
     RETURNING \"id\", \"name\", \"department\", \"color\"";
const INSERT_POSITIONS: &str = "INSERT INTO \"where_people_positions\" \
     (\"name\", \"sort_order\") \
     SELECT $1, coalesce(max(\"sort_order\") + 1, 0) FROM \"where_people_positions\" \
     RETURNING \"id\", \"name\", \"color\"";

const RENAME_DEPARTMENTS: &str = "UPDATE \"where_people_departments\" SET \"name\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 RETURNING \"id\", \"name\", \"color\"";
const RENAME_SUBDEPARTMENTS: &str = "UPDATE \"where_people_subdepartments\" SET \"name\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"department\", \"color\"";
const RENAME_POSITIONS: &str = "UPDATE \"where_people_positions\" SET \"name\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 RETURNING \"id\", \"name\", \"color\"";

const SET_DEPARTMENT_COLOR: &str = "UPDATE \"where_people_departments\" SET \"color\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 RETURNING \"id\", \"name\", \"color\"";
const SET_SUBDEPARTMENT_COLOR: &str = "UPDATE \"where_people_subdepartments\" \
     SET \"color\" = $2, \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"department\", \"color\"";
const SET_POSITION_COLOR: &str = "UPDATE \"where_people_positions\" SET \"color\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 RETURNING \"id\", \"name\", \"color\"";
const SET_LOCATION_COLOR: &str = "UPDATE \"where_people_locations\" SET \"color\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \"is_text\"";

const DELETE_DEPARTMENTS: &str = "DELETE FROM \"where_people_departments\" WHERE \"id\" = $1";
const DELETE_SUBDEPARTMENTS: &str = "DELETE FROM \"where_people_subdepartments\" WHERE \"id\" = $1";
const DELETE_POSITIONS: &str = "DELETE FROM \"where_people_positions\" WHERE \"id\" = $1";

const SELECT_NAME_DEPARTMENTS: &str =
    "SELECT \"name\" FROM \"where_people_departments\" WHERE \"id\" = $1";
const SELECT_NAME_SUBDEPARTMENTS: &str =
    "SELECT \"name\" FROM \"where_people_subdepartments\" WHERE \"id\" = $1";
const SELECT_NAME_POSITIONS: &str =
    "SELECT \"name\" FROM \"where_people_positions\" WHERE \"id\" = $1";
const SELECT_NAME_LOCATIONS: &str =
    "SELECT \"name\" FROM \"where_people_locations\" WHERE \"id\" = $1";

const SELECT_SUBDEPARTMENT_PARENT: &str =
    "SELECT \"department\" FROM \"where_people_subdepartments\" WHERE \"id\" = $1";
const SET_SUBDEPARTMENT_PARENT: &str = "UPDATE \"where_people_subdepartments\" \
     SET \"department\" = $2, \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"department\", \"color\"";

const SELECT_LOCATIONS: &str = "SELECT \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \
            \"is_text\" \
     FROM \"where_people_locations\" \
     ORDER BY \"sort_order\", lower(\"name\")";
const SET_LOCATION_DEPARTMENT: &str = "UPDATE \"where_people_locations\" \
     SET \"department\" = $2, \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \"is_text\"";
const SET_LOCATION_FOREMAN: &str = "UPDATE \"where_people_locations\" \
     SET \"needs_foreman\" = $2, \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \"is_text\"";
const INSERT_LOCATIONS: &str = "INSERT INTO \"where_people_locations\" \
     (\"name\", \"sort_order\") \
     SELECT $1, coalesce(max(\"sort_order\") + 1, 0) FROM \"where_people_locations\" \
     RETURNING \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \"is_text\"";
const RENAME_LOCATIONS: &str = "UPDATE \"where_people_locations\" SET \"name\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \"is_text\"";
const DELETE_LOCATIONS: &str = "DELETE FROM \"where_people_locations\" WHERE \"id\" = $1";
const REORDER_LOCATIONS: &str = "UPDATE \"where_people_locations\" SET \"sort_order\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1";

/// A row the board typed itself: a location whose name is free text.
const INSERT_TEXT_ROW: &str = "INSERT INTO \"where_people_locations\" \
     (\"name\", \"sort_order\", \"is_text\") \
     SELECT $1, coalesce(max(\"sort_order\") + 1, 0), true FROM \"where_people_locations\" \
     RETURNING \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \"is_text\"";
const RENAME_TEXT_ROW: &str = "UPDATE \"where_people_locations\" SET \"name\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1 AND \"is_text\" \
     RETURNING \"id\", \"name\", \"department\", \"needs_foreman\", \"color\", \"is_text\"";
const DELETE_TEXT_ROW: &str =
    "DELETE FROM \"where_people_locations\" WHERE \"id\" = $1 AND \"is_text\"";

/// The employees store department, position, sub-department and location as text; a new
/// name therefore has to travel along in the master data.
const RENAME_EMPLOYEE_DEPARTMENTS: &str = "UPDATE \"where_employees\" \
     SET \"department\" = $2, \"changed_at\" = now() \
     WHERE lower(\"department\") = lower($1)";
const RENAME_EMPLOYEE_SUBDEPARTMENTS: &str = "UPDATE \"where_employees\" \
     SET \"subdepartment\" = $2, \"changed_at\" = now() \
     WHERE lower(\"subdepartment\") = lower($1)";
const RENAME_SUBDEPARTMENT_PARENTS: &str = "UPDATE \"where_people_subdepartments\" \
     SET \"department\" = $2, \"changed_at\" = now() \
     WHERE lower(\"department\") = lower($1)";
const RENAME_EMPLOYEE_POSITIONS: &str = "UPDATE \"where_employees\" \
     SET \"position\" = $2, \"changed_at\" = now() \
     WHERE lower(\"position\") = lower($1)";
const RENAME_EMPLOYEE_LOCATIONS: &str = "UPDATE \"where_employees\" \
     SET \"location\" = $2, \"changed_at\" = now() \
     WHERE lower(\"location\") = lower($1)";

const REORDER_DEPARTMENTS: &str = "UPDATE \"where_people_departments\" SET \"sort_order\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1";
const REORDER_SUBDEPARTMENTS: &str = "UPDATE \"where_people_subdepartments\" \
     SET \"sort_order\" = $2, \"changed_at\" = now() WHERE \"id\" = $1";
const REORDER_POSITIONS: &str = "UPDATE \"where_people_positions\" SET \"sort_order\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1";

fn select_sql(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Departments => SELECT_DEPARTMENTS,
        ListKind::Subdepartments => SELECT_SUBDEPARTMENTS,
        ListKind::Positions => SELECT_POSITIONS,
        ListKind::Locations => SELECT_LOCATIONS,
    }
}

fn insert_sql(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Departments => INSERT_DEPARTMENTS,
        ListKind::Subdepartments => INSERT_SUBDEPARTMENTS,
        ListKind::Positions => INSERT_POSITIONS,
        ListKind::Locations => INSERT_LOCATIONS,
    }
}

fn rename_sql(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Departments => RENAME_DEPARTMENTS,
        ListKind::Subdepartments => RENAME_SUBDEPARTMENTS,
        ListKind::Positions => RENAME_POSITIONS,
        ListKind::Locations => RENAME_LOCATIONS,
    }
}

fn delete_sql(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Departments => DELETE_DEPARTMENTS,
        ListKind::Subdepartments => DELETE_SUBDEPARTMENTS,
        ListKind::Positions => DELETE_POSITIONS,
        ListKind::Locations => DELETE_LOCATIONS,
    }
}

fn select_name_sql(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Departments => SELECT_NAME_DEPARTMENTS,
        ListKind::Subdepartments => SELECT_NAME_SUBDEPARTMENTS,
        ListKind::Positions => SELECT_NAME_POSITIONS,
        ListKind::Locations => SELECT_NAME_LOCATIONS,
    }
}

/// Departments, sub-departments, positions and locations are also stored as text on the
/// employee, so a rename travels along with the master data.
fn rename_employees_sql(kind: ListKind) -> Option<&'static str> {
    match kind {
        ListKind::Departments => Some(RENAME_EMPLOYEE_DEPARTMENTS),
        ListKind::Subdepartments => Some(RENAME_EMPLOYEE_SUBDEPARTMENTS),
        ListKind::Positions => Some(RENAME_EMPLOYEE_POSITIONS),
        ListKind::Locations => Some(RENAME_EMPLOYEE_LOCATIONS),
    }
}

fn reorder_sql(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Departments => REORDER_DEPARTMENTS,
        ListKind::Subdepartments => REORDER_SUBDEPARTMENTS,
        ListKind::Positions => REORDER_POSITIONS,
        ListKind::Locations => REORDER_LOCATIONS,
    }
}

fn row_to_entry(row: &sqlx::postgres::PgRow) -> ListEntry {
    ListEntry {
        id: row.get("id"),
        name: row.get("name"),
        department: None,
        needs_foreman: true,
        is_text: false,
        color: row.get("color"),
    }
}

/// A department carries the tint of its employees in the schedule board.
fn row_to_department(row: &sqlx::postgres::PgRow) -> ListEntry {
    ListEntry {
        id: row.get("id"),
        name: row.get("name"),
        department: None,
        needs_foreman: true,
        is_text: false,
        color: row.get("color"),
    }
}

/// A location carries the department that works there, whether it needs a foreman
/// and its tint in the disposition row.
fn row_to_location(row: &sqlx::postgres::PgRow) -> ListEntry {
    ListEntry {
        id: row.get("id"),
        name: row.get("name"),
        department: row.get("department"),
        needs_foreman: row.get("needs_foreman"),
        is_text: row.get("is_text"),
        color: row.get("color"),
    }
}

/// A sub-department carries the department it sits under and a colour of its own.
fn row_to_subdepartment(row: &sqlx::postgres::PgRow) -> ListEntry {
    ListEntry {
        id: row.get("id"),
        name: row.get("name"),
        department: row.get("department"),
        needs_foreman: true,
        is_text: false,
        color: row.get("color"),
    }
}

/// CRUD over one of the lists.
#[derive(Clone)]
pub struct PeopleListRepository {
    pool: PgPool,
    kind: ListKind,
}

impl PeopleListRepository {
    pub fn new(pool: PgPool, kind: ListKind) -> Self {
        Self { pool, kind }
    }

    pub async fn list(&self) -> Result<Vec<ListEntry>, DomainError> {
        let rows = sqlx::query(select_sql(self.kind))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.iter().map(|row| self.to_entry(row)).collect())
    }

    fn to_entry(&self, row: &sqlx::postgres::PgRow) -> ListEntry {
        match self.kind {
            ListKind::Locations => row_to_location(row),
            ListKind::Departments => row_to_department(row),
            ListKind::Subdepartments => row_to_subdepartment(row),
            ListKind::Positions => row_to_entry(row),
        }
    }

    /// The tint of an entry; `None` removes it.
    pub async fn set_color(&self, id: i64, color: Option<&str>) -> Result<ListEntry, DomainError> {
        let sql = match self.kind {
            ListKind::Departments => SET_DEPARTMENT_COLOR,
            ListKind::Subdepartments => SET_SUBDEPARTMENT_COLOR,
            ListKind::Positions => SET_POSITION_COLOR,
            ListKind::Locations => SET_LOCATION_COLOR,
        };
        let row = sqlx::query(sql)
            .bind(id)
            .bind(color)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))?;
        Ok(self.to_entry(&row))
    }

    /// Binds a location to a department; `None` lets every department work there.
    pub async fn set_department(
        &self,
        id: i64,
        department: Option<&str>,
    ) -> Result<ListEntry, DomainError> {
        let sql = match self.kind {
            ListKind::Locations => SET_LOCATION_DEPARTMENT,
            ListKind::Subdepartments => SET_SUBDEPARTMENT_PARENT,
            ListKind::Departments | ListKind::Positions => {
                return Err(DomainError::Validation(
                    "This list carries no parent.".to_string(),
                ));
            }
        };
        let row = sqlx::query(sql)
            .bind(id)
            .bind(department)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))?;
        Ok(self.to_entry(&row))
    }

    /// The department a sub-department sits under; `None` when it has none.
    pub async fn parent_of(&self, id: i64) -> Result<Option<String>, DomainError> {
        sqlx::query_scalar(SELECT_SUBDEPARTMENT_PARENT)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))
    }

    /// Says whether a location is staffed with a foreman.
    pub async fn set_needs_foreman(
        &self,
        id: i64,
        needs_foreman: bool,
    ) -> Result<ListEntry, DomainError> {
        let row = sqlx::query(SET_LOCATION_FOREMAN)
            .bind(id)
            .bind(needs_foreman)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))?;
        Ok(row_to_location(&row))
    }

    pub async fn create(&self, input: &ListEntryInput) -> Result<ListEntry, DomainError> {
        let name = input.clean()?;
        let row = sqlx::query(insert_sql(self.kind))
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(self.to_entry(&row))
    }

    pub async fn rename(&self, id: i64, input: &ListEntryInput) -> Result<ListEntry, DomainError> {
        let name = input.clean()?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let previous: Option<String> = sqlx::query_scalar(select_name_sql(self.kind))
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        let previous = previous.ok_or_else(|| self.not_found(id))?;
        let row = sqlx::query(rename_sql(self.kind))
            .bind(id)
            .bind(&name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))?;
        if previous != name {
            if let Some(sql) = rename_employees_sql(self.kind) {
                sqlx::query(sql)
                    .bind(&previous)
                    .bind(&name)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
            // Sub-departments name their parent, so a renamed department takes them along.
            if self.kind == ListKind::Departments {
                sqlx::query(RENAME_SUBDEPARTMENT_PARENTS)
                    .bind(&previous)
                    .bind(&name)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
        }
        tx.commit().await.map_err(db_error)?;
        Ok(self.to_entry(&row))
    }

    pub async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let affected = sqlx::query(delete_sql(self.kind))
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if affected == 0 {
            return Err(self.not_found(id));
        }
        Ok(())
    }

    /// Adds a row the board typed itself and returns it; its name is free text.
    pub async fn create_text_row(&self, name: &str) -> Result<ListEntry, DomainError> {
        self.require_locations()?;
        let row = sqlx::query(INSERT_TEXT_ROW)
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row_to_location(&row))
    }

    /// Renames a typed row; the employees carry no such name, so nothing follows.
    pub async fn rename_text_row(&self, id: i64, name: &str) -> Result<ListEntry, DomainError> {
        self.require_locations()?;
        let row = sqlx::query(RENAME_TEXT_ROW)
            .bind(id)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))?;
        Ok(row_to_location(&row))
    }

    /// Removes a typed row with its events; a house stays untouched.
    pub async fn delete_text_row(&self, id: i64) -> Result<(), DomainError> {
        self.require_locations()?;
        let affected = sqlx::query(DELETE_TEXT_ROW)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if affected == 0 {
            return Err(self.not_found(id));
        }
        Ok(())
    }

    fn require_locations(&self) -> Result<(), DomainError> {
        if self.kind == ListKind::Locations {
            return Ok(());
        }
        Err(DomainError::Validation(
            "Only the locations carry rows of the board.".to_string(),
        ))
    }

    /// Writes the given order and returns the list as it now stands.
    pub async fn reorder(&self, ids: &[i64]) -> Result<Vec<ListEntry>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        for (position, id) in ids.iter().enumerate() {
            let affected = sqlx::query(reorder_sql(self.kind))
                .bind(id)
                .bind(position as i32)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?
                .rows_affected();
            if affected == 0 {
                return Err(self.not_found(*id));
            }
        }
        tx.commit().await.map_err(db_error)?;
        self.list().await
    }

    fn not_found(&self, id: i64) -> DomainError {
        DomainError::NotFound {
            entity: self.kind.table(),
            id: id as i32,
        }
    }
}
