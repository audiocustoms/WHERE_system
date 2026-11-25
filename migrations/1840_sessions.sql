-- Sessions of the login (`img/…/Login Logout`).
--
-- WHERE keeps one row per signed-in session: the digest of the token the browser
-- holds in `sessionStorage`, the account and the two stamps. Closing the window
-- drops the token on the client, `POST /api/v1/auth/logout` deletes the row, and a
-- session that has not been used for the configured time is dropped on the next
-- attempt to restore it. Passwords never travel in plain text — `"User_Prefs"`
-- carries a SHA-256 digest (see `user_repo.rs`).

CREATE TABLE IF NOT EXISTS "where_sessions" (
    "token_hash"   text PRIMARY KEY,
    "user_id"      bigint NOT NULL,
    "created_at"   timestamptz NOT NULL DEFAULT now(),
    "last_seen_at" timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE "where_sessions" IS
    'Signed-in sessions of the WHERE login; the token itself is never stored.';

CREATE INDEX IF NOT EXISTS "where_sessions_user_idx"
    ON "where_sessions" ("user_id");
