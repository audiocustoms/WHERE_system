-- Rights of the user groups (`img/Admin Menü/Benutzerverwaltung`).
--
-- The legacy table "UserGroups" keeps one integer per module (`Access_Adressen`,
-- `Access_Artikel`, …) and names the group; the single rights behind the tree of
-- the dialog ("Adresse anlegen", "Adresse lesen", …) have no column there. WHERE
-- keeps them next to the group: one row per ticked right and one row per scope
-- (the mandants and warehouses a group is valid for).

CREATE TABLE IF NOT EXISTS "where_user_group_rights" (
    "group_id"  integer NOT NULL,
    "scope"     text NOT NULL,
    "right_key" text NOT NULL,
    PRIMARY KEY ("group_id", "scope", "right_key")
);

COMMENT ON COLUMN "where_user_group_rights"."scope" IS
    'Rechtesatz des Dialogs: max (Max. Rechte) oder min (Min. Rechte).';

CREATE INDEX IF NOT EXISTS "where_user_group_rights_key_idx"
    ON "where_user_group_rights" ("right_key");

CREATE TABLE IF NOT EXISTS "where_user_group_scope" (
    "group_id" integer NOT NULL,
    "kind"     text NOT NULL,
    "ref_id"   integer NOT NULL,
    PRIMARY KEY ("group_id", "kind", "ref_id")
);

COMMENT ON COLUMN "where_user_group_scope"."kind" IS
    'Art der Zuordnung: mandant, lager oder client.';
