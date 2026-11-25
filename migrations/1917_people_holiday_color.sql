-- Colour of the public holidays in the duty plan.
--
-- The day column of a holiday is tinted and the name stands under the date.
-- The colour is maintained with the other settings of `Feiertage und Ferien`.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "holiday_color" text NOT NULL DEFAULT '#B45309';

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_holiday_color_ck";

ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_holiday_color_ck"
    CHECK ("holiday_color" ~ '^#[0-9A-F]{6}$');

COMMENT ON COLUMN "where_people_planning"."holiday_color" IS
    'Colour of the public-holiday columns in the duty plan (#RRGGBB).';
