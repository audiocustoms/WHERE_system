-- Conversion of absence days into hours.
--
-- A vacation or sick day counts as many hours as stored here; the planning can
-- therefore offset absences against the weekly hours.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "vacation_day_hours" numeric(4, 2) NOT NULL DEFAULT 8.00;

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "sick_day_hours" numeric(4, 2) NOT NULL DEFAULT 8.00;

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_absence_hours_ck";
ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_absence_hours_ck"
    CHECK ("vacation_day_hours" >= 0 AND "vacation_day_hours" <= 24
       AND "sick_day_hours" >= 0 AND "sick_day_hours" <= 24);

COMMENT ON COLUMN "where_people_planning"."vacation_day_hours" IS
    'Hours a vacation day counts.';
COMMENT ON COLUMN "where_people_planning"."sick_day_hours" IS
    'Hours a sick day counts.';
