-- Module registry: which of the four WHERE products share this database.
--
-- `WHERE - is my stuff?` (`stuff`) is the core. `go`, `people` and `money` are
-- modules. The first product installed creates the database and records itself
-- as `database_owner`; every later product reuses it. The registry lives in the
-- same database as the data, so "same database" and "same tenants" are the same
-- statement.
--
-- The Rust side (`where_modules::install`) creates these tables as well, because
-- a module install can run before the core migrations. Keeping the migration is
-- still required so a core installation with `run_migrations` has them.

CREATE TABLE IF NOT EXISTS "where_installation" (
    "key"             boolean PRIMARY KEY DEFAULT true CHECK ("key"),
    "installation_id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "created_at"      timestamptz NOT NULL DEFAULT now(),
    "database_owner"  text
);

COMMENT ON TABLE "where_installation" IS
    'One row per database; `database_owner` names the product that created it.';

CREATE TABLE IF NOT EXISTS "where_modules" (
    "id"           text PRIMARY KEY,
    "product_name" text NOT NULL,
    "kind"         text NOT NULL CHECK ("kind" IN ('core', 'module')),
    "installed_at" timestamptz NOT NULL DEFAULT now(),
    "installed_by" text
);

COMMENT ON TABLE "where_modules" IS
    'Installed products of this installation; `stuff` is the core application.';
