-- The two warehouse switches that steer the daily work (`Lager und Lagerplätze`).
--
-- `"where_zugriff"` says whether a warehouse takes part in the working masks at all —
-- goods receipt into it, transfer into it, stocktaking of it, and it is the list the
-- selects offer. `"where_entnahme"` says whether material may be **taken out** of it:
-- `Ausgeben`, `Verbrauchen` and `Umlegen` out of it. A display shelf or a consignment
-- store can stay visible with the withdrawal closed.
--
-- Both columns exist since `1860_warehouses.sql` and default to `false`, which would
-- block every warehouse that is already in use. The warehouses of an existing
-- installation were selectable and withdrawable before the switches meant anything, so
-- they keep both; only what the administration closes from now on is closed.

UPDATE "Storage_Lager"
   SET "where_zugriff" = true,
       "where_entnahme" = true
 WHERE "where_zugriff" = false
   AND "where_entnahme" = false;

COMMENT ON COLUMN "Storage_Lager"."where_zugriff" IS
    'Warehouse takes part in the working masks (goods receipt, transfer, stocktaking, selects)';
COMMENT ON COLUMN "Storage_Lager"."where_entnahme" IS
    'Material may be taken out of this warehouse (issue, consumption, transfer out)';
