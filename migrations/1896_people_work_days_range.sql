-- Make the working days per week freely enterable (instead of only 5 or 6).
--
-- A week has at most seven days, so only this range is checked; without a value
-- the field stays empty.

ALTER TABLE "where_employees"
    DROP CONSTRAINT IF EXISTS "where_employees_work_days_ck";

ALTER TABLE "where_employees"
    ADD CONSTRAINT "where_employees_work_days_ck"
    CHECK ("work_days" IS NULL OR "work_days" BETWEEN 1 AND 7);

COMMENT ON COLUMN "where_employees"."work_days" IS
    'Working days per week: 1 to 7; empty = not set.';
