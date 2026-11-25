-- Staffing demand of the duty plan: how many employees of a position a day needs.
--
-- The board opens it per date and copies the values onto following days
-- ("all days of the month"). A day without a row needs nobody.

CREATE TABLE IF NOT EXISTS "where_people_duty_demand" (
    "day" date NOT NULL,
    "position" text NOT NULL,
    "needed" integer NOT NULL,
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("day", "position"),
    CONSTRAINT "where_people_duty_demand_position_ck"
        CHECK (length(btrim("position")) BETWEEN 1 AND 100),
    CONSTRAINT "where_people_duty_demand_needed_ck"
        CHECK ("needed" BETWEEN 1 AND 99)
);

CREATE INDEX IF NOT EXISTS "where_people_duty_demand_day_idx"
    ON "where_people_duty_demand" ("day");

COMMENT ON TABLE "where_people_duty_demand" IS
    'Employees needed per day and position in the duty plan.';
COMMENT ON COLUMN "where_people_duty_demand"."needed" IS
    'Number of employees of that position needed on that day (1-99).';
