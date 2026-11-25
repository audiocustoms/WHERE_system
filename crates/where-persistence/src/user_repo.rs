//! Users and permission groups (`img/Admin Menü/Benutzerverwaltung`).
//!
//! Accounts live in `"User_Prefs"`, their contact data in `"User_ITPrefs"` (one row per
//! user and mandant), their assignments in `"User_Mandanten"` and `"User_Lager"`. The
//! groups are `"UserGroups"` with their members in `"UserGroupMember"`; the single
//! rights and the scope of a group live in WHERE's own tables
//! (`where_user_group_rights`, `where_user_group_scope`), because the legacy table
//! only carries one number per module.
//!
//! WHERE has no login yet: a password is stored as a SHA-256 digest with the prefix
//! `sha256:` so the legacy column never carries plain text.
//!
//! The rights of an account are the rights of the groups it belongs to —
//! [`effective_rights`] resolves them; the API layer checks them per route.

use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use where_domain::{
    DomainError, GroupMember, GroupRef, PreparedUser, User, UserGroup, UserGroupInput, UserInput,
    UserSummary, UserView, join_color, permissions, split_color,
};

use crate::error::db_error;

/// The e-mail account an account sends with — `"User_ITPrefs"` of the installation.
///
/// The password never leaves the server: it is read here, used by the transport and
/// never handed to a mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailAccount {
    /// `"Emailadress"`; empty when the account names none.
    pub email: String,
    pub smtp_server: String,
    pub smtp_port: u16,
    /// `"EmailPassword"` as stored; empty when the account has none.
    pub password: String,
    /// `"emailsignatur"`, appended to a message when it is set.
    pub signature: Option<String>,
    /// `"IT_BCCAdress"` when `"IT_SetBCC"` is set.
    pub bcc: Option<String>,
}

/// The rights of an account: the union of the `Max. Rechte` sets of every group it
/// belongs to.
///
/// `Min. Rechte` of the group dialog does **not** grant anything — it is the second
/// list of the mask, but only what stands on `Max. Rechte` opens a route. Keys that are
/// not in the catalogue are dropped, so a stale row of an older installation cannot
/// open a route.
pub async fn effective_rights(pool: &PgPool, user_id: i64) -> Result<Vec<String>, DomainError> {
    let rows = sqlx::query(
        "SELECT DISTINCT r.\"right_key\" AS key \
         FROM \"UserGroupMember\" m \
         JOIN \"where_user_group_rights\" r ON r.\"group_id\" = m.\"UserGroup_PrimKey\" \
         WHERE m.\"User_PrimKey\" = $1 AND r.\"scope\" = 'max'",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    let keys: Vec<String> = rows.iter().map(|row| row.get("key")).collect();
    Ok(known_rights(keys))
}

/// Sorts the keys and keeps the ones the catalogue knows.
pub fn known_rights(keys: Vec<String>) -> Vec<String> {
    let known: std::collections::HashSet<String> =
        permissions::all_right_keys().into_iter().collect();
    let mut out: Vec<String> = keys.into_iter().filter(|key| known.contains(key)).collect();
    out.sort();
    out.dedup();
    out
}

/// The administrator a fresh installation brings along.
pub const INITIAL_ADMIN_NAME: &str = "admin";
/// Its initial password; it has to be changed on the first sign-in.
pub const INITIAL_ADMIN_PASSWORD: &str = "123456789";
/// The group this administrator gets: the whole rights catalogue.
pub const INITIAL_ADMIN_GROUP: &str = "Administratoren";

/// Creates the initial administrator when the installation has no account yet.
///
/// Standalone/demo and server both pass through the server start; after the
/// migrations this is the one place where the initial installation gets its
/// account. The account carries `ChangeNextLogin`, so signing in leads to the
/// password change first. It joins the `Administratoren` group with all rights
/// — without a group it would have none.
///
/// Returns `true` when the account was created.
pub async fn ensure_initial_admin(pool: &PgPool) -> Result<bool, DomainError> {
    let vorhanden: i64 = sqlx::query_scalar("SELECT count(*) FROM \"User_Prefs\"")
        .fetch_one(pool)
        .await
        .map_err(db_error)?;
    if vorhanden > 0 {
        return Ok(false);
    }
    let mut tx = pool.begin().await.map_err(db_error)?;
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO \"User_Prefs\" (\"UserName\", \"User_anzeigeName\", \"UserKuerzel\", \
            \"Password\", \"Login_local\", \"ChangeNextLogin\", \"NeverExpires\", \
            \"AccountDisabled\", \"TA_ColorRed\", \"TA_ColorGreen\", \"TA_ColorBlue\") \
         VALUES ($1, 'Administrator', 'Adm', $2, true, true, true, false, 0, 0, 0) \
         RETURNING \"Primary_Key\"",
    )
    .bind(INITIAL_ADMIN_NAME)
    .bind(hash_password(INITIAL_ADMIN_PASSWORD))
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;

    let group: i32 = sqlx::query_scalar(
        "INSERT INTO \"UserGroups\" (\"Group_Name\", \"Owner_User_PrimKey\") VALUES ($1, $2) \
         RETURNING \"Primary_Key\"",
    )
    .bind(INITIAL_ADMIN_GROUP)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;
    for key in permissions::all_right_keys() {
        sqlx::query(
            "INSERT INTO \"where_user_group_rights\" (\"group_id\", \"scope\", \"right_key\") \
             VALUES ($1, 'max', $2) ON CONFLICT DO NOTHING",
        )
        .bind(group)
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    }
    sqlx::query(
        "INSERT INTO \"UserGroupMember\" (\"User_PrimKey\", \"UserGroup_PrimKey\") VALUES ($1, $2)",
    )
    .bind(id)
    .bind(group)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    Ok(true)
}

/// Digest of a password the way it is written into `"User_Prefs"."Password"`.
pub(crate) fn hash_password(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex(&hasher.finalize()))
}

/// For the test of `auth_repo`: both layers must produce the same digest.
#[cfg(test)]
pub(crate) fn hash_password_for_test(text: &str) -> String {
    hash_password(text)
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

const USER_COLUMNS: &str = "u.\"Primary_Key\"::bigint AS id, \
     COALESCE(u.\"UserName\", '') AS user_name, u.\"User_anzeigeName\" AS display_name, \
     u.\"UserKuerzel\" AS short_name, COALESCE(u.\"Login_local\", false) AS local_login, \
     COALESCE(u.\"Login_mobile\", false) AS mobile_login, \
     (COALESCE(u.\"Password\", '') <> '') AS has_password, \
     COALESCE(u.\"ChangeNextLogin\", false) AS change_next_login, \
     COALESCE(u.\"CannotChange\", false) AS cannot_change, \
     COALESCE(u.\"NeverExpires\", false) AS never_expires, \
     COALESCE(u.\"AccountDisabled\", false) AS disabled, u.\"Language\" AS language, \
     (u.\"HOTP\" IS NOT NULL AND length(u.\"HOTP\") > 0) AS two_factor, \
     u.\"TA_ColorRed\" AS color_red, u.\"TA_ColorGreen\" AS color_green, \
     u.\"TA_ColorBlue\" AS color_blue, COALESCE(u.\"SSO_AD_active\", false) AS sso_ad_active, \
     u.\"SSO_ActiveDirectoryName\" AS sso_ad_user, u.\"SSO_Domain\" AS sso_ad_domain, \
     COALESCE(u.\"SSO_MB_active\", false) AS sso_mb_active, \
     u.\"SSO_MB_OSuser\" AS sso_mb_os_user, u.\"SSO_MB_MachineName\" AS sso_mb_machine, \
     COALESCE(u.\"SSO_defaultEntries\", false) AS sso_default_entries, \
     u.\"SSO_defaultMandant\" AS sso_default_mandant, u.\"SSO_defaultLager\" AS sso_default_lager, \
     u.\"com_intDurchwahl\" AS extension, i.\"com_Funktion\" AS funktion, \
     i.\"com_Telefon\" AS phone, i.\"com_Mobile\" AS mobile, i.\"com_Fax\" AS fax, \
     COALESCE(i.\"com_eMail\", i.\"Emailadress\") AS email, \
     COALESCE(i.\"AuthProviderType\", 0) AS email_provider, i.\"SMTPServer\" AS smtp_server, \
     i.\"SMTPPort\" AS smtp_port, i.\"emailsignatur\" AS email_signature, \
     i.\"IT_BCCAdress\" AS bcc, i.\"com_IM1Type\" AS im1_type, i.\"com_IM1\" AS im1, \
     i.\"com_IM2Type\" AS im2_type, i.\"com_IM2\" AS im2, i.\"Mandanten_ID\" AS profile_mandant, \
     EXISTS (SELECT 1 FROM \"Adresse\" a WHERE a.\"User_Primkey\" = u.\"Primary_Key\") AS address_linked";

const USER_FROM: &str = "\"User_Prefs\" u \
     LEFT JOIN LATERAL (SELECT * FROM \"User_ITPrefs\" ip \
         WHERE ip.\"User_PrimKey\" = u.\"Primary_Key\" ORDER BY ip.\"Primary_Key\" LIMIT 1) i ON true";

fn row_to_user(row: &sqlx::postgres::PgRow) -> User {
    let red: Option<i32> = row.get("color_red");
    let green: Option<i32> = row.get("color_green");
    let blue: Option<i32> = row.get("color_blue");
    User {
        id: row.get("id"),
        user_name: row.get("user_name"),
        display_name: row.get("display_name"),
        short_name: row.get("short_name"),
        local_login: row.get("local_login"),
        mobile_login: row.get("mobile_login"),
        has_password: row.get("has_password"),
        change_next_login: row.get("change_next_login"),
        cannot_change: row.get("cannot_change"),
        never_expires: row.get("never_expires"),
        disabled: row.get("disabled"),
        language: row.get("language"),
        two_factor: row.get("two_factor"),
        color: join_color(red, green, blue),
        sso_ad_active: row.get("sso_ad_active"),
        sso_ad_user: row.get("sso_ad_user"),
        sso_ad_domain: row.get("sso_ad_domain"),
        sso_mb_active: row.get("sso_mb_active"),
        sso_mb_os_user: row.get("sso_mb_os_user"),
        sso_mb_machine: row.get("sso_mb_machine"),
        sso_default_entries: row.get("sso_default_entries"),
        sso_default_mandant: row.get("sso_default_mandant"),
        sso_default_lager: row.get("sso_default_lager"),
        function: row.get("funktion"),
        phone: row.get("phone"),
        mobile: row.get("mobile"),
        fax: row.get("fax"),
        email: row.get("email"),
        email_provider: row.get("email_provider"),
        smtp_server: row.get("smtp_server"),
        smtp_port: row.get("smtp_port"),
        email_signature: row.get("email_signature"),
        bcc: row.get("bcc"),
        im1_type: row.get("im1_type"),
        im1: row.get("im1"),
        im2_type: row.get("im2_type"),
        im2: row.get("im2"),
        profile_mandant: row.get("profile_mandant"),
        extension: row.get("extension"),
        address_linked: row.get("address_linked"),
        personal_linked: false,
        mandants: Vec::new(),
        lager: Vec::new(),
        groups: Vec::new(),
    }
}

/// Reads and writes the accounts of the installation.
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The e-mail account of one user — its address, server, port and password.
    ///
    /// `None` when the account has no row in `"User_ITPrefs"`; an incomplete row comes
    /// back as it stands, so the caller can name what is missing.
    pub async fn mail_account(&self, user_id: i64) -> Result<Option<MailAccount>, DomainError> {
        let row = sqlx::query(
            "SELECT COALESCE(NULLIF(TRIM(i.\"Emailadress\"), ''), \
                          NULLIF(TRIM(i.\"com_eMail\"), ''), '') AS email, \
                    COALESCE(TRIM(i.\"SMTPServer\"), '') AS smtp_server, \
                    COALESCE(i.\"SMTPPort\", 0) AS smtp_port, \
                    COALESCE(i.\"EmailPassword\", '') AS password, \
                    NULLIF(TRIM(i.\"emailsignatur\"), '') AS signature, \
                    CASE WHEN COALESCE(i.\"IT_SetBCC\", false) \
                         THEN NULLIF(TRIM(i.\"IT_BCCAdress\"), '') END AS bcc \
             FROM \"User_ITPrefs\" i \
             WHERE i.\"User_PrimKey\" = $1 \
             ORDER BY i.\"Primary_Key\" DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let port: i32 = row.get("smtp_port");
        Ok(Some(MailAccount {
            email: row.get("email"),
            smtp_server: row.get("smtp_server"),
            smtp_port: u16::try_from(port).unwrap_or(0),
            password: row.get("password"),
            signature: row.get("signature"),
            bcc: row.get("bcc"),
        }))
    }

    /// The users of one list of `Aktuelle Ansicht`, with their groups.
    pub async fn list(&self, view: UserView) -> Result<Vec<UserSummary>, DomainError> {
        let (local, mobile) = view.filters();
        let sql = "SELECT u.\"Primary_Key\"::bigint AS id, \
                   COALESCE(NULLIF(TRIM(u.\"User_anzeigeName\"), ''), u.\"UserName\", '') AS name, \
                   u.\"UserKuerzel\" AS short_name \
                   FROM \"User_Prefs\" u \
                   WHERE COALESCE(u.\"AccountDisabled\", false) = $1 \
                     AND ((COALESCE(u.\"Login_local\", false) AND $2::boolean) \
                       OR (COALESCE(u.\"Login_mobile\", false) AND $3::boolean)) \
                   ORDER BY name NULLS LAST, u.\"Primary_Key\"";
        let rows = sqlx::query(sql)
            .bind(view.disabled())
            .bind(local)
            .bind(mobile)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let groups = self.group_names().await?;
        Ok(rows
            .iter()
            .map(|row| {
                let id: i64 = row.get("id");
                UserSummary {
                    id,
                    name: row.get::<String, _>("name"),
                    short_name: row.get("short_name"),
                    groups: groups.get(&id).cloned().unwrap_or_default(),
                }
            })
            .collect())
    }

    /// Every group membership, keyed by user.
    async fn group_names(
        &self,
    ) -> Result<std::collections::HashMap<i64, Vec<GroupRef>>, DomainError> {
        let rows = sqlx::query(
            "SELECT m.\"User_PrimKey\"::bigint AS user_id, g.\"Primary_Key\"::bigint AS id, \
                    COALESCE(g.\"Group_Name\", '') AS name \
             FROM \"UserGroupMember\" m \
             JOIN \"UserGroups\" g ON g.\"Primary_Key\" = m.\"UserGroup_PrimKey\" \
             ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut map: std::collections::HashMap<i64, Vec<GroupRef>> =
            std::collections::HashMap::new();
        for row in rows {
            let user_id: Option<i64> = row.get("user_id");
            let entry = GroupRef {
                id: row.get("id"),
                name: row.get("name"),
            };
            if let Some(user_id) = user_id {
                map.entry(user_id).or_default().push(entry);
            }
        }
        Ok(map)
    }

    pub async fn get(&self, id: i64) -> Result<User, DomainError> {
        let sql = format!("SELECT {USER_COLUMNS} FROM {USER_FROM} WHERE u.\"Primary_Key\" = $1");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        let mut user = row.as_ref().map(row_to_user).ok_or(DomainError::NotFound {
            entity: "Benutzer",
            id: id as i32,
        })?;
        user.mandants = self.mandants(id).await?;
        user.lager = self.lager(id).await?;
        user.groups = self
            .group_names()
            .await?
            .get(&id)
            .cloned()
            .unwrap_or_default();
        Ok(user)
    }

    async fn mandants(&self, user: i64) -> Result<Vec<i32>, DomainError> {
        let rows = sqlx::query(
            "SELECT \"MandantenID\" AS id FROM \"User_Mandanten\" WHERE \"UserPrimKey\" = $1 \
             ORDER BY \"MandantenID\"",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(rows.iter().filter_map(|row| row.get("id")).collect())
    }

    async fn lager(&self, user: i64) -> Result<Vec<i32>, DomainError> {
        let rows = sqlx::query(
            "SELECT \"Lager_pk\" AS id FROM \"User_Lager\" WHERE \"UserPrimKey\" = $1 \
             ORDER BY \"Lager_pk\"",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(rows.iter().filter_map(|row| row.get("id")).collect())
    }

    pub async fn create(&self, input: UserInput) -> Result<User, DomainError> {
        let user = input.prepare()?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let (red, green, blue) = user
            .color
            .as_deref()
            .and_then(split_color)
            .map(|(r, g, b)| (Some(r), Some(g), Some(b)))
            .unwrap_or((None, None, None));
        let password = user.password.as_deref().map(hash_password);
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO \"User_Prefs\" (\"UserName\", \"User_anzeigeName\", \"UserKuerzel\", \
             \"Login_local\", \"Login_mobile\", \"Password\", \"ChangeNextLogin\", \"CannotChange\", \
             \"NeverExpires\", \"AccountDisabled\", \"Language\", \"TA_ColorRed\", \"TA_ColorGreen\", \
             \"TA_ColorBlue\", \"SSO_AD_active\", \"SSO_ActiveDirectoryName\", \"SSO_Domain\", \
             \"SSO_MB_active\", \"SSO_MB_OSuser\", \"SSO_MB_MachineName\", \"SSO_defaultEntries\", \
             \"SSO_defaultMandant\", \"SSO_defaultLager\", \"com_intDurchwahl\", \"CountLogins\", \
             \"lastPasswordChange\") \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
                     $18, $19, $20, $21, $22, $23, $24, 0, CURRENT_DATE) \
             RETURNING \"Primary_Key\"::bigint",
        )
        .bind(&user.user_name)
        .bind(&user.display_name)
        .bind(&user.short_name)
        .bind(user.local_login)
        .bind(user.mobile_login)
        .bind(&password)
        .bind(user.change_next_login)
        .bind(user.cannot_change)
        .bind(user.never_expires)
        .bind(user.disabled)
        .bind(&user.language)
        .bind(red)
        .bind(green)
        .bind(blue)
        .bind(user.sso_ad_active)
        .bind(&user.sso_ad_user)
        .bind(&user.sso_ad_domain)
        .bind(user.sso_mb_active)
        .bind(&user.sso_mb_os_user)
        .bind(&user.sso_mb_machine)
        .bind(user.sso_default_entries)
        .bind(user.sso_default_mandant)
        .bind(user.sso_default_lager)
        .bind(&user.extension)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        Self::write_assignments(&mut tx, id, &user).await?;
        Self::write_contact(&mut tx, id, &user).await?;
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(&self, id: i64, input: UserInput) -> Result<User, DomainError> {
        let user = input.prepare()?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let (red, green, blue) = user
            .color
            .as_deref()
            .and_then(split_color)
            .map(|(r, g, b)| (Some(r), Some(g), Some(b)))
            .unwrap_or((None, None, None));
        let password = user.password.as_deref().map(hash_password);
        let changed = sqlx::query(
            "UPDATE \"User_Prefs\" SET \"UserName\" = $2, \"User_anzeigeName\" = $3, \
             \"UserKuerzel\" = $4, \"Login_local\" = $5, \"Login_mobile\" = $6, \
             \"Password\" = COALESCE($7, \"Password\"), \
             \"lastPasswordChange\" = CASE WHEN $7 IS NULL THEN \"lastPasswordChange\" \
                                          ELSE CURRENT_DATE END, \
             \"ChangeNextLogin\" = $8, \"CannotChange\" = $9, \"NeverExpires\" = $10, \
             \"AccountDisabled\" = $11, \"Language\" = $12, \"TA_ColorRed\" = $13, \
             \"TA_ColorGreen\" = $14, \"TA_ColorBlue\" = $15, \"SSO_AD_active\" = $16, \
             \"SSO_ActiveDirectoryName\" = $17, \"SSO_Domain\" = $18, \"SSO_MB_active\" = $19, \
             \"SSO_MB_OSuser\" = $20, \"SSO_MB_MachineName\" = $21, \"SSO_defaultEntries\" = $22, \
             \"SSO_defaultMandant\" = $23, \"SSO_defaultLager\" = $24, \"com_intDurchwahl\" = $25 \
             WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .bind(&user.user_name)
        .bind(&user.display_name)
        .bind(&user.short_name)
        .bind(user.local_login)
        .bind(user.mobile_login)
        .bind(&password)
        .bind(user.change_next_login)
        .bind(user.cannot_change)
        .bind(user.never_expires)
        .bind(user.disabled)
        .bind(&user.language)
        .bind(red)
        .bind(green)
        .bind(blue)
        .bind(user.sso_ad_active)
        .bind(&user.sso_ad_user)
        .bind(&user.sso_ad_domain)
        .bind(user.sso_mb_active)
        .bind(&user.sso_mb_os_user)
        .bind(&user.sso_mb_machine)
        .bind(user.sso_default_entries)
        .bind(user.sso_default_mandant)
        .bind(user.sso_default_lager)
        .bind(&user.extension)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?
        .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Benutzer",
                id: id as i32,
            });
        }
        Self::write_assignments(&mut tx, id, &user).await?;
        Self::write_contact(&mut tx, id, &user).await?;
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    async fn write_assignments(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        user: &PreparedUser,
    ) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM \"User_Mandanten\" WHERE \"UserPrimKey\" = $1")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        for mandant in &user.mandants {
            sqlx::query(
                "INSERT INTO \"User_Mandanten\" (\"UserPrimKey\", \"MandantenID\") VALUES ($1, $2)",
            )
            .bind(id)
            .bind(mandant)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        }
        sqlx::query("DELETE FROM \"User_Lager\" WHERE \"UserPrimKey\" = $1")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        for lager in &user.lager {
            sqlx::query(
                "INSERT INTO \"User_Lager\" (\"UserPrimKey\", \"MandantenID\", \"Lager_pk\") \
                 VALUES ($1, 0, $2)",
            )
            .bind(id)
            .bind(lager)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        }
        Ok(())
    }

    /// Contact data: one row per user and mandant, changed in place.
    async fn write_contact(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        user: &PreparedUser,
    ) -> Result<(), DomainError> {
        let vorhanden: Option<i32> = sqlx::query_scalar(
            "SELECT \"Primary_Key\" FROM \"User_ITPrefs\" WHERE \"User_PrimKey\" = $1 \
             ORDER BY CASE WHEN \"Mandanten_ID\" = $2 THEN 0 ELSE 1 END, \"Primary_Key\" LIMIT 1",
        )
        .bind(id)
        .bind(user.profile_mandant)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_error)?;
        match vorhanden {
            Some(key) => {
                sqlx::query(
                    "UPDATE \"User_ITPrefs\" SET \"Mandanten_ID\" = $2, \"com_Funktion\" = $3, \
                     \"com_Telefon\" = $4, \"com_Mobile\" = $5, \"com_Fax\" = $6, \
                     \"com_eMail\" = $7, \"AuthProviderType\" = $8, \"SMTPServer\" = $9, \
                     \"SMTPPort\" = $10, \"emailsignatur\" = $11, \"IT_BCCAdress\" = $12, \
                     \"IT_SetBCC\" = ($12 IS NOT NULL), \"com_IM1Type\" = $13, \"com_IM1\" = $14, \
                     \"com_IM2Type\" = $15, \"com_IM2\" = $16 WHERE \"Primary_Key\" = $1",
                )
                .bind(key)
                .bind(user.profile_mandant)
                .bind(&user.function)
                .bind(&user.phone)
                .bind(&user.mobile)
                .bind(&user.fax)
                .bind(&user.email)
                .bind(user.email_provider)
                .bind(&user.smtp_server)
                .bind(user.smtp_port)
                .bind(&user.email_signature)
                .bind(&user.bcc)
                .bind(&user.im1_type)
                .bind(&user.im1)
                .bind(&user.im2_type)
                .bind(&user.im2)
                .execute(&mut **tx)
                .await
                .map_err(db_error)?;
            }
            None => {
                let leer = user.function.is_none()
                    && user.phone.is_none()
                    && user.mobile.is_none()
                    && user.fax.is_none()
                    && user.email.is_none()
                    && user.smtp_server.is_none()
                    && user.email_signature.is_none()
                    && user.bcc.is_none()
                    && user.im1.is_none()
                    && user.im2.is_none()
                    && user.profile_mandant.is_none();
                if leer {
                    return Ok(());
                }
                sqlx::query(
                    "INSERT INTO \"User_ITPrefs\" (\"User_PrimKey\", \"Mandanten_ID\", \
                     \"com_Funktion\", \"com_Telefon\", \"com_Mobile\", \"com_Fax\", \"com_eMail\", \
                     \"AuthProviderType\", \"SMTPServer\", \"SMTPPort\", \"emailsignatur\", \
                     \"IT_BCCAdress\", \"IT_SetBCC\", \"com_IM1Type\", \"com_IM1\", \"com_IM2Type\", \
                     \"com_IM2\") \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, \
                             ($12 IS NOT NULL), $13, $14, $15, $16)",
                )
                .bind(id)
                .bind(user.profile_mandant)
                .bind(&user.function)
                .bind(&user.phone)
                .bind(&user.mobile)
                .bind(&user.fax)
                .bind(&user.email)
                .bind(user.email_provider)
                .bind(&user.smtp_server)
                .bind(user.smtp_port)
                .bind(&user.email_signature)
                .bind(&user.bcc)
                .bind(&user.im1_type)
                .bind(&user.im1)
                .bind(&user.im2_type)
                .bind(&user.im2)
                .execute(&mut **tx)
                .await
                .map_err(db_error)?;
            }
        }
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        for sql in [
            "DELETE FROM \"UserGroupMember\" WHERE \"User_PrimKey\" = $1",
            "DELETE FROM \"User_Mandanten\" WHERE \"UserPrimKey\" = $1",
            "DELETE FROM \"User_Lager\" WHERE \"UserPrimKey\" = $1",
            "DELETE FROM \"User_ITPrefs\" WHERE \"User_PrimKey\" = $1",
        ] {
            sqlx::query(sql)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        let changed = sqlx::query("DELETE FROM \"User_Prefs\" WHERE \"Primary_Key\" = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Benutzer",
                id: id as i32,
            });
        }
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Puts a user into a group; a second call is a no-op.
    pub async fn add_to_group(&self, user: i64, group: i64) -> Result<(), DomainError> {
        self.get(user).await?;
        self.group(group).await?;
        sqlx::query(
            "INSERT INTO \"UserGroupMember\" (\"User_PrimKey\", \"UserGroup_PrimKey\") \
             SELECT $1, $2 WHERE NOT EXISTS (\
                 SELECT 1 FROM \"UserGroupMember\" WHERE \"User_PrimKey\" = $1 \
                   AND \"UserGroup_PrimKey\" = $2)",
        )
        .bind(user)
        .bind(group)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(())
    }

    pub async fn remove_from_group(&self, user: i64, group: i64) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM \"UserGroupMember\" WHERE \"User_PrimKey\" = $1 AND \"UserGroup_PrimKey\" = $2",
        )
        .bind(user)
        .bind(group)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(())
    }

    /// One group with its rights and its scope.
    pub async fn group(&self, id: i64) -> Result<UserGroup, DomainError> {
        let row = sqlx::query(
            "SELECT \"Primary_Key\"::bigint AS id, COALESCE(\"Group_Name\", '') AS name \
             FROM \"UserGroups\" WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let Some(row) = row else {
            return Err(DomainError::NotFound {
                entity: "Berechtigungsgruppe",
                id: id as i32,
            });
        };
        let mut group = UserGroup {
            id: row.get("id"),
            name: row.get("name"),
            max_rights: Vec::new(),
            min_rights: Vec::new(),
            mandants: Vec::new(),
            lager: Vec::new(),
            departments: Vec::new(),
            dispo_departments: Vec::new(),
            members: 0,
            member_users: Vec::new(),
        };
        let rights = sqlx::query(
            "SELECT \"scope\", \"right_key\" FROM \"where_user_group_rights\" \
             WHERE \"group_id\" = $1 ORDER BY \"right_key\"",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        for right in rights {
            let scope: String = right.get("scope");
            let key: String = right.get("right_key");
            if scope == "min" {
                group.min_rights.push(key);
            } else {
                group.max_rights.push(key);
            }
        }
        let scope = sqlx::query(
            "SELECT \"kind\", \"ref_id\" FROM \"where_user_group_scope\" WHERE \"group_id\" = $1 \
             ORDER BY \"ref_id\"",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        for entry in scope {
            let kind: String = entry.get("kind");
            let ref_id: i32 = entry.get("ref_id");
            match kind.as_str() {
                "mandant" => group.mandants.push(ref_id),
                "lager" => group.lager.push(ref_id),
                "personal-department" => group.departments.push(ref_id),
                "personaldispo-department" => group.dispo_departments.push(ref_id),
                _ => {}
            }
        }
        group.members = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM \"UserGroupMember\" WHERE \"UserGroup_PrimKey\" = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)? as usize;
        let members = sqlx::query(
            "SELECT u.\"Primary_Key\"::bigint AS id, \
                    COALESCE(NULLIF(TRIM(u.\"User_anzeigeName\"), ''), u.\"UserName\", '') AS name \
             FROM \"UserGroupMember\" m \
             JOIN \"User_Prefs\" u ON u.\"Primary_Key\" = m.\"User_PrimKey\" \
             WHERE m.\"UserGroup_PrimKey\" = $1 ORDER BY name",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        group.member_users = members
            .iter()
            .map(|row| GroupMember {
                id: row.get("id"),
                name: row.get::<String, _>("name"),
            })
            .collect();
        Ok(group)
    }

    /// The departments the groups of one account put it into — for `Personal` and for
    /// `Personal-Disposition` separately. `None` when no group names one, then that
    /// area is not restricted.
    ///
    /// The sets of the groups are united, like the rights: a group without a
    /// department entry does not widen the scope of another one.
    pub async fn department_scopes(
        &self,
        user_id: i64,
    ) -> Result<(Option<Vec<i64>>, Option<Vec<i64>>), DomainError> {
        let rows = sqlx::query(
            "SELECT DISTINCT s.\"kind\", s.\"ref_id\" FROM \"where_user_group_scope\" s \
             JOIN \"UserGroupMember\" m ON m.\"UserGroup_PrimKey\" = s.\"group_id\" \
             WHERE m.\"User_PrimKey\" = $1 \
             AND s.\"kind\" IN ('personal-department', 'personaldispo-department') \
             ORDER BY s.\"kind\", s.\"ref_id\"",
        )
        .bind(user_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut personal: Vec<i64> = Vec::new();
        let mut dispo: Vec<i64> = Vec::new();
        for row in rows {
            let kind: String = row.get("kind");
            let ref_id: i32 = row.get("ref_id");
            if kind == "personal-department" {
                personal.push(i64::from(ref_id));
            } else {
                dispo.push(i64::from(ref_id));
            }
        }
        let personal = if personal.is_empty() {
            None
        } else {
            Some(personal)
        };
        let dispo = if dispo.is_empty() { None } else { Some(dispo) };
        Ok((personal, dispo))
    }

    /// Every group with its rights, its scope and its accounts — the right-hand tree of
    /// the main window.
    pub async fn groups(&self) -> Result<Vec<UserGroup>, DomainError> {
        let rows = sqlx::query(
            "SELECT \"Primary_Key\"::bigint AS id FROM \"UserGroups\" \
             ORDER BY \"Group_Name\" NULLS LAST, \"Primary_Key\"",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut groups = Vec::with_capacity(rows.len());
        for row in rows {
            groups.push(self.group(row.get("id")).await?);
        }
        Ok(groups)
    }

    pub async fn create_group(&self, input: UserGroupInput) -> Result<UserGroup, DomainError> {
        let group = input.prepare()?;
        let id = sqlx::query_scalar::<_, i32>(
            "INSERT INTO \"UserGroups\" (\"Group_Name\", \"Group_InternalPrimKey\", \
             \"Owner_User_PrimKey\") VALUES ($1, $2, $3) RETURNING \"Primary_Key\"",
        )
        .bind(&group.name)
        .bind(0)
        .bind(0)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        self.write_group(id as i64, &group).await?;
        self.group(id as i64).await
    }

    pub async fn update_group(
        &self,
        id: i64,
        input: UserGroupInput,
    ) -> Result<UserGroup, DomainError> {
        let group = input.prepare()?;
        let changed =
            sqlx::query("UPDATE \"UserGroups\" SET \"Group_Name\" = $2 WHERE \"Primary_Key\" = $1")
                .bind(id as i32)
                .bind(&group.name)
                .execute(&self.pool)
                .await
                .map_err(db_error)?
                .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Berechtigungsgruppe",
                id: id as i32,
            });
        }
        self.write_group(id, &group).await?;
        self.group(id).await
    }

    /// Rights and scope of a group, replacing what was stored.
    async fn write_group(&self, id: i64, group: &UserGroup) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query("DELETE FROM \"where_user_group_rights\" WHERE \"group_id\" = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for (scope, keys) in [("max", &group.max_rights), ("min", &group.min_rights)] {
            for key in keys {
                sqlx::query(
                    "INSERT INTO \"where_user_group_rights\" (\"group_id\", \"scope\", \"right_key\") \
                     VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(id)
                .bind(scope)
                .bind(key)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            }
        }
        sqlx::query("DELETE FROM \"where_user_group_scope\" WHERE \"group_id\" = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for (kind, ids) in [
            ("mandant", &group.mandants),
            ("lager", &group.lager),
            ("personal-department", &group.departments),
            ("personaldispo-department", &group.dispo_departments),
        ] {
            for ref_id in ids {
                sqlx::query(
                    "INSERT INTO \"where_user_group_scope\" (\"group_id\", \"kind\", \"ref_id\") \
                     VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(id)
                .bind(kind)
                .bind(ref_id)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            }
        }
        // The one number per module that the legacy table keeps stays consistent:
        // a module counts as granted when at least one of its rights is ticked.
        for (column, area) in [
            ("Access_Adressen", "adressen"),
            ("Access_Artikel", "artikel"),
            ("Access_Auftraege", "auftrag"),
            ("Access_Angebote", "angebot"),
            ("Access_Rechnungen", "ausgangsrechnung"),
            ("Access_Mahnwesen", "mahnwesen"),
            ("Access_Kasse", "kassenbuch"),
            ("Access_Statistiken", "statistiken"),
            ("Access_TeamAgenda", "teamagenda"),
            ("Access_Barcode", "barcodesystem"),
            ("Access_email", "emailversand"),
            ("Access_TAPI", "tapi"),
            ("Access_QM", "qm"),
            ("Access_Werkstatt", "werkstatt"),
            ("Access_Scriptmenu", "scriptmenue"),
            ("Access_VDE", "dguv"),
            ("Access_Projekt", "projekt"),
            ("Access_Textvorlagen", "textverarbeitung"),
            ("Access_Prefs", "administrative-einstellungen"),
            ("Access_UserPrefs", "persoenliche-einstellungen"),
            ("Access_Bestellungen", "bestellvorgang"),
            ("Access_Zeingang", "zahlungseingang"),
            ("Access_ZAusgang", "zahlungsausgang"),
            ("Access_Interessenten", "interessenten"),
            ("Access_MADispo", "personaldispo"),
            ("Access_Lieferanten", "lieferanten"),
            ("Access_Fahrzeuge", "fahrzeuge"),
            ("Access_Transporte", "transporte"),
            ("Access_DataExchange", "im-export"),
            ("Access_Kunden", "kunden"),
            ("Access_Personal", "personal"),
            ("Access_Lieferscheine", "packschein"),
        ] {
            let granted = group
                .max_rights
                .iter()
                .any(|key| key.starts_with(&format!("{area}.")));
            let sql =
                format!("UPDATE \"UserGroups\" SET \"{column}\" = $2 WHERE \"Primary_Key\" = $1");
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(id as i32)
                .bind(if granted { 1 } else { 0 })
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// A copy of a group under a new name.
    pub async fn copy_group(&self, id: i64) -> Result<UserGroup, DomainError> {
        let group = self.group(id).await?;
        let input = UserGroupInput {
            name: format!("{} (Kopie)", group.name),
            max_rights: group.max_rights,
            min_rights: group.min_rights,
            mandants: group.mandants,
            lager: group.lager,
            departments: group.departments,
            dispo_departments: group.dispo_departments,
        };
        self.create_group(input).await
    }

    pub async fn delete_group(&self, id: i64) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        for sql in [
            "DELETE FROM \"UserGroupMember\" WHERE \"UserGroup_PrimKey\" = $1",
            "DELETE FROM \"where_user_group_rights\" WHERE \"group_id\" = $1",
            "DELETE FROM \"where_user_group_scope\" WHERE \"group_id\" = $1",
        ] {
            sqlx::query(sql)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        let changed = sqlx::query("DELETE FROM \"UserGroups\" WHERE \"Primary_Key\" = $1")
            .bind(id as i32)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Berechtigungsgruppe",
                id: id as i32,
            });
        }
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_is_never_stored_as_text() {
        let hashed = hash_password("geheim");
        assert!(hashed.starts_with("sha256:"));
        assert_eq!(hashed.len(), 7 + 64);
        assert!(!hashed.contains("geheim"));
    }

    #[test]
    fn only_catalogue_keys_survive_and_they_are_sorted() {
        let keys = known_rights(vec![
            "artikel.artikel-anlegen".into(),
            "nicht-vorhanden.zugriff".into(),
            "adressen.adresse-lesen".into(),
            "artikel.artikel-anlegen".into(),
        ]);
        assert_eq!(
            keys,
            vec![
                "adressen.adresse-lesen".to_string(),
                "artikel.artikel-anlegen".to_string()
            ]
        );
    }
}
