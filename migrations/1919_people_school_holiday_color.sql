-- Colour of the school holidays in the duty plan.
--
-- The bar under the date row and the name of a stretch use this colour; it is
-- maintained next to `holiday_color` in `Feiertage und Ferien`.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "school_holiday_color" text NOT NULL DEFAULT '#2563EB';

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_school_holiday_color_ck";

ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_school_holiday_color_ck"
    CHECK ("school_holiday_color" ~ '^#[0-9A-F]{6}$');

COMMENT ON COLUMN "where_people_planning"."school_holiday_color" IS
    'Colour of the school holidays in the duty plan (#RRGGBB).';
