-- Whether the board shows the holidays and the school holidays.
--
-- Independent of the import: an installation that imported the calendar can
-- still hide it, and one that hides it keeps the rows. Both default to `true`.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "show_holidays" boolean NOT NULL DEFAULT true;

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "show_school_holidays" boolean NOT NULL DEFAULT true;

COMMENT ON COLUMN "where_people_planning"."show_holidays" IS
    'Tint the public holidays in the duty plan and name them under the date.';
COMMENT ON COLUMN "where_people_planning"."show_school_holidays" IS
    'Mark the school holidays in the duty plan.';
