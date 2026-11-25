-- The demand counts employees of a position per shift and day.
--
-- A pair without a number is not needed; the board opens the numbers per date
-- and copies them onto following days ("all days of the month").

DROP TABLE IF EXISTS "where_people_duty_demand";

CREATE TABLE IF NOT EXISTS "where_people_duty_demand" (
    "day" date NOT NULL,
    "shift_id" bigint NOT NULL
        REFERENCES "where_people_shifts" ("id") ON DELETE CASCADE,
    "position" text NOT NULL,
    "needed" integer NOT NULL,
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("day", "shift_id", "position"),
    CONSTRAINT "where_people_duty_demand_position_ck"
        CHECK (length(btrim("position")) BETWEEN 1 AND 100),
    CONSTRAINT "where_people_duty_demand_needed_ck"
        CHECK ("needed" BETWEEN 1 AND 99)
);

CREATE INDEX IF NOT EXISTS "where_people_duty_demand_day_idx"
    ON "where_people_duty_demand" ("day");

COMMENT ON TABLE "where_people_duty_demand" IS
    'Employees of a position needed per day and shift in the duty plan.';
COMMENT ON COLUMN "where_people_duty_demand"."position" IS
    'Position of the personnel master data (text, as on the employee).';
COMMENT ON COLUMN "where_people_duty_demand"."needed" IS
    'Number of employees of that position needed in the shift (1-99).';
