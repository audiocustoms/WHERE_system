-- From which time of day an event belongs to the late shift.
--
-- The disposition of PeM names no shift, only a start time. The theatre splits
-- the day at 15:00: everything before belongs to the early shift (an event at
-- 14:30 is a handover to the late shift), everything from 15:00 on to the late
-- one. The apprentice shifts never move that boundary — apprentices assist, they
-- never take over a shift — and the Zwischenschichten are set by hand when
-- neither half of the day fits.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "late_from" text NOT NULL DEFAULT '15:00';

ALTER TABLE "where_people_planning"
    DROP CONSTRAINT IF EXISTS "where_people_planning_late_from_ck";
ALTER TABLE "where_people_planning"
    ADD CONSTRAINT "where_people_planning_late_from_ck"
    CHECK ("late_from" ~ '^[0-2][0-9]:[0-5][0-9]$');

COMMENT ON COLUMN "where_people_planning"."late_from" IS
    'From this time (HH:MM, default 15:00) an event of the disposition counts as late shift.';
