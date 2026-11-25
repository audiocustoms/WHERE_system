-- A free day as an entry of the catalogue.
--
-- Next to `early`, `late` and `intermediate` the catalogue now knows `free`: a whole
-- free day. It carries no times and no break (always 00:00–00:00, 0 minutes), counts
-- no hours into the actual time, never takes demand from the disposition and is a day
-- without work for the rest periods — but a visible marker in the plan and therefore
-- the base of the 24+1 hour rule.
ALTER TABLE "where_people_shifts"
    DROP CONSTRAINT IF EXISTS "where_people_shifts_band_ck";

ALTER TABLE "where_people_shifts"
    ADD CONSTRAINT "where_people_shifts_band_ck"
    CHECK ("band" IN ('frueh', 'spaet', 'zwischen', 'frei'));

COMMENT ON COLUMN "where_people_shifts"."band" IS
    'Half of the day the shift belongs to: frueh, spaet, zwischen — or frei for a whole free day.';
