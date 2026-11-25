-- From this date on the duty plan counts over- and under-hours.
--
-- Everything before it stays out of the balance, so a plan that starts in the
-- middle of the year does not carry the months before. `null` means 1 January
-- of the shown year.

ALTER TABLE "where_people_planning"
    ADD COLUMN IF NOT EXISTS "balance_from" date;

COMMENT ON COLUMN "where_people_planning"."balance_from" IS
    'Start of the over-/under-hour balance (null = 1 January of the shown year).';
