//! Master data of the employee mask: departments and positions.
//!
//! Both lists have the same shape — one name per row — and differ only in their
//! table. The mask of the employee suggests the values, here they are
//! maintained.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

/// Longest name a list takes.
pub const MAX_NAME: usize = 100;

/// Name a typed row of the board starts with; the planner renames it in place.
pub const DEFAULT_TEXT_ROW: &str = "Neue Zeile";

/// Which of the lists is meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// `where_people_departments`
    Departments,
    /// `where_people_subdepartments` — the second level under a department.
    Subdepartments,
    /// `where_people_positions`
    Positions,
    /// `where_people_locations` — the stages and rooms of the disposition.
    Locations,
}

impl ListKind {
    /// From the path `/people/lists/{kind}`.
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "departments" => Some(Self::Departments),
            "subdepartments" => Some(Self::Subdepartments),
            "positions" => Some(Self::Positions),
            "locations" => Some(Self::Locations),
            _ => None,
        }
    }

    /// The table the entries live in.
    pub fn table(self) -> &'static str {
        match self {
            Self::Departments => "where_people_departments",
            Self::Subdepartments => "where_people_subdepartments",
            Self::Positions => "where_people_positions",
            Self::Locations => "where_people_locations",
        }
    }
}

/// An entry of one of the lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ListEntry {
    pub id: i64,
    pub name: String,
    /// Department a location or a sub-department belongs to: the department that
    /// works in that house, or the department a sub-department sits under. Empty
    /// means "no parent" — every department may work in a location without one.
    #[serde(default)]
    pub department: Option<String>,
    /// Only a location carries this: `false` keeps the foreman out of the house,
    /// so neither the derived demand nor the generator plans one there.
    #[serde(default = "needs_foreman_default")]
    pub needs_foreman: bool,
    /// Only a location carries this: the row was typed on the board instead of being
    /// maintained as a house, so its name is free text and it stays out of the
    /// employee mask and the master-data list.
    #[serde(default)]
    pub is_text: bool,
    /// Only a department carries this: the tint of its employees in the duty
    /// board, as `#RRGGBB`. Empty means no tint.
    #[serde(default)]
    pub color: Option<String>,
}

/// Default of `needs_foreman`: a house is staffed with a foreman unless the
/// location says otherwise.
pub fn needs_foreman_default() -> bool {
    true
}

/// Whether a location is staffed with a foreman.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LocationForemanInput {
    pub needs_foreman: bool,
}

/// What the mask sends to create and rename.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListEntryInput {
    pub name: String,
}

/// Which department staffs a location.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LocationDepartmentInput {
    /// Empty clears the binding, so the location takes any department again.
    #[serde(default)]
    pub department: String,
}

/// The colour of a list entry; an empty string removes it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListColorInput {
    #[serde(default)]
    pub color: String,
}

impl ListColorInput {
    /// Trims the value and checks the `#RRGGBB` form.
    pub fn clean(&self) -> Result<Option<String>, DomainError> {
        let color = self.color.trim();
        if color.is_empty() {
            return Ok(None);
        }
        if !crate::shifts::is_color(color) {
            return Err(DomainError::Validation(
                "The colour is given as #RRGGBB.".to_string(),
            ));
        }
        Ok(Some(color.to_ascii_uppercase()))
    }
}

impl LocationDepartmentInput {
    /// Trims the name; an empty one means nobody is bound.
    pub fn clean(&self) -> Result<Option<String>, DomainError> {
        let department = self.department.trim();
        if department.is_empty() {
            return Ok(None);
        }
        if department.chars().count() > MAX_NAME {
            return Err(DomainError::Validation(format!(
                "The department must not be longer than {MAX_NAME} characters."
            )));
        }
        Ok(Some(department.to_string()))
    }
}

impl ListEntryInput {
    /// Trims the name and checks that it is neither empty nor too long.
    pub fn clean(&self) -> Result<String, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "The name must not be empty.".to_string(),
            ));
        }
        if name.chars().count() > MAX_NAME {
            return Err(DomainError::Validation(format!(
                "The name must not be longer than {MAX_NAME} characters."
            )));
        }
        Ok(name.to_string())
    }
}

/// The new order of the list — the ids from top to bottom.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListOrderInput {
    #[serde(default)]
    pub ids: Vec<i64>,
}

impl ListOrderInput {
    /// Removes duplicates so an id does not appear twice in the order.
    pub fn clean_ids(&self) -> Vec<i64> {
        let mut ids = Vec::with_capacity(self.ids.len());
        for id in &self.ids {
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_names_are_the_lists() {
        assert_eq!(
            ListKind::from_key("departments"),
            Some(ListKind::Departments)
        );
        assert_eq!(ListKind::from_key("positions"), Some(ListKind::Positions));
        assert_eq!(ListKind::from_key("locations"), Some(ListKind::Locations));
        assert_eq!(
            ListKind::from_key("subdepartments"),
            Some(ListKind::Subdepartments)
        );
        assert_eq!(ListKind::from_key("teams"), None);
        assert_eq!(ListKind::Departments.table(), "where_people_departments");
        assert_eq!(ListKind::Positions.table(), "where_people_positions");
        assert_eq!(ListKind::Locations.table(), "where_people_locations");
        assert_eq!(
            ListKind::Subdepartments.table(),
            "where_people_subdepartments"
        );
    }

    #[test]
    fn a_location_department_is_optional_and_trimmed() {
        let input = LocationDepartmentInput {
            department: "  Podium ".into(),
        };
        assert_eq!(input.clean().unwrap(), Some("Podium".to_string()));
        let empty = LocationDepartmentInput::default();
        assert_eq!(empty.clean().unwrap(), None);
        let too_long = LocationDepartmentInput {
            department: "x".repeat(MAX_NAME + 1),
        };
        assert!(too_long.clean().is_err());
    }

    #[test]
    fn a_name_is_trimmed_and_required() {
        let input = ListEntryInput {
            name: "  Beleuchtung  ".into(),
        };
        assert_eq!(input.clean().unwrap(), "Beleuchtung");
        assert!(ListEntryInput::default().clean().is_err());
        assert!(ListEntryInput { name: "   ".into() }.clean().is_err());
    }

    #[test]
    fn a_name_may_not_exceed_the_column_width() {
        let input = ListEntryInput {
            name: "x".repeat(MAX_NAME + 1),
        };
        assert!(input.clean().is_err());
        let input = ListEntryInput {
            name: "x".repeat(MAX_NAME),
        };
        assert!(input.clean().is_ok());
    }

    #[test]
    fn the_department_colour_is_a_hex_value_or_empty() {
        let input = ListColorInput {
            color: " #15803d ".into(),
        };
        // Stored in upper case like every other colour of the module.
        assert_eq!(input.clean().unwrap(), Some("#15803D".to_string()));
        // An empty field removes the tint.
        assert_eq!(ListColorInput::default().clean().unwrap(), None);
        for wrong in ["15803D", "#15803", "#15803DD", "rot", "#GGGGGG"] {
            assert!(
                ListColorInput {
                    color: wrong.into()
                }
                .clean()
                .is_err(),
                "{wrong} wurde angenommen"
            );
        }
    }

    #[test]
    fn the_order_keeps_each_id_once() {
        let order = ListOrderInput {
            ids: vec![3, 1, 3, 2, 1],
        };
        assert_eq!(order.clean_ids(), vec![3, 1, 2]);
        assert!(ListOrderInput::default().clean_ids().is_empty());
    }
}
