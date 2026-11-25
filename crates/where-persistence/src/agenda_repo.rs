//! Calendar: appointments, their recurrence and single removed occurrences.
//!
//! Storage is the legacy layout: one row in `"Scheduler"`, optionally one row in
//! `"Scheduler_RRules"` for the series and rows in `"Scheduler_RRExDate"` for
//! occurrences that were deleted individually. The series is expanded in Rust
//! ([`AgendaEvent::occurrences`]) because an SQL expansion of an unbounded rule is a
//! recursive query nobody wants to debug.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::PgPool;
use where_domain::{
    AgendaBirthday, AgendaCategory, AgendaEvent, AgendaEventInput, AgendaOccurrence, AgendaRange,
    AgendaRecurrence, DomainError, RecurrenceFrequency,
};

use crate::error::db_error;

/// Recurrence codes written to `"rrule_type"`. The legacy plugin numbers are not
/// documented in the inventory, so these are our own; the meaning is only ever read
/// back by us.
const RRULE_DAILY: i32 = 1;
const RRULE_WEEKLY: i32 = 2;
const RRULE_MONTHLY: i32 = 3;
const RRULE_YEARLY: i32 = 4;

/// `"rrule_End"`: how the series limited, mirroring the stored fields.
const END_NONE: i32 = 0;
const END_DATE: i32 = 1;
const END_COUNT: i32 = 2;

const LIST_SQL: &str = r#"
SELECT s."Primary_Key" AS id,
       s."EventText" AS title,
       s."Event_HeadText" AS head_text,
       s."Text" AS notes,
       s."StartDate" AS start_date,
       s."EndDate" AS end_date,
       s."StartTime" AS start_time,
       s."EndTime" AS end_time,
       COALESCE(s."is_Ganztagestermin", 0) <> 0 AS all_day,
       s."Type" AS category,
       c."Name" AS category_name,
       s."Veranstaltung" AS venue,
       s."Adress_Primkey" AS address_id,
       s."Adress_Name" AS address_name,
       s."Background_ColorRed" AS color_red,
       s."Background_ColorGreen" AS color_green,
       s."Background_ColorBlue" AS color_blue,
       COALESCE(s."Background_CustomColor", false) AS custom_color,
       s."PrivateUser_Primkey" AS private_user,
       COALESCE(s."Remind_Flag", false) AS remind,
       CASE WHEN s."Remind_TimeStamp" > 0 AND s."Start_TimeStamp" > 0
            THEN (s."Start_TimeStamp" - s."Remind_TimeStamp") / 60
       END AS remind_minutes,
       r."Primary_Key" AS rrule_id,
       r."rrule_type" AS rrule_type,
       r."rrule_interval" AS rrule_interval,
       r."rrule_End" AS rrule_end,
       r."rrule_EndDate" AS rrule_end_date,
       r."rrule_EndCount" AS rrule_end_count,
       s."sx_changeDate" AS change_date,
       s."sx_changeTime" AS change_time,
       s."sx_changeUser" AS change_user
  FROM "Scheduler" s
  LEFT JOIN "Storage_TACategory" c ON c."Primary_Key" = s."Type"
  LEFT JOIN "Scheduler_RRules" r ON r."Primary_Key" = s."RRules_Primkey"
 WHERE s."StartDate" IS NOT NULL
   AND s."StartDate" <= $2
   -- A series may reach into the range from any start; a single appointment must
   -- overlap it, otherwise past appointments would flood every later month.
   AND (COALESCE(s."RRules_Primkey", 0) <> 0 OR s."EndDate" >= $1)
   AND ($3::int IS NULL OR s."Type" = $3)
   AND (COALESCE(s."PrivateUser_Primkey", 0) = 0 OR s."PrivateUser_Primkey" = $4)
   AND ($5::text IS NULL
        OR s."EventText" ILIKE '%' || $5 || '%'
        OR s."Event_HeadText" ILIKE '%' || $5 || '%'
        OR s."Text" ILIKE '%' || $5 || '%'
        OR s."Veranstaltung" ILIKE '%' || $5 || '%'
        OR s."Adress_Name" ILIKE '%' || $5 || '%')
 ORDER BY s."StartDate", COALESCE(s."StartTime", TIME '00:00'), s."Primary_Key""#;

const GET_SQL: &str = r#"
SELECT s."Primary_Key" AS id,
       s."EventText" AS title,
       s."Event_HeadText" AS head_text,
       s."Text" AS notes,
       s."StartDate" AS start_date,
       s."EndDate" AS end_date,
       s."StartTime" AS start_time,
       s."EndTime" AS end_time,
       COALESCE(s."is_Ganztagestermin", 0) <> 0 AS all_day,
       s."Type" AS category,
       c."Name" AS category_name,
       s."Veranstaltung" AS venue,
       s."Adress_Primkey" AS address_id,
       s."Adress_Name" AS address_name,
       s."Background_ColorRed" AS color_red,
       s."Background_ColorGreen" AS color_green,
       s."Background_ColorBlue" AS color_blue,
       COALESCE(s."Background_CustomColor", false) AS custom_color,
       s."PrivateUser_Primkey" AS private_user,
       COALESCE(s."Remind_Flag", false) AS remind,
       CASE WHEN s."Remind_TimeStamp" > 0 AND s."Start_TimeStamp" > 0
            THEN (s."Start_TimeStamp" - s."Remind_TimeStamp") / 60
       END AS remind_minutes,
       r."Primary_Key" AS rrule_id,
       r."rrule_type" AS rrule_type,
       r."rrule_interval" AS rrule_interval,
       r."rrule_End" AS rrule_end,
       r."rrule_EndDate" AS rrule_end_date,
       r."rrule_EndCount" AS rrule_end_count,
       s."sx_changeDate" AS change_date,
       s."sx_changeTime" AS change_time,
       s."sx_changeUser" AS change_user
  FROM "Scheduler" s
  LEFT JOIN "Storage_TACategory" c ON c."Primary_Key" = s."Type"
  LEFT JOIN "Scheduler_RRules" r ON r."Primary_Key" = s."RRules_Primkey"
 WHERE s."Primary_Key" = $1"#;

/// Removed occurrences of the series that appear in the requested range.
const EXCEPTIONS_SQL: &str = r#"
SELECT "ScheduleRR_Primkey" AS rrule_id, "EXDate" AS date
  FROM "Scheduler_RRExDate"
 WHERE "ScheduleRR_Primkey" = ANY($1)
   AND "EXDate" IS NOT NULL"#;

const INSERT_SQL: &str = r#"
INSERT INTO "Scheduler" (
    "EventText", "Event_HeadText", "Text", "StartDate", "EndDate", "StartTime", "EndTime",
    "is_Ganztagestermin", "AllUsers_Flag", "Type", "Veranstaltung",
    "Adress_Primkey", "Adress_Name",
    "Background_ColorRed", "Background_ColorGreen", "Background_ColorBlue", "Background_CustomColor",
    "PrivateUser_Primkey", "Remind_Flag", "Remind_TimeStamp",
    "Start_TimeStamp", "End_TimeStamp", "Trigger_disable",
    "sx_createDate", "sx_createTime", "sx_changeDate", "sx_changeTime",
    "sx_createUser", "sx_changeUser"
) VALUES (
    $1, $2, $3, $4, $5, $6, $7,
    $8, $9, $10, $11,
    $12, $13,
    $14, $15, $16, $17,
    $18, $19, $20,
    $21, $22, false,
    $23, $24, $23, $24,
    $25, $25
)
RETURNING "Primary_Key""#;

const UPDATE_SQL: &str = r#"
UPDATE "Scheduler" SET
    "EventText" = $2, "Event_HeadText" = $3, "Text" = $4,
    "StartDate" = $5, "EndDate" = $6, "StartTime" = $7, "EndTime" = $8,
    "is_Ganztagestermin" = $9, "AllUsers_Flag" = $10, "Type" = $11, "Veranstaltung" = $12,
    "Adress_Primkey" = $13, "Adress_Name" = $14,
    "Background_ColorRed" = $15, "Background_ColorGreen" = $16, "Background_ColorBlue" = $17,
    "Background_CustomColor" = $18,
    "PrivateUser_Primkey" = $19, "Remind_Flag" = $20, "Remind_TimeStamp" = $21,
    "Start_TimeStamp" = $22, "End_TimeStamp" = $23,
    "sx_changeDate" = $24, "sx_changeTime" = $25, "sx_changeUser" = $26
 WHERE "Primary_Key" = $1
RETURNING "Primary_Key""#;

const DELETE_SQL: &str = r#"DELETE FROM "Scheduler" WHERE "Primary_Key" = $1"#;

/// Name of an address; the legacy application kept it next to the key.
const ADDRESS_NAME_SQL: &str = r#"SELECT "Name1" FROM "Adresse" WHERE "Primary_key" = $1"#;

/// Birthdays of all active addresses. Month and day are compared in Rust, the year
/// of the address must not decide whether a birthday shows up.
const BIRTHDAYS_SQL: &str = r#"SELECT "Primary_key" AS id, "Name1" AS name, "Person_Geburtstag" AS birthday
  FROM "Adresse"
 WHERE "Person_Geburtstag" IS NOT NULL
   AND COALESCE("active", true)"#;

const RRULE_INSERT_SQL: &str = r#"
INSERT INTO "Scheduler_RRules" (
    "FirstApp_PK", "rrule_type", "rrule_interval", "rrule_End", "rrule_EndDate", "rrule_EndCount",
    "sxrd_createDate", "sxrd_createTime", "sxrd_changeDate", "sxrd_changeTime",
    "sxrd_createUser", "sxrd_changeUser"
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $7, $8, $9, $9)
RETURNING "Primary_Key""#;

const RRULE_UPDATE_SQL: &str = r#"
UPDATE "Scheduler_RRules" SET
    "rrule_type" = $2, "rrule_interval" = $3, "rrule_End" = $4,
    "rrule_EndDate" = $5, "rrule_EndCount" = $6,
    "sxrd_changeDate" = $7, "sxrd_changeTime" = $8, "sxrd_changeUser" = $9
 WHERE "Primary_Key" = $1"#;

const RRULE_DELETE_SQL: &str = r#"DELETE FROM "Scheduler_RRules" WHERE "Primary_Key" = $1"#;

const SET_RRULE_SQL: &str =
    r#"UPDATE "Scheduler" SET "RRules_Primkey" = $2 WHERE "Primary_Key" = $1"#;

const EXCEPTION_INSERT_SQL: &str = r#"
INSERT INTO "Scheduler_RRExDate" ("ScheduleRR_Primkey", "EXDate", "EXDate_UTS")
VALUES ($1, $2, $3)"#;

const DELETE_EXCEPTIONS_SQL: &str =
    r#"DELETE FROM "Scheduler_RRExDate" WHERE "ScheduleRR_Primkey" = $1"#;

const CATEGORIES_SQL: &str = r#"
SELECT c."Primary_Key" AS id,
       c."Name" AS name,
       (SELECT count(*) FROM "Scheduler" s WHERE s."Type" = c."Primary_Key") AS appointments
  FROM "Storage_TACategory" c
 WHERE c."Name" IS NOT NULL AND btrim(c."Name") <> ''
 ORDER BY c."Name""#;

const CATEGORY_INSERT_SQL: &str = r#"
INSERT INTO "Storage_TACategory" ("Name") VALUES ($1) RETURNING "Primary_Key""#;

const CATEGORY_UPDATE_SQL: &str =
    r#"UPDATE "Storage_TACategory" SET "Name" = $2 WHERE "Primary_Key" = $1"#;

const CATEGORY_DELETE_SQL: &str = r#"DELETE FROM "Storage_TACategory" WHERE "Primary_Key" = $1"#;

#[derive(Debug, sqlx::FromRow)]
struct AgendaRow {
    id: i32,
    title: Option<String>,
    head_text: Option<String>,
    notes: Option<String>,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    end_time: Option<NaiveTime>,
    all_day: bool,
    category: Option<i32>,
    category_name: Option<String>,
    venue: Option<String>,
    address_id: Option<i32>,
    address_name: Option<String>,
    color_red: Option<i32>,
    color_green: Option<i32>,
    color_blue: Option<i32>,
    custom_color: bool,
    private_user: Option<i32>,
    remind: bool,
    remind_minutes: Option<i32>,
    rrule_id: Option<i32>,
    rrule_type: Option<i32>,
    rrule_interval: Option<i32>,
    rrule_end: Option<i32>,
    rrule_end_date: Option<NaiveDate>,
    rrule_end_count: Option<i32>,
    change_date: Option<NaiveDate>,
    change_time: Option<NaiveTime>,
    change_user: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ExceptionRow {
    rrule_id: i32,
    date: NaiveDate,
}

#[derive(Debug, sqlx::FromRow)]
struct BirthdayRow {
    id: i32,
    name: Option<String>,
    birthday: Option<NaiveDate>,
}

#[derive(Debug, sqlx::FromRow)]
struct CategoryRow {
    id: i32,
    name: Option<String>,
    appointments: i64,
}

impl AgendaRow {
    fn into_event(self) -> AgendaEvent {
        let recurrence = self.recurrence();
        AgendaEvent {
            id: self.id,
            title: self.title.unwrap_or_default(),
            head_text: self.head_text,
            notes: self.notes,
            start_date: self.start_date,
            end_date: self.end_date.unwrap_or(self.start_date),
            start_time: self.start_time,
            end_time: self.end_time,
            all_day: self.all_day,
            category: self.category,
            category_name: self.category_name,
            venue: self.venue,
            address_id: self.address_id,
            address_name: self.address_name,
            color_red: self.color_red,
            color_green: self.color_green,
            color_blue: self.color_blue,
            custom_color: self.custom_color,
            private_user: self.private_user,
            remind: self.remind,
            remind_minutes: self.remind_minutes,
            recurrence,
            changed_at: match (self.change_date, self.change_time) {
                (Some(date), Some(time)) => Some(NaiveDateTime::new(date, time).and_utc()),
                (Some(date), None) => Some(NaiveDateTime::new(date, NaiveTime::MIN).and_utc()),
                _ => None,
            },
            changed_by: self.change_user,
        }
    }

    fn recurrence(&self) -> Option<AgendaRecurrence> {
        let rrule_id = self.rrule_id?;
        if rrule_id == 0 {
            return None;
        }
        let frequency = match self.rrule_type.unwrap_or(RRULE_WEEKLY) {
            RRULE_DAILY => RecurrenceFrequency::Daily,
            RRULE_MONTHLY => RecurrenceFrequency::Monthly,
            RRULE_YEARLY => RecurrenceFrequency::Yearly,
            _ => RecurrenceFrequency::Weekly,
        };
        Some(AgendaRecurrence {
            frequency,
            interval: u16::try_from(self.rrule_interval.unwrap_or(1).max(1)).unwrap_or(1),
            until: match self.rrule_end.unwrap_or(END_NONE) {
                END_DATE => self.rrule_end_date,
                END_COUNT => None,
                _ => self.rrule_end_date.filter(|_| self.rrule_end.is_none()),
            },
            count: match self.rrule_end.unwrap_or(END_NONE) {
                END_COUNT => self.rrule_end_count.filter(|count| *count > 0),
                _ => None,
            },
        })
    }
}

/// Reads and writes the calendar.
#[derive(Clone)]
pub struct AgendaRepository {
    pool: PgPool,
}

impl AgendaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Appointments of the range, series expanded into single occurrences.
    pub async fn range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        category: Option<i32>,
        user: Option<i32>,
        search: Option<&str>,
    ) -> Result<AgendaRange, DomainError> {
        let rows = sqlx::query_as::<_, AgendaRow>(LIST_SQL)
            .bind(from)
            .bind(to)
            .bind(category)
            .bind(user)
            .bind(search)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;

        let mut pairs: Vec<(AgendaEvent, Option<i32>)> = Vec::with_capacity(rows.len());
        for row in rows {
            let rrule_id = row.rrule_id.filter(|id| *id > 0);
            pairs.push((row.into_event(), rrule_id));
        }

        let ids: Vec<i32> = pairs.iter().filter_map(|(_, id)| *id).collect();
        let exceptions = self.exceptions(&ids).await?;

        let mut occurrences = Vec::new();
        for (event, rrule_id) in &pairs {
            let removed = rrule_id
                .and_then(|id| exceptions.get(&id))
                .cloned()
                .unwrap_or_default();
            for date in event.occurrences(from, to, &removed) {
                occurrences.push(AgendaOccurrence::of(event, date));
            }
        }
        occurrences.sort_by(|a, b| {
            (a.date, a.start_time.unwrap_or(NaiveTime::MIN), a.event_id).cmp(&(
                b.date,
                b.start_time.unwrap_or(NaiveTime::MIN),
                b.event_id,
            ))
        });

        Ok(AgendaRange {
            from,
            to,
            occurrences,
            birthdays: self.birthdays(from, to).await?,
            categories: self.categories().await?,
        })
    }

    /// Birthdays of the range from the address master. The comparison happens in
    /// Rust because a birthday only carries month and day inside the visible year.
    async fn birthdays(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<AgendaBirthday>, DomainError> {
        let rows = sqlx::query_as::<_, BirthdayRow>(BIRTHDAYS_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut result: Vec<AgendaBirthday> = rows
            .into_iter()
            .filter_map(|row| AgendaBirthday::in_range(row.id, row.name?, row.birthday?, from, to))
            .collect();
        result.sort_by(|a, b| (a.date, &a.name).cmp(&(b.date, &b.name)));
        Ok(result)
    }

    /// Removed occurrences per series (`"Scheduler_RRules"` → dates).
    async fn exceptions(
        &self,
        rrule_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, Vec<NaiveDate>>, DomainError> {
        let mut result: std::collections::HashMap<i32, Vec<NaiveDate>> = Default::default();
        if rrule_ids.is_empty() {
            return Ok(result);
        }
        let rows = sqlx::query_as::<_, ExceptionRow>(EXCEPTIONS_SQL)
            .bind(rrule_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        for row in rows {
            result.entry(row.rrule_id).or_default().push(row.date);
        }
        Ok(result)
    }

    pub async fn get(&self, id: i32) -> Result<AgendaEvent, DomainError> {
        let row = sqlx::query_as::<_, AgendaRow>(GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Termin",
                id,
            })?;
        Ok(row.into_event())
    }

    pub async fn create(
        &self,
        input: AgendaEventInput,
        user: &str,
    ) -> Result<AgendaEvent, DomainError> {
        let mut event = input.into_event()?;
        event.address_name = self.address_name(event.address_id).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let now = Utc::now();
        let (today, time) = (now.date_naive(), now.time());

        let id: i32 = sqlx::query_scalar(INSERT_SQL)
            .bind(&event.title)
            .bind(&event.head_text)
            .bind(&event.notes)
            .bind(event.start_date)
            .bind(event.end_date)
            .bind(event.start_time)
            .bind(event.end_time)
            .bind(i16::from(event.all_day))
            .bind(i32::from(event.private_user.is_none()))
            .bind(event.category)
            .bind(&event.venue)
            .bind(event.address_id)
            .bind(&event.address_name)
            .bind(event.color_red)
            .bind(event.color_green)
            .bind(event.color_blue)
            .bind(event.custom_color)
            .bind(event.private_user)
            .bind(event.remind)
            .bind(remind_timestamp(&event))
            .bind(start_timestamp(&event))
            .bind(end_timestamp(&event))
            .bind(today)
            .bind(time)
            .bind(user)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;

        if let Some(recurrence) = &event.recurrence {
            let rrule_id = self
                .write_recurrence(&mut tx, id, recurrence, today, time, user)
                .await?;
            sqlx::query(SET_RRULE_SQL)
                .bind(id)
                .bind(rrule_id)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(
        &self,
        id: i32,
        input: AgendaEventInput,
        user: &str,
    ) -> Result<AgendaEvent, DomainError> {
        let mut event = input.into_event()?;
        event.address_name = self.address_name(event.address_id).await?;
        self.get(id).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let now = Utc::now();
        let (today, time) = (now.date_naive(), now.time());

        sqlx::query(UPDATE_SQL)
            .bind(id)
            .bind(&event.title)
            .bind(&event.head_text)
            .bind(&event.notes)
            .bind(event.start_date)
            .bind(event.end_date)
            .bind(event.start_time)
            .bind(event.end_time)
            .bind(i16::from(event.all_day))
            .bind(i32::from(event.private_user.is_none()))
            .bind(event.category)
            .bind(&event.venue)
            .bind(event.address_id)
            .bind(&event.address_name)
            .bind(event.color_red)
            .bind(event.color_green)
            .bind(event.color_blue)
            .bind(event.custom_color)
            .bind(event.private_user)
            .bind(event.remind)
            .bind(remind_timestamp(&event))
            .bind(start_timestamp(&event))
            .bind(end_timestamp(&event))
            .bind(today)
            .bind(time)
            .bind(user)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

        // The series may have been switched on, changed or switched off.
        let rrule_id = self.rrule_id(id).await?;
        match (&event.recurrence, rrule_id) {
            (Some(recurrence), Some(rrule_id)) => {
                self.update_recurrence(&mut tx, rrule_id, recurrence, today, time, user)
                    .await?;
            }
            (Some(recurrence), None) => {
                let new_id = self
                    .write_recurrence(&mut tx, id, recurrence, today, time, user)
                    .await?;
                sqlx::query(SET_RRULE_SQL)
                    .bind(id)
                    .bind(new_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
            (None, Some(rrule_id)) => {
                sqlx::query(SET_RRULE_SQL)
                    .bind(id)
                    .bind(0)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
                sqlx::query(DELETE_EXCEPTIONS_SQL)
                    .bind(rrule_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
                sqlx::query(RRULE_DELETE_SQL)
                    .bind(rrule_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
            (None, None) => {}
        }
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        let rrule_id = self.rrule_id(id).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if let Some(rrule_id) = rrule_id {
            sqlx::query(DELETE_EXCEPTIONS_SQL)
                .bind(rrule_id)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            sqlx::query(RRULE_DELETE_SQL)
                .bind(rrule_id)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        let deleted = sqlx::query(DELETE_SQL)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?
            .rows_affected();
        tx.commit().await.map_err(db_error)?;
        if deleted == 0 {
            return Err(DomainError::NotFound {
                entity: "Termin",
                id,
            });
        }
        Ok(())
    }

    /// Removes a single occurrence of a series; the appointment itself stays.
    pub async fn skip_occurrence(&self, id: i32, date: NaiveDate) -> Result<(), DomainError> {
        let rrule_id = self.rrule_id(id).await?.ok_or_else(|| {
            DomainError::Validation("Dieser Termin wiederholt sich nicht.".into())
        })?;
        let timestamp = date
            .and_time(NaiveTime::MIN)
            .and_utc()
            .timestamp()
            .try_into()
            .unwrap_or(0i32);
        sqlx::query(EXCEPTION_INSERT_SQL)
            .bind(rrule_id)
            .bind(date)
            .bind(timestamp)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    pub async fn categories(&self) -> Result<Vec<AgendaCategory>, DomainError> {
        let rows = sqlx::query_as::<_, CategoryRow>(CATEGORIES_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .into_iter()
            .map(|row| AgendaCategory {
                id: row.id,
                name: row.name.unwrap_or_default(),
                appointments: row.appointments,
            })
            .collect())
    }

    /// Creates a calendar (the legacy admin form `Storage_TACategory`).
    pub async fn create_category(&self, name: &str) -> Result<AgendaCategory, DomainError> {
        let id: i32 = sqlx::query_scalar(CATEGORY_INSERT_SQL)
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(AgendaCategory {
            id,
            name: name.to_string(),
            appointments: 0,
        })
    }

    pub async fn rename_category(
        &self,
        id: i32,
        name: &str,
    ) -> Result<AgendaCategory, DomainError> {
        let changed = sqlx::query(CATEGORY_UPDATE_SQL)
            .bind(id)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Kalender",
                id,
            });
        }
        let appointments: i64 =
            sqlx::query_scalar(r#"SELECT count(*) FROM "Scheduler" s WHERE s."Type" = $1"#)
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?;
        Ok(AgendaCategory {
            id,
            name: name.to_string(),
            appointments,
        })
    }

    /// Deletes an empty calendar; a calendar with appointments stays.
    pub async fn delete_category(&self, id: i32) -> Result<(), DomainError> {
        let used: i64 =
            sqlx::query_scalar(r#"SELECT count(*) FROM "Scheduler" s WHERE s."Type" = $1"#)
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?;
        if used > 0 {
            return Err(DomainError::Conflict(format!(
                "Der Kalender enthält noch {used} Termine und kann nicht gelöscht werden."
            )));
        }
        let deleted = sqlx::query(CATEGORY_DELETE_SQL)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if deleted == 0 {
            return Err(DomainError::NotFound {
                entity: "Kalender",
                id,
            });
        }
        Ok(())
    }

    /// Name of the linked address. The legacy application stored it next to the key
    /// (`"Adress_Name"`), so an export and the tile know the address without a join.
    async fn address_name(&self, address_id: Option<i32>) -> Result<Option<String>, DomainError> {
        let Some(id) = address_id else {
            return Ok(None);
        };
        let name: Option<Option<String>> = sqlx::query_scalar(ADDRESS_NAME_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(name.flatten())
    }

    async fn rrule_id(&self, id: i32) -> Result<Option<i32>, DomainError> {
        let value: Option<Option<i32>> = sqlx::query_scalar(
            r#"SELECT "RRules_Primkey" FROM "Scheduler" WHERE "Primary_Key" = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(match value {
            Some(Some(id)) if id > 0 => Some(id),
            _ => None,
        })
    }

    async fn write_recurrence(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        event_id: i32,
        recurrence: &AgendaRecurrence,
        today: NaiveDate,
        time: NaiveTime,
        user: &str,
    ) -> Result<i32, DomainError> {
        let (end, end_date, end_count) = recurrence_bounds(recurrence);
        sqlx::query_scalar(RRULE_INSERT_SQL)
            .bind(event_id)
            .bind(rrule_code(recurrence.frequency))
            .bind(i32::from(recurrence.interval))
            .bind(end)
            .bind(end_date)
            .bind(end_count)
            .bind(today)
            .bind(time)
            .bind(user)
            .fetch_one(&mut **tx)
            .await
            .map_err(db_error)
    }

    async fn update_recurrence(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        rrule_id: i32,
        recurrence: &AgendaRecurrence,
        today: NaiveDate,
        time: NaiveTime,
        user: &str,
    ) -> Result<(), DomainError> {
        let (end, end_date, end_count) = recurrence_bounds(recurrence);
        sqlx::query(RRULE_UPDATE_SQL)
            .bind(rrule_id)
            .bind(rrule_code(recurrence.frequency))
            .bind(i32::from(recurrence.interval))
            .bind(end)
            .bind(end_date)
            .bind(end_count)
            .bind(today)
            .bind(time)
            .bind(user)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        Ok(())
    }
}

fn rrule_code(frequency: RecurrenceFrequency) -> i32 {
    match frequency {
        RecurrenceFrequency::Daily => RRULE_DAILY,
        RecurrenceFrequency::Weekly => RRULE_WEEKLY,
        RecurrenceFrequency::Monthly => RRULE_MONTHLY,
        RecurrenceFrequency::Yearly => RRULE_YEARLY,
    }
}

fn recurrence_bounds(recurrence: &AgendaRecurrence) -> (i32, Option<NaiveDate>, Option<i32>) {
    match (recurrence.until, recurrence.count) {
        (Some(until), _) => (END_DATE, Some(until), None),
        (None, Some(count)) => (END_COUNT, None, Some(count)),
        (None, None) => (END_NONE, None, None),
    }
}

/// Unix seconds of the start; `"Start_TimeStamp"` of the legacy row.
fn start_timestamp(event: &AgendaEvent) -> i32 {
    let time = event.start_time.unwrap_or(NaiveTime::MIN);
    event
        .start_date
        .and_time(time)
        .and_utc()
        .timestamp()
        .try_into()
        .unwrap_or(0)
}

/// Unix seconds of the end; `"End_TimeStamp"` of the legacy row.
fn end_timestamp(event: &AgendaEvent) -> i32 {
    let time = event.end_time.unwrap_or(NaiveTime::MIN);
    event
        .end_date
        .and_time(time)
        .and_utc()
        .timestamp()
        .try_into()
        .unwrap_or(0)
}

/// Unix seconds the reminder fires at (`"Remind_TimeStamp"`), `0` without one.
fn remind_timestamp(event: &AgendaEvent) -> i32 {
    if !event.remind {
        return 0;
    }
    let minutes = i64::from(event.remind_minutes.unwrap_or(15));
    start_timestamp(event) - i32::try_from(minutes * 60).unwrap_or(0)
}
