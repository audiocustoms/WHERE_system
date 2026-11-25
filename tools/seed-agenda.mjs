// Fills the calendar with the appointments a theatre actually has, so the view can
// be judged with real data. Repeatable: appointments with the same title in the
// same calendar are skipped.
//
// Usage: node tools/seed-agenda.mjs [--api http://127.0.0.1:8090]
//                                   [--user admin2 --password start [--mandant 3] | --token <token>]

import { sessionHeader } from './session.mjs';

const apiIndex = process.argv.indexOf('--api');
const API = (
  (apiIndex > 0 ? process.argv[apiIndex + 1] : process.env.WHERE_API) ?? 'http://127.0.0.1:8090'
).replace(/\/+$/, '');

const session = await sessionHeader(API);

async function call(path, init = {}) {
  const response = await fetch(`${API}/api/v1${path}`, {
    ...init,
    headers: { 'content-type': 'application/json', ...session, ...(init.headers ?? {}) },
  });
  if (!response.ok) throw new Error(`${path} -> ${response.status} ${await response.text()}`);
  return response.status === 204 ? null : response.json();
}

const iso = (date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;

/** Monday of the week `offset` weeks from today. */
function weekStart(offset = 0) {
  const now = new Date();
  const weekday = (now.getDay() + 6) % 7;
  const monday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - weekday + offset * 7);
  return monday;
}

function plusDays(date, days) {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + days);
  return copy;
}

const CALENDARS = [
  { name: 'Produktion', color: [32, 102, 178] },
  { name: 'Transport', color: [204, 128, 24] },
  { name: 'Werkstatt', color: [45, 128, 78] },
  { name: 'Team', color: [112, 78, 158] },
];

const existing = await call('/agenda/categories');
const calendars = {};
let created = 0;
for (const calendar of CALENDARS) {
  let hit = existing.find((c) => c.name === calendar.name);
  if (!hit) {
    hit = await call('/agenda/categories', {
      method: 'POST',
      body: JSON.stringify({ name: calendar.name }),
    });
    created++;
  }
  calendars[calendar.name] = hit.id;
}

const week = weekStart(0);
const nextWeek = weekStart(1);

// Ein Termin mit Adresse zeigt, wo aufgebaut wird. Die Auswahl nimmt die erste
// passende Adresse und lässt das Feld leer, wenn keine vorhanden ist.
const addressPage = await call('/addresses?limit=100');
const addresses = addressPage.items ?? [];
const addressId = (needle) =>
  addresses.find((a) => (a.name1 ?? '').toLowerCase().includes(needle))?.id ?? null;

const APPOINTMENTS = [
  {
    title: 'Aufbau Bühne 1',
    head_text: 'Theater Nordstadt',
    venue: 'Premiere „Der Sturm"',
    calendar: 'Produktion',
    address_id: addressId('schauspielhaus'),
    start: plusDays(week, 0),
    end: addDaysTo(plusDays(week, 0), 1),
    start_time: '08:00',
    end_time: '18:00',
  },
  {
    title: 'Sicherheitsprüfung Licht',
    calendar: 'Werkstatt',
    start: plusDays(week, 2),
    start_time: '09:00',
    end_time: '12:00',
    remind: true,
    remind_minutes: 60,
  },
  {
    title: 'Wöchentliche Teamrunde',
    calendar: 'Team',
    start: plusDays(week, 4),
    start_time: '10:00',
    end_time: '11:00',
    recurrence: { frequency: 'weekly', interval: 1, until: null, count: 8 },
  },
  {
    title: 'Transport Fachtagung',
    head_text: 'Kongresshaus',
    calendar: 'Transport',
    address_id: addressId('kulturzentrum'),
    start: plusDays(nextWeek, 1),
    start_time: '06:30',
    end_time: '14:00',
    venue: 'Fachtagung Veranstaltungstechnik',
  },
  {
    title: 'Betriebsversammlung',
    calendar: 'Team',
    start: plusDays(nextWeek, 3),
    all_day: true,
    notes: 'Alle Abteilungen, Saal 2',
  },
  {
    title: 'Inventur Lager 2',
    calendar: 'Produktion',
    start: plusDays(nextWeek, 5),
    end: addDaysTo(plusDays(nextWeek, 5), 2),
    all_day: true,
  },
];

function addDaysTo(date, days) {
  return plusDays(date, days);
}

const existingRange = await call(
  `/agenda?from=${iso(plusDays(week, -7))}&to=${iso(plusDays(nextWeek, 14))}`,
);
const known = new Set(existingRange.occurrences.map((o) => `${o.event_id}|${o.title}`));

let events = 0;
for (const entry of APPOINTMENTS) {
  const already = existingRange.occurrences.some((o) => o.title === entry.title);
  if (already) continue;
  await call('/agenda', {
    method: 'POST',
    body: JSON.stringify({
      title: entry.title,
      head_text: entry.head_text ?? null,
      notes: entry.notes ?? null,
      venue: entry.venue ?? null,
      address_id: entry.address_id ?? null,
      start_date: iso(entry.start),
      end_date: iso(entry.end ?? entry.start),
      start_time: entry.start_time ?? null,
      end_time: entry.end_time ?? null,
      all_day: entry.all_day ?? false,
      category: calendars[entry.calendar] ?? null,
      color_red: null,
      color_green: null,
      color_blue: null,
      custom_color: false,
      private_user: null,
      remind: entry.remind ?? false,
      remind_minutes: entry.remind_minutes ?? null,
      recurrence: entry.recurrence ?? null,
    }),
  });
  events++;
}

console.log(
  `agenda: ${created} Kalender angelegt, ${events} Termine angelegt, ${known.size} bereits vorhanden.`,
);
