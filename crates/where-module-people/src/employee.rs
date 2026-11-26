//! Employee master data of `WHERE - are my people?`.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

/// One employee record — the mask shows it in the tabs "Person",
/// "Beschäftigung" and "Qualifikationen".
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Employee {
    /// `None` when creating.
    pub id: Option<i64>,
    /// Payroll number, unique when given.
    pub personnel_no: Option<String>,

    // ---- Person
    pub salutation: Option<String>,
    pub title: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub birth_date: Option<NaiveDate>,
    pub birth_place: Option<String>,
    pub nationality: Option<String>,
    pub marital_status: Option<String>,

    // ---- Address
    pub street: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,

    // ---- Contact
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,

    // ---- Employment
    pub entry_date: Option<NaiveDate>,
    pub exit_date: Option<NaiveDate>,
    pub employment_type: Option<String>,
    /// Agreed hours per week (`numeric(5,2)`).
    pub weekly_hours: Option<Decimal>,
    /// Working days per week (1–7), empty = not set.
    pub work_days: Option<i16>,
    /// Vacation days and remaining vacation days, empty = not set.
    pub vacation_days: Option<i16>,
    pub position: Option<String>,
    pub department: Option<String>,
    /// The sub-department under the department, for example `Podium` under `Beleuchtung`.
    pub subdepartment: Option<String>,
    /// The house of the disposition this employee belongs to.
    pub location: Option<String>,
    pub cost_center: Option<String>,
    /// Name of the supervisor.
    pub manager: Option<String>,
    pub social_security_no: Option<String>,
    pub tax_id: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    /// The employee is an apprentice.
    pub is_apprentice: bool,
    /// The plan generator may place this employee; `false` = planned by hand only.
    pub auto_plan: bool,
    /// Place on the schedule board once a planner arranged the rows by hand; empty
    /// follows the position order of the master data.
    #[serde(default)]
    pub board_order: Option<i32>,
    /// The account of the user administration this employee signs in with, if any
    /// (`migrations/1947_people_employee_user.sql`). The employee master never writes
    /// it — the user administration sets it when it makes an employee a user.
    #[serde(default)]
    pub user_id: Option<i32>,

    // ---- Qualifications
    #[serde(default)]
    pub qualifications: Vec<String>,

    // ---- Audit
    pub created_at: Option<DateTime<Utc>>,
    pub changed_at: Option<DateTime<Utc>>,
}

/// What the user administration sends when it names the account behind an employee.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct EmployeeUserInput {
    /// Key of the account in `"User_Prefs"`; `null` takes the reference away.
    #[serde(default)]
    pub user_id: Option<i32>,
}

/// What the employee mask sends.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct EmployeeInput {
    #[serde(default)]
    pub personnel_no: Option<String>,
    #[serde(default)]
    pub salutation: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub birth_date: Option<NaiveDate>,
    #[serde(default)]
    pub birth_place: Option<String>,
    #[serde(default)]
    pub nationality: Option<String>,
    #[serde(default)]
    pub marital_status: Option<String>,
    #[serde(default)]
    pub street: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub entry_date: Option<NaiveDate>,
    #[serde(default)]
    pub exit_date: Option<NaiveDate>,
    #[serde(default)]
    pub employment_type: Option<String>,
    #[serde(default)]
    pub weekly_hours: Option<Decimal>,
    #[serde(default)]
    pub work_days: Option<i16>,
    #[serde(default)]
    pub vacation_days: Option<i16>,
    #[serde(default)]
    pub position: Option<String>,
    #[serde(default)]
    pub department: Option<String>,
    /// The sub-department under the department, for example `Podium` under `Beleuchtung`.
    #[serde(default)]
    pub subdepartment: Option<String>,
    /// The house of the disposition this employee belongs to.
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub cost_center: Option<String>,
    #[serde(default)]
    pub manager: Option<String>,
    #[serde(default)]
    pub social_security_no: Option<String>,
    #[serde(default)]
    pub tax_id: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default = "active_default")]
    pub active: bool,
    /// Off by default; `Auszubildende/r` in the mask.
    #[serde(default)]
    pub is_apprentice: bool,
    /// On by default; `nicht automatisch disponierbar` in the mask.
    #[serde(default = "auto_plan_default")]
    pub auto_plan: bool,
    #[serde(default)]
    pub qualifications: Vec<String>,
}

/// Default of the mask's `active` switch when a client omits it.
pub fn active_default() -> bool {
    true
}

/// Default of `auto_plan`: everybody may be planned automatically unless the
/// employee record says otherwise.
pub fn auto_plan_default() -> bool {
    true
}

/// The next personnel number the number series offers.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NextPersonnelNo {
    pub personnel_no: String,
}

impl Employee {
    /// Clears the personal fields for callers without
    /// `personal.persoenliche-daten-sehen`.
    pub fn without_personal_data(mut self) -> Self {
        self.birth_date = None;
        self.birth_place = None;
        self.nationality = None;
        self.marital_status = None;
        self.street = None;
        self.postal_code = None;
        self.city = None;
        self.country = None;
        self.social_security_no = None;
        self.tax_id = None;
        self
    }
}

impl EmployeeInput {
    /// Trims the texts and checks the values the mask can get wrong.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.first_name.trim().is_empty() || self.last_name.trim().is_empty() {
            return Err(DomainError::Validation(
                "First and last name are required fields.".to_string(),
            ));
        }
        if let Some(email) = &self.email
            && !email.trim().is_empty()
            && !email.contains('@')
        {
            return Err(DomainError::Validation(
                "The e-mail address is invalid.".to_string(),
            ));
        }
        if let (Some(entry), Some(exit)) = (self.entry_date, self.exit_date)
            && exit < entry
        {
            return Err(DomainError::Validation(
                "The exit date lies before the entry date.".to_string(),
            ));
        }
        if let Some(hours) = self.weekly_hours
            && hours.is_sign_negative()
        {
            return Err(DomainError::Validation(
                "The weekly hours must not be negative.".to_string(),
            ));
        }
        if let Some(days) = self.work_days
            && !(1..=7).contains(&days)
        {
            return Err(DomainError::Validation(
                "The working days are between 1 and 7.".to_string(),
            ));
        }
        if let Some(days) = self.vacation_days
            && !(0..=366).contains(&days)
        {
            return Err(DomainError::Validation(
                "The vacation days are between 0 and 366.".to_string(),
            ));
        }
        if let Some(birth) = self.birth_date
            && birth > Utc::now().date_naive()
        {
            return Err(DomainError::Validation(
                "The birth date lies in the future.".to_string(),
            ));
        }
        Ok(())
    }

    /// The personal fields that need the personal-data right are cleared here,
    /// so a client cannot set them around the UI.
    pub fn without_personal_data(mut self) -> Self {
        self.birth_date = None;
        self.birth_place = None;
        self.nationality = None;
        self.marital_status = None;
        self.street = None;
        self.postal_code = None;
        self.city = None;
        self.country = None;
        self.social_security_no = None;
        self.tax_id = None;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> EmployeeInput {
        EmployeeInput {
            first_name: "Ada".into(),
            last_name: "Lovelace".into(),
            active: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_name_is_required() {
        let mut input = input();
        assert!(input.validate().is_ok());
        input.last_name = "  ".into();
        assert!(input.validate().is_err());
    }

    #[test]
    fn an_email_needs_an_at_sign() {
        let mut input = input();
        input.email = Some("ada.example.com".into());
        assert!(input.validate().is_err());
        input.email = Some("ada@example.com".into());
        assert!(input.validate().is_ok());
    }

    #[test]
    fn the_exit_must_not_precede_the_entry() {
        let mut input = input();
        input.entry_date = NaiveDate::from_ymd_opt(2026, 1, 1);
        input.exit_date = NaiveDate::from_ymd_opt(2025, 12, 31);
        assert!(input.validate().is_err());
        input.exit_date = NaiveDate::from_ymd_opt(2026, 6, 30);
        assert!(input.validate().is_ok());
    }

    #[test]
    fn weekly_hours_must_not_be_negative() {
        let mut input = input();
        input.weekly_hours = Some(Decimal::from(-1));
        assert!(input.validate().is_err());
        input.weekly_hours = Some(Decimal::from(40));
        assert!(input.validate().is_ok());
    }

    #[test]
    fn the_week_has_one_to_seven_working_days() {
        let mut input = input();
        input.work_days = Some(3);
        assert!(input.validate().is_ok());
        input.work_days = Some(5);
        assert!(input.validate().is_ok());
        input.work_days = Some(6);
        assert!(input.validate().is_ok());
        input.work_days = Some(7);
        assert!(input.validate().is_ok());
        input.work_days = Some(0);
        assert!(input.validate().is_err());
        input.work_days = Some(8);
        assert!(input.validate().is_err());
        input.work_days = None;
        assert!(input.validate().is_ok());
    }

    #[test]
    fn vacation_days_stay_within_a_year() {
        let mut input = input();
        input.vacation_days = Some(0);
        assert!(input.validate().is_ok());
        input.vacation_days = Some(30);
        assert!(input.validate().is_ok());
        input.vacation_days = Some(366);
        assert!(input.validate().is_ok());
        input.vacation_days = Some(-1);
        assert!(input.validate().is_err());
        input.vacation_days = Some(367);
        assert!(input.validate().is_err());
        input.vacation_days = None;
        assert!(input.validate().is_ok());
    }

    #[test]
    fn a_birthday_in_the_future_is_refused() {
        let mut input = input();
        input.birth_date = Some(Utc::now().date_naive() + chrono::Days::new(1));
        assert!(input.validate().is_err());
    }

    #[test]
    fn without_the_right_the_personal_fields_are_dropped() {
        let input = EmployeeInput {
            birth_date: NaiveDate::from_ymd_opt(1990, 4, 1),
            birth_place: Some("London".into()),
            street: Some("Musterweg 1".into()),
            social_security_no: Some("12345678A900".into()),
            phone: Some("0123 456".into()),
            ..input()
        };
        let cleared = input.without_personal_data();
        assert!(cleared.birth_date.is_none());
        assert!(cleared.birth_place.is_none());
        assert!(cleared.street.is_none());
        assert!(cleared.social_security_no.is_none());
        // Contact data and the name stay with the mask.
        assert_eq!(cleared.phone.as_deref(), Some("0123 456"));
        assert_eq!(cleared.last_name, "Lovelace");
    }

    #[test]
    fn an_employee_can_be_reduced_to_the_public_fields() {
        let employee = Employee {
            last_name: "Lovelace".into(),
            city: Some("London".into()),
            tax_id: Some("12345/67890".into()),
            ..Default::default()
        };
        let cleared = employee.without_personal_data();
        assert!(cleared.city.is_none());
        assert!(cleared.tax_id.is_none());
        assert_eq!(cleared.last_name, "Lovelace");
    }
}
