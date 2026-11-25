-- Whether a location needs a foreman (the position `Meister`).
--
-- A big house is run by a foreman; a bar or a foyer is not. The setting belongs
-- to the location: the demand derived from the disposition and the plan
-- generator leave the foreman out where the house does not need one.
ALTER TABLE "where_people_locations"
    ADD COLUMN IF NOT EXISTS "needs_foreman" boolean NOT NULL DEFAULT true;

COMMENT ON COLUMN "where_people_locations"."needs_foreman" IS
    'The house is staffed with a foreman (position "Meister") in the generated plan.';
