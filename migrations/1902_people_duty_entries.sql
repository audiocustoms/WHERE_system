-- Duty plans: who works on which day in which shift.
--
-- One entry per employee and day; a second save on the same day replaces the
-- shift (upsert), an empty field removes the entry.

CREATE TABLE IF NOT EXISTS "where_people_duty_entries" (
    "id" bigserial PRIMARY KEY,
    "employee_id" bigint NOT NULL REFERENCES "where_employees" ("id") ON DELETE CASCADE,
    "shift_id" bigint NOT NULL REFERENCES "where_people_shifts" ("id") ON DELETE CASCADE,
    "day" date NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_duty_employee_day_uq" UNIQUE ("employee_id", "day")
);

CREATE INDEX IF NOT EXISTS "where_people_duty_day_ix"
    ON "where_people_duty_entries" ("day");

COMMENT ON TABLE "where_people_duty_entries" IS
    'Duty plan: shift of one employee on one day (at most one per day).';
