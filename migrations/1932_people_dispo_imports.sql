-- Remembers which disposition files were already imported per month.
--
-- The fingerprint is taken over the scanned content (locations, days, times,
-- texts, flags), so the same disposition is recognised a second time even when
-- PeM wrote a fresh xlsx around it. Importing the same planning twice would
-- double every entry; it is therefore refused unless the month is replaced.
CREATE TABLE IF NOT EXISTS "where_people_dispo_imports" (
    "id" bigserial PRIMARY KEY,
    "period_start" date NOT NULL,
    "period_end" date NOT NULL,
    "fingerprint" text NOT NULL,
    "locations" integer NOT NULL,
    "entries" integer NOT NULL,
    "imported_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_dispo_imports_fingerprint_ck"
        CHECK (length(btrim("fingerprint")) BETWEEN 1 AND 64)
);

CREATE UNIQUE INDEX IF NOT EXISTS "where_people_dispo_imports_period_idx"
    ON "where_people_dispo_imports" ("period_start", "fingerprint");

COMMENT ON TABLE "where_people_dispo_imports" IS
    'The PeM dispositions that were imported: one row per month and content.';
COMMENT ON COLUMN "where_people_dispo_imports"."period_start" IS
    'First day of the month the disposition was booked on.';
COMMENT ON COLUMN "where_people_dispo_imports"."period_end" IS
    'Last day of that month.';
COMMENT ON COLUMN "where_people_dispo_imports"."fingerprint" IS
    'Hash over the scanned content; the same disposition always gives the same value.';
COMMENT ON COLUMN "where_people_dispo_imports"."entries" IS
    'Number of entries the import wrote.';
