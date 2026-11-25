-- Whether the duty plan generator may place this employee.
--
-- The board still takes the person by hand; only the automatic generation of a
-- plan skips them — for a manager who is planned separately, for somebody who
-- works in another house, or for staff that is only filled in on request.
ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "auto_plan" boolean NOT NULL DEFAULT true;

COMMENT ON COLUMN "where_employees"."auto_plan" IS
    'The generator may place this employee in a duty plan; false = planned by hand only.';
