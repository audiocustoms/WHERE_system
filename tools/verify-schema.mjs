// Verifies the migrations and the repository SQL against a real PostgreSQL engine
// without needing a database server: PGlite is PostgreSQL compiled to WebAssembly.
//
// Usage:
//   cd tools && npm install
//   node tools/verify-schema.mjs
//
// What it proves:
//   1. Every migration applies in order (tables, primary keys, indexes).
//   2. The SQL constants in where-persistence parse and execute against that schema,
//      with real parameter binding — including the round trip.
//   3. For the article master, every value lands in the column it is named after.
//      This is what catches a shifted parameter list in the 57-column INSERT/UPDATE.
//
// The statements are read from the Rust sources, so this cannot silently drift from
// the code under test. It does not replace a test run against a real server (network,
// pooling, concurrency and SQLx type mapping are outside PGlite's scope).

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { PGlite } from "@electric-sql/pglite";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "..");

function sqlConstants(relativeFile) {
  const source = fs.readFileSync(path.join(repo, relativeFile), "utf8");
  const found = {};
  for (const match of source.matchAll(/const\s+(\w+_SQL):\s*&str\s*=\s*r#"?([\s\S]*?)"#;/g)) {
    found[match[1]] = match[2];
  }
  // A typo or a removed constant must not slip through as "passed":
  // db.query(undefined) would silently check nothing.
  return new Proxy(found, {
    get(target, prop) {
      if (typeof prop === "string" && prop.endsWith("_SQL") && !(prop in target)) {
        throw new Error(`${relativeFile}: SQL constant ${prop} does not exist (renamed or removed?)`);
      }
      return target[prop];
    },
  });
}

  /** Any `const NAME: &str = r#"…"#;` — also the fragments that are composed into statements. */
  function sqlFragment(relativeFile, name) {
    const source = fs.readFileSync(path.join(repo, relativeFile), "utf8");
    const match = source.match(new RegExp(`const\\s+${name}:\\s*&str\\s*=\\s*r#"([\\s\\S]*?)"#;`));
    if (!match) throw new Error(`${relativeFile}: ${name} does not exist (renamed?)`);
    return match[1];
  }

const db = await PGlite.create();
let failures = 0;

const version = (await db.query("select version()")).rows[0].version.split(",")[0];
console.log(`engine: ${version}\n`);

const migrations = fs
  .readdirSync(path.join(repo, "migrations"))
  .filter((f) => f.endsWith(".sql"))
  .sort();

let statements = 0;
for (const file of migrations) {
  const sql = fs.readFileSync(path.join(repo, "migrations", file), "utf8");
  const count = (sql.match(/CREATE (TABLE|INDEX|UNIQUE INDEX) /g) || []).length;
  try {
    await db.exec(sql);
    statements += count;
    console.log(`  OK   ${file.padEnd(26)} ${String(count).padStart(4)} statements`);
  } catch (error) {
    failures++;
    console.log(`  FAIL ${file}: ${error.message}`);
  }
}

const scalar = async (q) => (await db.query(q)).rows[0].c;
const tables = await scalar("select count(*)::int c from information_schema.tables where table_schema='public'");
const primaryKeys = await scalar("select count(*)::int c from information_schema.table_constraints where constraint_type='PRIMARY KEY' and table_schema='public'");
const columns = await scalar("select count(*)::int c from information_schema.columns where table_schema='public'");
const withoutPk = await db.query(`
  select t.table_name from information_schema.tables t
   where t.table_schema='public'
     and not exists (select 1 from information_schema.table_constraints c
                      where c.table_schema='public' and c.table_name=t.table_name
                        and c.constraint_type='PRIMARY KEY')
   order by 1`);

console.log(`\nmigrations: ${statements} statements -> ${tables} tables, ${primaryKeys} primary keys, ${columns} columns`);
if (withoutPk.rows.length) {
  failures++;
  console.log(`  ! tables without primary key: ${withoutPk.rows.map((r) => r.table_name).join(", ")}`);
}

const flagColumns = await db.query(`
  select column_name from information_schema.columns
   where table_schema='public' and table_name='Seriennummern'
     and column_name in ('where_inventory','where_rental','where_sale','where_used_sale')
   order by 1`);
if (flagColumns.rows.length !== 4) {
  failures++;
  console.log(`  ! Einsatzbereich-Flags fehlen auf "Seriennummern": ${flagColumns.rows.map((r) => r.column_name).join(", ") || "keine"}`);
}

// --- repository statements, executed exactly as written in the Rust sources ---
const addresses = sqlConstants("crates/where-persistence/src/address_repo.rs");
// The address search assembles both of its statements from this fixed WHERE block.
const addressFilter = (() => {
  const source = fs.readFileSync(
    path.join(repo, "crates/where-persistence/src/address_repo.rs"),
    "utf8",
  );
  const match = source.match(/const SEARCH_FILTER: &str = r#"([\s\S]*?)"#;/);
  if (!match) throw new Error("address_repo.rs: SEARCH_FILTER does not exist (renamed?)");
  return match[1];
})();
const articles = sqlConstants("crates/where-persistence/src/article_repo.rs");
const files = sqlConstants("crates/where-persistence/src/file_repo.rs");
const gruppenRepo = sqlConstants("crates/where-persistence/src/article_group_repo.rs");
const sucheRepo = sqlConstants("crates/where-persistence/src/article_search_repo.rs");

const texteInsert = () => [
  inserted.k.Primary_Key, 1, "Scheinwerfer", "SW",
  "Kurztext", "Langtext", "Technik", "Lieferumfang", "Packinfo", "Webshop",
];
const cases = [];

const inserted = {};

async function caseRun(label, fn) {
  try {
    await fn();
    return true;
  } catch (error) {
    failures++;
    console.log(`  ! ${label}: ${error.message.split("\n")[0]}`);
    return false;
  }
}

// ------------------------------------------------------------------- Address
// Order exactly as the `bind_columns!` macro in AddressRepository.
const A_COLUMNS = [
  1001, "Dr.", "Muster", null, null, "Herr", "Weg 1", "DE", "Berlin", "DE123456",
  "10001", "030 123456", "muster@example.test", "Theater", true, false, false, false,
  true, false, null, "Hinterhaus", "10115", "Berlin", "030 123457", "0170 1234567",
  "https://muster.example", false, false, false, true,
  "DE123456789012345", "12/345/67890", "HRB 12345", "Berlin", 5000, 5, 3,
  "Sparkasse", "123456789", "10050000", "DE89370400440532013000", "COBADEFFXXX",
  2001, "70001", 0, 2, 4,
  "Commerzbank", "987654321", "50040000", "DE12500105170648489890", "COBADEBBXXX",
];

/** Copies the column list and replaces the indices given in `over`. */
const aCols = (over = {}) =>
  A_COLUMNS.map((value, index) => (index in over ? over[index] : value));

// Changed values of the update: title, name, customer number, phone, e-mail,
// group and the supplier role.
const aIns = aCols();
const aUpd = [
  1,
  ...aCols({
    0: 1002,
    1: "Muster GmbH",
    2: "Muster",
    11: "030 654321",
    12: "neu@example.test",
    13: "Veranstalter",
    15: true,
  }),
];

cases.push(["address INSERT", async () => { inserted.a = (await db.query(addresses.INSERT_SQL, aIns)).rows[0]; }]);
// The address list composes its statements from this fixed WHERE block and one
// fixed ORDER BY expression per sortable column, exactly as the repository does.
const addressListFilter = sqlFragment("crates/where-persistence/src/address_repo.rs", "LIST_FILTER");
const addressSorts = Object.fromEntries(
  ["TYPE", "NUMBER", "NAME", "COUNTRY", "POSTAL_CODE", "CITY"].map((key) => [
    key.toLowerCase(),
    sqlFragment("crates/where-persistence/src/address_repo.rs", `SORT_EXPR_${key}`),
  ]),
);
/** `$1` limit, `$2` offset, `$3` search, `$4` state — the order of `AddressRepository::list`. */
const addressListArgs = (search = null, state = "active") => [50, 0, search, state];
cases.push([
  "address LIST count",
  () => db.query(`SELECT count(*) FROM "Adresse"${addressListFilter} LIMIT $1 OFFSET $2`, addressListArgs()),
]);
for (const [column, expression] of Object.entries(addressSorts)) {
  cases.push([
    `address LIST sort ${column}`,
    () =>
      db.query(
        `SELECT * FROM "Adresse"${addressListFilter} ORDER BY ${expression} ASC NULLS LAST, "Primary_key" LIMIT $1 OFFSET $2`,
        addressListArgs(),
      ),
  ]);
  cases.push([
    `address LIST sort ${column} desc`,
    () =>
      db.query(
        `SELECT * FROM "Adresse"${addressListFilter} ORDER BY ${expression} DESC NULLS LAST, "Primary_key" LIMIT $1 OFFSET $2`,
        addressListArgs(),
      ),
  ]);
}
cases.push([
  "address LIST search",
  () => db.query(`SELECT * FROM "Adresse"${addressListFilter} LIMIT $1 OFFSET $2`, addressListArgs("Must")),
]);
cases.push([
  "address LIST inactive",
  () => db.query(`SELECT * FROM "Adresse"${addressListFilter} LIMIT $1 OFFSET $2`, addressListArgs(null, "inactive")),
]);
cases.push([
  "address LIST all",
  () => db.query(`SELECT * FROM "Adresse"${addressListFilter} LIMIT $1 OFFSET $2`, addressListArgs(null, "all")),
]);
cases.push(["address GET", () => db.query(addresses.GET_SQL, [inserted.a.Primary_key])]);
cases.push(["address UPDATE", () => db.query(addresses.UPDATE_SQL, aUpd)]);
cases.push(["address DELETE missing", () => db.query(addresses.DELETE_SQL, [999999])]);
cases.push(["address INDEX VALUES", () => db.query(addresses.INDEX_VALUES_SQL, [])]);
cases.push(["address DUPLICATES count", () => db.query(addresses.DUPLICATES_COUNT_SQL, ["Must"])]);
cases.push(["address DUPLICATES page", () => db.query(addresses.DUPLICATES_SQL, [25, "Must"])]);
cases.push(["address DUPLICATES without term", () => db.query(addresses.DUPLICATES_SQL, [25, null])]);

// Address search: same parameter list as AddressRepository::search.
const aSearch = [
  50, 0, "customers", "%Muster%", true, "%Theater%", 1001, "%10001%",
  "%Berlin%", "%030%", "%example.test%", null, "contains_any", false,
];
cases.push([
  "address SEARCH count",
  () => db.query(`SELECT count(*) FROM "Adresse" a${addressFilter} LIMIT $1 OFFSET $2`, aSearch),
]);
cases.push([
  "address SEARCH page",
  () =>
    db.query(
      `SELECT * FROM "Adresse" a${addressFilter} ORDER BY a."Name1", a."Primary_key" LIMIT $1 OFFSET $2`,
      aSearch,
    ),
]);
cases.push([
  "address SEARCH by index",
  () =>
    db.query(`SELECT * FROM "Adresse" a${addressFilter} LIMIT $1 OFFSET $2`, [
      ...aSearch.slice(0, 11),
      ["Bühne"],
      "contains_all",
      true,
    ]),
]);

// ------------------------------------------------------------------ Article
// Order exactly as the bind chain in ArticleRepository::create.
const now = new Date().toISOString();
const kIns = [
  4711, "INV-1", "Scheinwerfer", "SW", "SW750", "Stk",
  true, false, true, false, false,
  true, false, false, true, false,
  "4001234567890", "ETC-4711", "ETC", "DE", "1299.00", "9405.4010", "9405.4090",
  "1500.00", "1800.00", "Licht-Team", "PK1",
  "32.5", "40.0", "25.0", "0.0325", "6.50",
  "38.0", "46.0", "31.0", "0.054188", "8.20",
  "230.0", "240.0", "1.20", "0.750", 15, 10, 30, 20,
  2, 6, 30,
  true, "120.0", true, true, false, 3, 6,
  // Miete: Berechnung + Eigenschaften
  false, false, false,
  true, false, false, false, true,
  true, false,
  true, true, false,
  // Verkauf: Eigenschaften
  true, true, "Karton", "4.0", "12.0",
  now, now,
];

cases.push(["article COUNT", () => db.query(articles.COUNT_SQL, [null, null, null])]);
cases.push(["article COUNT group", () => db.query(articles.COUNT_SQL, [null, 1, null])]);
cases.push(["article COUNT active", () => db.query(articles.COUNT_SQL, [null, null, true])]);
cases.push(["article INSERT", async () => { inserted.k = (await db.query(articles.INSERT_SQL, kIns)).rows[0]; }]);cases.push(["article LIST", () => db.query(articles.LIST_SQL, [50, 0, null, null, null])]);
cases.push(["article LIST search", () => db.query(articles.LIST_SQL, [50, 0, "Schein", null, null])]);
cases.push(["article LIST group", () => db.query(articles.LIST_SQL, [50, 0, null, 1, null])]);
cases.push(["article LIST group subtree", () => db.query(articles.LIST_SQL, [50, 0, null, 2, null])]);
cases.push(["article LIST active", () => db.query(articles.LIST_SQL, [50, 0, null, null, true])]);
cases.push(["article LIST inactive", () => db.query(articles.LIST_SQL, [50, 0, null, null, false])]);
cases.push(["article GET", () => db.query(articles.GET_SQL, [inserted.k.Primary_Key])]);
cases.push(["article languages", () => db.query(articles.LANGUAGES_SQL, [])]);

cases.push(["article TEXTS INSERT", () => db.query(articles.TEXTS_INSERT_SQL, texteInsert())]);
cases.push(["article TEXTS SELECT", () => db.query(articles.TEXTS_SQL, [inserted.k.Primary_Key])]);
cases.push(["article TEXTS UPDATE", () => db.query(articles.TEXTS_UPDATE_SQL, [
  inserted.k.Primary_Key, 1, "Scheinwerfer", "SW", "Kurztext 2", "Langtext", "Technik",
  "Lieferumfang", "Packinfo", "Webshop",
])]);
cases.push(["article GROUPS INSERT", () => db.query(articles.ARTICLE_GROUPS_INSERT_SQL, [inserted.k.Primary_Key, 1])]);
cases.push(["article GROUPS INSERT 2", () => db.query(articles.ARTICLE_GROUPS_INSERT_SQL, [inserted.k.Primary_Key, 2])]);
cases.push(["article GROUPS SELECT", () => db.query(articles.ARTICLE_GROUPS_SQL, [inserted.k.Primary_Key])]);
cases.push(["article GROUPS DELETE", () => db.query(articles.ARTICLE_GROUPS_DELETE_SQL, [inserted.k.Primary_Key])]);

// -------------------------------------------------- Pyro (Artikel) und Lieferanten
// Order exactly as the bind chain in ArticleRepository::save_pyro.
const pyroWerte = (id) => [
  id,
  "1.4G", "UN0336 Feuerwerkskörper", "1.4G", 50,
  "8.0", "12.0", "25.0", "120.0", "15.0",
  "D", "2.5", "6.0", "Kategorie 2", "BAM-1234", "PM-II",
  "II", "4-1234", "1.25", "kg", "60", "3",
];
cases.push(["article PYRO SAVE", () => db.query(articles.PYRO_SAVE_SQL, pyroWerte(inserted.k.Primary_Key))]);
cases.push(["article PYRO stored", async () => {
  const r = (await db.query(articles.GET_SQL, [inserted.k.Primary_Key])).rows[0];
  if (r.pyro_UN_number !== "1.4G") throw new Error(`pyro_UN_number ${r.pyro_UN_number}`);
  if (Number(r.pyro_caliber) !== 50) throw new Error(`pyro_caliber ${r.pyro_caliber}`);
  if (Number(r.pyro_neq) !== 1.25) throw new Error(`pyro_neq ${r.pyro_neq}`);
  if (r.pyro_transport_category !== "3") throw new Error(`pyro_transport_category ${r.pyro_transport_category}`);
}]);

// Lieferanten: the article stores addresses with "Type_Lieferant".
cases.push(["article SUPPLIERS DELETE", () => db.query(articles.SUPPLIERS_DELETE_SQL, [inserted.k.Primary_Key])]);
cases.push(["article SUPPLIERS INSERT", () => db.query(articles.SUPPLIERS_INSERT_SQL, [
  inserted.k.Primary_Key, inserted.a.Primary_key, 1, "120.00", 7, true, false, "L-4711",
])]);
cases.push(["article SUPPLIERS SELECT", async () => {
  const rows = (await db.query(articles.SUPPLIERS_SQL, [inserted.k.Primary_Key])).rows;
  if (rows.length !== 1) throw new Error(`expected 1 supplier, got ${rows.length}`);
  const s = rows[0];
  if (s.supplier_id !== inserted.a.Primary_key) throw new Error("wrong supplier id");
  if (s.name !== "Muster") throw new Error(`name ${s.name}`);
  if (Number(s.purchase_price) !== 120) throw new Error(`purchase price ${s.purchase_price}`);
  if (Number(s.delivery_days) !== 7) throw new Error(`delivery days ${s.delivery_days}`);
  if (!s.favorite || s.sales_article) throw new Error("flags wrong");
  if (s.supplier_article_no !== "L-4711") throw new Error("supplier article no missing");
}]);

// ------------------------------------------------------- Prices (Miete/Verkauf)
// Order exactly as the bind chain in ArticleRepository::save_price_data.
const preisWerte = (id) => [
  id,
  "45.50", "30.00", "500.00",
  "50.00", "35.00",
  "55.00", "38.00", "300.00",
  "3800.00", "4522.00", "4290.00",
  "3432.00", "2600.00",
  "2400.00", "2500.00", "2600.00", "2700.00",
  19, 19, "35.50",
  "8400", "8401", "KST10", "KST20",
  1, null, null, 2,
  "1900.00", "Regal L3",
];
cases.push(["article PRICE INSERT", () => db.query(articles.PRICE_INSERT_SQL, preisWerte(inserted.k.Primary_Key))]);
cases.push(["article PRICE ROW", async () => {
  const rows = (await db.query(articles.PRICE_ROW_SQL, [inserted.k.Primary_Key])).rows;
  if (rows.length !== 1) throw new Error(`expected 1 price row, got ${rows.length}`);
  inserted.preis = rows[0].Primary_key;
}]);
cases.push(["article PRICE SELECT", async () => {
  const p = (await db.query(articles.PRICE_SQL, [inserted.k.Primary_Key])).rows[0];
  if (Number(p.Mietpreis) !== 45.5) throw new Error(`Mietpreis ${p.Mietpreis}`);
  if (Number(p.VKPreisNetto) !== 3800) throw new Error(`VKPreisNetto ${p.VKPreisNetto}`);
  if (p.FiBu_KtoErtragMiete !== "8400" || p.FiBu_KST_Verkauf !== "KST20") {
    throw new Error("accounts not stored");
  }
  if (p.MwStMietArtikel !== 19 || p.MwStVKArtikel !== 19) throw new Error("VAT rate not stored");
}]);
cases.push(["article PRICE UPDATE", async () => {
  const werte = preisWerte(inserted.preis).slice(1);
  werte[0] = "48.00";
  werte[24] = 2;
  await db.query(articles.PRICE_UPDATE_SQL, [inserted.preis, ...werte]);
  const p = (await db.query(articles.PRICE_SQL, [inserted.k.Primary_Key])).rows[0];
  if (Number(p.Mietpreis) !== 48) throw new Error(`Mietpreis ${p.Mietpreis}`);
  if (p.ZRabStaffel_PK !== 2) throw new Error(`ZRabStaffel ${p.ZRabStaffel_PK}`);
}]);
cases.push(["scales of the rental", async () => {
  const rows = (await db.query(articles.SCALE_CATEGORIES_SQL, [false])).rows;
  if (!rows.length) throw new Error("no factor scale present — migration 1600 created none");
  if (rows.some((r) => r.Verkauf)) throw new Error("sale scale in the rental list");
}]);
cases.push(["scales of the sale", async () => {
  const rows = (await db.query(articles.SCALE_CATEGORIES_SQL, [true])).rows;
  if (!rows.length) throw new Error("no quantity scale present — migration 1600 created none");
  if (rows.some((r) => !r.Verkauf)) throw new Error("rental scale in the sale list");
}]);
cases.push(["scale steps", async () => {
  const rows = (await db.query(articles.SCALE_STEPS_SQL, [2])).rows;
  if (rows.length < 2) throw new Error(`expected steps, got ${rows.length}`);
  if (Number(rows[0].Miettage) !== 10) throw new Error(`first step starts at ${rows[0].Miettage}`);
}]);

// UPDATE: $1 = id, then the same fields as in the INSERT (without audit timestamps).
// As a function so `inserted.k` is read only when it runs.
const kUpd = () => [inserted.k.Primary_Key, 4711, "INV-1", "Scheinwerfer LED", "SW-LED", "SW750L", "Stk",
  true, false, true, false, true,
  true, false, false, true, false,
  "4001234567890", "ETC-4711", "ETC", "DE", "1399.00", "9405.4010", "9405.4090",
  "1500.00", "1800.00", "Licht-Team", "PK1",
  "33.0", "40.0", "25.0", "0.033", "6.60",
  "38.0", "46.0", "31.0", "0.0542", "8.30",
  "230.0", "240.0", "1.20", "0.750", 15, 11, 30, 20,
  2, 6, 31,
  true, "130.0", true, true, true, 4, 7,
  // Miete
  true, true, true,
  false, true, true, false, false,
  false, true,
  false, false, true,
  // Verkauf
  false, false, "Kiste", "6.0", "10.0"];
cases.push(["article UPDATE", async () => {
  const result = await db.query(articles.UPDATE_SQL, kUpd());
  // A wrongly bound id would silently change zero rows here.
  if (result.rows.length !== 1) throw new Error(`expected 1 changed row, got ${result.rows.length}`);
}]);
cases.push(["article DELETE missing", () => db.query(articles.DELETE_SQL, [999999])]);
// The counter of the 'article' series is pulled up to the highest number already
// issued before the next allocation.
cases.push(["article align number series", async () => {
  await db.query('UPDATE "where_number_series" SET "current_value" = 0 WHERE "key" = $1', [
    "article",
  ]);
  await db.query(articles.ALIGN_MAX_SQL);
  const counter = (await db.query(
    'SELECT "current_value" FROM "where_number_series" WHERE "key" = $1',
    ["article"],
  )).rows[0];
  const max = (await db.query('SELECT COALESCE(MAX("ArtikelNr"), 0) AS m FROM "Artikel"')).rows[0];
  if (Number(counter.current_value) !== Number(max.m)) {
    throw new Error(`counter ${counter.current_value} instead of MAX(ArtikelNr) ${max.m}`);
  }
}]);

// ---------------------------------------------------------------- File storage
cases.push(["file MAX_VERSION", () => db.query(files.MAX_VERSION_SQL, [inserted.k?.Primary_Key, "Datenblatt", "sheet.pdf"])]);
cases.push(["file INSERT", async () => {
  inserted.d = (await db.query(files.INSERT_SQL, [
    inserted.k?.Primary_Key, "Datenblatt", "sheet.pdf", false, null,
    "application/pdf", 1024, "a".repeat(64), 1, "tester",
  ])).rows[0];
}]);
cases.push(["file LIST", () => db.query(files.LIST_SQL, [inserted.k?.Primary_Key])]);
cases.push(["file LIST ALL", () => db.query(files.LIST_ALL_SQL, [inserted.k?.Primary_Key])]);
cases.push(["file GET", () => db.query(files.GET_SQL, [inserted.d?.id])]);
cases.push(["file CLEAR CURRENT", () => db.query(files.CLEAR_CURRENT_SQL, [inserted.k?.Primary_Key, "Datenblatt", "sheet.pdf"])]);
cases.push(["file HISTORY", () => db.query(files.HISTORY_SQL, [inserted.d?.id, "created", "Version 1", "tester"])]);
cases.push(["file SOFT DELETE", () => db.query(files.SOFT_DELETE_SQL, [inserted.d?.id])]);

// ------------------------------------------------------- Article groups (admin)
// Order as in ArticleGroupRepository::create / update / delete.
cases.push(["article group INSERT root", async () => {
  inserted.w = (await db.query(gruppenRepo.INSERT_SQL, ["Admin-Prüfung", null])).rows[0];
}]);
cases.push(["article group INSERT child", async () => {
  const parent = (await db.query(gruppenRepo.INSERT_SQL, ["Projektor", inserted.w?.id])).rows[0];
  inserted.wu = parent;
}]);
cases.push(["article group LIST without search", async () => {
  const rows = (await db.query(gruppenRepo.LIST_SQL, [null])).rows;
  const kind = rows.find((r) => r.id === inserted.wu?.id);
  if (!kind) throw new Error("created child group is missing from the list");
  if (kind.parent_id !== inserted.w.id) throw new Error(`parent_id ${kind.parent_id} instead of ${inserted.w.id}`);
  if (kind.name !== "Projektor") throw new Error(`name ${kind.name}`);
}]);
cases.push(["article group LIST with search", () => db.query(gruppenRepo.LIST_SQL, ["projektor"])]);
cases.push(["article group GET", () => db.query(gruppenRepo.GET_SQL, [inserted.wu?.id])]);
cases.push(["article group DUPLICATE", async () => {
  // The repository call trims the name and passes 0 as "except this id".
  const rows = (await db.query(gruppenRepo.DUPLICATE_SQL, [" admin-prüfung ", 0])).rows;
  if (rows.length !== 1) throw new Error("duplicate was not detected");
}]);
cases.push(["article group USAGE", () => db.query(gruppenRepo.USAGE_SQL, [inserted.w?.id])]);
cases.push(["article group CHILDREN", async () => {
  const rows = (await db.query(gruppenRepo.CHILDREN_SQL, [inserted.w?.id])).rows;
  if (Number(rows[0].count) !== 1) throw new Error(`expected 1 child group, got ${rows[0].count}`);
}]);
cases.push(["article group IS_DESCENDANT", async () => {
  const rows = (await db.query(gruppenRepo.IS_DESCENDANT_SQL, [inserted.w?.id, inserted.wu?.id])).rows;
  if (Number(rows[0].count) !== 1) throw new Error("cycle check does not find the child group");
}]);
cases.push(["article group UPDATE", async () => {
  const result = await db.query(gruppenRepo.UPDATE_SQL, [inserted.wu?.id, "Projektor neu", null]);
  if (result.rows.length !== 1) throw new Error(`expected 1 changed row, got ${result.rows.length}`);
  // Column check: name -> "Index", parent -> "Parent_ID", 0 = none.
  const row = (await db.query(
    'select "Index","Parent_ID" from "Storage_Warengruppe" where "Primary_Key"=$1',
    [inserted.wu.id],
  )).rows[0];
  if (row.Index !== "Projektor neu") throw new Error(`"Index" is ${row.Index}`);
  if (Number(row.Parent_ID) !== 0) throw new Error(`"Parent_ID" is ${row.Parent_ID} instead of 0`);
}]);
cases.push(["article group DELETE", () => db.query(gruppenRepo.DELETE_SQL, [inserted.wu?.id])]);
cases.push(["article group DELETE root", () => db.query(gruppenRepo.DELETE_SQL, [inserted.w?.id])]);

// --------------------------------------------------------------- Number series
const series = sqlConstants("crates/where-persistence/src/number_series_repo.rs");
cases.push(["number series LIST", async () => {
  const rows = (await db.query(series.LIST_SQL)).rows;
  if (rows.length < 21) throw new Error(`expected at least 21 series, got ${rows.length}`);
  if (!rows.some((r) => r.key === "article")) throw new Error("series 'article' is missing");
}]);
cases.push(["number series GET", async () => {
  const rows = (await db.query(series.GET_SQL, ["customer"])).rows;
  if (rows.length !== 1) throw new Error(`expected 1 row, got ${rows.length}`);
}]);
// UPDATE: $1 key, $2..$9 settings, $10 counter value.
cases.push(["number series UPDATE", async () => {
  const result = await db.query(series.UPDATE_SQL, [
    "invoice", "Invoice number", "RE-", "{year}-{number}", 3, 1, 1, true, true, 0,
  ]);
  if (result.rows.length !== 1) throw new Error(`expected 1 changed row, got ${result.rows.length}`);
  const row = (await db.query(series.GET_SQL, ["invoice"])).rows[0];
  if (row.prefix !== "RE-" || row.format !== "{year}-{number}") throw new Error("prefix/format not stored");
  if (row.digits !== 3 || row.yearly_reset !== true) throw new Error("digits/yearly reset not stored");
}]);
// RESERVE: $2 is the current year.
cases.push(["number series RESERVE first number", async () => {
  const row = (await db.query(series.RESERVE_SQL, ["invoice", 2026])).rows[0];
  if (Number(row.value) !== 1) throw new Error(`expected start value 1, got ${row.value}`);
  if (row.prefix !== "RE-" || row.digits !== 3) throw new Error("prefix/digits not returned");
}]);
cases.push(["number series RESERVE keeps counting", async () => {
  const row = (await db.query(series.RESERVE_SQL, ["invoice", 2026])).rows[0];
  if (Number(row.value) !== 2) throw new Error(`expected 2, got ${row.value}`);
}]);
cases.push(["number series RESERVE new year", async () => {
  const row = (await db.query(series.RESERVE_SQL, ["invoice", 2027])).rows[0];
  if (Number(row.value) !== 1) throw new Error(`expected restart at 1, got ${row.value}`);
}]);
cases.push(["number series RESET", async () => {
  const result = await db.query(series.RESET_SQL, ["invoice"]);
  if (result.rows.length !== 1) throw new Error("reset did not change a row");
  const row = (await db.query(series.GET_SQL, ["invoice"])).rows[0];
  if (Number(row.current_value) !== 0 || row.year !== null) throw new Error("counter not reset");
}]);
cases.push(["number series RESERVE inactive series", async () => {
  await db.query('UPDATE "where_number_series" SET "active" = false WHERE "key" = $1', ["invoice"]);
  const rows = (await db.query(series.RESERVE_SQL, ["invoice", 2026])).rows;
  if (rows.length !== 0) throw new Error("inactive series handed out a number");
}]);

// ---------------------------------------------------------------- Stock levels
// Stock comes from devices/batches ("Seriennummern"); "Artikel_Env" is recomputed
// from it — exactly the order StockRepository uses.
const stock = sqlConstants("crates/where-persistence/src/stock_repo.rs");
const firstStockId = async () =>
  (await db.query('SELECT min("Primary_Key") AS id FROM "Storage_Lager"')).rows[0].id;

async function recomputeTotals(articleId, stockIdValue) {
  const s = (await db.query(stock.TOTALS_SQL, [articleId, stockIdValue])).rows[0];
  const ts = Math.floor(Date.now() / 1000);
  const changed = (
    await db.query(stock.ENV_UPDATE_SQL, [
      articleId,
      stockIdValue,
      s.total,
      s.rental,
      s.sale,
      s.inventory,
      s.used_sale,
      ts,
    ])
  ).rowCount;
  if (changed === 0) {
    await db.query(stock.ENV_INSERT_SQL, [
      articleId,
      stockIdValue,
      s.total,
      s.rental,
      s.sale,
      s.inventory,
      s.used_sale,
      ts,
    ]);
  }
}

cases.push(["stock without devices", async () => {
  const rows = (await db.query(stock.STOCK_SQL, [inserted.k.Primary_Key])).rows;
  if (!rows.length) throw new Error("no warehouse present — migration 1200 created none");
  if (!rows.some((r) => r.is_default)) throw new Error("no default warehouse marked");
  if (rows.some((r) => r.total !== null)) throw new Error("warehouse without devices reports stock");
}]);
cases.push(["device GOODS RECEIPT", async () => {
  const id = await firstStockId();
  const today = "2026-11-11";
  const receipt = (
    await db.query(stock.GOODS_RECEIPT_SQL, [
      inserted.k.Primary_Key, // $1
      "TEST-0001", // $2 SerienNr
      false, // $3 ohneSN
      1, // $4 Menge
      true, // $5 Inventar
      true, // $6 Miete  — mehrere Bereiche gleichzeitig
      false, // $7 Verkauf
      false, // $8 Gebraucht
      id, // $9 Lager
      "Regal A1", // $10 Lagerplatz
      "gut", // $11 Zustand
      99.5, // $12 Kaufpreis
      today, // $13 purchase date
      10001, // $14 Lieferant
      "INV-TEST-1", // $15 InventarNr
      "Lieferschein Nr. 42", // $16 Bemerkung
      today, // $17 date
      "10:00:00", // $18 Zeit
      "tester", // $19 Benutzer
    ])
  ).rows[0];
  inserted.g = { id: receipt.Primary_Key, stock: id };
  const g = (await db.query(stock.DEVICE_SQL, [inserted.k.Primary_Key, receipt.Primary_Key])).rows[0];
  if (g.serial_no !== "TEST-0001") throw new Error(`SerienNr ${g.serial_no}`);
  if (g.bin_location !== "Regal A1" || g.condition !== "gut") throw new Error("bin location/condition not stored");
  if (Number(g.purchase_price) !== 99.5) throw new Error(`purchase price ${g.purchase_price}`);
  if (g.note !== "Lieferschein Nr. 42") throw new Error("reason not in the note");
  if (!g.inventory || !g.rental || g.sale || g.used_sale) throw new Error("usage flags not stored");
}]);
cases.push(["device duplicate serial number", async () => {
  const rows = (await db.query(stock.SERIAL_DUPLICATE_SQL, [inserted.k.Primary_Key, " test-0001 "])).rows;
  if (rows.length !== 1) throw new Error("duplicate was not detected");
}]);
cases.push(["stock TOTALS after goods receipt", async () => {
  const s = (await db.query(stock.TOTALS_SQL, [inserted.k.Primary_Key, inserted.g.stock])).rows[0];
  if (Number(s.total) !== 1) throw new Error(`total ${s.total}`);
  // A device with inventory + rental counts in both usages but only once in the total.
  if (Number(s.inventory) !== 1) throw new Error(`inventory ${s.inventory}`);
  if (Number(s.rental) !== 1) throw new Error(`rental ${s.rental}`);
  if (Number(s.sale) !== 0) throw new Error(`sale ${s.sale}`);
}]);
cases.push(["stock recompute totals", async () => {
  await recomputeTotals(inserted.k.Primary_Key, inserted.g.stock);
  const rows = (await db.query(stock.STOCK_SQL, [inserted.k.Primary_Key])).rows;
  const row0 = rows.find((r) => r.stock_id === inserted.g.stock);
  if (!row0) throw new Error("warehouse missing");
  if (Number(row0.total) !== 1) throw new Error(`total ${row0.total}`);
  // Overlap: inventory and rental are 1 each, the total stays 1.
  if (Number(row0.inventory) !== 1 || Number(row0.rental) !== 1) {
    throw new Error(`inventory ${row0.inventory} / rental ${row0.rental}`);
  }
}]);
cases.push(["device DEVICES list", async () => {
  const rows = (await db.query(stock.DEVICES_SQL, [inserted.k.Primary_Key, "TEST-00"])).rows;
  if (rows.length !== 1) throw new Error(`search finds ${rows.length} devices`);
}]);
cases.push(["device UPDATE", async () => {
  const id = await firstStockId();
  const today = "2026-11-12";
  await db.query(stock.DEVICE_UPDATE_SQL, [
    inserted.k.Primary_Key, // $1
    inserted.g.id, // $2
    "TEST-0001", // $3 SerienNr
    "INV-TEST-1", // $4 InventarNr
    "INV-TEST-1b", // $5 InventarNr2
    id, // $6 Lager
    "Regal B2", // $7 Lagerplatz
    "gebraucht", // $8 Zustand
    true, // $9 Inventar
    true, // $10 Miete
    true, // $11 Verkauf
    false, // $12 Gebraucht
    false, // $13 Ausgesondert
    88.25, // $14 Kaufpreis
    today, // $15 purchase date
    10002, // $16 Lieferant
    "Haus", // $17 Eigentuemer
    12, // $18 Garantie
    "BC-123", // $19 Barcode
    2020, // $20 Baujahr
    "geändert", // $21 Bemerkung
    today, // $22 date
    "11:00:00", // $23 Zeit
    "tester", // $24 Benutzer
  ]);
  const s = (await db.query(stock.TOTALS_SQL, [inserted.k.Primary_Key, id])).rows[0];
  if (Number(s.inventory) !== 1 || Number(s.rental) !== 1 || Number(s.sale) !== 1) {
    throw new Error(
      `multiple usage not effective: inventory ${s.inventory} / rental ${s.rental} / sale ${s.sale}`,
    );
  }
  if (Number(s.total) !== 1) throw new Error(`total counts twice: ${s.total}`);
  const g = (await db.query(stock.DEVICE_SQL, [inserted.k.Primary_Key, inserted.g.id])).rows[0];
  if (g.bin_location !== "Regal B2") throw new Error("change not stored");
  if (!g.sale || g.used_sale) throw new Error("usage flags wrong after the change");
  if (Number(g.warranty_months) !== 12 || g.ext_barcode !== "BC-123") throw new Error("warranty/barcode not stored");
}]);
cases.push(["article LIST shows stock", async () => {
  await recomputeTotals(inserted.k.Primary_Key, inserted.g.stock);
  const rows = (await db.query(articles.LIST_SQL, [200, 0, null, null, null])).rows;
  const row0 = rows.find((r) => r.Primary_Key === inserted.k.Primary_Key);
  if (!row0) throw new Error("article missing from the list");
  if (Number(row0.stock_on_hand) !== 1) throw new Error(`stock ${row0.stock_on_hand}`);
  if (Number(row0.available_stock) !== 1) throw new Error(`available ${row0.available_stock}`);
  // Der Satz wurde zuvor auf Miete + Verkauf + Inventar umgestellt (siehe oben).
  if (Number(row0.rental_stock) !== 1) throw new Error(`rental stock ${row0.rental_stock}`);
  if (Number(row0.sale_stock) !== 1) throw new Error(`sale stock ${row0.sale_stock}`);
  if (Number(row0.rental_price) !== 48) throw new Error(`rental price ${row0.rental_price}`);
  if (Number(row0.sale_price) !== 3800) throw new Error(`sale price ${row0.sale_price}`);
}]);
cases.push(["device DELETE", async () => {
  await db.query(stock.DEVICE_DELETE_SQL, [inserted.k.Primary_Key, inserted.g.id]);
  await recomputeTotals(inserted.k.Primary_Key, inserted.g.stock);
  const s = (await db.query(stock.TOTALS_SQL, [inserted.k.Primary_Key, inserted.g.stock])).rows[0];
  if (Number(s.total) !== 0) throw new Error(`stock after removal ${s.total}`);
  const rows = (await db.query(stock.STOCK_SQL, [inserted.k.Primary_Key])).rows;
  const row0 = rows.find((r) => r.stock_id === inserted.g.stock);
  if (Number(row0.total) !== 0) throw new Error(`warehouse reports ${row0.total}`);
}]);

// -------------------------------------------------------------- Article search
// The form has 12 general and 27 pyro criteria; SEARCH_SQL binds them in a fixed
// order ($1..$43). The helper leaves all of them "not set" and overrides only
// what the respective case needs.
const sucheParams = (o = {}) => [
  o.limit ?? 100, o.offset ?? 0, o.withBundle ?? false,
  o.articleType ?? "active", o.name ?? null, o.includeDescriptions ?? false,
  o.articleNo ?? null, o.inventoryNo ?? null, o.serialNo ?? null,
  o.articleGroups ?? null, o.groupsOp ?? "contains",
  o.searchIndex ?? null, o.searchIndexOp ?? "contains",
  o.suppliers ?? null, o.manufacturer ?? null, o.owner ?? null,
  o.excludeVirtual ?? false,
  o.unNumber ?? null, o.hazardClass ?? null, o.ceNo ?? null,
  o.classification ?? null, o.kategorie ?? null, o.approvalNumber ?? null,
  o.packingGroup ?? null, o.tunnelCode ?? null, o.transportCategory ?? null,
  o.neqFrom ?? null, o.neqTo ?? null, o.unPoints ?? null,
  o.lateralFrom ?? null, o.lateralTo ?? null,
  o.verticalFrom ?? null, o.verticalTo ?? null,
  o.ascendFrom ?? null, o.ascendTo ?? null,
  o.soundFrom ?? null, o.soundTo ?? null,
  o.rangeFrom ?? null, o.rangeTo ?? null,
  o.delayFrom ?? null, o.delayTo ?? null,
  o.effectFrom ?? null, o.effectTo ?? null,
];

const sucheIds = async (o = {}) =>
  (await db.query(sucheRepo.SEARCH_SQL, sucheParams(o))).rows.map((r) => r.Primary_Key);

// Extra data for the search cases: article groups, search index, owner on the device,
// supplier name and a bundle partner of the test article. They are created in a
// separate case because the test article only exists after the cases before it.
const such = {};
cases.push(["search: create extra data", async () => {
  such.wgLicht = (await db.query(
    `INSERT INTO "Storage_Warengruppe" ("Index","Parent_ID","Child_ID") VALUES ('Licht',0,0) RETURNING "Primary_Key"`,
  )).rows[0].Primary_Key;
  such.wgTon = (await db.query(
    `INSERT INTO "Storage_Warengruppe" ("Index","Parent_ID","Child_ID") VALUES ('Ton',0,0) RETURNING "Primary_Key"`,
  )).rows[0].Primary_Key;
  await db.query(
    `INSERT INTO "Artikel_Warengruppe" ("Artikel_Primkey","Storage_Primkey","Verkaufsartikel") VALUES ($1,$2,false)`,
    [inserted.k.Primary_Key, such.wgLicht],
  );
  await db.query(
    `INSERT INTO "Artikel_Searchindex" ("Artikel_Key","Artikelindex") VALUES ($1,'Gobo')`,
    [inserted.k.Primary_Key],
  );
  await db.query(`UPDATE "Artikel" SET "Lieferant_o"='Lichtverleih Süd' WHERE "Primary_Key"=$1`, [
    inserted.k.Primary_Key,
  ]);
  such.bPartner = (await db.query(
    `INSERT INTO "Artikel" ("ArtikelNr","Name","Aktiv") VALUES (9999,'Bundlematerial XY',true) RETURNING "Primary_Key"`,
  )).rows[0].Primary_Key;
  await db.query(
    `INSERT INTO "Artikel_Bundle" ("MainArtikel_PrimKey","MainArtikel_ArtNr","Material_PrimKey","Material_ArtNr","Position","Material_Anzahl")
     VALUES ($1,$2,$3,9999,1,1)`,
    [inserted.k.Primary_Key, inserted.k.ArtikelNr, such.bPartner],
  );
}]);

cases.push(["search: article type active finds the article", async () => {
  const ids = await sucheIds({});
  if (!ids.includes(inserted.k.Primary_Key)) throw new Error("article missing for active");
  const inactive = await sucheIds({ articleType: "inactive" });
  if (inactive.includes(inserted.k.Primary_Key)) throw new Error("article counts as inactive");
}]);
cases.push(["search: name also hits short name/match code", async () => {
  const ids = await sucheIds({ name: "SW" });
  if (!ids.includes(inserted.k.Primary_Key)) throw new Error("short name not found");
}]);
cases.push(["search: incl. article descriptions", async () => {
  const ohne = await sucheIds({ name: "Langtext" });
  if (ohne.includes(inserted.k.Primary_Key)) throw new Error("text block found without the checkbox");
  const mit = await sucheIds({ name: "Langtext", includeDescriptions: true });
  if (!mit.includes(inserted.k.Primary_Key)) throw new Error("text block not found with the checkbox");
}]);
cases.push(["search: article no. and inventory no.", async () => {
  const perNr = await sucheIds({ articleNo: inserted.k.ArtikelNr });
  if (!perNr.includes(inserted.k.Primary_Key)) throw new Error("article number does not match");
  const falsch = await sucheIds({ articleNo: 999999 });
  if (falsch.length !== 0) throw new Error(`unknown number yields ${falsch.length} hits`);
  const perInv = await sucheIds({ inventoryNo: "INV" });
  if (!perInv.includes(inserted.k.Primary_Key)) throw new Error("inventory number does not match");
}]);
cases.push(["search: serial no. via the devices", async () => {
  await db.query(stock.GOODS_RECEIPT_SQL, [
    inserted.k.Primary_Key, "SUCH-1", false, 1, true, false, false, false,
    inserted.g.stock, "Regal A1", "new", 0, "2026-01-15", null, "EIG-77", "Test", "2026-01-15", "11:30:00", "tester",
  ]);
  const ids = await sucheIds({ serialNo: "SUCH" });
  if (!ids.includes(inserted.k.Primary_Key)) throw new Error("serial number does not match");
  const nichts = await sucheIds({ serialNo: "GIBT-ES-NICHT" });
  if (nichts.length !== 0) throw new Error("unknown serial number yields hits");
}]);
cases.push(["search: group contains / contains all / contains none", async () => {
  const eine = await sucheIds({ articleGroups: [such.wgLicht] });
  if (!eine.includes(inserted.k.Primary_Key)) throw new Error("article group not found");
  const beide = await sucheIds({ articleGroups: [such.wgLicht, such.wgTon] });
  if (!beide.includes(inserted.k.Primary_Key)) throw new Error("'contains' does not find the single group");
  const allHits = await sucheIds({ articleGroups: [such.wgLicht, such.wgTon], groupsOp: "contains_all" });
  if (allHits.includes(inserted.k.Primary_Key)) throw new Error("'contains all' ignores the second group");
  const nurEine = await sucheIds({ articleGroups: [such.wgLicht], groupsOp: "contains_all" });
  if (!nurEine.includes(inserted.k.Primary_Key)) throw new Error("'contains all' with one group is empty");
  const nicht = await sucheIds({ articleGroups: [such.wgLicht], groupsOp: "contains_none" });
  if (nicht.includes(inserted.k.Primary_Key)) throw new Error("'contains none' does not filter");
  // The later column check expects an article without article groups.
  await db.query(`DELETE FROM "Artikel_Warengruppe" WHERE "Artikel_Primkey"=$1`, [
    inserted.k.Primary_Key,
  ]);
}]);
cases.push(["search: search index", async () => {
  const ids = await sucheIds({ searchIndex: ["Gobo"] });
  if (!ids.includes(inserted.k.Primary_Key)) throw new Error("search index does not match");
  const nicht = await sucheIds({ searchIndex: ["Gobo"], searchIndexOp: "contains_none" });
  if (nicht.includes(inserted.k.Primary_Key)) throw new Error("search index 'contains none' does not filter");
}]);
cases.push(["search: supplier via Lieferant_o", async () => {
  const ids = await sucheIds({ suppliers: ["Lichtverleih"] });
  if (!ids.includes(inserted.k.Primary_Key)) throw new Error("supplier does not match");
}]);
cases.push(["search: manufacturer", async () => {
  await db.query(`UPDATE "Artikel" SET "Hersteller"='Philips' WHERE "Primary_Key"=$1`, [
    inserted.k.Primary_Key,
  ]);
  const ids = await sucheIds({ manufacturer: "Phil" });
  if (!ids.includes(inserted.k.Primary_Key)) throw new Error("manufacturer does not match");
  // The later column check expects the original manufacturer again.
  await db.query(`UPDATE "Artikel" SET "Hersteller"='ETC' WHERE "Primary_Key"=$1`, [
    inserted.k.Primary_Key,
  ]);
}]);
cases.push(["search: owner via the device", async () => {
  await db.query(`UPDATE "Seriennummern" SET "Eigentuemer"='Stadttheater' WHERE "Artikel_PrimKey"=$1`, [
    inserted.k.Primary_Key,
  ]);
  const ids = await sucheIds({ owner: "Stadttheater" });
  if (!ids.includes(inserted.k.Primary_Key)) throw new Error("owner does not match");
}]);
cases.push(["search: excl. virtual article", async () => {
  await db.query(`UPDATE "Artikel" SET "virtuellerArtikel"=true WHERE "Primary_Key"=$1`, [
    inserted.k.Primary_Key,
  ]);
  const mit = await sucheIds({ excludeVirtual: false });
  if (!mit.includes(inserted.k.Primary_Key)) throw new Error("article missing without the checkbox");
  if ((await sucheIds({ excludeVirtual: true })).includes(inserted.k.Primary_Key)) {
    throw new Error("virtual article not excluded");
  }
  await db.query(`UPDATE "Artikel" SET "virtuellerArtikel"=false WHERE "Primary_Key"=$1`, [
    inserted.k.Primary_Key,
  ]);
}]);
cases.push(["search: bundle material 'include related'", async () => {
  const ohne = await sucheIds({ name: "Bundlematerial XY" });
  if (!ohne.includes(such.bPartner)) throw new Error("material missing");
  if (ohne.includes(inserted.k.Primary_Key)) throw new Error("main article present without the checkbox");
  const mit = await sucheIds({ name: "Bundlematerial XY", withBundle: true });
  if (!mit.includes(inserted.k.Primary_Key)) throw new Error("main article missing with the checkbox");
}]);
cases.push(["search: pyro text and range", async () => {
  await db.query(
    `UPDATE "Artikel" SET "pyro_UN_number"='0336', "pyro_delay_time"=3, "pyro_neq"=1.5 WHERE "Primary_Key"=$1`,
    [inserted.k.Primary_Key],
  );
  if (!(await sucheIds({ unNumber: "0336" })).includes(inserted.k.Primary_Key)) {
    throw new Error("UN number does not match");
  }
  if (!(await sucheIds({ delayFrom: 2, delayTo: 4 })).includes(inserted.k.Primary_Key)) {
    throw new Error("delay time in range not found");
  }
  if ((await sucheIds({ delayTo: 2 })).includes(inserted.k.Primary_Key)) {
    throw new Error("range upper bound has no effect");
  }
  if (!(await sucheIds({ neqFrom: 1 })).includes(inserted.k.Primary_Key)) {
    throw new Error("NEQ lower bound has no effect");
  }
}]);
cases.push(["search: total comes from the query", async () => {
  const rows = (await db.query(sucheRepo.SEARCH_SQL, sucheParams({ limit: 1 }))).rows;
  if (rows.length !== 1) throw new Error(`limit returns ${rows.length} rows`);
  const allHits = (await db.query(sucheRepo.SEARCH_SQL, sucheParams({ limit: 500 }))).rows;
  if (Number(rows[0].total) !== allHits.length) {
    throw new Error(`total ${rows[0].total} does not match ${allHits.length} rows`);
  }
}]);
cases.push(["search: saved search create/overwrite/delete", async () => {
  const saved = (
    await db.query(sucheRepo.SEARCH_SAVE_SQL, ["Nur Licht", '{"name":"Licht"}'])
  ).rows[0];
  if (saved.name !== "Nur Licht") throw new Error("name not stored");
  const ersetzt = (
    await db.query(sucheRepo.SEARCH_SAVE_SQL, ["nur licht ", '{"name":"Licht","articleType":"all"}'])
  ).rows[0];
  if (ersetzt.id !== saved.id) throw new Error("same name creates a second entry");
  const liste = (await db.query(sucheRepo.SAVED_SEARCHES_SQL)).rows;
  if (!liste.some((r) => r.id === saved.id)) throw new Error("saved search missing from the list");
  await db.query(sucheRepo.SEARCH_DELETE_SQL, [saved.id]);
  const danach = (await db.query(sucheRepo.SAVED_SEARCHES_SQL)).rows;
  if (danach.some((r) => r.id === saved.id)) throw new Error("saved search not deleted");
}]);
cases.push(["search index: tree readable", async () => {
  await db.query(
    `INSERT INTO "Storage_ArtikelSearchIndex" ("Index","Parent_ID","Child_ID") VALUES ('Effekte',0,0)`,
  );
  const rows = (await db.query(sucheRepo.SEARCH_INDEX_SQL)).rows;
  if (!rows.some((r) => r.name === "Effekte")) throw new Error("node missing");
}]);

// --------------------------------------------------------------- Zentrale
// The start page reads aggregates. Each statement is executed as written in the
// repository, against the rows the cases above inserted.
const zentrale = sqlConstants("crates/where-persistence/src/dashboard_repo.rs");
/** Warehouse the dashboard stock cases fill, so the warehouse sums are not zero. */
let zentraleStockId = null;

cases.push(["dashboard: article counts", async () => {
  const row = (await db.query(zentrale.ARTICLES_SQL)).rows[0];
  if (Number(row.total) < 1) throw new Error("no articles counted");
  if (Number(row.active) < 1) throw new Error("active count is 0");
  if (Number(row.active) + Number(row.inactive) !== Number(row.total)) {
    throw new Error("active + inactive is not the total");
  }
  if (Number(row.created_last_30_days) < 1) throw new Error("new article not counted in the last 30 days");
}]);
cases.push(["dashboard: address counts", async () => {
  const row = (await db.query(zentrale.ADDRESSES_SQL)).rows[0];
  if (Number(row.total) < 1) throw new Error("no addresses counted");
  if (Number(row.customers) < 1) throw new Error("inserted customer not counted");
  if (Number(row.active) > Number(row.total)) throw new Error("more active addresses than addresses");
}]);
cases.push(["dashboard: stock sums", async () => {
  // The cases above delete their device again, so create one of our own —
  // a dashboard that only sums zeros would not prove anything.
  const stockId = await firstStockId();
  const today = "2026-11-11";
  await db.query(stock.GOODS_RECEIPT_SQL, [
    inserted.k.Primary_Key,
    "ZENTRALE-0001",
    false,
    2,
    false,
    true,
    false,
    false,
    stockId,
    "Regal B2",
    "gut",
    10,
    today,
    10001,
    null,
    "Zentrale",
    today,
    "08:00:00",
    "tester",
  ]);
  await recomputeTotals(inserted.k.Primary_Key, stockId);
  zentraleStockId = stockId;

  const row = (await db.query(zentrale.STOCK_SQL)).rows[0];
  if (Number(row.rental) !== 2) throw new Error(`rental ${row.rental}`);
  if (Number(row.sale) !== 0) throw new Error(`sale ${row.sale}`);
  if (Number(row.reserved) !== 0 || Number(row.withdrawn) !== 0) throw new Error("reservation/withdrawal not 0");
}]);
cases.push(["dashboard: device counts", async () => {
  const row = (await db.query(zentrale.DEVICES_SQL)).rows[0];
  if (Number(row.devices) < 1) throw new Error("device not counted");
  if (Number(row.discarded) !== 0) throw new Error(`discarded ${row.discarded}`);
}]);
cases.push(["dashboard: pyro register", async () => {
  await db.query(
    'UPDATE "Artikel" SET "pyro_UN_number" = \'0336\', "pyro_hazard_class_o" = \'1.4G\' WHERE "Primary_Key" = $1',
    [inserted.k.Primary_Key],
  );
  const row = (await db.query(zentrale.PYRO_SQL)).rows[0];
  if (Number(row.articles) < 1) throw new Error("pyro article not counted");
  if (Number(row.articles_with_hazard_class) < 1) throw new Error("hazard class not counted");
  const classes = (await db.query(zentrale.PYRO_CLASSES_SQL, [6])).rows;
  if (!classes.some((c) => c.label === "1.4G" && Number(c.count) >= 1)) {
    throw new Error("1.4G missing from the hazard class distribution");
  }
}]);
cases.push(["dashboard: warehouses", async () => {
  const rows = (await db.query(zentrale.WAREHOUSES_SQL)).rows;
  if (!rows.length) throw new Error("no warehouses listed");
  const withStock = rows.find((r) => r.id === zentraleStockId);
  if (!withStock) throw new Error("warehouse of the goods receipt missing");
  if (Number(withStock.rental) !== 2) throw new Error(`rental in warehouse ${withStock.rental}`);
  if (!withStock.name) throw new Error("warehouse without a display name");
}]);
cases.push(["dashboard: quality", async () => {
  const before = (await db.query(zentrale.QUALITY_SQL)).rows[0];
  for (const key of [
    "articles_without_group",
    "articles_without_rental_price",
    "articles_without_stock",
    "addresses_without_email",
    "addresses_without_number",
  ]) {
    if (Number(before[key]) < 0) throw new Error(`${key} is negative`);
  }

  // Filing the article into a group must lower exactly that counter by one.
  const groupId = (await db.query('SELECT min("Primary_Key") AS id FROM "Storage_Warengruppe"')).rows[0].id;
  await db.query(
    'INSERT INTO "Artikel_Warengruppe" ("Artikel_Primkey","Storage_Primkey","Verkaufsartikel") VALUES ($1,$2,false)',
    [inserted.k.Primary_Key, groupId],
  );
  const after = (await db.query(zentrale.QUALITY_SQL)).rows[0];
  await db.query('DELETE FROM "Artikel_Warengruppe" WHERE "Artikel_Primkey" = $1', [
    inserted.k.Primary_Key,
  ]);
  if (Number(after.articles_without_group) !== Number(before.articles_without_group) - 1) {
    throw new Error(
      `without group did not drop by one: ${before.articles_without_group} -> ${after.articles_without_group}`,
    );
  }
}]);
cases.push(["dashboard: article groups", async () => {
  const groupId = (await db.query('SELECT min("Primary_Key") AS id FROM "Storage_Warengruppe"')).rows[0].id;
  if (groupId === null || groupId === undefined) throw new Error("no article group — migration created none");
  await db.query(
    'INSERT INTO "Artikel_Warengruppe" ("Artikel_Primkey","Storage_Primkey","Verkaufsartikel") VALUES ($1,$2,false)',
    [inserted.k.Primary_Key, groupId],
  );
  const rows = (await db.query(zentrale.GROUPS_SQL, [8])).rows;
  const hit = rows.find((r) => r.id === groupId);
  if (!hit) throw new Error("group missing from the distribution");
  if (Number(hit.articles) < 1) throw new Error("group reports no articles");
  // The asset allocation checks below expect the article without groups again.
  await db.query('DELETE FROM "Artikel_Warengruppe" WHERE "Artikel_Primkey" = $1', [
    inserted.k.Primary_Key,
  ]);
}]);
cases.push(["dashboard: recent articles", async () => {
  const rows = (await db.query(zentrale.RECENT_ARTICLES_SQL, [5])).rows;
  const hit = rows.find((r) => r.id === inserted.k.Primary_Key);
  if (!hit) throw new Error("inserted article missing from the recent list");
  if (!hit.article_no) throw new Error("article number missing");
  if (rows.length > 5) throw new Error("more rows than the limit");
}]);
cases.push(["dashboard: recent addresses", async () => {
  const rows = (await db.query(zentrale.RECENT_ADDRESSES_SQL, [5])).rows;
  if (!rows.some((r) => r.id === inserted.a.Primary_key)) {
    throw new Error("inserted address missing from the recent list");
  }
  if (rows.length > 5) throw new Error("more rows than the limit");
}]);

cases.push(["article LIST group 0 = ohne Warengruppe", async () => {
  const ohne = () => db.query(articles.LIST_SQL, [50, 0, null, 0, null]);
  const alle = () => db.query(articles.LIST_SQL, [50, 0, null, null, null]);
  const countOhne = () => db.query(articles.COUNT_SQL, [null, 0, null]);

  // Without a group the article belongs to "ohne Warengruppe".
  const vorher = (await ohne()).rows;
  if (!vorher.some((r) => r.Primary_Key === inserted.k.Primary_Key)) {
    throw new Error("article without group missing from the group 0 list");
  }
  if (Number((await countOhne()).rows[0].count) !== vorher.length) {
    throw new Error("count and page disagree");
  }

  // Filed into a group, it leaves that list — but stays in the unfiltered one.
  const groupId = (await db.query('SELECT min("Primary_Key") AS id FROM "Storage_Warengruppe"')).rows[0].id;
  await db.query(
    'INSERT INTO "Artikel_Warengruppe" ("Artikel_Primkey","Storage_Primkey","Verkaufsartikel") VALUES ($1,$2,false)',
    [inserted.k.Primary_Key, groupId],
  );
  const danach = (await ohne()).rows;
  const unfiltered = (await alle()).rows;
  await db.query('DELETE FROM "Artikel_Warengruppe" WHERE "Artikel_Primkey" = $1', [
    inserted.k.Primary_Key,
  ]);
  if (danach.some((r) => r.Primary_Key === inserted.k.Primary_Key)) {
    throw new Error("grouped article still listed as without group");
  }
  if (!unfiltered.some((r) => r.Primary_Key === inserted.k.Primary_Key)) {
    throw new Error("grouped article vanished from the unfiltered list");
  }
}]);

// ----------------------------------------------------------------- Kalender
// The calendar stores one row per appointment, a rule row per series and one row
// per removed occurrence. The cases below write and read them exactly as the
// repository does.
const agenda = sqlConstants("crates/where-persistence/src/agenda_repo.rs");

const agendaInsert = (categoryId) => [
  "Aufbau Bühne 1", // $1 EventText
  "Theater Nordstadt", // $2 Event_HeadText
  "Tagesaufbau", // $3 Text
  "2026-11-10", // $4 StartDate
  "2026-11-11", // $5 EndDate
  "09:00", // $6 StartTime
  "17:00", // $7 EndTime
  0, // $8 is_Ganztagestermin
  1, // $9 AllUsers_Flag
  categoryId, // $10 Type
  "Premiere", // $11 Veranstaltung
  null, // $12 Adress_Primkey
  null, // $13 Adress_Name
  32, // $14 ColorRed
  120, // $15 ColorGreen
  200, // $16 ColorBlue
  true, // $17 CustomColor
  null, // $18 PrivateUser_Primkey
  true, // $19 Remind_Flag
  1762675200 - 900, // $20 Remind_TimeStamp (15 Minuten vorher)
  1762675200, // $21 Start_TimeStamp
  1762704000, // $22 End_TimeStamp
  "2026-11-01", // $23 sx_createDate
  "10:00", // $24 sx_createTime
  "tester", // $25 sx_createUser
];

const agendaRow = { id: null, rrule: null };

cases.push(["agenda: Kategorie", async () => {
  const row = (
    await db.query(
      'INSERT INTO "Storage_TACategory" ("Name") VALUES ($1) RETURNING "Primary_Key"',
      ["Produktion"],
    )
  ).rows[0];
  agendaRow.category = row.Primary_Key;
  const list = (await db.query(agenda.CATEGORIES_SQL)).rows;
  if (!list.some((c) => c.id === agendaRow.category)) throw new Error("category missing");
  if (Number(list.find((c) => c.id === agendaRow.category).appointments) !== 0) {
    throw new Error("category reports appointments before any was created");
  }
}]);
cases.push(["agenda: INSERT", async () => {
  agendaRow.id = (await db.query(agenda.INSERT_SQL, agendaInsert(agendaRow.category))).rows[0]
    .Primary_Key;
  if (!agendaRow.id) throw new Error("no key returned");
}]);
cases.push(["agenda: LIST shows the appointment", async () => {
  const rows = (
    await db.query(agenda.LIST_SQL, [
      "2026-11-01",
      "2026-11-30",
      null,
      null,
      null,
    ])
  ).rows;
  const hit = rows.find((r) => r.id === agendaRow.id);
  if (!hit) throw new Error("appointment missing in the range");
  if (hit.title !== "Aufbau Bühne 1") throw new Error(`title ${hit.title}`);
  if (hit.all_day) throw new Error("all-day flag set although it is a timed appointment");
  if (hit.category_name !== "Produktion") throw new Error(`category ${hit.category_name}`);
  // 09:00 start with a reminder 15 minutes earlier.
  if (Number(hit.remind_minutes) !== 15) throw new Error(`remind minutes ${hit.remind_minutes}`);
}]);
cases.push(["agenda: LIST filters", async () => {
  const outside = (await db.query(agenda.LIST_SQL, ["2026-12-01", "2026-12-31", null, null, null])).rows;
  if (outside.some((r) => r.id === agendaRow.id)) throw new Error("range filter ignored");
  const otherCategory = (await db.query(agenda.LIST_SQL, ["2026-11-01", "2026-11-30", 999999, null, null])).rows;
  if (otherCategory.some((r) => r.id === agendaRow.id)) throw new Error("category filter ignored");
  const otherUser = (await db.query(agenda.LIST_SQL, ["2026-11-01", "2026-11-30", null, 4711, null])).rows;
  if (!otherUser.some((r) => r.id === agendaRow.id)) throw new Error("public appointment hidden from a user");
  const bySearch = (await db.query(agenda.LIST_SQL, ["2026-11-01", "2026-11-30", null, null, "Premiere"])).rows;
  if (!bySearch.some((r) => r.id === agendaRow.id)) throw new Error("search did not find the venue");
  const miss = (await db.query(agenda.LIST_SQL, ["2026-11-01", "2026-11-30", null, null, "gibtsnicht"])).rows;
  if (miss.some((r) => r.id === agendaRow.id)) throw new Error("search matched a foreign word");
}]);
cases.push(["agenda: UPDATE", async () => {
  const params = agendaInsert(agendaRow.category);
  params[0] = "Abbau Bühne 1";
  params[7] = 1; // jetzt Ganztagstermin
  params[5] = null;
  params[6] = null;
  await db.query(agenda.UPDATE_SQL, [agendaRow.id, ...params.slice(0, 25)]);
  const row = (await db.query(agenda.GET_SQL, [agendaRow.id])).rows[0];
  if (row.title !== "Abbau Bühne 1") throw new Error(`title ${row.title}`);
  if (!row.all_day) throw new Error("all-day flag not stored");
  if (row.start_time !== null) throw new Error("time kept on an all-day appointment");
}]);
cases.push(["agenda: Serie und Ausnahme", async () => {
  const rrule = (
    await db.query(agenda.RRULE_INSERT_SQL, [
      agendaRow.id, // $1 FirstApp_PK
      2, // $2 rrule_type: wöchentlich
      1, // $3 interval
      1, // $4 rrule_End: bis Datum
      "2026-12-31", // $5 rrule_EndDate
      null, // $6 rrule_EndCount
      "2026-11-01", // $7 create/change date
      "10:00", // $8 create/change time
      "tester", // $9 user
    ])
  ).rows[0];
  agendaRow.rrule = rrule.Primary_Key;
  await db.query(agenda.SET_RRULE_SQL, [agendaRow.id, agendaRow.rrule]);

  const row = (await db.query(agenda.GET_SQL, [agendaRow.id])).rows[0];
  if (row.rrule_id !== agendaRow.rrule) throw new Error("series not linked");
  if (Number(row.rrule_type) !== 2 || Number(row.rrule_interval) !== 1) {
    throw new Error("rule not stored");
  }
  if (row.rrule_end_date === null) throw new Error("end date of the series lost");

  await db.query(agenda.EXCEPTION_INSERT_SQL, [agendaRow.rrule, "2026-11-17", 1763337600]);
  const exceptions = (await db.query(agenda.EXCEPTIONS_SQL, [[agendaRow.rrule]])).rows;
  const dates = exceptions.map((e) => new Date(e.date).toISOString().slice(0, 10));
  if (!dates.includes("2026-11-17")) throw new Error(`exception missing (${dates.join(", ")})`);
}]);
cases.push(["agenda: Adressname zur Adresse", async () => {
  const address = (await db.query('SELECT "Primary_key", "Name1" FROM "Adresse" LIMIT 1')).rows[0];
  if (!address) throw new Error("no address in the database");
  const found = (await db.query(agenda.ADDRESS_NAME_SQL, [address.Primary_key])).rows[0];
  if (found?.Name1 !== address.Name1) throw new Error("address name not resolved");
  const missing = (await db.query(agenda.ADDRESS_NAME_SQL, [999999])).rows;
  if (missing.length !== 0) throw new Error("unknown address returned a name");
}]);
cases.push(["agenda: Geburtstage aus dem Adressstamm", async () => {
  const id = inserted.a.Primary_key;
  await db.query(
    'UPDATE "Adresse" SET "Person_Geburtstag" = $2, "active" = true WHERE "Primary_key" = $1',
    [id, "1975-11-13"],
  );
  const rows = (await db.query(agenda.BIRTHDAYS_SQL)).rows;
  const hit = rows.find((r) => r.id === id);
  if (!hit) throw new Error("birthday of the address missing");
  const day = new Date(hit.birthday).toISOString().slice(0, 10);
  if (day !== "1975-11-13") throw new Error(`birthday ${day}`);
  await db.query('UPDATE "Adresse" SET "Person_Geburtstag" = NULL WHERE "Primary_key" = $1', [id]);
  const after = (await db.query(agenda.BIRTHDAYS_SQL)).rows;
  if (after.some((r) => r.id === id)) throw new Error("cleared birthday still listed");
}]);
cases.push(["agenda: Aufräumen", async () => {
  await db.query(agenda.DELETE_EXCEPTIONS_SQL, [agendaRow.rrule]);
  await db.query(agenda.RRULE_DELETE_SQL, [agendaRow.rrule]);
  const deleted = (await db.query(agenda.DELETE_SQL, [agendaRow.id])).rowCount;
  if (deleted !== 1) throw new Error("appointment not deleted");
  const rest = (await db.query(agenda.GET_SQL, [agendaRow.id])).rows;
  if (rest.length) throw new Error("appointment still readable");
  const rules = (await db.query("SELECT * FROM \"Scheduler_RRules\" WHERE \"Primary_Key\" = $1", [agendaRow.rrule])).rows;
  if (rules.length) throw new Error("rule left behind");
}]);

let passed = 0;
for (const [label, run] of cases) {
  if (await caseRun(label, run)) passed++;
}
console.log(`repository statements: ${passed}/${cases.length} passed`);

// ---------------------------------------------------------- Column assertions
const art = (await db.query(
  `select "ArtikelNr","Name","Import_kurzname","Matchcode","Import_Einheit",
          "Aktiv","Inventar_Flag","ZurMiete_Flag","ZumVerkauf_Flag","Textsync_Flag",
          "Bestandskontrolle","Dispokontrolle","EANCode","HerstellerArtNr","Hersteller",
          "Herstellerland","Import_VKPreisEmpfehlung","Zolltarifnummer_export",
          "Import_VersicherungswertNetto","TeamAgenda_Gruppe","Proviklasse",
          "Breite_Netto","Gewicht_Brutto","BetriebsspannungVon","BetriebsspannungBis",
          "Stromaufnahme","Leistung","Aufbauzeit","Abbauzeit","CheckOut_time","CheckIn_time",
          "VDE_Schutzklasse","VDECheck_Intervall","VDE_test_time",
          "WM_FlagEinsatztage","WM_maxEinsatztage","WM_FlagWartung",
          "WM_AnzahlTage","WM_AnzahlMonate","audit_created_at",
          "RolltagePreise_aktiv","OffDayPreise_aktiv","WEPreis_aktiv",
          "Verpackungseinheit","RabattMOFF","VersicherungMOFF","NurMaterial","Unikatartikel",
          "MieteForceSNScan","MieteWiredBundle","Sichtpruefung","Funktionspruefung",
          "Verbrauchserfassung","RabattVOFF","KolliVielfaches","KolliEinheit",
          "KolliGroesse","VerkaufMindBestand"
     from "Artikel" where "Primary_Key"=$1`,
  [inserted.k.Primary_Key],
)).rows[0];

const texte = (await db.query(
  `select "Beschreibung_short","Beschreibung_long","Beschreibung_tech" from "Artikel_Beschreibung" where "Artikel_Primkey"=$1`,
  [inserted.k.Primary_Key],
)).rows[0];

const groups = (await db.query(
  `select count(*)::int c from "Artikel_Warengruppe" where "Artikel_Primkey"=$1`,
  [inserted.k.Primary_Key],
)).rows[0].c;

const row = (await db.query(`select "Primary_key","KundenNr","Name1","import_Ort","import_Telefon","import_Email","GruppenNameAdresse",
       "active","adr_Einzelperson","Type_Kunde","Type_Interessent","Type_Lieferant","Type_VIP","Type_Location","Type_Personal",
       "lief_LieferantenNr","lief_Kreditor","KreditLimit","import_knd_RabattMiete","knd_IBAN","lief_BIC"
  from "Adresse" where "Primary_key"=$1`, [inserted.a.Primary_key])).rows[0];

const checks = [
  ["addresses: update persisted", row?.Name1 === "Muster" && row?.KundenNr === 1002],
  ["addresses: search column is import_Ort", row?.import_Ort === "Berlin"],
  ["addresses: phone column is import_Telefon", row?.import_Telefon === "030 654321"],
  ["addresses: e-mail column is import_Email", row?.import_Email === "neu@example.test"],
  ["addresses: group column is GruppenNameAdresse", row?.GruppenNameAdresse === "Veranstalter"],
  ["addresses: identity generated a key", inserted.a.Primary_key > 0],
  // Roles and the customer/supplier tabs of the address mask
  ["addresses: active", row?.active === true],
  ["addresses: Type_Kunde stays set", row?.Type_Kunde === true && inserted.a.Type_Kunde === true],
  ["addresses: Type_Lieferant", row?.Type_Lieferant === true],
  ["addresses: Type_Interessent", row?.Type_Interessent === false],
  ["addresses: Type_VIP", row?.Type_VIP === true],
  ["addresses: adr_Einzelperson", row?.adr_Einzelperson === false],
  ["addresses: lief_LieferantenNr", row?.lief_LieferantenNr === 2001],
  ["addresses: lief_Kreditor", row?.lief_Kreditor === "70001"],
  ["addresses: KreditLimit", Number(row?.KreditLimit) === 5000],
  ["addresses: import_knd_RabattMiete", Number(row?.import_knd_RabattMiete) === 5],
  ["addresses: knd_IBAN", row?.knd_IBAN === "DE89370400440532013000"],
  ["addresses: lief_BIC", row?.lief_BIC === "COBADEBBXXX"],
  // Article: every value must land in its own column
  ["articles: Name", art?.Name === "Scheinwerfer LED"],
  ["articles: Import_kurzname", art?.Import_kurzname === "SW-LED"],
  ["articles: Matchcode", art?.Matchcode === "SW750L"],
  ["articles: Import_Einheit", art?.Import_Einheit === "Stk"],
  ["articles: Kategorie-Flags", art?.Aktiv === true && art?.Inventar_Flag === false && art?.ZurMiete_Flag === true && art?.ZumVerkauf_Flag === false && art?.Textsync_Flag === true],
  ["articles: Bestandsführung", art?.Bestandskontrolle === true && art?.Dispokontrolle === true],
  ["articles: Herstellerangaben", art?.EANCode === "4001234567890" && art?.Hersteller === "ETC" && art?.Zolltarifnummer_export === "9405.4010"],
  ["articles: UVP in Import_VKPreisEmpfehlung", String(art?.Import_VKPreisEmpfehlung) === "1399.0000"],
  ["articles: Versicherung/TeamAgenda/Proviklasse", String(art?.Import_VersicherungswertNetto) === "1500.0000" && art?.TeamAgenda_Gruppe === "Licht-Team" && art?.Proviklasse === "PK1"],
  ["articles: Maße", String(art?.Breite_Netto) === "33.0000" && String(art?.Gewicht_Brutto) === "8.3000"],
  ["articles: Betriebswerte", String(art?.BetriebsspannungBis) === "240.0000" && art?.Aufbauzeit === 15 && art?.Abbauzeit === 11 && art?.CheckIn_time === 20],
  ["articles: DGUV", art?.VDE_Schutzklasse === 2 && art?.VDECheck_Intervall === 6 && art?.VDE_test_time === 31],
  ["articles: Werkstattintervall", art?.WM_FlagEinsatztage === true && art?.WM_AnzahlMonate === 7 && art?.WM_AnzahlTage === 4],
  ["articles: Miete-Preisvarianten", art?.RolltagePreise_aktiv === true && art?.OffDayPreise_aktiv === true && art?.WEPreis_aktiv === true],
  ["articles: Miete-Eigenschaften", art?.Verpackungseinheit === false && art?.RabattMOFF === true && art?.VersicherungMOFF === true && art?.NurMaterial === false && art?.Unikatartikel === false],
  ["articles: CheckOut/In-Eigenschaften", art?.MieteForceSNScan === false && art?.MieteWiredBundle === true && art?.Sichtpruefung === false && art?.Funktionspruefung === false && art?.Verbrauchserfassung === true],
  ["articles: Verkauf-Eigenschaften", art?.RabattVOFF === false && art?.KolliVielfaches === false && art?.KolliEinheit === "Kiste" && String(art?.KolliGroesse) === "6.0000" && String(art?.VerkaufMindBestand) === "10.0000"],
  ["articles: Audit gesetzt", art?.audit_created_at !== null],
  ["articles: Textblock kurz aktualisiert", texte?.Beschreibung_short === "Kurztext 2"],
  ["articles: Textblock lang/tech belegt", texte?.Beschreibung_long === "Langtext" && texte?.Beschreibung_tech === "Technik"],
  ["articles: Warengruppen ersetzt", groups === 0],
  ["datei: weich gelöscht, nicht mehr aktuell", (await db.query('select "deleted","is_current" from "article_file" where "id"=$1', [inserted.d.id])).rows[0].deleted === true],
];
for (const [label, ok] of checks) {
  if (!ok) failures++;
  console.log(`  ${ok ? "OK  " : "FAIL"} ${label}`);
}

await db.close();

console.log(failures ? `\n${failures} failure(s)` : "\nall checks passed");
process.exitCode = failures ? 1 : 0;
