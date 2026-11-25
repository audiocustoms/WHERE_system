-- Positions a flag does not use.
--
-- The staffing of a flag lists the positions it needs; the exclusions next to it
-- name the positions it never takes (for example a flag that is never covered by
-- an apprentice). The mask offers the remaining positions only, like the
-- exclusions of a shift.

CREATE TABLE IF NOT EXISTS "where_people_flag_excluded_positions" (
    "id" bigserial PRIMARY KEY,
    "flag_id" bigint NOT NULL
        REFERENCES "where_people_flags" ("id") ON DELETE CASCADE,
    "position" text NOT NULL,
    "sort_order" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "where_people_flag_excluded_positions_position_ck"
        CHECK (length(btrim("position")) BETWEEN 1 AND 100)
);

CREATE UNIQUE INDEX IF NOT EXISTS "where_people_flag_excluded_positions_idx"
    ON "where_people_flag_excluded_positions" ("flag_id", lower(btrim("position")));

COMMENT ON TABLE "where_people_flag_excluded_positions" IS
    'Positions a flag never uses; the staffing of that flag does not offer them.';
