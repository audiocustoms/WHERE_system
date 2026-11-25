// Checks the settings lists of the admin menu against a running instance: every
// list must answer, and its table and columns must exist. This is the live
// counterpart of `verify-schema.mjs`, which only knows the SQL constants.
//
// Usage: node tools/check-settings.mjs [--api http://127.0.0.1:8090]

const apiIndex = process.argv.indexOf('--api');
const API = apiIndex > 0 ? process.argv[apiIndex + 1] : 'http://127.0.0.1:8090';

const response = await fetch(`${API}/api/v1/settings`);
if (!response.ok) {
  console.error(`GET /api/v1/settings -> ${response.status}`);
  process.exit(1);
}
const areas = await response.json();

let failures = 0;
for (const area of areas) {
  const list = await fetch(`${API}/api/v1/settings/${area.key}`);
  if (!list.ok) {
    console.error(`! ${area.key}: ${list.status} ${(await list.text()).slice(0, 200)}`);
    failures++;
    continue;
  }
  const body = await list.json();
  const fehlend = area.fields.filter((field) => !(field.column in (body.rows[0]?.values ?? {})));
  if (body.rows.length > 0 && fehlend.length > 0) {
    console.error(`! ${area.key}: Spalten fehlen in der Antwort: ${fehlend.map((f) => f.column).join(', ')}`);
    failures++;
    continue;
  }
  console.log(`  OK   ${area.key.padEnd(20)} ${String(body.rows.length).padStart(3)} Zeilen  ${area.table}`);
}

console.log(
  failures === 0
    ? `\nall ${areas.length} settings lists answered`
    : `\n${failures} of ${areas.length} settings lists failed`,
);
process.exit(failures === 0 ? 0 : 1);
