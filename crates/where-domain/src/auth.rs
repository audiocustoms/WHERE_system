//! Login and logout (`img/Admin Menü/Login Logout`).
//!
//! The mask offers the accounts of the installation in a list and a manual entry of
//! name and password. A successful sign-in returns a token; the browser keeps it in
//! `sessionStorage`, so closing the window or the desktop application ends the
//! session. The server stores only the digest of the token.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::DomainError;

/// The right that opens the `Admin` menu and every settings endpoint.
///
/// It is the checkbox `Zugriff erlauben` of the area `administrative Einstellungen`
/// in the branch `Admin` of the rights tree.
pub const ADMIN_RIGHT: &str = "administrative-einstellungen.zugriff";

/// One account of the picker on the login mask.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginUser {
    pub id: i64,
    /// `UserName` — what the manual entry has to type.
    pub user_name: String,
    /// `User_anzeigeName` — the name the list shows.
    pub display_name: Option<String>,
    /// `UserKuerzel` — the `[Adm]` behind the name.
    pub short_name: Option<String>,
    /// `TA_ColorRed/Green/Blue` as `#rrggbb`, for the dot in the list.
    pub color: Option<String>,
    /// A password is filed, so the account can be opened.
    pub has_password: bool,
    /// `"Mandanten"."Primary_Key"` of the tenants the account is assigned to
    /// (`"User_Mandanten"`). Empty means "only through the admin right".
    #[serde(default)]
    pub mandants: Vec<i64>,
}

/// One tenant of the picker next to the account list.
///
/// Every tenant has its own database; the picker selects which installation the account
/// belongs to, and only the accounts of that tenant are offered.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginTenant {
    pub id: i64,
    /// `Shortname` — the abbreviation in front of the name.
    pub short_name: Option<String>,
    /// `Mandanten_Name`.
    pub name: String,
}

/// What the login mask sends.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub user_name: String,
    #[serde(default)]
    pub password: String,
    /// The tenant chosen in the mask (`"Mandanten"."Primary_Key"`); `None` = every tenant.
    #[serde(default)]
    pub mandant: Option<i64>,
}

/// A new password, twice — the mask asks for it and the API checks it again.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PasswordChange {
    pub password: String,
    #[serde(default)]
    pub repeat: String,
}

impl PasswordChange {
    /// The shortest password the installation accepts.
    pub const MIN_LENGTH: usize = 8;

    /// Checks the two fields and returns the trimmed new password.
    pub fn prepare(self) -> Result<String, DomainError> {
        if self.password.chars().count() < Self::MIN_LENGTH {
            return Err(DomainError::Validation(format!(
                "Das neue Kennwort muss mindestens {} Zeichen haben.",
                Self::MIN_LENGTH
            )));
        }
        if self.password != self.repeat {
            return Err(DomainError::Validation(
                "Die Kennwörter stimmen nicht überein.".to_string(),
            ));
        }
        Ok(self.password)
    }
}

/// The signed-in account as the header shows it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionUser {
    pub id: i64,
    pub user_name: String,
    pub display_name: Option<String>,
    pub short_name: Option<String>,
    pub color: Option<String>,
    /// The rights the account holds through its permission groups
    /// (`where_user_group_rights`) — the union of the `Max. Rechte` and
    /// `Min. Rechte` sets of every group it belongs to.
    #[serde(default)]
    pub rights: Vec<String>,
    /// `"User_Prefs"."ChangeNextLogin"`: the password has to be changed before the
    /// work may continue — the account of a fresh installation starts like this.
    #[serde(default)]
    pub change_next_login: bool,
    /// The tenant chosen in the mask (`"Mandanten"."Primary_Key"`), `None` when the
    /// sign-in did not name one.
    #[serde(default)]
    pub mandant: Option<i64>,
    /// Its name, so the header can show which installation is open.
    #[serde(default)]
    pub mandant_name: Option<String>,
    /// The tenants the account is assigned to (`"User_Mandanten"`); the admin right
    /// opens all of them.
    #[serde(default)]
    pub mandants: Vec<i64>,
}

impl SessionUser {
    /// The name the header prints: the full name, otherwise the login name.
    pub fn label(&self) -> String {
        let name = self
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(&self.user_name);
        match self
            .short_name
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            Some(short) => format!("{name} [{short}]"),
            None => name.to_string(),
        }
    }

    /// Whether the account holds a single right.
    pub fn has(&self, key: &str) -> bool {
        self.rights.iter().any(|right| right == key)
    }

    /// Whether the account may open the `Admin` menu and the settings endpoints.
    pub fn is_admin(&self) -> bool {
        self.has(ADMIN_RIGHT)
    }

    /// Whether the work may start: a pending password change blocks everything.
    pub fn must_change_password(&self) -> bool {
        self.change_next_login
    }
}

/// A successful sign-in: the token for `Authorization: Bearer …` and the account.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: SessionUser,
}

impl LoginRequest {
    /// Trims the input the way the mask does: name, password and the tenant.
    pub fn prepare(self) -> Result<(String, String, Option<i64>), DomainError> {
        let user_name = self.user_name.trim().to_string();
        if user_name.is_empty() {
            return Err(DomainError::Validation(
                "Bitte den Benutzernamen eingeben.".to_string(),
            ));
        }
        if self.password.is_empty() {
            return Err(DomainError::Validation(
                "Bitte das Kennwort eingeben.".to_string(),
            ));
        }
        Ok((user_name, self.password, self.mandant))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_label_holds_the_short_name() {
        let user = SessionUser {
            id: 1,
            user_name: "admin".into(),
            display_name: Some("Administrator".into()),
            short_name: Some("Adm".into()),
            color: None,
            rights: Vec::new(),
            change_next_login: false,
            mandant: None,
            mandant_name: None,
            mandants: Vec::new(),
        };
        assert_eq!(user.label(), "Administrator [Adm]");

        let ohne = SessionUser {
            display_name: None,
            short_name: None,
            ..user
        };
        assert_eq!(ohne.label(), "admin");
    }

    #[test]
    fn the_admin_right_opens_the_admin_menu() {
        let mut user = SessionUser {
            id: 2,
            user_name: "user1".into(),
            display_name: None,
            short_name: None,
            color: None,
            rights: vec!["artikel.artikel-anlegen".into()],
            change_next_login: false,
            mandant: Some(1),
            mandant_name: Some("Theater Ulm".into()),
            mandants: vec![1],
        };
        assert_eq!(user.mandant_name.as_deref(), Some("Theater Ulm"));
        assert!(user.has("artikel.artikel-anlegen"));
        assert!(!user.has("artikel.artikel-loeschen"));
        assert!(!user.is_admin());

        user.rights.push(ADMIN_RIGHT.to_string());
        assert!(user.is_admin());
    }

    #[test]
    fn a_new_password_needs_eight_characters_and_the_same_twice() {
        let ok = PasswordChange {
            password: "geheim123".into(),
            repeat: "geheim123".into(),
        }
        .prepare()
        .unwrap();
        assert_eq!(ok, "geheim123");

        assert!(
            PasswordChange {
                password: "kurz".into(),
                repeat: "kurz".into(),
            }
            .prepare()
            .is_err()
        );
        assert!(
            PasswordChange {
                password: "geheim123".into(),
                repeat: "geheim124".into(),
            }
            .prepare()
            .is_err()
        );
    }

    #[test]
    fn a_login_needs_a_name_and_a_password() {
        let ok = LoginRequest {
            user_name: " admin ".into(),
            password: "geheim".into(),
            mandant: Some(3),
        }
        .prepare()
        .unwrap();
        assert_eq!(ok, ("admin".to_string(), "geheim".to_string(), Some(3)));

        assert!(
            LoginRequest {
                user_name: "  ".into(),
                password: "geheim".into(),
                mandant: None,
            }
            .prepare()
            .is_err()
        );
        assert!(
            LoginRequest {
                user_name: "admin".into(),
                password: String::new(),
                mandant: Some(1),
            }
            .prepare()
            .is_err()
        );
    }
}
