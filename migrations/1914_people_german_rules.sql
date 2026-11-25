-- Two rules of the German working time law that the duty plan can follow.
--
-- `free_weekend_every_six_weeks`: with a six-day week every sixth week has to
-- leave a whole weekend (Saturday and Sunday together) free.
--
-- `extended_sunday_rest`: after a Sunday shift the rest period is one full day
-- plus the regular rest period (24 + 11 hours). It is only meaningful while
-- Sunday work is allowed; the API stores it as `false` otherwise.
--
-- Both switches default to `true`, which is the German setting. Installations
-- outside Germany switch them off.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "free_weekend_every_six_weeks" boolean NOT NULL DEFAULT true;

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "extended_sunday_rest" boolean NOT NULL DEFAULT true;

COMMENT ON COLUMN "where_people_planning"."free_weekend_every_six_weeks" IS
    'With a six-day week every sixth week has to leave Saturday and Sunday free.';
COMMENT ON COLUMN "where_people_planning"."extended_sunday_rest" IS
    'After a Sunday shift the rest period is 24 hours plus the regular rest period.';
