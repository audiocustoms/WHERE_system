-- A shift can belong to a department: a dispatcher may only see and assign the
-- shifts of the departments of `Personal-Disposition`. Empty = the shift is for
-- everybody.
ALTER TABLE "where_people_shifts"
    ADD COLUMN IF NOT EXISTS "department" text;
