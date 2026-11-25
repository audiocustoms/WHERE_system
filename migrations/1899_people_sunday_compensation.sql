-- Compensation of Sunday work.
--
-- If Sunday work is allowed, it can be set that every worked Sunday is
-- compensated on another weekday.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "compensate_sunday" boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN "where_people_planning"."compensate_sunday" IS
    'Sunday work is compensated on another weekday.';
