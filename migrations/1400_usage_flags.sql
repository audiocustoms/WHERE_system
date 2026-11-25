-- Usage areas as combinable flags.
--
-- Previously each device had exactly one value in "Seriennummern"."Einsatzgebiet" (1..4). A
-- device can satisfy inventory, rental and sale at the same time — the legacy schema
-- models it on article level the same way, with separate columns ("Inventar_Flag", "ZurMiete_Flag",
-- "ZumVerkauf_Flag"). WHERE therefore uses the same shape per device/batch.
--
-- "Einsatzgebiet" stays untouched (the semantics of the legacy codes cannot be proven);
-- the existing values are transferred into the flags once:
--   1 = inventory, 2 = sale, 3 = rental, 4 = used sale

ALTER TABLE "Seriennummern" ADD COLUMN IF NOT EXISTS "where_inventory" boolean DEFAULT false;
ALTER TABLE "Seriennummern" ADD COLUMN IF NOT EXISTS "where_rental" boolean DEFAULT false;
ALTER TABLE "Seriennummern" ADD COLUMN IF NOT EXISTS "where_sale" boolean DEFAULT false;
ALTER TABLE "Seriennummern" ADD COLUMN IF NOT EXISTS "where_used_sale" boolean DEFAULT false;

UPDATE "Seriennummern"
   SET "where_inventory" = COALESCE("where_inventory", false) OR "Einsatzgebiet" = 1,
       "where_sale" = COALESCE("where_sale", false) OR "Einsatzgebiet" = 2,
       "where_rental" = COALESCE("where_rental", false) OR "Einsatzgebiet" = 3,
       "where_used_sale" = COALESCE("where_used_sale", false) OR "Einsatzgebiet" = 4
 WHERE "Einsatzgebiet" IS NOT NULL;

-- Rebuild the totals per warehouse from the flags (the columns can now overlap:
-- a device counts in every usage area it satisfies).
UPDATE "Artikel_Env" e
   SET "Bestand_GesamtInventar" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                             WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                               AND g."Lager_Primkey" = e."Lager_Primkey"
                                               AND COALESCE(g."Ausgesondert", false) = false
                                               AND COALESCE(g."where_inventory", false)), 0),
       "Bestand_GesamtNVerkauf" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                             WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                               AND g."Lager_Primkey" = e."Lager_Primkey"
                                               AND COALESCE(g."Ausgesondert", false) = false
                                               AND COALESCE(g."where_sale", false)), 0),
       "Bestand_GesamtMiete" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                          WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                            AND g."Lager_Primkey" = e."Lager_Primkey"
                                            AND COALESCE(g."Ausgesondert", false) = false
                                            AND COALESCE(g."where_rental", false)), 0),
       "Bestand_GesamtGVerkauf" = COALESCE((SELECT SUM(COALESCE(g."Menge", 1)) FROM "Seriennummern" g
                                             WHERE g."Artikel_PrimKey" = e."Artikel_Primkey"
                                               AND g."Lager_Primkey" = e."Lager_Primkey"
                                               AND COALESCE(g."Ausgesondert", false) = false
                                               AND COALESCE(g."where_used_sale", false)), 0);
