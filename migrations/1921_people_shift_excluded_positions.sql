-- Positions a shift does not have, and demand rows with an open position.
--
-- A shift may exclude positions of the personnel master data (for example no
-- `Meister` in the late shift): the demand dialog does not offer them for that
-- shift, and planning an employee of such a position into it is refused.
--
-- In `where_people_duty_demand`.`position` the value `*` means the position does
-- not matter — any employee of the shift covers that row.

CREATE TABLE IF NOT EXISTS "where_people_shift_excluded_positions" (
    "shift_id" bigint NOT NULL
        REFERENCES "where_people_shifts" ("id") ON DELETE CASCADE,
    "position" text NOT NULL,
    "changed_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("shift_id", "position"),
    CONSTRAINT "where_people_shift_excluded_positions_position_ck"
        CHECK (length(btrim("position")) BETWEEN 1 AND 100 AND btrim("position") <> '*')
);

COMMENT ON TABLE "where_people_shift_excluded_positions" IS
    'Positions that do not occur in a shift; the demand ignores them and planning one is refused.';
COMMENT ON COLUMN "where_people_shift_excluded_positions"."position" IS
    'Position of the personnel master data (text, as on the employee).';

COMMENT ON COLUMN "where_people_duty_demand"."position" IS
    'Position of the personnel master data, or `*` when the position does not matter.';
