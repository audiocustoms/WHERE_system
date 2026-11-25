-- Working days per week of the employment.
--
-- The mask offers 5 or 6 days; without a value the field stays empty so legacy
-- data is not silently pinned to one variant.

ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "work_days" smallint;

ALTER TABLE "where_employees"
    DROP CONSTRAINT IF EXISTS "where_employees_work_days_ck";
ALTER TABLE "where_employees"
    ADD CONSTRAINT "where_employees_work_days_ck"
    CHECK ("work_days" IS NULL OR "work_days" IN (5, 6));

COMMENT ON COLUMN "where_employees"."work_days" IS
    'Working days per week: 5 or 6; empty = not set.';
