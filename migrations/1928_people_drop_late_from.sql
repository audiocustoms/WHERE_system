-- The boundary of the day is not a setting: a shift has a window.
--
-- `migrations/1927_people_late_from.sql` introduced `late_from`. It is not
-- needed: an event belongs to the shift whose window contains its start time
-- (`src/dispo_demand.rs`), so the shift catalogue alone decides.

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_late_from_ck";
ALTER TABLE "where_people_planning"
    DROP COLUMN IF EXISTS "late_from";
