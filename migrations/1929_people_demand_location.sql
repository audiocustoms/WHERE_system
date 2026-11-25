-- Der Bedarf gilt je Standort.
--
-- Zwei Veranstaltungen in zwei Häusern brauchen ihre Besetzung getrennt: die
-- Vorstellung im Großen Haus und die im Podium addieren sich nicht, jede hat
-- ihre eigene Zeile. Ein leerer Standort heißt „egal wo" — das ist der Bedarf,
-- den die Maske von Hand einträgt.
ALTER TABLE "where_people_duty_demand"
    ADD COLUMN IF NOT EXISTS "location" text NOT NULL DEFAULT '';

ALTER TABLE "where_people_duty_demand"
    DROP CONSTRAINT IF EXISTS "where_people_duty_demand_pkey";

ALTER TABLE "where_people_duty_demand"
    ADD CONSTRAINT "where_people_duty_demand_pkey"
    PRIMARY KEY ("day", "shift_id", "position", "location");

ALTER TABLE "where_people_duty_demand"
    DROP CONSTRAINT IF EXISTS "where_people_duty_demand_location_ck";

ALTER TABLE "where_people_duty_demand"
    ADD CONSTRAINT "where_people_duty_demand_location_ck"
    CHECK (length("location") <= 100);

COMMENT ON COLUMN "where_people_duty_demand"."location" IS
    'Standort der Dispo; leer, wenn der Bedarf an keinen Ort gebunden ist.';

-- Welche Abteilung an einem Standort arbeitet. Die Planung besetzt einen
-- Standort nur mit Mitarbeitern dieser Abteilung; bleibt das Feld leer, darf
-- jede Abteilung dort arbeiten.
ALTER TABLE "where_people_locations"
    ADD COLUMN IF NOT EXISTS "department" text;

ALTER TABLE "where_people_locations"
    DROP CONSTRAINT IF EXISTS "where_people_locations_department_ck";

ALTER TABLE "where_people_locations"
    ADD CONSTRAINT "where_people_locations_department_ck"
    CHECK ("department" IS NULL OR length("department") <= 100);

COMMENT ON COLUMN "where_people_locations"."department" IS
    'Abteilung, die diesen Standort besetzt; leer heißt: jede Abteilung.';
