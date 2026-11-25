//! Login and logout (`img/Admin Menü/Login Logout`).
//!
//! Accounts live in `"User_Prefs"`; a password is stored as a SHA-256 digest with the
//! prefix `sha256:` (see [`crate::user_repo`]). A session is one row in
//! `"where_sessions"`, keyed by the digest of the token the client holds — the token
//! itself is never written down.

use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use where_domain::{DomainError, LoginResponse, LoginTenant, LoginUser, SessionUser, join_color};

use crate::error::db_error;

/// How long a session stays valid without being used.
const SESSION_DAYS: i32 = 30;

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Digest of a password, the shape `user_repo` writes.
fn hash_password(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex(&hasher.finalize()))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("session:{}", hex(&hasher.finalize()))
}

/// A fresh token: two UUIDs, so 64 hex characters of randomness.
fn new_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Reads the accounts and keeps the sessions of the login.
#[derive(Clone)]
pub struct AuthRepository {
    pool: PgPool,
}

impl AuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The tenants the mask offers next to the account list (`"Mandanten"`).
    pub async fn tenants(&self) -> Result<Vec<LoginTenant>, DomainError> {
        let rows = sqlx::query(
            "SELECT \"Primary_Key\"::bigint AS id, m.\"Shortname\" AS short_name, \
                    COALESCE(m.\"Mandanten_Name\", '') AS name \
             FROM \"Mandanten\" m \
             ORDER BY COALESCE(NULLIF(TRIM(m.\"Mandanten_Name\"), ''), m.\"Shortname\"), \
                      m.\"Primary_Key\"",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(rows
            .iter()
            .map(|row| LoginTenant {
                id: row.get("id"),
                short_name: row.get("short_name"),
                name: row.get("name"),
            })
            .collect())
    }

    /// The accounts the login mask offers: local logins that are not disabled.
    ///
    /// With a tenant the list holds the accounts assigned to it (`"User_Mandanten"`) and
    /// every account holding the admin right — the administrator works in all tenants.
    pub async fn candidates(&self, mandant: Option<i64>) -> Result<Vec<LoginUser>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM ( \
                 SELECT u.\"Primary_Key\"::bigint AS id, COALESCE(u.\"UserName\", '') AS user_name, \
                        u.\"User_anzeigeName\" AS display_name, u.\"UserKuerzel\" AS short_name, \
                        u.\"TA_ColorRed\" AS red, u.\"TA_ColorGreen\" AS green, \
                        u.\"TA_ColorBlue\" AS blue, \
                        (COALESCE(u.\"Password\", '') <> '') AS has_password, \
                        EXISTS (SELECT 1 FROM \"UserGroupMember\" g \
                                  JOIN \"where_user_group_rights\" r \
                                    ON r.\"group_id\" = g.\"UserGroup_PrimKey\" \
                                 WHERE g.\"User_PrimKey\" = u.\"Primary_Key\" \
                                   AND r.\"scope\" = 'max' AND r.\"right_key\" = $2) AS is_admin \
                 FROM \"User_Prefs\" u \
                 WHERE COALESCE(u.\"Login_local\", false) \
                   AND NOT COALESCE(u.\"AccountDisabled\", false) \
             ) k \
             WHERE $1::bigint IS NULL OR k.is_admin OR EXISTS ( \
                     SELECT 1 FROM \"User_Mandanten\" um \
                      WHERE um.\"UserPrimKey\" = k.id AND um.\"MandantenID\" = $1) \
             ORDER BY COALESCE(NULLIF(TRIM(k.display_name), ''), k.user_name)",
        )
        .bind(mandant)
        .bind(where_domain::ADMIN_RIGHT)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let id: i64 = row.get("id");
            out.push(LoginUser {
                id,
                user_name: row.get("user_name"),
                display_name: row.get("display_name"),
                short_name: row.get("short_name"),
                color: join_color(row.get("red"), row.get("green"), row.get("blue")),
                has_password: row.get("has_password"),
                mandants: self.mandants_of(id).await?,
            });
        }
        Ok(out)
    }

    /// The tenants an account is assigned to (`"User_Mandanten"`).
    async fn mandants_of(&self, user: i64) -> Result<Vec<i64>, DomainError> {
        let rows = sqlx::query(
            "SELECT \"MandantenID\"::bigint AS id FROM \"User_Mandanten\" \
             WHERE \"UserPrimKey\" = $1 ORDER BY \"MandantenID\"",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(rows.iter().filter_map(|row| row.get("id")).collect())
    }

    /// Whether an account may work in a tenant: it is assigned to it, or it holds the
    /// admin right.
    async fn allows_mandant(&self, user: i64, mandant: i64) -> Result<bool, DomainError> {
        let allowed: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM \"User_Mandanten\" \
                             WHERE \"UserPrimKey\" = $1 AND \"MandantenID\" = $2) \
                    OR EXISTS (SELECT 1 FROM \"UserGroupMember\" g \
                                 JOIN \"where_user_group_rights\" r \
                                   ON r.\"group_id\" = g.\"UserGroup_PrimKey\" \
                                WHERE g.\"User_PrimKey\" = $1 \
                                  AND r.\"scope\" = 'max' AND r.\"right_key\" = $3)",
        )
        .bind(user)
        .bind(mandant as i32)
        .bind(where_domain::ADMIN_RIGHT)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(allowed)
    }

    /// The name of a tenant for the header, `None` when the id is unknown.
    async fn mandant_name(&self, mandant: i64) -> Result<Option<String>, DomainError> {
        let name: Option<Option<String>> = sqlx::query_scalar(
            "SELECT COALESCE(NULLIF(TRIM(\"Mandanten_Name\"), ''), \"Shortname\") \
             FROM \"Mandanten\" WHERE \"Primary_Key\" = $1",
        )
        .bind(mandant as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(name.flatten())
    }

    /// Checks name and password and opens a session.
    ///
    /// With a tenant the account has to belong to it (assignment or admin right);
    /// otherwise the sign-in is refused. The tenant travels with the session, so the
    /// header can show which installation is open after a reload.
    pub async fn login(
        &self,
        user_name: &str,
        password: &str,
        mandant: Option<i64>,
    ) -> Result<LoginResponse, DomainError> {
        let row = sqlx::query(
            "SELECT \"Primary_Key\"::bigint AS id, COALESCE(\"UserName\", '') AS user_name, \
                    \"User_anzeigeName\" AS display_name, \"UserKuerzel\" AS short_name, \
                    \"TA_ColorRed\" AS red, \"TA_ColorGreen\" AS green, \"TA_ColorBlue\" AS blue, \
                    COALESCE(\"Password\", '') AS hash, \
                    COALESCE(\"Login_local\", false) AS local_login, \
                    COALESCE(\"AccountDisabled\", false) AS disabled, \
                    COALESCE(\"ChangeNextLogin\", false) AS change_next_login \
             FROM \"User_Prefs\" WHERE lower(\"UserName\") = lower($1) \
             ORDER BY \"Primary_Key\" LIMIT 1",
        )
        .bind(user_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;

        let Some(row) = row else {
            return Err(DomainError::Unauthorized(
                "Benutzername oder Kennwort ist falsch.".to_string(),
            ));
        };
        let disabled: bool = row.get("disabled");
        let local_login: bool = row.get("local_login");
        if disabled || !local_login {
            return Err(DomainError::Unauthorized(
                "Dieser Benutzer darf sich nicht lokal anmelden.".to_string(),
            ));
        }
        let hash: String = row.get("hash");
        if hash.is_empty() {
            return Err(DomainError::Unauthorized(
                "Für diesen Benutzer ist kein Kennwort gesetzt.".to_string(),
            ));
        }
        let id: i64 = row.get("id");
        if hash.starts_with("sha256:") {
            if hash != hash_password(password) {
                return Err(DomainError::Unauthorized(
                    "Benutzername oder Kennwort ist falsch.".to_string(),
                ));
            }
        } else if hash == password {
            // A row from the legacy application: the password is still plain text.
            // The successful attempt replaces it with the digest.
            sqlx::query("UPDATE \"User_Prefs\" SET \"Password\" = $2 WHERE \"Primary_Key\" = $1")
                .bind(id)
                .bind(hash_password(password))
                .execute(&self.pool)
                .await
                .map_err(db_error)?;
        } else {
            return Err(DomainError::Unauthorized(
                "Benutzername oder Kennwort ist falsch.".to_string(),
            ));
        }

        // Only accounts of this tenant — the administrator belongs to all of them.
        let mandant_name = match mandant {
            Some(mandant) => {
                if !self.allows_mandant(id, mandant).await? {
                    return Err(DomainError::Forbidden(
                        "Dieser Benutzer gehört nicht zu diesem Mandanten.".to_string(),
                    ));
                }
                self.mandant_name(mandant).await?
            }
            None => None,
        };

        let user = SessionUser {
            id,
            user_name: row.get("user_name"),
            display_name: row.get("display_name"),
            short_name: row.get("short_name"),
            color: join_color(row.get("red"), row.get("green"), row.get("blue")),
            rights: crate::user_repo::effective_rights(&self.pool, id).await?,
            change_next_login: row.get("change_next_login"),
            mandant,
            mandant_name,
            mandants: self.mandants_of(id).await?,
        };
        let token = new_token();
        // Expired sessions drop out here — the table stays small.
        sqlx::query(
            "DELETE FROM \"where_sessions\" \
             WHERE \"last_seen_at\" < now() - make_interval(days => $1)",
        )
        .bind(SESSION_DAYS)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        sqlx::query(
            "INSERT INTO \"where_sessions\" (\"token_hash\", \"user_id\", \"mandant\") \
             VALUES ($1, $2, $3)",
        )
        .bind(hash_token(&token))
        .bind(id)
        .bind(mandant)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        sqlx::query(
            "UPDATE \"User_Prefs\" SET \"CountLogins\" = COALESCE(\"CountLogins\", 0) + 1, \
             \"lastLogin\" = CURRENT_DATE WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(LoginResponse { token, user })
    }

    /// The account behind a token, or `None` when the session is unknown or stale.
    pub async fn session(&self, token: &str) -> Result<Option<SessionUser>, DomainError> {
        if token.trim().is_empty() {
            return Ok(None);
        }
        let row = sqlx::query(
            "SELECT u.\"Primary_Key\"::bigint AS id, COALESCE(u.\"UserName\", '') AS user_name, \
                    u.\"User_anzeigeName\" AS display_name, u.\"UserKuerzel\" AS short_name, \
                    u.\"TA_ColorRed\" AS red, u.\"TA_ColorGreen\" AS green, u.\"TA_ColorBlue\" AS blue, \
                    COALESCE(u.\"ChangeNextLogin\", false) AS change_next_login, \
                    s.\"mandant\"::bigint AS mandant, m.\"Mandanten_Name\" AS mandant_name \
             FROM \"where_sessions\" s JOIN \"User_Prefs\" u ON u.\"Primary_Key\" = s.\"user_id\" \
             LEFT JOIN \"Mandanten\" m ON m.\"Primary_Key\" = s.\"mandant\" \
             WHERE s.\"token_hash\" = $1 \
               AND s.\"last_seen_at\" > now() - make_interval(days => $2) \
               AND NOT COALESCE(u.\"AccountDisabled\", false)",
        )
        .bind(hash_token(token))
        .bind(SESSION_DAYS)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        // The session lives from its last use; one hour is a sufficient rhythm so
        // that not every request writes a row.
        sqlx::query(
            "UPDATE \"where_sessions\" SET \"last_seen_at\" = now() \
             WHERE \"token_hash\" = $1 AND \"last_seen_at\" < now() - interval '1 hour'",
        )
        .bind(hash_token(token))
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(Some(SessionUser {
            id: row.get("id"),
            user_name: row.get("user_name"),
            display_name: row.get("display_name"),
            short_name: row.get("short_name"),
            color: join_color(row.get("red"), row.get("green"), row.get("blue")),
            rights: crate::user_repo::effective_rights(&self.pool, row.get("id")).await?,
            change_next_login: row.get("change_next_login"),
            mandant: row.get("mandant"),
            mandant_name: row.get("mandant_name"),
            mandants: self.mandants_of(row.get("id")).await?,
        }))
    }

    /// Ends a session; an unknown token is not an error.
    pub async fn logout(&self, token: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM \"where_sessions\" WHERE \"token_hash\" = $1")
            .bind(hash_token(token))
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    /// Moves the session of `token` to another tenant the account may work in.
    ///
    /// The tenant decides which database the masks read (`tenant_databases`), so a
    /// switch moves the session, not the data. The account has to be assigned to the
    /// tenant or hold the admin right ([`AuthRepository::allows_mandant`]).
    pub async fn switch_mandant(
        &self,
        token: &str,
        mandant: i64,
    ) -> Result<SessionUser, DomainError> {
        let user = self.session(token).await?.ok_or_else(|| {
            DomainError::Unauthorized(
                "Die Sitzung ist abgelaufen — bitte neu anmelden.".to_string(),
            )
        })?;
        if !self.allows_mandant(user.id, mandant).await? {
            return Err(DomainError::Forbidden(
                "Dieser Benutzer gehört nicht zu diesem Mandanten.".to_string(),
            ));
        }
        if self.mandant_name(mandant).await?.is_none() {
            return Err(DomainError::NotFound {
                entity: "Mandant",
                id: mandant as i32,
            });
        }
        sqlx::query("UPDATE \"where_sessions\" SET \"mandant\" = $2 WHERE \"token_hash\" = $1")
            .bind(hash_token(token))
            .bind(mandant as i32)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        self.session(token).await?.ok_or_else(|| {
            DomainError::Unauthorized(
                "Die Sitzung ist abgelaufen — bitte neu anmelden.".to_string(),
            )
        })
    }

    /// Sets a new password for the account of `token`.
    ///
    /// The new one has to differ from the stored one — otherwise the mask of the first
    /// login could be passed by typing the initial password again. `ChangeNextLogin`
    /// falls with it, so the work may start afterwards. Other sessions of the account
    /// end here; the calling one stays.
    pub async fn change_password(&self, token: &str, password: &str) -> Result<(), DomainError> {
        let row = sqlx::query(
            "SELECT u.\"Primary_Key\"::bigint AS id, COALESCE(u.\"Password\", '') AS hash, \
                    COALESCE(u.\"CannotChange\", false) AS cannot_change \
             FROM \"where_sessions\" s JOIN \"User_Prefs\" u ON u.\"Primary_Key\" = s.\"user_id\" \
             WHERE s.\"token_hash\" = $1 \
               AND s.\"last_seen_at\" > now() - make_interval(days => $2)",
        )
        .bind(hash_token(token))
        .bind(SESSION_DAYS)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let Some(row) = row else {
            return Err(DomainError::Unauthorized(
                "Die Sitzung ist abgelaufen.".to_string(),
            ));
        };
        if row.get::<bool, _>("cannot_change") {
            return Err(DomainError::Forbidden(
                "Dieser Benutzer darf sein Kennwort nicht ändern.".to_string(),
            ));
        }
        let hash: String = row.get("hash");
        if hash == hash_password(password) {
            return Err(DomainError::Validation(
                "Das neue Kennwort muss sich vom bisherigen unterscheiden.".to_string(),
            ));
        }
        let id: i64 = row.get("id");
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(
            "UPDATE \"User_Prefs\" SET \"Password\" = $2, \"ChangeNextLogin\" = false, \
             \"lastPasswordChange\" = CURRENT_DATE WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .bind(hash_password(password))
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        sqlx::query(
            "DELETE FROM \"where_sessions\" WHERE \"user_id\" = $1 AND \"token_hash\" <> $2",
        )
        .bind(id)
        .bind(hash_token(token))
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_long_and_a_digest_is_not_the_token() {
        let token = new_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(token, new_token());
        let digest = hash_token(&token);
        assert!(digest.starts_with("session:"));
        assert!(!digest.contains(&token));
    }

    #[test]
    fn the_password_digest_matches_the_user_repository() {
        // The same shape `user_repo::hash_password` writes — otherwise a password set in
        // the Benutzerverwaltung would not open the login.
        assert_eq!(
            hash_password("geheim"),
            "sha256:addb0f5e7826c857d7376d1bd9bc33c0c544790a2eac96144a8af22b1298c940"
        );
        assert_eq!(
            crate::user_repo::hash_password_for_test("geheim"),
            hash_password("geheim")
        );
    }
}
