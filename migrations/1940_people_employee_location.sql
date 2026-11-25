-- The location an employee belongs to.
--
-- `Abteilung` and `Position` of the personnel mask are master data names; the
-- location of the disposition (`Großes Haus`, `Podium`, …) joins them, so a person
-- can be bound to one house. It is a name like the department: a renamed location
-- follows into the records, a deleted one leaves the text behind.

ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "location" text;

COMMENT ON COLUMN "where_employees"."location" IS
    'Location of the disposition the employee belongs to (name of where_people_locations).';
