-- Next to shifts the duty plan also holds absences: sick or vacation.
--
-- `shift_id` is therefore optional; `kind` says what stands in the cell. It stays
-- at most one entry per employee and day.

ALTER TABLE "where_people_duty_entries"
    ALTER COLUMN "shift_id" DROP NOT NULL;

ALTER TABLE "where_people_duty_entries"
    ADD COLUMN IF NOT EXISTS "kind" text NOT NULL DEFAULT 'shift';

ALTER TABLE "where_people_duty_entries"
    DROP CONSTRAINT IF EXISTS "where_people_duty_kind_ck";
ALTER TABLE "where_people_duty_entries"
    ADD CONSTRAINT "where_people_duty_kind_ck"
    CHECK ("kind" IN ('shift', 'sick', 'vacation'));

ALTER TABLE "where_people_duty_entries"
    DROP CONSTRAINT IF EXISTS "where_people_duty_shift_ck";
ALTER TABLE "where_people_duty_entries"
    ADD CONSTRAINT "where_people_duty_shift_ck"
    CHECK (("kind" = 'shift') = ("shift_id" IS NOT NULL));

COMMENT ON COLUMN "where_people_duty_entries"."kind" IS
    'shift, sick or vacation.';
