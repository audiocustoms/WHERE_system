// Development database: PGlite (PostgreSQL compiled to WebAssembly) exposed over
// the PostgreSQL wire protocol, so the unmodified Axum/SQLx server can connect.
//
// Usage: node tools/dev-db.mjs [--port 5432] [--data data/pg]
//
// Purpose: let you click through the rebuild without installing a PostgreSQL server.
// For anything beyond local development, use a real PostgreSQL instance — PGlite is
// single-connection and has no background writer.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { PGlite } from "@electric-sql/pglite";
import { PGLiteSocketServer } from "@electric-sql/pglite-socket";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "..");

const args = { port: 5432, data: path.join(repo, "data", "pg"), reinit: false };
for (let i = 2; i < process.argv.length; i++) {
  if (process.argv[i] === "--port") args.port = Number(process.argv[++i]);
  else if (process.argv[i] === "--data") args.data = path.resolve(process.argv[++i]);
  else if (process.argv[i] === "--reinit") args.reinit = true;
}

if (args.reinit) {
  fs.rmSync(args.data, { recursive: true, force: true });
  console.log(`reset: ${args.data}`);
}

fs.mkdirSync(args.data, { recursive: true });

const db = await PGlite.create({ dataDir: args.data });
const version = (await db.query("select version()")).rows[0].version.split(",")[0];
console.log(`engine: ${version}`);
console.log(`data:   ${args.data}`);

const migrationsDir = path.join(repo, "migrations");
const files = fs.readdirSync(migrationsDir).filter((f) => f.endsWith(".sql")).sort();

// An own registry instead of "does table Adresse exist?": later added migrations
// are applied without re-running ones that were already applied.
await db.exec(`
  CREATE TABLE IF NOT EXISTS "_dev_migrations" (
    "filename" text PRIMARY KEY,
    "applied_at" timestamptz NOT NULL DEFAULT now()
  )`);

const applied = new Set(
  (await db.query('select "filename" from "_dev_migrations"')).rows.map((r) => r.filename)
);

// Guard against a registry that is empty for an existing schema: otherwise
// 0100_system.sql would run again and abort with "already exists".
if (applied.size === 0) {
  const existingTables = (
    await db.query(
      `select count(*)::int c from information_schema.tables
        where table_schema = 'public' and table_name <> '_dev_migrations'`
    )
  ).rows[0].c;
  if (existingTables > 0) {
    console.error(
      `\nSchema present (${existingTables} tables) but no migration registry.\n` +
        `Recreate it with:  node tools/dev-db.mjs --reinit\n`
    );
    await db.close();
    process.exit(1);
  }
}

const pending = files.filter((f) => !applied.has(f));
if (pending.length === 0) {
  console.log(`schema current (${files.length} migration files already applied)`);
} else {
  console.log(`applying ${pending.length} migration file(s)...`);
  for (const file of pending) {
    const sql = fs.readFileSync(path.join(migrationsDir, file), "utf8");
    await db.exec(sql);
    await db.query('insert into "_dev_migrations" ("filename") values ($1)', [file]);
    console.log(`  applied ${file}`);
  }
}

const server = new PGLiteSocketServer({ db, host: "127.0.0.1", port: args.port });
await server.start();
console.log(`listening on postgres://where:where@127.0.0.1:${args.port}/where`);
console.log("press Ctrl+C to stop");

const shutdown = async () => {
  console.log("\nstopping...");
  await server.stop();
  await db.close();
  process.exit(0);
};
process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
