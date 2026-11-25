//! Users and their permission groups (`img/Admin Menü/Benutzerverwaltung`).
//!
//! One row per account in the legacy table `"User_Prefs"` (login and single sign-on),
//! the contact data in `"User_ITPrefs"` (one row per user and mandant), the
//! assignments in `"User_Mandanten"` and `"User_Lager"`, and the groups themselves in
//! `"UserGroups"` with their members in `"UserGroupMember"`. The single rights of a
//! group live in WHERE's own table (see [`crate::permissions`]).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{DomainError, permissions};

/// The four lists behind `Aktuelle Ansicht` of the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum UserView {
    LocalActive,
    MobileActive,
    LocalInactive,
    MobileInactive,
}

impl UserView {
    /// `local-active`, `mobile-active`, `local-inactive`, `mobile-inactive`.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "local-active" | "lokale-aktive" | "" => Some(Self::LocalActive),
            "mobile-active" | "mobile-aktive" => Some(Self::MobileActive),
            "local-inactive" | "lokale-inaktive" => Some(Self::LocalInactive),
            "mobile-inactive" | "mobile-inaktive" => Some(Self::MobileInactive),
            _ => None,
        }
    }

    /// Local logins or mobile logins, active or disabled.
    pub fn filters(self) -> (bool, bool) {
        match self {
            Self::LocalActive => (true, false),
            Self::MobileActive => (false, true),
            Self::LocalInactive => (true, false),
            Self::MobileInactive => (false, true),
        }
    }

    pub fn without(self) -> Self {
        match self {
            Self::LocalActive => Self::MobileActive,
            Self::MobileActive => Self::LocalActive,
            Self::LocalInactive => Self::MobileInactive,
            Self::MobileInactive => Self::LocalInactive,
        }
    }

    pub fn disabled(self) -> bool {
        matches!(self, Self::LocalInactive | Self::MobileInactive)
    }
}

/// The columns the two lists of the main window show.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserSummary {
    pub id: i64,
    /// `User_anzeigeName`.
    pub name: String,
    /// `UserKuerzel` — the `[Adm]` behind the name.
    pub short_name: Option<String>,
    /// `UserGroups` of the user, for the right-hand list.
    pub groups: Vec<GroupRef>,
}

/// One permission group of a user.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GroupRef {
    pub id: i64,
    pub name: String,
}

/// One account with all columns of the four tabs of its dialog.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: i64,
    /// `UserName` — `Anmeldename`.
    pub user_name: String,
    /// `User_anzeigeName` — `Vollständiger Name`.
    pub display_name: Option<String>,
    /// `UserKuerzel`, at most three characters.
    pub short_name: Option<String>,
    /// `Login_local` / `Login_mobile`.
    pub local_login: bool,
    pub mobile_login: bool,
    /// A password is filed — the mask shows a filled field, never the password.
    pub has_password: bool,
    pub change_next_login: bool,
    pub cannot_change: bool,
    pub never_expires: bool,
    pub disabled: bool,
    /// `Language`; empty means "nach Systemsprache".
    pub language: Option<String>,
    /// Two-factor authentication is set up (`HOTP`).
    pub two_factor: bool,
    /// `TA_ColorRed/Green/Blue` as `#rrggbb`.
    pub color: Option<String>,
    pub sso_ad_active: bool,
    pub sso_ad_user: Option<String>,
    pub sso_ad_domain: Option<String>,
    pub sso_mb_active: bool,
    pub sso_mb_os_user: Option<String>,
    pub sso_mb_machine: Option<String>,
    pub sso_default_entries: bool,
    pub sso_default_mandant: Option<i32>,
    pub sso_default_lager: Option<i32>,
    /// Contact data of `"User_ITPrefs"`.
    pub function: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub fax: Option<String>,
    pub email: Option<String>,
    /// `AuthProviderType` — `0` `SMTP Mailserver`, `1` `OAuth 2.0`.
    pub email_provider: i32,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub email_signature: Option<String>,
    pub bcc: Option<String>,
    pub im1_type: Option<String>,
    pub im1: Option<String>,
    pub im2_type: Option<String>,
    pub im2: Option<String>,
    /// `Mandanten_ID` of the contact row — the `Standardprofil für Mandant`.
    pub profile_mandant: Option<i32>,
    /// `com_intDurchwahl` of `"User_Prefs"`.
    pub extension: Option<String>,
    /// An address points at this account (`"Adresse"."User_Primkey"`).
    pub address_linked: bool,
    /// WHERE has no personnel module yet, so this is always false.
    pub personal_linked: bool,
    /// `"User_Mandanten"`.
    pub mandants: Vec<i32>,
    /// `"User_Lager"`.
    pub lager: Vec<i32>,
    pub groups: Vec<GroupRef>,
}

/// What the user dialog sends.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UserInput {
    pub user_name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub short_name: Option<String>,
    #[serde(default)]
    pub local_login: bool,
    #[serde(default)]
    pub mobile_login: bool,
    /// Empty keeps the stored password (or none).
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub change_next_login: bool,
    #[serde(default)]
    pub cannot_change: bool,
    #[serde(default)]
    pub never_expires: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub sso_ad_active: bool,
    #[serde(default)]
    pub sso_ad_user: Option<String>,
    #[serde(default)]
    pub sso_ad_domain: Option<String>,
    #[serde(default)]
    pub sso_mb_active: bool,
    #[serde(default)]
    pub sso_mb_os_user: Option<String>,
    #[serde(default)]
    pub sso_mb_machine: Option<String>,
    #[serde(default)]
    pub sso_default_entries: bool,
    #[serde(default)]
    pub sso_default_mandant: Option<i32>,
    #[serde(default)]
    pub sso_default_lager: Option<i32>,
    #[serde(default)]
    pub function: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub fax: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_provider: Option<i32>,
    #[serde(default)]
    pub smtp_server: Option<String>,
    #[serde(default)]
    pub smtp_port: Option<i32>,
    #[serde(default)]
    pub email_signature: Option<String>,
    #[serde(default)]
    pub bcc: Option<String>,
    #[serde(default)]
    pub im1_type: Option<String>,
    #[serde(default)]
    pub im1: Option<String>,
    #[serde(default)]
    pub im2_type: Option<String>,
    #[serde(default)]
    pub im2: Option<String>,
    #[serde(default)]
    pub profile_mandant: Option<i32>,
    #[serde(default)]
    pub extension: Option<String>,
    #[serde(default)]
    pub mandants: Vec<i32>,
    #[serde(default)]
    pub lager: Vec<i32>,
}

/// Trimmed value; an empty text is no value.
fn optional(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// `#rrggbb` back to the three colour columns of the legacy table.
pub fn split_color(color: &str) -> Option<(i32, i32, i32)> {
    let text = color.trim().trim_start_matches('#');
    if text.len() != 6 {
        return None;
    }
    let red = i32::from_str_radix(&text[0..2], 16).ok()?;
    let green = i32::from_str_radix(&text[2..4], 16).ok()?;
    let blue = i32::from_str_radix(&text[4..6], 16).ok()?;
    Some((red, green, blue))
}

/// The three colour columns back to `#rrggbb`.
pub fn join_color(red: Option<i32>, green: Option<i32>, blue: Option<i32>) -> Option<String> {
    match (red, green, blue) {
        (Some(red), Some(green), Some(blue)) => Some(format!("#{red:02x}{green:02x}{blue:02x}")),
        _ => None,
    }
}

impl UserInput {
    /// Checks the input the way the dialog does.
    pub fn prepare(self) -> Result<PreparedUser, DomainError> {
        let user_name = self.user_name.trim().to_string();
        if user_name.is_empty() {
            return Err(DomainError::Validation(
                "Für den Benutzer fehlt der Anmeldename.".to_string(),
            ));
        }
        let short_name = optional(self.short_name);
        if let Some(short) = short_name.as_deref()
            && short.chars().count() > 3
        {
            return Err(DomainError::Validation(format!(
                "Das Benutzerkürzel „{short}\u{201c} ist länger als drei Zeichen."
            )));
        }
        if !self.local_login && !self.mobile_login {
            return Err(DomainError::Validation(
                "Der Benutzer braucht mindestens eine Anmeldeart (Login local oder Login mobile)."
                    .to_string(),
            ));
        }
        if let Some(color) = self.color.as_deref()
            && !color.trim().is_empty()
            && split_color(color).is_none()
        {
            return Err(DomainError::Validation(format!(
                "„{color}\u{201c} ist keine Farbe (#rrggbb)."
            )));
        }
        Ok(PreparedUser {
            user_name,
            display_name: optional(self.display_name),
            short_name,
            local_login: self.local_login,
            mobile_login: self.mobile_login,
            password: optional(self.password),
            change_next_login: self.change_next_login,
            cannot_change: self.cannot_change,
            never_expires: self.never_expires,
            disabled: self.disabled,
            language: optional(self.language),
            color: optional(self.color),
            sso_ad_active: self.sso_ad_active,
            sso_ad_user: optional(self.sso_ad_user),
            sso_ad_domain: optional(self.sso_ad_domain),
            sso_mb_active: self.sso_mb_active,
            sso_mb_os_user: optional(self.sso_mb_os_user),
            sso_mb_machine: optional(self.sso_mb_machine),
            sso_default_entries: self.sso_default_entries,
            sso_default_mandant: self.sso_default_mandant.filter(|id| *id > 0),
            sso_default_lager: self.sso_default_lager.filter(|id| *id > 0),
            function: optional(self.function),
            phone: optional(self.phone),
            mobile: optional(self.mobile),
            fax: optional(self.fax),
            email: optional(self.email),
            email_provider: self.email_provider.unwrap_or(0),
            smtp_server: optional(self.smtp_server),
            smtp_port: self.smtp_port,
            email_signature: optional(self.email_signature),
            bcc: optional(self.bcc),
            im1_type: optional(self.im1_type),
            im1: optional(self.im1),
            im2_type: optional(self.im2_type),
            im2: optional(self.im2),
            profile_mandant: self.profile_mandant.filter(|id| *id > 0),
            extension: optional(self.extension),
            mandants: clean_ids(self.mandants),
            lager: clean_ids(self.lager),
        })
    }
}

fn clean_ids(ids: Vec<i32>) -> Vec<i32> {
    let mut ids: Vec<i32> = ids.into_iter().filter(|id| *id > 0).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// A checked user, ready for the repository.
#[derive(Debug, Clone)]
pub struct PreparedUser {
    pub user_name: String,
    pub display_name: Option<String>,
    pub short_name: Option<String>,
    pub local_login: bool,
    pub mobile_login: bool,
    pub password: Option<String>,
    pub change_next_login: bool,
    pub cannot_change: bool,
    pub never_expires: bool,
    pub disabled: bool,
    pub language: Option<String>,
    pub color: Option<String>,
    pub sso_ad_active: bool,
    pub sso_ad_user: Option<String>,
    pub sso_ad_domain: Option<String>,
    pub sso_mb_active: bool,
    pub sso_mb_os_user: Option<String>,
    pub sso_mb_machine: Option<String>,
    pub sso_default_entries: bool,
    pub sso_default_mandant: Option<i32>,
    pub sso_default_lager: Option<i32>,
    pub function: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub fax: Option<String>,
    pub email: Option<String>,
    pub email_provider: i32,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub email_signature: Option<String>,
    pub bcc: Option<String>,
    pub im1_type: Option<String>,
    pub im1: Option<String>,
    pub im2_type: Option<String>,
    pub im2: Option<String>,
    pub profile_mandant: Option<i32>,
    pub extension: Option<String>,
    pub mandants: Vec<i32>,
    pub lager: Vec<i32>,
}

/// One account of a group — the child line under a group in the main window.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GroupMember {
    pub id: i64,
    pub name: String,
}

/// One permission group with its two sets of rights.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserGroup {
    pub id: i64,
    /// `Group_Name`.
    pub name: String,
    /// The ticked rights of `Max. Rechte`.
    pub max_rights: Vec<String>,
    /// The ticked rights of `Min. Rechte`.
    pub min_rights: Vec<String>,
    /// The mandants of the first list of the dialog.
    pub mandants: Vec<i32>,
    /// The warehouses of the second list.
    pub lager: Vec<i32>,
    /// The departments the group may work in — `Personal`. Empty = every department.
    pub departments: Vec<i32>,
    /// The departments of `Personal-Disposition`. Empty = every department.
    pub dispo_departments: Vec<i32>,
    /// Number of accounts in the group.
    pub members: usize,
    /// The accounts of the group — the child lines under it in the main window.
    pub member_users: Vec<GroupMember>,
}

/// What the group dialog sends.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UserGroupInput {
    pub name: String,
    #[serde(default)]
    pub max_rights: Vec<String>,
    #[serde(default)]
    pub min_rights: Vec<String>,
    #[serde(default)]
    pub mandants: Vec<i32>,
    #[serde(default)]
    pub lager: Vec<i32>,
    /// Departments of `where_people_departments` for `Personal`; empty = all of them.
    #[serde(default)]
    pub departments: Vec<i32>,
    /// The same for `Personal-Disposition`; empty = all of them.
    #[serde(default)]
    pub dispo_departments: Vec<i32>,
}

impl UserGroupInput {
    /// Checks the name and drops rights that the catalogue does not know.
    pub fn prepare(self) -> Result<UserGroup, DomainError> {
        let name = self.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Für die Berechtigungsgruppe fehlt der Name.".to_string(),
            ));
        }
        let known = permissions::all_right_keys();
        let filter = |keys: Vec<String>| {
            let mut keys: Vec<String> = keys
                .into_iter()
                .map(|key| key.trim().to_string())
                .filter(|key| known.iter().any(|known| known == key))
                .collect();
            keys.sort();
            keys.dedup();
            keys
        };
        Ok(UserGroup {
            id: 0,
            name,
            max_rights: filter(self.max_rights),
            min_rights: filter(self.min_rights),
            mandants: clean_ids(self.mandants),
            lager: clean_ids(self.lager),
            departments: clean_ids(self.departments),
            dispo_departments: clean_ids(self.dispo_departments),
            members: 0,
            member_users: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> UserInput {
        UserInput {
            user_name: "  m.mustermann  ".into(),
            display_name: Some(" Max Mustermann ".into()),
            short_name: Some("MM".into()),
            local_login: true,
            mobile_login: false,
            password: Some("geheim".into()),
            change_next_login: true,
            cannot_change: false,
            never_expires: false,
            disabled: false,
            language: None,
            color: Some("#1a2b3c".into()),
            sso_ad_active: false,
            sso_ad_user: None,
            sso_ad_domain: None,
            sso_mb_active: false,
            sso_mb_os_user: None,
            sso_mb_machine: None,
            sso_default_entries: false,
            sso_default_mandant: Some(0),
            sso_default_lager: None,
            function: Some("  ".into()),
            phone: Some("0731 1234".into()),
            mobile: None,
            fax: None,
            email: None,
            email_provider: None,
            smtp_server: None,
            smtp_port: None,
            email_signature: None,
            bcc: None,
            im1_type: None,
            im1: None,
            im2_type: None,
            im2: None,
            profile_mandant: None,
            extension: Some("4711".into()),
            mandants: vec![1, 1, 0, 2],
            lager: vec![],
        }
    }

    #[test]
    fn a_user_needs_a_login_name_and_a_kind_of_login() {
        let user = input().prepare().unwrap();
        assert_eq!(user.user_name, "m.mustermann");
        assert_eq!(user.display_name.as_deref(), Some("Max Mustermann"));
        assert_eq!(user.function, None);
        assert_eq!(user.sso_default_mandant, None);
        assert_eq!(user.mandants, vec![1, 2]);

        let mut ohne_name = input();
        ohne_name.user_name = "   ".into();
        assert!(ohne_name.prepare().is_err());

        let mut ohne_login = input();
        ohne_login.local_login = false;
        assert!(ohne_login.prepare().is_err());

        let mut langes_kuerzel = input();
        langes_kuerzel.short_name = Some("ABCD".into());
        assert!(langes_kuerzel.prepare().is_err());

        let mut falsche_farbe = input();
        falsche_farbe.color = Some("rot".into());
        assert!(falsche_farbe.prepare().is_err());
    }

    #[test]
    fn colours_travel_as_rrggbb() {
        assert_eq!(split_color("#1a2b3c"), Some((0x1a, 0x2b, 0x3c)));
        assert_eq!(
            join_color(Some(26), Some(43), Some(60)).as_deref(),
            Some("#1a2b3c")
        );
        assert_eq!(join_color(None, Some(0), Some(0)), None);
    }

    fn leere_gruppe() -> UserGroupInput {
        UserGroupInput {
            name: String::new(),
            max_rights: vec![],
            min_rights: vec![],
            mandants: vec![],
            lager: vec![],
            departments: vec![],
            dispo_departments: vec![],
        }
    }

    #[test]
    fn a_group_drops_rights_the_catalogue_does_not_know() {
        let group = UserGroupInput {
            name: " Buchhaltung ".into(),
            max_rights: vec![
                "kassenbuch.kasseneintrag-anlegen".into(),
                "gibt.es.nicht".into(),
                "kassenbuch.kasseneintrag-anlegen".into(),
            ],
            min_rights: vec!["statistiken.zugriff".into()],
            mandants: vec![1],
            lager: vec![3, 3],
            departments: vec![7, 7, 0, 4],
            dispo_departments: vec![4, 0],
        }
        .prepare()
        .unwrap();
        assert_eq!(group.name, "Buchhaltung");
        assert_eq!(group.max_rights, vec!["kassenbuch.kasseneintrag-anlegen"]);
        assert_eq!(group.min_rights, vec!["statistiken.zugriff"]);
        assert_eq!(group.lager, vec![3]);
        // The two lists stay apart.
        assert_eq!(group.departments, vec![4, 7]);
        assert_eq!(group.dispo_departments, vec![4]);

        assert!(leere_gruppe().prepare().is_err());
        let benannt = UserGroupInput {
            name: "Gruppe".into(),
            ..leere_gruppe()
        };
        assert!(benannt.prepare().is_ok());
    }

    #[test]
    fn the_views_parse_and_filter() {
        assert_eq!(
            UserView::parse("mobile-active"),
            Some(UserView::MobileActive)
        );
        assert_eq!(UserView::parse(""), Some(UserView::LocalActive));
        assert_eq!(UserView::parse("irgendwas"), None);
        assert_eq!(UserView::MobileInactive.filters(), (false, true));
        assert!(UserView::LocalInactive.disabled());
        assert!(!UserView::LocalActive.disabled());
    }
}
