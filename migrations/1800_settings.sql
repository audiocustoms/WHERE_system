-- Settings that belong to WHERE itself.
--
-- The legacy installation keeps such options in per-form preference tables that
-- only its own code reads (`Prefs_*`, `SX_Prefs`, …). WHERE collects them in one
-- small key/value table instead: a key carries the form and the option
-- (`address.neukunde.indexpflicht`), so a new option needs no migration.

CREATE TABLE IF NOT EXISTS "where_settings" (
    "key"        text PRIMARY KEY,
    "value"      text,
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    "changed_by" text
);

COMMENT ON TABLE "where_settings" IS
    'Options of the WHERE masks; the legacy preferences of the old forms.';
