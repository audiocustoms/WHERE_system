-- How long an employee keeps a shift before early and late may swap.
--
-- The planning follows the legal rules (rest period, working time, breaks) and
-- in addition keeps a rhythm: an employee stays on the same shift for this many
-- days before the planning may move them from early to late or the other way
-- round. A two-week block (14) is the default. Zero days would let the planning
-- jump between the shifts every day, one day would be the same, so the value
-- starts at 1.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "shift_rotation_days" smallint NOT NULL DEFAULT 14;

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_shift_rotation_ck";
ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_shift_rotation_ck"
    CHECK ("shift_rotation_days" BETWEEN 1 AND 365);

COMMENT ON COLUMN "where_people_planning"."shift_rotation_days" IS
    'Days an employee keeps the same shift before early and late may swap (default 14).';
