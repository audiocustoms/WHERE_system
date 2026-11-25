-- A cell of the duty plan may carry its own time window.
--
-- Next to the master-data shifts a cell can be a free time (`kind = 'free'`)
-- with its own start, end and break — for the day that does not fit any shift.
-- Only a free cell has these columns; every other kind keeps them empty.

ALTER TABLE "where_people_duty_entries"
    ADD COLUMN IF NOT EXISTS "starts_at" text;
ALTER TABLE "where_people_duty_entries"
    ADD COLUMN IF NOT EXISTS "ends_at" text;
ALTER TABLE "where_people_duty_entries"
    ADD COLUMN IF NOT EXISTS "break_minutes" smallint;

ALTER TABLE "where_people_duty_entries"
    DROP CONSTRAINT IF EXISTS "where_people_duty_kind_ck";
ALTER TABLE "where_people_duty_entries"
    ADD CONSTRAINT "where_people_duty_kind_ck"
    CHECK ("kind" IN ('shift', 'sick', 'vacation', 'free'));

-- The times belong to a free cell and to nothing else.
ALTER TABLE "where_people_duty_entries"
    DROP CONSTRAINT IF EXISTS "where_people_duty_free_ck";
ALTER TABLE "where_people_duty_entries"
    ADD CONSTRAINT "where_people_duty_free_ck"
    CHECK (
        ("kind" = 'free')
        = ("starts_at" IS NOT NULL AND "ends_at" IS NOT NULL AND "break_minutes" IS NOT NULL)
    );

ALTER TABLE "where_people_duty_entries"
    DROP CONSTRAINT IF EXISTS "where_people_duty_free_time_ck";
ALTER TABLE "where_people_duty_entries"
    ADD CONSTRAINT "where_people_duty_free_time_ck"
    CHECK (
        ("starts_at" IS NULL OR "starts_at" ~ '^([01][0-9]|2[0-3]):[0-5][0-9]$')
        AND ("ends_at" IS NULL OR "ends_at" ~ '^([01][0-9]|2[0-3]):[0-5][0-9]$')
        AND ("break_minutes" IS NULL OR "break_minutes" BETWEEN 0 AND 720)
    );

COMMENT ON COLUMN "where_people_duty_entries"."kind" IS
    'shift, sick, vacation or free.';
COMMENT ON COLUMN "where_people_duty_entries"."starts_at" IS
    'Start of a free time window as HH:MM.';
COMMENT ON COLUMN "where_people_duty_entries"."ends_at" IS
    'End of a free time window as HH:MM; before the start it runs past midnight.';
COMMENT ON COLUMN "where_people_duty_entries"."break_minutes" IS
    'Break of a free time window in minutes.';
