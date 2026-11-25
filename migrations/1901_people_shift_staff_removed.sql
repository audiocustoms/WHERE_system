-- Minimum staffing does not belong in the duty plan master.
--
-- It is maintained elsewhere later; the shift stays a time window with break and
-- colour.

ALTER TABLE "where_people_shifts"
    DROP CONSTRAINT IF EXISTS "where_people_shifts_staff_ck";

ALTER TABLE "where_people_shifts"
    DROP COLUMN IF EXISTS "min_staff";
