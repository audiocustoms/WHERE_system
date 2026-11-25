//! Storage of the employee master (`where_employees` and its qualifications).

use std::collections::HashSet;

use chrono::{Datelike, Utc};
use sqlx::{PgPool, Postgres, Row, Transaction};
use where_domain::DomainError;
use where_persistence::db_error;
use where_persistence::number_series_repo::{align_on, format, reserve_on};

use crate::employee::{Employee, EmployeeInput};

/// Key of the number series of the personnel numbers (`where_number_series`).
pub const NUMBER_SERIES: &str = "employee";

/// Issues the next personnel number in the running transaction.
async fn reserve_personnel_no(tx: &mut Transaction<'_, Postgres>) -> Result<String, DomainError> {
    let today = Utc::now().date_naive();
    let raw = reserve_on(&mut **tx, NUMBER_SERIES, today.year())
        .await
        .map_err(db_error)?
        .ok_or_else(|| {
            DomainError::Conflict(format!(
                "The number series “Personnel number” ({NUMBER_SERIES}) is missing or inactive."
            ))
        })?;
    Ok(format(
        &raw.prefix,
        &raw.format,
        raw.digits,
        raw.value,
        today,
    ))
}

/// Raises the number series to a manually assigned number so it is not offered
/// a second time later (`P-1001` counts as 1001).
async fn align_personnel_no(
    tx: &mut Transaction<'_, Postgres>,
    personnel_no: Option<&str>,
) -> Result<(), DomainError> {
    let Some(text) = personnel_no else {
        return Ok(());
    };
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    let value = match digits.parse::<i64>() {
        Ok(value) if (1..=i64::from(i32::MAX)).contains(&value) => value,
        _ => return Ok(()),
    };
    align_on(&mut **tx, NUMBER_SERIES, value as i32, Utc::now().year())
        .await
        .map_err(db_error)
}

const SELECT_ALL_SQL: &str = "SELECT \"id\", \"personnel_no\", \"salutation\", \"title\", \
     \"first_name\", \"last_name\", \"birth_date\", \"birth_place\", \"nationality\", \
     \"marital_status\", \"street\", \"postal_code\", \"city\", \"country\", \"email\", \
     \"phone\", \"mobile\", \"entry_date\", \"exit_date\", \"employment_type\", \
     \"weekly_hours\", \"work_days\", \"vacation_days\", \"position\", \"department\", \"subdepartment\", \"location\", \"cost_center\", \"manager\", \
     \"social_security_no\", \"tax_id\", \"notes\", \"active\", \"is_apprentice\", \"auto_plan\", \
     \"board_order\", \"user_id\", \"created_at\", \"changed_at\" \
     FROM \"where_employees\" ORDER BY \"last_name\", \"first_name\"";

const SELECT_SEARCH_SQL: &str = "SELECT \"id\", \"personnel_no\", \"salutation\", \"title\", \
     \"first_name\", \"last_name\", \"birth_date\", \"birth_place\", \"nationality\", \
     \"marital_status\", \"street\", \"postal_code\", \"city\", \"country\", \"email\", \
     \"phone\", \"mobile\", \"entry_date\", \"exit_date\", \"employment_type\", \
     \"weekly_hours\", \"work_days\", \"vacation_days\", \"position\", \"department\", \"subdepartment\", \"location\", \"cost_center\", \"manager\", \
     \"social_security_no\", \"tax_id\", \"notes\", \"active\", \"is_apprentice\", \"auto_plan\", \
     \"board_order\", \"user_id\", \"created_at\", \"changed_at\" \
     FROM \"where_employees\" \
     WHERE \"first_name\" ILIKE $1 OR \"last_name\" ILIKE $1 \
        OR \"personnel_no\" ILIKE $1 OR \"department\" ILIKE $1 \
        OR \"subdepartment\" ILIKE $1 OR \"position\" ILIKE $1 \
     ORDER BY \"last_name\", \"first_name\"";

const SELECT_ONE_SQL: &str = "SELECT \"id\", \"personnel_no\", \"salutation\", \"title\", \
     \"first_name\", \"last_name\", \"birth_date\", \"birth_place\", \"nationality\", \
     \"marital_status\", \"street\", \"postal_code\", \"city\", \"country\", \"email\", \
     \"phone\", \"mobile\", \"entry_date\", \"exit_date\", \"employment_type\", \
     \"weekly_hours\", \"work_days\", \"vacation_days\", \"position\", \"department\", \"subdepartment\", \"location\", \"cost_center\", \"manager\", \
     \"social_security_no\", \"tax_id\", \"notes\", \"active\", \"is_apprentice\", \"auto_plan\", \
     \"board_order\", \"user_id\", \"created_at\", \"changed_at\" \
     FROM \"where_employees\" WHERE \"id\" = $1";

const INSERT_SQL: &str = "INSERT INTO \"where_employees\" \
     (\"personnel_no\", \"salutation\", \"title\", \"first_name\", \"last_name\", \
      \"birth_date\", \"birth_place\", \"nationality\", \"marital_status\", \
      \"street\", \"postal_code\", \"city\", \"country\", \"email\", \"phone\", \"mobile\", \
      \"entry_date\", \"exit_date\", \"employment_type\", \"weekly_hours\", \"work_days\", \"position\", \
      \"department\", \"cost_center\", \"manager\", \"social_security_no\", \"tax_id\", \
      \"notes\", \"active\", \"vacation_days\", \"is_apprentice\", \"auto_plan\", \"location\", \"subdepartment\") \
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
             $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34) \
     RETURNING \"id\"";

const UPDATE_SQL: &str = "UPDATE \"where_employees\" SET \
     \"personnel_no\" = $2, \"salutation\" = $3, \"title\" = $4, \"first_name\" = $5, \
     \"last_name\" = $6, \"birth_date\" = $7, \"birth_place\" = $8, \
     \"nationality\" = $9, \"marital_status\" = $10, \"street\" = $11, \
     \"postal_code\" = $12, \"city\" = $13, \"country\" = $14, \"email\" = $15, \
     \"phone\" = $16, \"mobile\" = $17, \"entry_date\" = $18, \"exit_date\" = $19, \
     \"employment_type\" = $20, \"weekly_hours\" = $21, \"work_days\" = $22, \"position\" = $23, \
     \"department\" = $24, \"cost_center\" = $25, \"manager\" = $26, \
     \"social_security_no\" = $27, \"tax_id\" = $28, \"notes\" = $29, \"active\" = $30, \"vacation_days\" = $31, \
     \"is_apprentice\" = $32, \"auto_plan\" = $33, \"location\" = $34, \"subdepartment\" = $35, \
     \"changed_at\" = now() \
     WHERE \"id\" = $1";

fn row_to_employee(row: &sqlx::postgres::PgRow) -> Employee {
    Employee {
        id: Some(row.get("id")),
        personnel_no: row.get("personnel_no"),
        salutation: row.get("salutation"),
        title: row.get("title"),
        first_name: row.get("first_name"),
        last_name: row.get("last_name"),
        birth_date: row.get("birth_date"),
        birth_place: row.get("birth_place"),
        nationality: row.get("nationality"),
        marital_status: row.get("marital_status"),
        street: row.get("street"),
        postal_code: row.get("postal_code"),
        city: row.get("city"),
        country: row.get("country"),
        email: row.get("email"),
        phone: row.get("phone"),
        mobile: row.get("mobile"),
        entry_date: row.get("entry_date"),
        exit_date: row.get("exit_date"),
        employment_type: row.get("employment_type"),
        weekly_hours: row.get("weekly_hours"),
        work_days: row.get("work_days"),
        vacation_days: row.get("vacation_days"),
        position: row.get("position"),
        department: row.get("department"),
        subdepartment: row.get("subdepartment"),
        location: row.get("location"),
        cost_center: row.get("cost_center"),
        manager: row.get("manager"),
        social_security_no: row.get("social_security_no"),
        tax_id: row.get("tax_id"),
        notes: row.get("notes"),
        active: row.get("active"),
        is_apprentice: row.get("is_apprentice"),
        auto_plan: row.get("auto_plan"),
        board_order: row.get("board_order"),
        user_id: row.get("user_id"),
        qualifications: Vec::new(),
        created_at: row.get("created_at"),
        changed_at: row.get("changed_at"),
    }
}

/// CRUD over the employee master.
#[derive(Clone)]
pub struct EmployeeRepository {
    pool: PgPool,
}

impl EmployeeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, search: Option<&str>) -> Result<Vec<Employee>, DomainError> {
        let rows = match search.map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => {
                sqlx::query(SELECT_SEARCH_SQL)
                    .bind(format!("%{text}%"))
                    .fetch_all(&self.pool)
                    .await
            }
            None => sqlx::query(SELECT_ALL_SQL).fetch_all(&self.pool).await,
        }
        .map_err(db_error)?;
        Ok(rows.iter().map(row_to_employee).collect())
    }

    pub async fn get(&self, id: i64) -> Result<Employee, DomainError> {
        let row = sqlx::query(SELECT_ONE_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "employee",
                id: id as i32,
            })?;
        let mut employee = row_to_employee(&row);
        employee.qualifications = self.load_qualifications(id).await?;
        Ok(employee)
    }

    pub async fn create(&self, input: &EmployeeInput) -> Result<Employee, DomainError> {
        input.validate()?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let personnel_no = match trimmed(&input.personnel_no) {
            Some(value) => {
                align_personnel_no(&mut tx, Some(&value)).await?;
                Some(value)
            }
            // Without input the number series issues the next personnel number.
            None => Some(reserve_personnel_no(&mut tx).await?),
        };
        let id: i64 = sqlx::query_scalar(INSERT_SQL)
            .bind(personnel_no)
            .bind(trimmed(&input.salutation))
            .bind(trimmed(&input.title))
            .bind(input.first_name.trim())
            .bind(input.last_name.trim())
            .bind(input.birth_date)
            .bind(trimmed(&input.birth_place))
            .bind(trimmed(&input.nationality))
            .bind(trimmed(&input.marital_status))
            .bind(trimmed(&input.street))
            .bind(trimmed(&input.postal_code))
            .bind(trimmed(&input.city))
            .bind(trimmed(&input.country))
            .bind(trimmed(&input.email))
            .bind(trimmed(&input.phone))
            .bind(trimmed(&input.mobile))
            .bind(input.entry_date)
            .bind(input.exit_date)
            .bind(trimmed(&input.employment_type))
            .bind(input.weekly_hours)
            .bind(input.work_days)
            .bind(trimmed(&input.position))
            .bind(trimmed(&input.department))
            .bind(trimmed(&input.cost_center))
            .bind(trimmed(&input.manager))
            .bind(trimmed(&input.social_security_no))
            .bind(trimmed(&input.tax_id))
            .bind(trimmed(&input.notes))
            .bind(input.active)
            .bind(input.vacation_days)
            .bind(input.is_apprentice)
            .bind(input.auto_plan)
            .bind(trimmed(&input.location))
            .bind(trimmed(&input.subdepartment))
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;
        save_qualifications(&mut tx, id, &input.qualifications).await?;
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(&self, id: i64, input: &EmployeeInput) -> Result<Employee, DomainError> {
        input.validate()?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let personnel_no = trimmed(&input.personnel_no);
        align_personnel_no(&mut tx, personnel_no.as_deref()).await?;
        let affected = sqlx::query(UPDATE_SQL)
            .bind(id)
            .bind(personnel_no)
            .bind(trimmed(&input.salutation))
            .bind(trimmed(&input.title))
            .bind(input.first_name.trim())
            .bind(input.last_name.trim())
            .bind(input.birth_date)
            .bind(trimmed(&input.birth_place))
            .bind(trimmed(&input.nationality))
            .bind(trimmed(&input.marital_status))
            .bind(trimmed(&input.street))
            .bind(trimmed(&input.postal_code))
            .bind(trimmed(&input.city))
            .bind(trimmed(&input.country))
            .bind(trimmed(&input.email))
            .bind(trimmed(&input.phone))
            .bind(trimmed(&input.mobile))
            .bind(input.entry_date)
            .bind(input.exit_date)
            .bind(trimmed(&input.employment_type))
            .bind(input.weekly_hours)
            .bind(input.work_days)
            .bind(trimmed(&input.position))
            .bind(trimmed(&input.department))
            .bind(trimmed(&input.cost_center))
            .bind(trimmed(&input.manager))
            .bind(trimmed(&input.social_security_no))
            .bind(trimmed(&input.tax_id))
            .bind(trimmed(&input.notes))
            .bind(input.active)
            .bind(input.vacation_days)
            .bind(input.is_apprentice)
            .bind(input.auto_plan)
            .bind(trimmed(&input.location))
            .bind(trimmed(&input.subdepartment))
            .execute(&mut *tx)
            .await
            .map_err(db_error)?
            .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "employee",
                id: id as i32,
            });
        }
        save_qualifications(&mut tx, id, &input.qualifications).await?;
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let affected = sqlx::query("DELETE FROM \"where_employees\" WHERE \"id\" = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "employee",
                id: id as i32,
            });
        }
        Ok(())
    }

    /// Writes the order of the duty board: the ids come top to bottom. Only the
    /// rows of the board (the active employees) get a place, so a person who is
    /// reactivated later keeps out of an order that was arranged without them.
    pub async fn set_board_order(&self, ids: &[i64]) -> Result<usize, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let mut written = 0;
        for (platz, id) in ids.iter().enumerate() {
            written += sqlx::query(
                "UPDATE \"where_employees\" SET \"board_order\" = $2 WHERE \"id\" = $1",
            )
            .bind(id)
            .bind(platz as i32)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?
            .rows_affected();
        }
        tx.commit().await.map_err(db_error)?;
        Ok(written as usize)
    }

    /// Names the account of the user administration that belongs to this employee.
    ///
    /// `None` unlinks it. An account belongs to one employee only: a second employee
    /// that names the same account takes it over from the first.
    pub async fn set_user(&self, id: i64, user_id: Option<i32>) -> Result<Employee, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if let Some(user_id) = user_id {
            sqlx::query(
                "UPDATE \"where_employees\" SET \"user_id\" = NULL, \"changed_at\" = now() \
                 WHERE \"user_id\" = $1 AND \"id\" <> $2",
            )
            .bind(user_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }
        let affected = sqlx::query(
            "UPDATE \"where_employees\" SET \"user_id\" = $2, \"changed_at\" = now() \
             WHERE \"id\" = $1",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?
        .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "employee",
                id: id as i32,
            });
        }
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    async fn load_qualifications(&self, id: i64) -> Result<Vec<String>, DomainError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT \"qualification\" FROM \"where_employee_qualifications\" \
             WHERE \"employee_id\" = $1 ORDER BY \"position\", \"id\"",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(rows)
    }
}

async fn save_qualifications(
    tx: &mut Transaction<'_, Postgres>,
    employee_id: i64,
    values: &[String],
) -> Result<(), DomainError> {
    sqlx::query("DELETE FROM \"where_employee_qualifications\" WHERE \"employee_id\" = $1")
        .bind(employee_id)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;

    let mut seen = HashSet::new();
    let mut position = 0i32;
    for raw in values {
        let text = raw.trim();
        if text.is_empty() || !seen.insert(text.to_lowercase()) {
            continue;
        }
        sqlx::query(
            "INSERT INTO \"where_employee_qualifications\" \
             (\"employee_id\", \"qualification\", \"position\") VALUES ($1, $2, $3)",
        )
        .bind(employee_id)
        .bind(text)
        .bind(position)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
        position += 1;
    }
    Ok(())
}

fn trimmed(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}
