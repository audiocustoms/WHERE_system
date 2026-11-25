-- The staffing demand counts employees per shift, not per position.
--
-- A shift that is not needed on a day simply has no row; the board opens the
-- numbers per date and copies them onto following days ("all days of the month").

DROP TABLE IF EXISTS "where_people_duty_demand";

CREATE TABLE IF NOT EXISTS "where_people_duty_demand" (
    "day" date NOT NULL,
    "shift_id" bigint NOT NULL
        REFERENCES "where_people_shifts" ("id") ON DELETE CASCADE,
    "needed" integer NOT NULL,
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("day", "shift_id"),
    CONSTRAINT "where_people_duty_demand_needed_ck"
        CHECK ("needed" BETWEEN 1 AND 99)
);

CREATE INDEX IF NOT EXISTS "where_people_duty_demand_day_idx"
    ON "where_people_duty_demand" ("day");

COMMENT ON TABLE "where_people_duty_demand" IS
    'Employees needed per day and shift in the duty plan.';
COMMENT ON COLUMN "where_people_duty_demand"."needed" IS
    'Number of employees needed in that shift on that day (1-99).';
