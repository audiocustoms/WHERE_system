-- Public and school holidays of the tenant.
--
-- The duty-plan settings decide what is imported, the sources are fixed
-- (OpenHolidays, no key): `import_public_holidays` for the nationwide holidays,
-- `import_regional_holidays` and `import_school_holidays` for the ones of the
-- `holiday_state` (a Bundesland such as `BY`). The import replaces the rows of
-- the year it runs for, so a changed state does not leave old rows behind.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "import_public_holidays" boolean NOT NULL DEFAULT false;

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "import_regional_holidays" boolean NOT NULL DEFAULT false;

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "import_school_holidays" boolean NOT NULL DEFAULT false;

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "holiday_state" text;

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_holiday_state_ck";

ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_holiday_state_ck"
    CHECK (
        "holiday_state" IS NULL
        OR "holiday_state" IN (
            'BW', 'BY', 'BE', 'BB', 'HB', 'HH', 'HE', 'MV',
            'NI', 'NW', 'RP', 'SL', 'SN', 'ST', 'SH', 'TH'
        )
    );

CREATE TABLE IF NOT EXISTS "where_people_holidays" (
    "id" bigserial PRIMARY KEY,
    "day" date NOT NULL,
    "name" text NOT NULL,
    "scope" text NOT NULL,
    "state" text,
    "source" text NOT NULL,
    "fetched_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_holidays_scope_ck"
        CHECK ("scope" IN ('national', 'regional')),
    CONSTRAINT "where_people_holidays_unique" UNIQUE ("day", "name")
);

CREATE INDEX IF NOT EXISTS "where_people_holidays_day_ix"
    ON "where_people_holidays" ("day");

CREATE TABLE IF NOT EXISTS "where_people_school_holidays" (
    "id" bigserial PRIMARY KEY,
    "starts_on" date NOT NULL,
    "ends_on" date NOT NULL,
    "name" text NOT NULL,
    "state" text NOT NULL,
    "source" text NOT NULL,
    "fetched_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_school_holidays_order_ck"
        CHECK ("ends_on" >= "starts_on"),
    CONSTRAINT "where_people_school_holidays_unique"
        UNIQUE ("starts_on", "ends_on", "state")
);

CREATE INDEX IF NOT EXISTS "where_people_school_holidays_range_ix"
    ON "where_people_school_holidays" ("starts_on", "ends_on");

COMMENT ON COLUMN "where_people_planning"."import_public_holidays" IS
    'Import the nationwide public holidays of Germany.';
COMMENT ON COLUMN "where_people_planning"."import_regional_holidays" IS
    'Import the regional public holidays of the chosen federal state.';
COMMENT ON COLUMN "where_people_planning"."import_school_holidays" IS
    'Import the school holidays of the chosen federal state.';
COMMENT ON COLUMN "where_people_planning"."holiday_state" IS
    'Federal state (BW, BY, ...) for regional and school holidays.';

COMMENT ON TABLE "where_people_holidays" IS
    'Public holidays of the tenant, imported from OpenHolidays.';
COMMENT ON TABLE "where_people_school_holidays" IS
    'School holidays of the federal state, imported from OpenHolidays.';
