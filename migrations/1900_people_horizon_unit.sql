-- Planning horizon in weeks or months.
--
-- `planning_weeks` is now called `planning_horizon`; the unit stands next to it.

ALTER TABLE "where_people_planning"
    RENAME COLUMN "planning_weeks" TO "planning_horizon";

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "planning_horizon_unit" text NOT NULL DEFAULT 'weeks';

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_streak_ck";
ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_streak_ck"
    CHECK ("max_consecutive_days" BETWEEN 1 AND 31
       AND "min_days_off_per_week" BETWEEN 0 AND 7
       AND "planning_horizon" BETWEEN 1 AND
           (CASE WHEN "planning_horizon_unit" = 'months' THEN 12 ELSE 52 END));

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_horizon_unit_ck";
ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_horizon_unit_ck"
    CHECK ("planning_horizon_unit" IN ('weeks', 'months'));

COMMENT ON COLUMN "where_people_planning"."planning_horizon" IS
    'Length of the planning period in the unit planning_horizon_unit.';
COMMENT ON COLUMN "where_people_planning"."planning_horizon_unit" IS
    'weeks or months.';
