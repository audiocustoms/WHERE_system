-- The order of the employees on the duty board.
--
-- The board sorts by the position order of the master data and the last name until
-- a planner drags a row. From then on this number decides, so the crew can stand
-- the way the house reads it. `NULL` = never arranged by hand: such rows follow the
-- arranged ones, sorted as before.

ALTER TABLE "where_employees"
    ADD COLUMN IF NOT EXISTS "board_order" integer;

COMMENT ON COLUMN "where_employees"."board_order" IS
    'Place of the employee on the duty board (NULL = sorted by position and name).';
