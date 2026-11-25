//! Which departments a session may work in.
//!
//! A permission group names the departments it may work in — one list for `Personal`
//! and one for `Personal-Disposition` (`where_user_group_scope`, kinds
//! `personal-department` and `personaldispo-department`). The middleware puts those
//! keys next to the request ([`DepartmentScope`]) and this module resolves them
//! against its own department list. The ids are `where_people_departments` keys, so a
//! renamed department keeps its scope while a deleted one silently falls out of it.
//!
//! The two areas stay apart: a department of `Personal` never opens the board and the
//! other way round. Only looking at people is shared — the board needs the names of
//! the staff it disposes, and the employee list shows what the board shows.

use std::collections::HashSet;

use where_api::ApiError;
use where_api::authz::DepartmentScope;
use where_api::state::AppState;
use where_domain::DomainError;

use crate::employee::Employee;
use crate::list_repo::PeopleListRepository;
use crate::lists::ListKind;

/// Forbidden answer of a mask that reaches outside the scope.
pub fn outside_scope() -> ApiError {
    ApiError::from(DomainError::Forbidden(
        "Diese Abteilung gehört nicht zu den Abteilungen der Berechtigungsgruppe.".to_string(),
    ))
}

/// The departments of a session; everything open when no group names one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeopleScope {
    /// Lower case department names of `Personal`, `None` = every department.
    personal: Option<HashSet<String>>,
    /// The same for `Personal-Disposition`.
    dispo: Option<HashSet<String>>,
}

impl PeopleScope {
    /// Resolves both group scopes against the departments of the installation.
    pub async fn load(state: &AppState, scope: &DepartmentScope) -> Result<Self, ApiError> {
        if scope.all() {
            return Ok(Self::default());
        }
        let departments = PeopleListRepository::new(state.pool.clone(), ListKind::Departments)
            .list()
            .await?;
        let resolve = |ids: &Option<Vec<i64>>| match ids {
            None => None,
            Some(ids) => {
                let chosen: HashSet<i64> = ids.iter().copied().collect();
                Some(
                    departments
                        .iter()
                        .filter(|entry| chosen.contains(&entry.id))
                        .map(|entry| entry.name.trim().to_lowercase())
                        .collect::<HashSet<String>>(),
                )
            }
        };
        Ok(Self {
            personal: resolve(&scope.personal),
            dispo: resolve(&scope.dispo),
        })
    }

    /// `true` when `Personal` is not restricted.
    pub fn personal_all(&self) -> bool {
        self.personal.is_none()
    }

    /// `true` when `Personal-Disposition` is not restricted.
    pub fn dispo_all(&self) -> bool {
        self.dispo.is_none()
    }

    /// `true` when the session may work in that department of `Personal`.
    pub fn allows_personal(&self, department: &str) -> bool {
        matches(&self.personal, department)
    }

    /// `true` when the session may dispose that department.
    pub fn allows_dispo(&self, department: &str) -> bool {
        matches(&self.dispo, department)
    }

    /// `true` when the employee is in at least one of the two areas.
    pub fn allows_employee(&self, employee: &Employee) -> bool {
        let department = employee.department.as_deref().unwrap_or("");
        self.allows_personal(department) || self.allows_dispo(department)
    }

    /// Refuses an employee outside both areas.
    pub fn check(&self, employee: &Employee) -> Result<(), ApiError> {
        if self.allows_employee(employee) {
            Ok(())
        } else {
            Err(outside_scope())
        }
    }

    /// Refuses a department outside `Personal` — the area that writes the master data.
    pub fn check_personal(&self, department: Option<&str>) -> Result<(), ApiError> {
        if self.allows_personal(department.unwrap_or("")) {
            Ok(())
        } else {
            Err(outside_scope())
        }
    }

    /// Refuses a shift department outside `Personal-Disposition`. A shift may carry no
    /// department at all; it then belongs to every dispatcher, so a restricted session
    /// may only read it — changing it is administration work.
    pub fn check_dispo(&self, department: Option<&str>) -> Result<(), ApiError> {
        let allowed =
            self.dispo_all() || department.is_some_and(|department| self.allows_dispo(department));
        if allowed {
            Ok(())
        } else {
            Err(outside_scope())
        }
    }

    /// The department names of one area, `None` when it is not restricted.
    pub fn departments(&self, personal: bool) -> Option<Vec<String>> {
        let set = if personal {
            &self.personal
        } else {
            &self.dispo
        };
        set.as_ref().map(|names| names.iter().cloned().collect())
    }

    /// The ids of the employees of one area — the plan carries a department only
    /// indirectly. `None` when that area is not restricted.
    pub async fn employee_ids(
        &self,
        state: &AppState,
        personal: bool,
    ) -> Result<Option<HashSet<i64>>, ApiError> {
        let open = if personal {
            self.personal_all()
        } else {
            self.dispo_all()
        };
        if open {
            return Ok(None);
        }
        let list = crate::employee_repo::EmployeeRepository::new(state.pool.clone())
            .list(None)
            .await?;
        Ok(Some(
            list.into_iter()
                .filter(|employee| {
                    let department = employee.department.as_deref().unwrap_or("");
                    if personal {
                        self.allows_personal(department)
                    } else {
                        self.allows_dispo(department)
                    }
                })
                .filter_map(|employee| employee.id)
                .collect(),
        ))
    }

    /// The employees of both areas.
    pub fn keep(&self, list: Vec<Employee>) -> Vec<Employee> {
        list.into_iter()
            .filter(|employee| self.allows_employee(employee))
            .collect()
    }
}

/// `true` when the name is in that set — or when there is no set.
fn matches(set: &Option<HashSet<String>>, department: &str) -> bool {
    match set {
        None => true,
        Some(names) => names.contains(&department.trim().to_lowercase()),
    }
}

/// `true` when the id is in the allowed set — or when there is no set.
pub fn allows_id(allowed: Option<&HashSet<i64>>, id: i64) -> bool {
    match allowed {
        None => true,
        Some(ids) => ids.contains(&id),
    }
}

/// Refuses an employee id outside the scope.
pub fn check_id(allowed: Option<&HashSet<i64>>, id: i64) -> Result<(), ApiError> {
    if allows_id(allowed, id) {
        Ok(())
    } else {
        Err(outside_scope())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &[&str]) -> Option<HashSet<String>> {
        Some(
            names
                .iter()
                .map(|name| name.trim().to_lowercase())
                .collect::<HashSet<String>>(),
        )
    }

    fn employee(department: Option<&str>) -> Employee {
        let mut employee = Employee::default();
        employee.department = department.map(str::to_string);
        employee
    }

    #[test]
    fn without_a_group_entry_every_department_counts() {
        let all = PeopleScope::default();
        assert!(all.personal_all());
        assert!(all.dispo_all());
        assert!(all.allows_personal("Beleuchtung"));
        assert!(all.allows_dispo("Beleuchtung"));
        assert!(all.check(&employee(None)).is_ok());
    }

    #[test]
    fn the_two_areas_do_not_open_each_other() {
        let scope = PeopleScope {
            personal: set(&["Beleuchtung"]),
            dispo: set(&["Tontechnik"]),
        };
        assert!(scope.allows_personal(" beleuchtung "));
        assert!(!scope.allows_personal("Tontechnik"));
        assert!(scope.allows_dispo("Tontechnik"));
        assert!(!scope.allows_dispo("Beleuchtung"));
        // The employees of both areas are visible, a third one is not.
        assert!(scope.check(&employee(Some("Beleuchtung"))).is_ok());
        assert!(scope.check(&employee(Some("Tontechnik"))).is_ok());
        assert!(scope.check(&employee(Some("Requisite"))).is_err());
        assert!(scope.check(&employee(None)).is_err());
    }

    #[test]
    fn a_restricted_dispo_takes_no_shift_without_a_department() {
        let scope = PeopleScope {
            personal: None,
            dispo: set(&["Beleuchtung"]),
        };
        assert!(scope.check_dispo(Some("Beleuchtung")).is_ok());
        assert!(scope.check_dispo(Some(" beleuchtung ")).is_ok());
        assert!(scope.check_dispo(Some("Tontechnik")).is_err());
        // Without a department a shift belongs to every dispatcher: a restricted
        // session reads it but does not change it.
        assert!(scope.check_dispo(None).is_err());
        // Without a group entry everything is open, also a shift without one.
        assert!(PeopleScope::default().check_dispo(None).is_ok());
    }

    #[test]
    fn only_personal_is_restricted_and_the_board_stays_open() {
        let scope = PeopleScope {
            personal: set(&["Beleuchtung"]),
            dispo: None,
        };
        assert!(!scope.personal_all());
        assert!(scope.dispo_all());
        assert!(scope.allows_dispo("Requisite"));
        assert!(scope.check_personal(Some("Beleuchtung")).is_ok());
        assert!(scope.check_personal(Some("Requisite")).is_err());
        assert!(scope.check(&employee(Some("Requisite"))).is_ok());
    }

    #[test]
    fn an_empty_scope_sees_nobody() {
        let nichts = PeopleScope {
            personal: set(&[]),
            dispo: set(&[]),
        };
        assert!(!nichts.personal_all());
        assert!(nichts.check(&employee(Some("Beleuchtung"))).is_err());
        assert!(nichts.keep(vec![employee(Some("Beleuchtung"))]).is_empty());
    }
}
