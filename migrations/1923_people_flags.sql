-- Flags of the disposition (`Vst`, `Probe`, `TE`, ...) and the staffing each of
-- them needs.
--
-- PeM writes the flag at the end of an event text (`Max und Moritz Vst`,
-- `Cosi fan tutte BO 1`). The import recognises it against this list and keeps
-- it on the entry, so the demand of a day can later be derived from the flags of
-- its events. Every flag carries a default staffing: which positions it needs
-- and how many of them. `*` means "any position" (shown as `nicht relevant`).

CREATE TABLE IF NOT EXISTS "where_people_flags" (
    "id" bigserial PRIMARY KEY,
    "code" text NOT NULL,
    "name" text NOT NULL,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_flags_code_ck"
        CHECK (length(btrim("code")) BETWEEN 1 AND 20),
    CONSTRAINT "where_people_flags_name_ck"
        CHECK (length(btrim("name")) BETWEEN 1 AND 100)
);

-- One code per list, regardless of upper and lower case.
CREATE UNIQUE INDEX IF NOT EXISTS "where_people_flags_code_idx"
    ON "where_people_flags" (lower(btrim("code")));

CREATE TABLE IF NOT EXISTS "where_people_flag_staffing" (
    "id" bigserial PRIMARY KEY,
    "flag_id" bigint NOT NULL
        REFERENCES "where_people_flags" ("id") ON DELETE CASCADE,
    "position" text NOT NULL,
    "needed" integer NOT NULL,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_flag_staffing_position_ck"
        CHECK (length(btrim("position")) BETWEEN 1 AND 100),
    CONSTRAINT "where_people_flag_staffing_needed_ck"
        CHECK ("needed" BETWEEN 1 AND 99)
);

-- One row per position and flag; `*` counts as a position of its own.
CREATE UNIQUE INDEX IF NOT EXISTS "where_people_flag_staffing_position_idx"
    ON "where_people_flag_staffing" ("flag_id", lower(btrim("position")));

ALTER TABLE "where_people_dispo_entries"
    ADD COLUMN IF NOT EXISTS "flag_id" bigint NULL
        REFERENCES "where_people_flags" ("id") ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS "where_people_dispo_entries_flag_idx"
    ON "where_people_dispo_entries" ("flag_id");

COMMENT ON TABLE "where_people_flags" IS
    'Flags of the disposition (Vst, Probe, TE, ...); the import recognises them in the event texts.';
COMMENT ON TABLE "where_people_flag_staffing" IS
    'Default staffing of a flag: which positions it needs and how many, `*` for any position.';
COMMENT ON COLUMN "where_people_dispo_entries"."flag_id" IS
    'Flag recognised in the title; empty when the text carries none.';
