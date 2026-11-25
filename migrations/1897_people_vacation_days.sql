-- Vacation days of the employment.
--
-- The value keeps the vacation days and remaining vacation days of the employee
-- and appears in the employee list behind the position; without a value it stays
-- empty.

ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "vacation_days" smallint;

ALTER TABLE "where_employees"
    DROP CONSTRAINT IF EXISTS "where_employees_vacation_days_ck";
ALTER TABLE "where_employees"
    ADD CONSTRAINT "where_employees_vacation_days_ck"
    CHECK ("vacation_days" IS NULL OR "vacation_days" BETWEEN 0 AND 366);

COMMENT ON COLUMN "where_employees"."vacation_days" IS
    'Vacation days and remaining vacation days; empty = not set.';
