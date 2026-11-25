-- Stock: warehouses and stock bookings.
--
-- The legacy schema keeps current stock in "Artikel_Env" — one row per article and
-- warehouse ("Lager_Primkey") with "Bestand_Gesamt", reservations ("Bestand_Festblockade",
-- "Bestand_Mengenblockade") and withdrawals ("Bestand_EntnahmeGesamt"). The warehouses
-- themselves live in "Storage_Lager" (Name, "__DefaultLager").
--
-- WHERE needs at least one warehouse before the first booking. The migration creates
-- one only when none exists; existing warehouses from an import stay
-- untouched. Stock is deliberately not invented — an article without a booking
-- keeps no row in "Artikel_Env" and is shown as "—" in the list.

INSERT INTO "Storage_Lager" ("Name", "__DefaultLager", "Mandanten_ID",
                             "SymetrischeLagerortung", "VordefinierteLagerortung")
SELECT 'Hauptlager', true, 0, false, false
WHERE NOT EXISTS (SELECT 1 FROM "Storage_Lager");

-- One warehouse must be recognisable as default, otherwise the UI does not know
-- where the first booking goes.
UPDATE "Storage_Lager"
   SET "__DefaultLager" = true
 WHERE "Primary_Key" = (SELECT min("Primary_Key") FROM "Storage_Lager")
   AND NOT EXISTS (SELECT 1 FROM "Storage_Lager" WHERE "__DefaultLager");

-- The stock queries of the list and the stock view filter by article and warehouse.
CREATE INDEX IF NOT EXISTS "Artikel_Env_Artikel_Lager_idx"
    ON "Artikel_Env" ("Artikel_Primkey", "Lager_Primkey");
