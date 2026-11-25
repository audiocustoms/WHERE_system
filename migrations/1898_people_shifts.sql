-- Shifts and rules for the duty planning.
--
-- Several shifts lie on one day, start and end are given as `HH:MM`. If the end
-- lies before the start, the shift runs past midnight.

CREATE TABLE IF NOT EXISTS "where_people_shifts" (
    "id" bigserial PRIMARY KEY,
    "name" text NOT NULL,
    "short_name" text,
    "starts_at" text NOT NULL,
    "ends_at" text NOT NULL,
    "break_minutes" smallint NOT NULL DEFAULT 0,
    "min_staff" smallint NOT NULL DEFAULT 1,
    "color" text,
    "active" boolean NOT NULL DEFAULT true,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_shifts_clock_ck"
        CHECK ("starts_at" ~ '^[0-2][0-9]:[0-5][0-9]$'
           AND "ends_at" ~ '^[0-2][0-9]:[0-5][0-9]$'),
    CONSTRAINT "where_people_shifts_differ_ck" CHECK ("starts_at" <> "ends_at"),
    CONSTRAINT "where_people_shifts_break_ck" CHECK ("break_minutes" BETWEEN 0 AND 720),
    CONSTRAINT "where_people_shifts_staff_ck" CHECK ("min_staff" BETWEEN 0 AND 999)
);

CREATE UNIQUE INDEX IF NOT EXISTS "where_people_shifts_name_uq"
    ON "where_people_shifts" (lower("name"));
CREATE INDEX IF NOT EXISTS "where_people_shifts_order_ix"
    ON "where_people_shifts" ("sort_order", lower("name"));

COMMENT ON TABLE "where_people_shifts" IS
    'Shifts of the duty planning: name, time window, break and minimum staffing.';

-- Exactly one row with the rules the planning follows.
CREATE TABLE IF NOT EXISTS "where_people_planning" (
    "id" smallint PRIMARY KEY DEFAULT 1,
    "min_rest_hours" smallint NOT NULL DEFAULT 11,
    "max_daily_hours" smallint NOT NULL DEFAULT 10,
    "max_weekly_hours" smallint NOT NULL DEFAULT 48,
    "break_after_hours" smallint NOT NULL DEFAULT 6,
    "break_minutes" smallint NOT NULL DEFAULT 30,
    "long_break_after_hours" smallint NOT NULL DEFAULT 9,
    "long_break_minutes" smallint NOT NULL DEFAULT 60,
    "max_consecutive_days" smallint NOT NULL DEFAULT 6,
    "min_days_off_per_week" smallint NOT NULL DEFAULT 1,
    "sunday_work_allowed" boolean NOT NULL DEFAULT false,
    "holiday_work_allowed" boolean NOT NULL DEFAULT false,
    "respect_absence" boolean NOT NULL DEFAULT true,
    "planning_weeks" smallint NOT NULL DEFAULT 4,
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_planning_single_ck" CHECK ("id" = 1),
    CONSTRAINT "where_people_planning_rest_ck" CHECK ("min_rest_hours" BETWEEN 0 AND 24),
    CONSTRAINT "where_people_planning_daily_ck" CHECK ("max_daily_hours" BETWEEN 1 AND 24),
    CONSTRAINT "where_people_planning_weekly_ck" CHECK ("max_weekly_hours" BETWEEN 1 AND 168),
    CONSTRAINT "where_people_planning_break_ck"
        CHECK ("break_after_hours" BETWEEN 1 AND 24
           AND "break_minutes" BETWEEN 0 AND 240
           AND "long_break_after_hours" BETWEEN 1 AND 24
           AND "long_break_minutes" BETWEEN 0 AND 240),
    CONSTRAINT "where_people_planning_streak_ck"
        CHECK ("max_consecutive_days" BETWEEN 1 AND 31
           AND "min_days_off_per_week" BETWEEN 0 AND 7
           AND "planning_weeks" BETWEEN 1 AND 52)
);

INSERT INTO "where_people_planning" ("id") VALUES (1) ON CONFLICT DO NOTHING;

COMMENT ON TABLE "where_people_planning" IS
    'Rules of the duty planning: rest period, working time, breaks, week and horizon.';
