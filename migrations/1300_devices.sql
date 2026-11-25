-- Stock, second iteration: stock is made of devices/batches.
--
-- The form (img/stock_01.PNG) shows two grids:
--   * warehouse totals on top (F/M/VK/INV/G-VK/Res…)        -> "Artikel_Env"
--   * below the devices/batches with serial no., quantity, warehouse -> "Seriennummern"
-- New stock arrives through the goods receipt (img/goods-receipt.PNG): one
-- row in "Seriennummern" per device/batch.
--
-- WHERE recomputes the totals in "Artikel_Env" from it instead of setting them by hand.
-- The column split matches the form:
--   total   = sum of all active devices
--   rental  = "Bestand_GesamtMiete"      legacy code 3
--   sale    = "Bestand_GesamtNVerkauf"   legacy code 2
--   inventory = "Bestand_GesamtInventar" legacy code 1
--   used    = "Bestand_GesamtGVerkauf"   legacy code 4
--
-- The codes of "Einsatzgebiet" cannot be proven without the legacy source; WHERE read
-- The codes of "Einsatzgebiet" cannot be proven without the legacy source; WHERE read
-- them as follows at first (migration 1400 replaces this with combinable flags):
--   1 = inventory, 2 = sale, 3 = rental, 4 = used sale
-- "VK_Status" stays untouched — the status codes of the form ("in stock", "on loan")
-- cannot be reconstructed either. WHERE tracks stock through "Ausgesondert".

CREATE INDEX IF NOT EXISTS "Seriennummern_Artikel_Lager_idx"
    ON "Seriennummern" ("Artikel_PrimKey", "Lager_Primkey");
CREATE INDEX IF NOT EXISTS "Seriennummern_Artikel_SerienNr_idx"
    ON "Seriennummern" ("Artikel_PrimKey", "SerienNr");
CREATE INDEX IF NOT EXISTS "Seriennummern_Ausgesondert_idx"
    ON "Seriennummern" ("Ausgesondert");

-- Recompute existing totals once from the devices: "Artikel_Env" is derived,
-- and a hand-set stock without devices would be a number without backing.
-- (Assumes "Seriennummern" is complete — true when both tables are imported.)
UPDATE "Artikel_Env" e
   SET "Bestand_Gesamt" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                     WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                       AND g."Lager_Primkey" = e."Lager_Primkey"
                                       AND COALESCE(g."Ausgesondert", false) = false), 0),
       "Bestand_GesamtInventar" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                             WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                               AND g."Lager_Primkey" = e."Lager_Primkey"
                                               AND COALESCE(g."Ausgesondert", false) = false
                                               AND g."Einsatzgebiet" = 1), 0),
       "Bestand_GesamtNVerkauf" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                             WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                               AND g."Lager_Primkey" = e."Lager_Primkey"
                                               AND COALESCE(g."Ausgesondert", false) = false
                                               AND g."Einsatzgebiet" = 2), 0),
       "Bestand_GesamtMiete" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                          WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                            AND g."Lager_Primkey" = e."Lager_Primkey"
                                            AND COALESCE(g."Ausgesondert", false) = false
                                            AND g."Einsatzgebiet" = 3), 0),
       "Bestand_GesamtGVerkauf" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                             WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                               AND g."Lager_Primkey" = e."Lager_Primkey"
                                               AND COALESCE(g."Ausgesondert", false) = false
                                               AND g."Einsatzgebiet" = 4), 0),
       -- Reservations and withdrawals only appear with allocation and rental; until then
       -- there is no source for them, so no legacy data either.
       "Bestand_Festblockade" = 0,
       "Bestand_EntnahmeGesamt" = 0,
       "Bestand_EntnahmeMiete" = 0,
       "Bestand_EntnahmeNVerkauf" = 0,
       "Bestand_EntnahmeGVerkauf" = 0;
