# WHERE - are my people?

Module for human resources: the employee master and the personnel areas of the
TeamAgenda.

## Installation

```powershell
cargo run -p where-server --bin where-installer -- --module people
```

The module shares the database and the tenants with `WHERE - is my stuff?` as
soon as the core application is installed. If it is installed first, it creates
the database; the core application takes it over later.

### Standing alone as a desktop application

The module also ships as a separate desktop application that installs and serves
nothing but itself:

```powershell
cd apps/desktop
npm run installer:people     # WHERE - are my people?, identifier de.where.are-my-people
```

The bundle identifier selects the product, so the same code base starts with the
`people` module registered instead of the core. Data directory, embedded database and
the API ports are shared with `WHERE - is my stuff?` — on a machine where the core is
installed later, both applications work on the same database and the same tenants
(see the main [README](../../README.md#two-desktop-applications-one-database)).

An installation without the core application starts on the **duty plan**
(`/people/schedule`) — the working page of the product; an account that may not read it
lands on the employee list instead.

The local preview ([`tools/preview.mjs`](../../tools/preview.mjs), `Vorschau.cmd`)
registers the module by itself before it starts the API, so nothing has to be
installed by hand there. `--no-modules` skips that step, `--modules go,people`
registers a different set.

## Contents

| Path | Contents |
| --- | --- |
| `module.toml` | Manifest (id, product name, capabilities) |
| `web/menu.ts` | `Personal`, `Abteilungen und Positionen` and `Dienstplan` in Stammdaten, `WHERE - are my people?` under Admin → Module, tiles `Dienstplan` and `Personal` (in that order) |
| `web/EmployeesPage.svelte` | Employee master (`/people`) |
| `web/EmployeeForm.svelte` | Input form (tabs `Person`, `Beschäftigung`, `Qualifikationen`) |
| `web/PeopleListsPage.svelte` | Master-data form (`/people/lists`) |
| `web/PeopleListPanel.svelte` | one of the four lists (create, rename, remove, colour; a location picks its sub-department, a sub-department its department) |
| `web/PlanningPage.svelte` | Settings (`/people/planning`): shifts and rules |
| `web/ModuleSettingsPage.svelte` | Settings of the product (`/admin/modules/people`): the resets of a month |
| `web/ResetMonthDialog.svelte` | Asks for the month a reset applies to |
| `web/DutyPlanPage.svelte` | Duty plan (`/people/schedule`): board of employees × days |
| `web/DutyPrintDialog.svelte` | Printing the plan: period, paper, preview, pages of one week |
| `web/DutyHideLocationsDialog.svelte` | `Ausblenden`: which locations the board shows |
| `web/DutyExchangeDialog.svelte` | `Import / Export` of a month: the sheet, the disposition, the plan and the PeM export |
| `web/DutyLocationFlags.svelte` | The flags of a location: the crew this house needs for each |
| `web/roster-report.ts` | The month as the windows read it (`ladeMonat`, `baueBericht`) |
| `web/roster-export.ts` | The printed sheet, the exchange tables and the three file formats |
| `web/roster-exchange.ts` | The two exchange sheets: disposition and plan of a month |
| `web/people-window.css` | Window frame of the form (reuses `lib/search/search-window.css`) |
| `TODO.md` | Open work: locking the plan after print/export, import manager for dates |
| `crates/where-module-people` | Backend: `/api/v1/people*`, tables `where_employees`, `where_employee_qualifications`, `where_people_departments`, `where_people_positions`, `where_people_shifts`, `where_people_planning`, `where_people_duty_entries` |

## Code language

Only what the user reads is German — labels, buttons, messages. Identifiers, comments,
CSS class names, migration comments, API paths and this documentation are English. What
stays German on purpose: PeM table and column names, the class names of the design system
(`sw-*`), the sheet headers of the exchange (`Schicht`, `Urlaub`, `Art`) and the words the
master-data import accepts as band (`früh`, `spät`, `zwischen`, `frei`).

The backend (`crates/where-module-people`) is converted; `ShiftBand` is
`Early | Late | Intermediate | Free` and stores `early | late | intermediate | free`
(`migrations/1950_people_shift_band_english.sql`). In the web part the board
(`DutyPlanPage.svelte`) and the dialogs `DutyJumpDialog`, `DutyDepartmentDialog`,
`DutyHideLocationsDialog`, `DutyLocationDialog`, `DutyLocationFlags`, `DutySeriesDialog`,
`DutyFreeTimeDialog`, `DutyDispoDayDialog`, `DutyGenerateDialog`, `ResetMonthDialog`,
`ModuleSettingsPage`, `PeopleListsPage`, `PeopleListPanel`, `EmployeesPage` and
`EmployeeForm` are converted. Still to convert: `PlanningPage`, `DutyPrintDialog`,
`DutyExchangeDialog`, `DutyDemandDialog` and the export chain they share
(`roster-export.ts`, `roster-report.ts`, `roster-exchange.ts`,
`master-data-exchange.ts`, `MasterExchangeDialog.svelte`) — and after that the core under
`apps/web/src`. The session log holds the detailed plan for that rest: order of work, the
concrete rename map per file, the rename helper and the before/after check of print preview,
CSV, XLSX and PDF (section *Next steps — the German cleanup, continued*).

## Form

The list (`/people`) is built like the article list: filters on the left
(active/inactive, departments), the list with quick search and `Mitarbeiter +` on
the right; it shows the personnel no., name (without salutation, first name only
as an initial), department, sub-department, position and the remaining vacation days. The list
opens **sorted by personnel number, ascending**; a click on a column header sorts
ascending or descending (empty values come last, numbers are compared
numerically). A click on a row opens the form, which offers deletion in the
footer. The quick search (`GET /api/v1/people?q=`) looks through first and last
name, personnel number, department, sub-department and position.

The form writes person, address, employment and qualifications
(`migrations/1891_people_details.sql`). Under `Beschäftigung` it holds, next to
entry/exit, employment type and weekly hours, the **working days per week** (1–7,
free numeric input, `migrations/1895_people_work_days.sql`,
`1896_people_work_days_range.sql`) and the **vacation days** (0–366,
`migrations/1897_people_vacation_days.sql`), which appear in the list as the column
`Resturlaub`; without an entry the respective field stays empty. `Abteilung`,
`Unterabteilung`, `Position` and **`Standort`** are selects over the maintained master
data (`where_employees."subdepartment"`, `migrations/1942_people_subdepartments.sql`,
and `where_employees."location"`, `migrations/1940_people_employee_location.sql`): the
location is the house of the disposition (`Großes Haus`, `Podium`, …) the employee
belongs to, and the sub-department is the department's subdivision
(`Beleuchtung → Podium`) — its select only offers the sub-departments of the department
chosen beside it; a value that is no longer in the master data stays selectable, and a
renamed location follows into the records. Above the
`Status` switch sits **`Mitarbeiterart` / `Auszubildende/r`**
(`is_apprentice`, off by default, `migrations/1915_people_apprentice_rule.sql`);
it is what the apprentice rule of the duty plan looks at. Underneath `aktiv` sits
the row `Disposition` with **`nicht automatisch disponierbar`** (`auto_plan`, on by
default = the person may be planned automatically,
`migrations/1933_people_auto_plan.sql`): when it is ticked, the generator of the
duty plan skips the employee. Planning by hand on the board is unaffected, and a
cell set by hand keeps counting as coverage. Birth date, place
of birth, address, social security and tax number belong to the right
`personal.persoenliche-daten-sehen`: without this right the API neither returns nor
stores them, and the form locks the fields.

Validation in the backend (`crates/where-module-people/src/employee.rs`): first and
last name are mandatory, the e-mail needs an `@`, the exit date may not lie before
the entry date, weekly hours may not be negative, working days per week lie
between 1 and 7, vacation days between 0 and 366, the birth date must not lie in
the future. The personnel number is unique — a duplicate answers with `409`.

## Personnel number

The personnel number runs through the number series (`where_number_series`, key
`employee`, `migrations/1894_people_number_series.sql`): it starts at **1001**, step
1, yearly without reset. It therefore also appears under `Admin →
Nummernkreisläufe`. The form fetches the suggestion via `GET /api/v1/people/new`
and puts it into the `Pers.-Nr.` field; if the field stays empty, the API assigns
the number on creation (`POST /api/v1/people`) in the same transaction. A number
entered by hand raises the counter (`P-1001` counts as 1001) so that it is not
offered again.

## Employee and user account

An employee can be the person behind an account of the user administration:
`where_employees."user_id"` (`migrations/1947_people_employee_user.sql`) names the
account, one account belongs to one employee (unique index). The reference is loose on
purpose — the accounts live in the installation database, the employees in the database
of the tenant — so nothing is cascaded.

- `Admin → Benutzerverwaltung` offers the row `Mitarbeiterliste` below `Anmeldename`
  when the personnel module is installed. Picking an employee fills the mask: the login
  name as *first letter of the first name, dot, last name* (`m.apolloni`), the full name,
  the user short name from the initials (`MA`), the function (the employee's position),
  phone, mobile and e-mail. Saving writes the reference (`PUT /api/v1/people/{id}/user`
  with `{ "user_id": … }`), deleting the account takes it away again; a second employee
  that names the same account takes it over from the first.
- The employee list marks such a name with a green check (`✓` behind the name), and the
  mask of the account shows `Benutzer nicht mit Personal verlinkt` unticked.

## Departments, sub-departments, positions and locations

`Stammdaten → Personal → Abteilungen und Positionen` (`/people/lists`)
maintains the suggestion lists of the form and the places of the disposition in
four panels — `Abteilungen`, `Unterabteilungen`, `Standorte`, `Positionen`
(tables `migrations/1892_people_lists.sql`, `migrations/1922_people_dispo.sql` and
`migrations/1942_people_subdepartments.sql`). The API lives under
`/api/v1/people/lists/{departments|subdepartments|positions|locations}`: reading with
`personal.personal-lesen` **or** `personaldispo.personaldispo-lesen`, creating with
`Personaldispo anlegen`, renaming, colouring, sorting, re-parenting and deleting with
`Personaldispo editieren` — the administration (`administrative-einstellungen.zugriff`)
may always do all of it (`where-api/src/authz.rs`, `Guard::Any`). Like every working
page it hangs on those rights alone — its menu entry appears with either read right, a
group that may only read sees the panels without buttons and without `Import / Export`.
A name may occur
only once per list — regardless of upper and lower
case (`409`); the name is trimmed and may have at most 100 characters
(`422`). The `Abteilung`, `Unterabteilung`, `Position` and `Standort` fields of the
personnel form are **selects** over the maintained values, in the maintained order, with
`—` for "none"; the browser's suggestion list would only show what matches the text
already in the field. A value that is not in the list (an older record, or an entry
deleted from the master data) stays selectable as its own option, so opening and
saving the form never drops it. The page uses the whole window (`class="breit"`, like the
board) and lays the four panels out as cards that carry as many columns as the window has
room for — four of them on a normal desktop.

**A sub-department belongs to a department** (`where_people_subdepartments`,
`migrations/1942_people_subdepartments.sql`): `Beleuchtung → Podium`, `Beleuchtung →
Großes Haus`. Each row of the `Unterabteilungen` panel carries a small selector over the
maintained departments (`— keine —` or one of them), written through
`PUT /api/v1/people/lists/subdepartments/{id}/department` with
`{ "department": "Beleuchtung" }` (an empty string clears it, the administrative right
applies). The `Unterabteilung` field of the personnel form only offers the
sub-departments of the department chosen beside it (`—` and the whole list while no
department is chosen), so a person stays in one department no matter which of the two
fields is filled. The generator counts a person for a house that is bound to a
sub-department only when **their sub-department** is that name — the department above
it never opens a house.

The order is freely selectable (`migrations/1893_people_lists_order.sql`): the
arrows ↑ / ↓ of the form move an entry and write the new sequence via
`PUT /api/v1/people/lists/{kind}/order` with `{ "ids": [...] }` (top to bottom).
New entries are appended at the end; the name is only the secondary sort key.

A **rename** propagates into the employees: the API applies the new name in one
operation for all records that carry the old label (ignoring upper and lower case)
in `Abteilung`, `Unterabteilung` or `Position`; a renamed department also renames
its sub-departments. A deletion leaves the employees unchanged — the label stays
there.

**Every entry carries a colour** (`where_people_departments."color"`,
`migrations/1938_people_department_color.sql`; `where_people_locations."color"`,
`migrations/1941_people_location_color.sql`; sub-departments and positions,
`migrations/1943_people_list_colors.sql`): the colour field of the row writes
`PUT /api/v1/people/lists/{kind}/{id}/color` with `{ "color": "#2563EB" }`
(administrative right, upper case, `422` for anything but `#RRGGBB`); an empty string
removes it again. Without a colour the field shows a white square with a stroke through
it, and a click opens the picker. The board tints
the name column of every employee of that sub-department — of their department when the
sub-department has no colour — and the name cell of the location's disposition row — see
[Duty plan (board)](#duty-plan-board); the colours of a position are master data without
a consumer yet. An employee without a colour on either level, a row without a colour, or
a value that is no longer in the master data stays untinted.

**A location carries its settings**: a click on the name of a location row of the
duty board — or on `⚙` of a row the board typed itself (see
[Duty plan (board)](#duty-plan-board)) — opens `web/DutyLocationDialog.svelte` with
`Einstellungen`: the switch
**`Meister benötigt`** (`where_people_locations."needs_foreman"`,
`migrations/1934_people_location_foreman.sql`, default on) and the **flags of that
house** (`web/DutyLocationFlags.svelte`, see [Flags](#flags)). Cleared, the house is
run without a foreman: neither the demand derived from the disposition nor the
generator plans the position `Meister` there, and a hand-entered demand row for it
is dropped without becoming a gap. Written through
`PUT /api/v1/people/lists/locations/{id}/foreman` with `{ "needs_foreman": false }`
(administrative right); `GET …/lists/locations` returns the value per location, and
the settings are read-only for everybody without that right.

A **location** names the sub-department that works there
(`where_people_locations."department"`, `migrations/1929_people_demand_location.sql`):
each row of the `Standorte` panel carries a small selector
(`— alle Unterabteilungen —` or one of the maintained sub-departments), written through
`PUT /api/v1/people/lists/{kind}/{id}/department` with `{ "department": "Podium" }`
(an empty string clears it, the administrative right applies — the same route binds a
sub-department to its department). The generator only plans employees **of that
sub-department** into the demand rows of the house; the department above it does not
count, so somebody without a sub-department is planned by hand only. Without a binding
every employee may work there. The binding is a name, like `Abteilung` on the employee —
a renamed sub-department does not follow, so the selector is where it is corrected. A
location bound before sub-departments existed keeps its department name; the selector
offers it as its own option until it is replaced.

**Locations** (`where_people_locations`) are the master data of the board's
disposition rows: `Großes Haus`, `Podium`, `Podium.bar`, `Oberes Foyer`, … The PeM
import creates them by name (a name that already exists — ignoring case — is
reused), and they can be renamed, reordered and deleted here like the other three
lists. PeM appends the house and the city to every location
(`Großes Haus, Theater Ulm, Ulm`); the import drops that tail, cut back to a comma,
as soon as at least two locations carry it — a single location keeps its whole
name, because its tail would be its name. Renaming a location renames the board
row and keeps its entries: the entries hang on the id, not on the text. A deleted
location takes its disposition entries with it (foreign key with `ON DELETE
CASCADE`).

## Who sees which department

A permission group can be limited to departments in
`Admin → Benutzerverwaltung → Einstellungen: Gruppenrechte` — **separately for
`Personal` and for `Personal-Disposition`**: each area has its own block `Abteilungen`
with `+ Abteilung hinzufügen`, offering `alle` and the departments of `Abteilungen und
Positionen`. The two lists are kept apart in `where_user_group_scope` with the kinds
`personal-department` and `personaldispo-department` and the key of
`where_people_departments` (`crates/where-persistence/src/user_repo.rs`), travel in
`UserGroup.departments` and `UserGroup.dispo_departments` and are united over the groups
of an account — a group without a department entry does not widen the scope of another
one. No entry means no restriction for that area; the administration (`Admin` right) is
never restricted.

The middleware resolves the keys once per request against the departments of the
tenant and hands them to the module (`where_api::authz::DepartmentScope`,
`crates/where-module-people/src/scope.rs`), so a changed group takes effect without a
new sign-in. Inside the module:

- **`Personal`** — `GET /api/v1/people` lists the staff of this area and of the
  disposition, `GET`, `PUT` and `DELETE /api/v1/people/{id}` answer `403` outside of
  them, and `POST`/`PUT` refuse a department that is not in this scope. The personnel
  file (`…/masterdata/employees`) follows the same list.
- **`Personal-Disposition`** — the board (`GET …/people/duty`) shows only the cells of
  this scope; `PUT …/duty`, `…/duty/series` refuse another employee, the audit checks
  only their cells, and `POST …/duty/generate` plans only them — including `overwrite`,
  which clears only their planned shifts. `POST …/duty/reset` clears only their month,
  and the plan file (`…/duty/exchange/plan`) is refused for a restricted disposition.
  The **shift catalogue** follows the same list: `GET …/people/shifts` leaves out every
  shift whose department is outside this scope, naming such a shift in `PUT …/duty` or a
  series answers `403`, and the generator plans only the shifts it was handed. The
  catalogue itself is maintained along the same line: `POST …/people/shifts` only takes a
  department of this scope, `PUT` and `DELETE` refuse a shift outside of it (`403`), and
  the order of the shifts and the rules of the planning stay with the administration.
  The **locations of the board** are scoped as well: a location belongs to the department
  of its sub-department, and `GET …/people/duty/locations` returns only those of this
  scope (a location without a sub-department belongs to nobody); `GET …/people/duty/dispo`
  and `…/duty/dispo-demand` drop the events of the other houses.
  A department of `Personal` never opens the board and the other way round.
- Master data (departments, positions, locations, flags, planning) and the disposition
  (`…/duty/dispo*`, which names houses and events, not people) stay reachable — they are
  the tool of the disposition. The shifts and the rows of the board are the exception:
  they follow the departments of this scope (see above).

An unknown key silently falls out of the scope, and an employee without a department
belongs to no scope at all: such a person is planned and maintained by hand only, which
is the same rule the generator follows for a house (see below).

## Flags

The flags are the short codes PeM writes at the end of an event text —
`Max und Moritz Vst`, `Cosi fan tutte BO 1`, `Magic TE`. They are maintained in the
**`Einstellungen` window of a location**: a click on the name of a location row of
the board opens it (table `where_people_flags`, `migrations/1923_people_flags.sql`) —
a code (1–20 characters, unique ignoring case), the name behind it (`Vst` →
`Vorstellung`) and the crew the flag needs **in this house**.

- **The staffing belongs to a location** (`where_people_flag_staffing`,
  `migrations/1937_people_flag_location.sql`): one row per position and house, so a
  `Vst` in the Großes Haus asks for another crew than a `Vst` in the Podium. `*`
  means "any position" and reads `nicht relevant` in the mask. `Vorlagen` creates the
  flags of the PeM legend with their numbers for every location, `Neu` and `Löschen`
  maintain the code and the name (both count for all houses), and each row saves as
  soon as it is changed (`PUT /api/v1/people/flags/{id}/staffing/{location}` with
  `{ "entries": [{ "position": "Meister", "needed": 1 }] }`). A position appears once
  per flag and house (the list of that house is replaced), at most 50 rows of 1–99
  employees each; an empty list is a flag that asks for nobody in that house.
- The migration copied the rows that were global until then to every location, so
  nothing changed until a house was given its own numbers. A location that exists
  later starts without rows.
- **A house without rows for a flag** shows up in the demand of the day as
  `Flag ohne Besetzung` (`no_staffing`), so an empty house is named instead of being
  silent.
- **The legend of the theatre** lives in code (`crates/where-module-people/src/
  flags.rs`, `PRESETS`): `Vst` Vorstellung, `Probe` Probe, `TE` Technische
  Einrichtung, `BO` Bühnenorchesterprobe, `BEL` Beleuchtungsprobe, `Belkorr.`
  Beleuchtungskorrekturen, `HP1`/`HP2` Hauptprobe 1/2, `GP` Generalprobe,
  `Premiere` Premierenvorstellung, `Dern` Dernière — each with its default
  staffing. `POST /api/v1/people/flags/presets` creates the ones that are missing —
  with their rows in every location — and leaves the existing ones alone.
- **The PeM import recognises the legend before the flags exist**: the scan reads the
  event texts against the codes of the master data *and* the legend
  (`flags::vocabulary`), and the import creates the codes its disposition uses
  (`flags::preset` matches ignoring case and the trailing dot of `Belkorr.`). A flag of
  its own that the legend does not know is recognised once it stands in the master
  data.
- API: `GET/POST /api/v1/people/flags`, `PUT/DELETE /api/v1/people/flags/{id}`,
  `PUT …/{id}/staffing/{location}`, `PUT …/{id}/exclusions`, `POST …/presets`. Reading
  with `personal.personal-lesen`, everything else only with the administrative right —
  like the lists they belong to. The exclusions (`…/{id}/exclusions`) stay in the
  model and the API but have no mask any more: the staffing of a house says what it
  needs.

## Duty-plan settings

`Stammdaten → Personal → Dienstplan` (`/people/planning`) collects everything
that the generation of duty plans needs (`migrations/1898_people_shifts.sql`).
Reading takes `personaldispo.personaldispo-lesen`, a new shift
`personaldispo.personaldispo-anlegen`, changing one `personaldispo.personaldispo-editieren`
(the administration always). A dispatcher only reaches the shifts of the departments his
group carries in `Personal-Disposition`: `POST`, `PUT` and `DELETE` answer `403` for a
shift of a foreign department and for a shift without one, which belongs to every
dispatcher. The order of the shifts (`PUT …/shifts/order`) and the rules of the planning
(`PUT /people/planning`) stay with the administration.

**Shifts** (`where_people_shifts`, `/api/v1/people/shifts`): any number of
entries, each with

| Field | Meaning |
| --- | --- |
| `name` | Label, unique per installation (case-insensitive, `409`) |
| `short_name` | Short code for the plan (at most 10 characters) |
| `starts_at`, `ends_at` | Start and end in 24-hour format as `HH:MM`; if the end lies before the start, the shift runs past midnight |
| `band` | Half of the day the shift belongs to: `frueh`, `spaet`, `zwischen` or `frei` (`migrations/1926_people_shift_bands.sql`, `1930_people_shift_free_band.sql`) |
| `department` | Department the shift belongs to, stored by name; empty means every dispatcher may plan it (`migrations/1946_people_shift_department.sql`) |
| `apprentice_only` | Only apprentices may be planned into this shift |
| `break_minutes` | Break within the shift (0–720, shorter than the shift) |
| `color` | Color in the plan as `#RRGGBB`, chosen with the colour picker of the app (palette of 32 or wheel with HEX/RGB/HSL) |
| `active` | Inactive shifts are not planned by the planner |
| `sort_order` | Order, maintained via ↑ / ↓ (`PUT …/shifts/order`) |

The form accepts times in **24-hour format**. When leaving the field, the input is
normalized to `HH:MM` (`6` → `06:00`, `630` → `06:30`, `6.30` → `06:30`); anything
that cannot be parsed is reported by the form before saving, and the API checks the
format once more.

A minimum staffing deliberately does **not** belong to the shift — it is maintained
elsewhere (`migrations/1901_people_shift_staff_removed.sql`).

**The band** says which half of the day a shift serves. `frueh` and `spaet` are the
two halves; `zwischen` marks the Zwischenschichten — an emergency means for the days
where neither half fits, never a target of the derived demand. The theatre marks
`F`, `F-AZ` as `frueh`, `S`, `S-WE`, `S-AZ`, `S-WE-AZ` as `spaet` and `ZS1`, `ZS2`
as `zwischen`.

**`frei` is not a shift but the free day** (`migrations/1930_people_shift_free_band.sql`).
Choosing `Art → Freier Tag` in the catalogue closes `Von`, `Bis` and `Pause`: the
mask sends `00:00`–`23:59` and no break, and the API normalises any other input to
exactly that (`src/shifts.rs`, `FREE_DAY_START` / `FREE_DAY_END` — the table keeps
`starts_at <> ends_at`). A free day counts **0 hours** — it never adds to the target
time — and it takes **no demand**: the derivation skips it like a `zwischen` shift.
It is a marker on the board (hatched cell, `Frei` in the cell) and the input for the
rest rules: the cell ends the `max_consecutive_days` run, starts
`min_consecutive_days` again and counts as a day off, while `shift_rotation_days`
is untouched — the employee keeps the band they worked before the free day. A
free day is never read as work (the rule check skips it, `src/duty_repo.rs`), a
series of free days needs no rule check at all, and a regeneration of the plan
leaves the free days standing like an absence — without `overwrite` every free day
is a cell that keeps its day, and only `overwrite` clears the month and writes the
free days anew.

**`apprentice_only`** closes a shift for everybody who is not an apprentice. The
apprentice shifts of the theatre (`F-AZ`, `S-AZ`, `S-WE-AZ`, `BS`) carry it, and
`src/duty_repo.rs` refuses anybody else with `409` (*Only apprentices may be
planned into this shift.*) — for a single cell and for a series alike. The rule
reads one way only: an apprentice may work a regular shift as well. The planner
often writes the apprentice's days by hand **before** the generator fills the rest,
so a regular shift is no refusal for them — the print check warns when nobody shares
it. The generator plans apprentices into their own shifts only.

**Weekend** (`src/shifts.rs`, `works_weekend`): Saturday and Sunday take no regular
late shift. A shift of the band `spaet` may only be worked there when it carries the
weekend marker `WE` in its short name or its name (`S-WE`, `S-WE-AZ`); the early
shifts and the `Zwischenschichten` are free, a `WE` inside a word (`Wochenende`)
counts nothing. `src/duty_repo.rs` answers `409` (*The late shift is not worked on
Saturday or Sunday.*), a series stops with `blocked.reason = weekend`, and the
generator never gives `S` there.

**Department** (`migrations/1946_people_shift_department.sql`): a shift may belong
to one department — the field `Abteilung` sits right next to `Art` in the catalogue,
fed by the departments of the master data (`—` = no department). The name is stored,
like on employees and locations, so a renamed department keeps the old name until
the shift is edited. A dispatcher only sees, plans and maintains the shifts his
`Personal-Disposition` scope covers (`src/routes.rs`, `list_shifts`,
`check_stored_shift`): the list leaves out every shift of a foreign department,
`PUT …/duty` and a series answer `403`, `POST`/`PUT`/`DELETE` of the catalogue refuse a
foreign department, and the generator only picks from the shifts he may see. An empty
department is open to everybody with the disposition right — it is read by all of them
but changed by the administration alone.

**Excluded positions** (`where_people_shift_excluded_positions`,
`migrations/1921_people_shift_excluded_positions.sql`): positions that never work
that shift — a night shift without apprentices, for example. The list belongs to
the shift (it holds for every day) and is maintained in the staffing demand of a
day, where it is closest to the plan (`POSITIONEN AUSSCHLIESSEN` below each shift).
`GET /api/v1/people/shifts` returns it as `excluded_positions` on every shift;
`PUT /api/v1/people/shifts/{id}/exclusions` with `{ "positions": ["Meister"] }`
replaces the list of one shift and answers with the shift (empty strings dropped,
case-insensitive duplicates counted once, at most 50 entries, at most 100
characters each, `*` refused with `422`). Planning respects it in `src/duty_repo.rs`:
a single cell answers `409` (`The position Meister is excluded from this shift.`),
a series stops with `blocked.reason = excluded_position`, and the cell menu greys
the shift out for that employee with `· Position ausgeschlossen`.

**Rules** (`where_people_planning`, exactly one row, `/api/v1/people/planning`).
The defaults are a starting point and can be adjusted per installation:

| Group | Setting | Default |
| --- | --- | --- |
| Rest period and working time | Rest period between two shifts | 11 h |
| | Maximum working time per day | 10 h |
| | Maximum working time per week | 48 h |
| | Maximum working days in a row (`max_consecutive_days`) | 6 days |
| | Minimum working days in a row (`min_consecutive_days`) | 1 day |
| | Days off per week | 1 day |
| Breaks | Break from working time | 6 h |
| | Break duration | 30 min |
| | Long break from working time | 9 h |
| | Long break duration | 60 min |
| Planning | Planning horizon | 4 weeks (switchable to months) |
| Vacation and sickness | 1 vacation day equals | 8.00 h |
| | 1 sick day equals | 8.00 h |
| | Color of the vacation cells | `#0369A1` |
| | Color of the sick cells | `#B91C1C` |
| Week and absences | Sunday work allowed | no |
| | Compensation of worked Sundays | no — switched on automatically when Sunday work is allowed |
| | Rest after Sunday work `24 + 11` hours | yes — only while Sunday work is allowed |
| | Six-day week: whole free weekend every 6 weeks | yes |
| | Apprentice never alone in a shift | yes |
| | Holiday work allowed | no |
| | Import public holidays | no |
| | Import regional holidays (by federal state) | no |
| | Import school holidays (by federal state) | no |
| | Federal state for the holiday imports | none |
| | Color of the holiday columns | `#B45309` |
| | Color of the school holidays | `#2563EB` |
| | Show holidays on the board | yes (display only) |
| | Show school holidays on the board | yes (display only) |
| | Consider vacation and absences | yes |

**Sunday work**: if it is allowed, `Kompensation` (compensation) can be switched on
below it — every worked Sunday is then compensated on another weekday — and
likewise `24 + 11 Stunden Ruhezeit` (the long rest). Without allowed Sunday work
the API stores both switches as `false`
(`where_people_planning.compensate_sunday`,
`migrations/1899_people_sunday_compensation.sql`;
`where_people_planning.extended_sunday_rest`,
`migrations/1914_people_german_rules.sql`).

**Rules for Germany**: three switches carry rules that exist in Germany only and
can be switched off elsewhere (`migrations/1914_people_german_rules.sql`,
`1915_people_apprentice_rule.sql`):

- `extended_sunday_rest` — after a Sunday shift the law asks for a **compensation
  day** (§ 11 Abs. 3 ArbZG): one whole day without work, and the duty after that day
  may only start once the regular rest has passed as well — **24 + 11 hours** with
  the default 11 hours. The day *before* the compensation day keeps the ordinary rest:
  a Monday after Sunday work is allowed when the 11 hours hold, and the compensation
  may fall on any later day of the week (the first day without a duty after the
  Sunday). `src/duty_rules.rs` checks it in `compensation_kept`
  (`BLOCK_COMPENSATION` = `compensation_rest`), the generator keeps the same state in
  `generate.rs` (`Record::book`/`Record::rest`) and the audit reads the rule with the
  rest.
- `free_weekend_every_six_weeks` — with a six-day week (`work_days = 6`) every
  sixth week has to leave Saturday and Sunday together free. The rule spans weeks
  and is stored for the plan proposals; the series check does not look at it.
- `apprentice_needs_company` — an apprentice (`where_employees.is_apprentice`)
  may never be alone in a shift: on every day a **colleague who is
  no apprentice** has to be planned in the **same** shift — his position,
  department and location do not matter, and another apprentice does not count
  (`duty_audit.rs` builds that set from `where_employees` and takes
  `NOT is_apprentice`). Free time windows have no shift, so any non-apprentice
  planned that day counts.
  Writing a cell does **not** ask the rule: the planner sets the apprentice's days
  by hand before the generator fills the rest, and a `409` there would stand in the
  way of exactly that. The rule is what the **print check** asks: `Drucken` runs
  `POST /api/v1/people/duty/audit` and lists every cell that breaks a rule —
  `apprentice_alone` (*Auszubildende/r darf nicht allein in einer Schicht sein*),
  the window offers `Abbrechen` / `Trotzdem drucken`, and the board marks the day.
  It applies to the shifts open to everybody — a regular shift the apprentice holds
  alone is the case it is there for. An `apprentice_only` shift takes no journeyman
  at all, so the rule would report every `BS` day; the audit keeps it out there
  (`duty_audit.rs` sets `Company::apprentice` to `false` for such a shift).
  The plan generator asks the rule itself and never plans an apprentice alone
  (`src/generate.rs`), and only into apprentice shifts.

**Holidays and school holidays** (`Feiertage und Ferien`): three switches and a
federal state decide what is imported, the addresses are fixed — nothing has to be
typed in (`migrations/1916_people_holidays.sql`):

| Switch | Imported |
| --- | --- |
| `import_public_holidays` | the nationwide holidays of Germany |
| `import_regional_holidays` | the holidays of `holiday_state` (`BW`, `BY`, …) |
| `import_school_holidays` | the school holidays of `holiday_state` |

- **Source**: [OpenHolidays](https://openholidaysapi.org) — free, no key.
  `GET https://openholidaysapi.org/PublicHolidays?countryIsoCode=DE&languageIsoCode=DE&validFrom=…&validTo=…`
  (with `&subdivisionCode=DE-BY` for the regional ones) and
  `…/SchoolHolidays?…&subdivisionCode=DE-BY`. Names come in German
  (`languageIsoCode=DE`).
- **Check / import**: `Prüfen und laden` in the settings saves the rules and calls
  `POST /api/v1/people/holidays/import` with `{ "year": 2026 }` (without a year the
  current one). The answer is
  `{ year, state, national_holidays, regional_holidays, school_holidays, errors }`;
  a source that fails is named in `errors`, the other list is still stored. Only
  when *every* requested source fails is the answer `502`.
- **Tables**: `where_people_holidays` (`day`, `name`, `scope` `national`/`regional`,
  `state`, `source`, `fetched_at`) and `where_people_school_holidays` (`starts_on`,
  `ends_on`, `name`, `state`). An import replaces the rows of that year; a list
  whose source failed leaves the stored rows alone. Switching an option off stops
  the import — stored rows stay.
- **Reading**: `GET /api/v1/people/holidays?from=&to=` returns
  `{ holidays, school_holidays }` for the period (`personal.personal-lesen`).
- **Board**: the day column of a holiday is tinted and the name stands under the
  date (`1. Weihnachtsfeiertag`); the colour is `holiday_color`, picked next to
  the import switches (`migrations/1917_people_holiday_color.sql`). A school
  holiday gets a bar under the date row and its name at the start of the stretch;
  its colour is `school_holiday_color` (`migrations/1919_people_school_holiday_color.sql`).
- **Show or hide** (`show_holidays`, `show_school_holidays`,
  `migrations/1918_people_show_calendar.sql`): two switches that only change the
  board. The planning rules always use the imported holidays — a hidden holiday
  still blocks a shift when `holiday_work_allowed` is off.
- **Rule**: with `holiday_work_allowed` off, a shift on an imported public holiday
  is refused — on the series with `blocked.reason = holiday`, on the single cell
  with `409`.

`holiday_state` is validated against the 16 states; `import_regional_holidays` and
`import_school_holidays` require it (`422`).

**Planning horizon**: number plus unit `weeks` or `months` (`planning_horizon`,
`planning_horizon_unit`, `migrations/1900_people_horizon_unit.sql`). Weeks allow
1–52, months 1–12; with months the planned period starts on the **1st of the
month**.

**Shift rotation**: `shift_rotation_days` (1–365, default **14**,
`migrations/1925_people_shift_rotation.sql`) keeps an employee in one band
(`frueh` / `spaet`) instead of letting them jump between early and late. It is a
rule of the generator, not a wish: the first worked day of a band starts a
**block**, and for `shift_rotation_days` days from that day the other band is not a
choice for the employee — the row stays open instead of switching them, and the
report shows it as a gap. Only when the window has passed may the next cell start
the other band, which begins the next block; `Zwischenschichten` stay the exception
for the days where neither block fits and they neither end nor start a block. A
free day is no work: it does not end the block either, the window keeps running
across it. To read the running block the generator looks `shift_rotation_days`
days back (at least 8, `generate::history_days`). The board uses the same blocks in
the cell menu: the shift of the running block is marked `· bleibt (14 Tage)`.

**Working days in a row** are two rules
(`migrations/1931_people_min_consecutive.sql`). `max_consecutive_days` (1–31,
default **6**) is the most days an employee may work without a day off — it is
checked by the series and by the generator. `min_consecutive_days` (1–31, default
**1**, never above the maximum) is the fewest days a block should have before a day
off follows: with `2` a single worked day between two free days is not wanted —
either both days or neither. Only the **generator** uses it, as a preference: while
an employee's block is still shorter than the minimum they are ranked first, so the
plan finishes the block instead of leaving it hanging. A hand-written series is not
refused for it — the rule is a wish of the planning, not a law. A `frei` cell and
every absence end a block, so both rules count from the next worked day on;
`shift_rotation_days` is not affected by a free day.

**The boundary of the day does not exist as a setting.** A shift has a window, and
an event belongs to the shift that serves it best — the shift catalogue
(`Stammdaten → Dienstplan`) is the single source of truth. A shift serves the event
when it holds the start and keeps **two hours** of it afterwards
(`MIN_AFTER_START`); of those the **regular shifts come first** — a Zwischenschicht
is never a competitor, and a nearer start only decides between equals. The three
hours before the curtain (`MIN_LEAD`) may be shared by two crews of the same house.

**Demand from the disposition**: `GET /api/v1/people/duty/dispo-demand?from=&to=`
(`personal.personal-lesen`) derives, per day, the staffing the disposition asks for
(`src/dispo_demand.rs`), as `{ day, entries, unclear, marked }`:

- Every event of the day adds the staffing of its **flag** to the shift that serves it
  best (rules above): a 10:00 and a 12:00 event belong to `F` (08:00–15:30), a 19:30
  event to `S`, a lone 15:00 performance to `ZS2` (10:00). An event no shift can serve
  lands in `unclear` with its reason instead of being pushed into a shift.
- **The band comes first** (`serving_shift` in `src/dispo_demand.rs`): a shift serves
  the event when it holds the start, keeps **two hours** of it and begins **three
  hours** before it; of those the regular shifts come first — a Zwischenschicht is
  never a competitor, only the shift of last resort. A 12:00 event therefore stays
  with `F` even though `ZS1` begins exactly three hours before it.
- **A show that runs into the next shift belongs to the crew of its house**
  (`planned_shift`): a day on which the house already works `F` (an 11:00 matinée, a
  10:00 probe) has an early crew, and a 15:15 show — a quarter of an hour before `F`
  ends — is covered by that crew instead of asking for a Zwischenschicht of its own.
  Only when the house works no band at all that day (a lone evening performance, a
  matinee without a flag) does the `ZS2` stand in. The crew counts per house and day,
  and only when its shift still holds the start of the event.
- **The run-up is shared by the crews of the house** (`lead_covered`): the three
  hours before the curtain have to be staffed, not carried by one shift. On 02.11.2026
  the early crew ends at 15:30 and the 16:00 show needs its crew from 13:00 — the
  early shift covers 13:00–15:30, the late shift 15:00–16:00, and together the run-up
  is complete. The event therefore goes to `S`, not to a `ZS2` that would have to be
  there alone at 10:00.
- **Three hours before the curtain** (`MIN_LEAD` in `src/dispo_demand.rs`): the crew
  has to be there before it starts — light, sound and the stage need their time. The
  run-up may be **shared**: when the house already works an earlier band that day, the
  two crews overlap around the curtain. An early shift ending at 15:30 covers two and a
  half of the three hours a 16:00 show needs, so the late shift bridges the rest and
  takes the event. Only a shift that has to cover the whole run-up alone needs the full
  three hours: a lone 15:00 performance is `ZS2` (10:00) because `F` keeps half an hour
  of it and `S` begins with the curtain.
- **Two hours after the curtain** (`MIN_AFTER_START`): the shift has to keep at least
  two hours after the event start — nobody leaves an hour after the show began. A
  shift that ends too early cannot take the event; when nothing serves it at all, the
  shift that began last before it takes over (a show shortly before the end of `S`
  stays with `S`). An event that no shift holds — before the first, after the last
  one — keeps no shift and stays `unclear`.
- **Saturday and Sunday**: the shift that works the weekend (`S-WE`) wins over the
  regular one of its band, on a working day the other way round. The marker is a
  preference, not a prohibition: when the weekend shift cannot serve the event in time
  (a 15:00 show on a Sunday, `S-WE` begins at 14:00), the crew rule above decides
  first and the regular shifts come next.
- **The demand is per location** (`migrations/1929_people_demand_location.sql`):
  a performance in the big house and one in the podium on the same evening are
  **two rows per position**, not one row needing two employees. A row without a
  location is the demand the mask enters by hand and is not bound to a house.
- **A house without a foreman** (`needs_foreman = false`) asks for none: the
  staffing row of the flag for `Meister` is dropped before it becomes demand, so the
  board shows neither a demand badge nor a gap for it.
- **One house, one band, one crew.** Inside one location and shift the events do
  not add up but take the **maximum**: on 01.12.2026 the disposition names
  `Max und Moritz Vst` at 09:00 and again at 11:00, both in the big house and both
  inside `F` — the same people cover both, so the row stays 1× per position. Of two
  different flags in one band the fuller one decides (`BEL` with 2 Meister next to
  `Vst` with 1 → 2). Two houses still staff separately, and two events of one house
  in **different** bands are two rows, because a crew works one band.
- An event without a flag asks for nobody and is no error. Everything else the
  derivation cannot read lands in `unclear` with a reason: `no_time` (no start
  time), `unknown_flag` (a code the master data does not know), `no_staffing` (the
  flag needs nobody) and `no_shift` (no regular shift of that band).
- `marked` is `true` as soon as one event is unclear; the board shows a **`?`** in
  the date cell for that day, and the `Personalbedarf` window lists the events
  under `Bedarf aus der Dispo` together with their reason. `Übernehmen` copies the
  derived rows into the rows of the day — `Speichern` writes them.
- **Setting an event aside** (`ignored`, `PUT /api/v1/people/duty/dispo/entry/{id}/ignored`
  with `{ "ignored": true }`): the event stays in the disposition and on the board,
  but it asks for nobody and no longer marks its day — the `Ignorieren` button sits
  next to the event in `Personalbedarf`, `Beachten` takes it back. The flag travels
  with the entry when the day is saved again, and an import never sets it.
- **Conflicts of the plan** are marked on the other side of the date: a red **`!`**
  in the top left corner of the date cell stands for a rule the written plan of that
  day breaks. A click on the date opens `Personalbedarf`, which names them at the top
  (`Regelverstöße`: who, and which rule — the same list the print window shows). The
  audit comes from `GET /api/v1/people/duty/audit` for the shown month and is
  refreshed after every written cell. The number in the top right is the staffing
  demand, the `?` next to it the unclear disposition.

The API validates the values (`422`): the weekly working time must cover the daily
working time, the long break does not start before the short one and is not
shorter, rest period 0–24 h, horizon 1–52 weeks. `GET` returns the same defaults
without a stored row.

**Vacation and sickness**: `vacation_day_hours` and `sick_day_hours` record how many
hours a vacation or sick day counts (`numeric(4,2)`, 0–24,
`migrations/1903_people_absence_hours.sql`). The form shows both values as
decimals with step `0.25`; the API accepts them as text or number and answers with
`422` when a value exceeds one day.

**Absence colors**: `vacation_color` and `sick_color` hold the cell colors the board
uses for `Urlaub` and `Krank` (`#RRGGBB`, stored in upper case,
`migrations/1905_people_absence_colors.sql`). The form offers one colour picker per
absence — the picker of the app with the palette of 32 and the wheel; an invalid value
answers with `422`.

## Windows and `Esc`

Every window of the app closes with **`Esc`**, and it closes exactly the way its
`Abbrechen` does — including the question that button asks when input would be lost. A
second `Esc` answers that question with "no". The mechanism is one listener for the whole
app (`apps/web/src/lib/escape.svelte.ts`): a window registers itself with
`escapeFenster(...)` and only the topmost one reacts, so one key press never closes two
windows. The cell menu of the board is registered the same way; popups that belong to a
control — the colour picker, the inline rename of a list entry — keep their own `Esc`.

## Module settings

The admin menu carries the product under `Admin → Module`, right below
`Modulverwaltung`: the entry `WHERE - are my people?` opens `/admin/modules/people`
(`web/ModuleSettingsPage.svelte`, route `apps/web/src/routes/admin/modules/people`). It
is contributed by `web/menu.ts` with `module: 'people'`, so it appears only while the
product is installed, and the route sits behind `ModulePage` like every other module
page — an uninstalled module explains itself instead of showing empty settings.

Two options empty one month of the module's data; each one first asks for the month
(`web/ResetMonthDialog.svelte`, month and year selects, opening on the current month,
`Abbrechen` / the option's own name):

| Option | API | What it removes |
| --- | --- | --- |
| `Schichten zurücksetzen` | `POST /people/duty/reset?month=11&year=2026` | every shift of that month (free days included), every free time window and every vacation |
| `Dispo zurücksetzen` | `POST /people/duty/dispo/reset?month=11&year=2026` | every entry of `where_people_dispo_entries` in that month |

Both answer `{ "cleared": 12 }` and the page names the result
(`12 Schichten im November 2026 entfernt.`, `Keine Termine im April 2027.`).

- The shift reset uses `DutyRepository::clear_shifts` and empties the plan of the
  month: `kind = 'shift'` — the free days `FT`/`SoFT` (`where_people_shifts.band =
  'frei'`) included —, `kind = 'free'` (a free time window) and `kind = 'vacation'`.
  Only `kind = 'sick'` stays: a sick day is a record of the personnel file, not a
  part of the plan. The generator keeps its own `clear_planned_shifts`, which spares
  the free days: it fills them itself, and a person it does not plan again would
  lose their `FT`.
- The disposition reset deletes the entries only. The rows in
  `where_people_dispo_imports` stay — a month without entries takes the same PeM file
  again, so a corrected import needs no extra step.
- Both routes are admin-only (`crates/where-api/src/authz.rs`) and sit behind the
  module gate; `month` outside 1–12 and a year outside 1900–2999 answer `422`
  (`duty::month_range`, also used by the PeM import).

## Duty plan (board)

The `Dienstplan` tile of the side bar opens `/people/schedule`: a board
with the active employees as rows and the days of the month as columns
(`web/DutyPlanPage.svelte`). It always shows **one month**, with ◀ / Heute / ▶.
Above the board a legend lists every shift with its
short name, time and color, next to `Krank (K)` and `Urlaub (U)` in the configured
absence colors (`web/DutyPlanPage.svelte`). It carries **one line per entry** —
short name, name and time side by side, the absences in two columns at the right —
so it stays around 60 px high and the board keeps the height of the window.

**The rows of the board can be arranged by hand**: the name cell of a row is a drag
handle (`draggable`, only with `personal.personal-editieren`), and dropping it on
another row puts the employee in front of or behind it — the pointer decides, the
upper half goes before. The new sequence is written at once
(`PUT /api/v1/people/board-order` with `{ "ids": [...] }`, top to bottom,
`where_employees."board_order"` from `migrations/1939_people_board_order.sql`) and
redrawn without reloading the month. Until the first drag the board sorts by the
position order of the master data and then by last name; an employee who never stood
in an arranged list (`board_order` empty) follows the arranged rows. The order is the
order of the month's board everywhere it is handed on: print, exchange files and
`Trotzdem drucken` take the rows as they stand.

**A sub-department tints its employees, the department otherwise**: the name column of a
row is mixed from the sub-department colour of the employee — or the colour of their
department when the sub-department carries none — and the surface
(`color-mix(in srgb, var(--mitarbeiterfarbe, var(--surface)) 30%, var(--surface))`), so
the sticky column stays opaque. The colours come from
`GET /api/v1/people/lists/subdepartments` and `…/departments`; no colour on either, no
sub-department and no department, or a name that no longer stands in the master data
leaves the row as it was.

**A location tints its disposition row**: the name cell of a `Standorte` row is mixed
from the location colour the same way (`tr.dispo.standortfarbe`), so the houses of a
month are told apart at a glance; without a colour the row keeps the light band it
always had.

**Only the houses of the own department**: the `Standorte` rows come from
`GET /api/v1/people/duty/locations`, which hands out the locations of the departments of
the session's `Personal-Disposition` — a location belongs to the department of the
sub-department that works there, and a location without a sub-department belongs to
nobody, so a restricted dispatcher does not see it. The events follow:
`GET …/people/duty/dispo` and `…/duty/dispo-demand` drop the days of the other houses.
Without a department entry in the group every location is shown, as before.

**`Zeige Abteilung`** narrows the board for the administration: the button sits right next
to `Springe zu` and opens a window (`web/DutyDepartmentDialog.svelte`) with `alle` and the
departments of the master data — the current one is marked. A click reloads the month with
`?department=<name>` — the houses of the department, their disposition and their demand,
and the employees of that department; the button then carries the choice
(`Zeige Abteilung · Beleuchtung`). `Esc` and `Schließen` close the window without a choice.
The **legend** follows the choice as well: it lists the shifts of that department and those
that belong to none (they are open to every dispatcher), never the shifts of another
department. The restriction of a group and this filter combine: the filter can only take
away, never add. The choice holds for the shown month only; a reload starts at `alle`.

The board uses the **whole window** — the page marks itself with `class="breit"`, and
the shell drops its `72rem` for it — and the table fills that width (`width: 100 %`,
at least 250 px for the names plus 140 px per day). Only the board itself scrolls:
the page keeps heading, toolbar, day row and legend in place, so `rasterHoehe` measures
from the top edge of the board down to the padding of the shell (`24 px`, read from it,
two pixels of air) — again on resize, whenever the legend or the month changes, and
through `bind:clientHeight` on the legend, whose height the board takes. Inside the board the **head sticks on
top** — the row of dates sits in the `thead` and stays while the shifts scroll
underneath. A `tbody` row does not stick in Gecko (Firefox ignores `position: sticky` on
`tr` there, and on its cells too), while a `thead` does — one block for both browsers,
and no measurement of the row heights. The rows of the disposition sit in the `thead`
as well, right under the dates; the switch `Sticky | Scroll` in the name cell of the
date row (left of `+`, which adds a note row) moves them into the `tbody`, where they
scroll with the shifts, and keeps the choice in `localStorage`
(`where-people-dispo-sticky`).
The name column sticks to the left as before; the cells carry opaque colors
(`color-mix(… var(--surface))`) so nothing shows through, and the sticking cells draw
their line as an inset shadow instead of a border: a collapsed border belongs to the
table and would stay behind while the block travels.

**`Ausblenden`** sits under the `Sticky | Scroll` switch, in the same design and **exactly
as wide as it** — both live in one grid of the name cell (`grid-template-columns: max-content
1fr`, the switch and `Ausblenden` in its first column, `+` right-aligned in the second), so
the two frames stay flush even when the label grows to `Ausblenden · 2`; the buttons inside
`.dispoModus` fill their frame (`flex: 1 1 auto`). It opens
`web/DutyHideLocationsDialog.svelte` with every location the user may see (the list of
`GET …/people/lists/locations` of that scope), each with a tick — all ticked when the
window opens. Removing a tick hides that location in the board at once: its row of the
disposition disappears and the switch shows how many are hidden (`Ausblenden · 2`). It
is a **view of the work only** — the choice lives in the board's state and nowhere else,
so a reload shows every location again, and the print, the PDF, the XLSX and the CSV
load their own month and stay complete (`web/roster-report.ts` `ladeMonat`). `Esc`
and the `×` close the window like `Schließen`.

- **The day row** under the toolbar carries one button per day of the month the
  board shows (`1 … 28/29/30/31`, spread over the full width, Sundays tinted); a
  click jumps the board to that day and scrolls the
  board so that the day sits directly next to the names, as far as the period
  reaches: the days that follow stay visible, so the last days of a period end up
  where the board ends.
- **Springe zu** in the toolbar opens a calendar window
  (`web/DutyJumpDialog.svelte`) with the month the board shows; ‹ › page through
  the months. It marks today and the days of the shown period; a click on a day
  jumps the board to its month and closes the window (Escape and the `×` close it
  without a jump).
- **Dispo (PeM)** is one of the scopes of the import in `Import / Export` (only with
  `personal.personal-editieren`, see [Exchanging a month](#exchanging-a-month)): it
  takes the xlsx export of PeM, which carries no year — the month and year come from
  the window. Every location becomes a **row above the employees**
  (drawn from `where_people_locations`), and **every event of a day becomes its own
  line** in the day cell — `18:00` in a narrow column, the text next to it
  (`where_people_dispo_entries`, `migrations/1922_people_dispo.sql`). The time is
  stored apart from the text, so both stay editable; a location that already exists
  (ignoring case) is reused. `Einträge dieses Monats ersetzen` empties that month
  first. Days of the month that the file does not carry stay untouched.
- **A disposition is imported once per month**: `where_people_dispo_imports`
  (`migrations/1932_people_dispo_imports.sql`) keeps a fingerprint over the scanned
  content — locations, days, times, texts and flags, not the bytes of the file — so
  the same planning is recognised even when PeM writes a fresh xlsx around it. A
  second import of that content into a month that still holds entries is a `409`
  (`Set `replace` to import it again`); with `Einträge dieses Monats ersetzen` it is
  written again, and a month whose entries were deleted takes the file without a
  complaint. `dispo::adds_nothing` is the second net behind the fingerprint: an append
  whose every event (location ignoring case, day, time, title, flag) already stands in
  the month is refused as well — the fingerprint only knows what the module wrote
  itself, a month that came from somewhere else would otherwise be doubled. The scan —
  and the exchange scan of the disposition sheet, which uses the same fingerprint —
  answers `already_imported` and `existing_entries`: the window then names the duplicate
  (`Diese Datei wurde für … bereits geschrieben`) and keeps `Importieren` disabled
  until `Einträge dieses Monats ersetzen` is ticked.
- **A day's entries are edited in place**: a click on a day cell of a disposition row
  opens `web/DutyDispoDayDialog.svelte` with one row per entry — a narrow time field
  and a text field, `−` removes a line, `+ Termin hinzufügen` appends one, `Speichern`
  writes the whole day through `PUT /api/v1/people/duty/dispo/{location}/{day}` with
  `{ "entries": [{ "starts_at": "18:00", "title": "…", "flag": "Vst" }] }`. The day
  is replaced as a
  whole (ids churn, nothing references them), the order of the array is the order on
  the board, an unknown location answers `404`, an unknown flag code `422`. Every
  entry carries a **flag selector** (`Vst · Vorstellung`, `Probe · Probe`, … or
  `— ohne Flag —`); the import fills it, here it stays editable. The time field
  accepts `8`, `830`, `8.30` and `18:00` and stores `HH:MM`; the text is trimmed and
  takes 1–200
  characters, a day at most 50 entries. `Schließen` (and Escape) leave the day as it
  was. Read side: `GET /api/v1/people/duty/dispo?from=&to=`.
- **Dienstplan generieren** — the green button at the right end of the toolbar (only
  with `personal.personal-editieren`) opens `web/DutyGenerateDialog.svelte`. It asks
  for the period as `Von – bis` (opening on the days the board shows), as `Woche`
  (`KW` + year) or as `Monat` (month + year); the line below the switches names the
  resolved period and its length, and `Bereits geplante Schichten ersetzen` decides
  whether the shift cells that are already there are rebuilt. `Generieren` writes the
  plan through `POST /api/v1/people/duty/generate` and reports
  `<n> Zellen geschrieben · <n> unverändert[ · <n> ersetzt]`, how many days it
  checked, how many demand rows stayed open per day and which days had no readable
  shift in the disposition (`?` on the board). `Fertig` closes the window; the board
  has already read the new plan, and the `Ist` hours follow it.
- **How a location is found** (`src/dispo.rs`): PeM writes the disposition as a
  print layout, so the height of a block cannot be counted from its content. PeM
  merges the location cell over exactly the rows of its block (`A3:D8`), and that
  merge gives the rows: 3, 6 or 14 make no difference, and rows without an event
  stay in their block. The days come from the date header (`01.12`), each day owns
  a pair of columns (`E:F`, `G:H`, …), and the band ends where the staff plan starts
  (`Soll`/`Arbzeit` in B/C) or at the first row without text — the staff plan below
  is never read. **The flag** of an event is a word at the end of its first line —
  the last one, or one of the three words before it — read against the codes of the
  master data (`Vst`, `Probe`, `Belkorr.`, …) and against the legend of PeM, so the
  first disposition of a fresh installation recognises its flags too; a bare number
  behind it is skipped, so `BO 1` becomes the flag `BO`, and so is a word behind the
  code, so `Ehebrecherinnen TE Ton` becomes `TE`. A text
  without a known code keeps none (`Kipppunkte Einführung`), and only the first line
  counts — the notes below it (`… BEL-Besprechung inkl. Toneinrichtung`) are no flag.
  A copy of the export covers this in the tests
  (`crates/where-module-people/tests/fixtures/peM-dispo.xlsx`).
  API: `POST /api/v1/people/duty/dispo/scan?month=&year=` and `…/dispo/import` take
  the xlsx as the request body and answer
  `{ locations, days, warnings, already_imported, existing_entries }` and
  `{ locations, locations_created, flags_created, days, entries_created, warnings }`;
  `replace=true` empties the month first, `only_with_time=true` drops texts without
  a time (`i.A. an die Vorstellung`). A second import of the same content into a
  month that still holds entries answers `409`.
- **The flags of the file come into the master data with the import**: every code the
  disposition uses that the legend knows and the master data does not is created
  before the entries are written (`FlagRepository::create_presets_of`), so the entries
  carry their flag and the demand of the days follows without a hand on it. Created
  flags bring the crew of the legend for every house; a house that had no crew at all
  when the import began — the houses the import itself creates — gets it right after
  (`staff_presets_of`), while a house that is configured keeps exactly what it has.
  `flags_created` names how many the import added and the window appends
  `… Flags angelegt` to its result line.
- The rows are ordered by the **position order maintained under master data**
  (`Stammdaten → Personal → Abteilungen und Positionen`, `PUT …/lists/positions/order`);
  inside a position block by last name, employees without a position come last. Each
  row shows the position below the name, and a line with a little extra space
  separates the position blocks.
- The employee column carries the **hours** of every employee in three lines: the
  heading `Soll | Ist | +/−` (empty in front), the name next to `Sollzeit des Monats`,
  `Ist-Zeit laut Dienstplan` and `Über- oder Minusstunden seit Jahresbeginn`, then the
  position. `Soll` follows the contract — `weekly_hours` spread over `work_days`
  working days a week (Monday to Friday for up to five days, plus Saturday from six,
  plus Sunday for seven) and the working days of the shown month; employees without
  `weekly_hours` show `—`. `Ist` adds up the plan month: every shift with its time
  span minus its break (an end before the start runs past midnight), every free
  time the same way from its own clocks, every vacation
  and sick day with `vacation_day_hours` and `sick_day_hours`. `+/−` accumulates
  `Ist − Soll` from the start of the hours account to the shown month — that start
  is `balance_from` in the settings (`migrations/1913_people_planning_balance_from.sql`),
  without a date it is 1 January. The board reads the plan of the whole year for
  that (`GET …/duty?from=<year>-01-01&to=<end of month>`).
- Besides the houses of the master data the board carries **rows of its own**: a
  disposition row whose first column is **free text** instead of a location
  ("Technik", "Bühne links"). Such a row *is* a location with `is_text = true`
  (`migrations/1949_people_location_text.sql`), so it behaves in everything like a
  house: its day cells take Termine (time, text and flag — the dialog of a
  location), its name opens `Einstellungen` with `Meister benötigt` and the staffing
  of every flag (`web/DutyLocationDialog.svelte`, `web/DutyLocationFlags.svelte`),
  and its events reach the demand and the generator like the events of a house. The
  only differences: the name is typed (unique only among houses — two typed rows may
  share one), the mask and the master-data list leave the rows out, and they belong
  to no department, so every dispatcher sees them.
  The `+` beside `Sticky | Scroll` adds a row (`Neue Zeile`, at the end of the
  disposition); its name is edited in place, `⚙` opens the settings and `−` removes
  the row with its events. API: `POST /api/v1/people/duty/rows`,
  `PUT /api/v1/people/duty/rows/{id}` with `{ name }`, `DELETE …/rows/{id}`
  (a house answers `404`; name empty or longer than 100 characters `422`).- Clicking a **date** opens the **staffing demand** of that day
  (`web/DutyDemandDialog.svelte`). It shows one block per shift with a row
  per position and
  the number of employees needed (`where_people_duty_demand`, primary key
  `day, shift_id, position`, `migrations/1912_people_duty_demand_position.sql`).
  A row without a number is not needed and is not stored. Next to the demand the
  dialog shows how many employees of that position are already in the shift
  (`Geplant`, red when it stays below the demand). `+` adds another position to a
  shift, `−` removes the row. **Übertragen** copies the day onto the next `n` days,
  onto the whole week, the rest of the month, or all days of the month; the target
  days are replaced. **Speichern** writes the demand of the day and closes the
  window (an error keeps it open). The date head shows the demand of the day as a small
  badge in the top right corner of the date cell (red while the plan stays below it) and
  the footer counts `Eingeteilt/Bedarf`. The flags of a house are not part of this
  window any more: they live in the `Einstellungen` window of the location (see above).
  API: `GET /api/v1/people/duty/demand?from=&to` (at most 400 days),
  `PUT …/demand/{day}` with `{ entries: [{ shift_id, position, location, needed }] }`
  (replaces the day, numbers 0–99, unknown shifts `404`, `422` otherwise) and
  `POST …/demand/copy` with `{ from, to: [days] }`. A row that a house binds carries
  its location as a small chip next to the position (`Meister · Podium`).
- **nicht relevant** is a position row that does not name a position: the database
  stores it as `*` (`src/demand.rs`, `ANY_POSITION`; the PK allows it,
  `migrations/1921_people_shift_excluded_positions.sql` documents it). It counts
  the employees of the shift that **no** named row of that shift claims — with
  `1 Meister` and `1 nicht relevant` a shift of two Meister and one Beleuchter
  fills both rows. Under such a row the dialog names the employee with the fewest
  hours of the month who is still free that day (`Vorschlag (wenigste Stunden)`)
  — a hint, not an assignment: only `Dienstplan generieren` writes a cell without a
  click on it.
  `POSITIONEN AUSSCHLIESSEN` below each shift maintains the **excluded positions**
  of that shift as chips with `×` and a `+ ausschließen …` selector; the positions
  in it are neither offered nor accepted there (see above).
- Every cell is a button that opens a **menu** of the maintained shifts plus
  `freie Eingabe …`, `Krank (K)` and `Urlaub (U)`; over the list stands a **header with the
  day and the employee** (`Di., 17.11.2026` / `Johannes Grebing`), sticky while a long list
  scrolls. The menu lists every shift with
  short name, name, time and net hours, and marks the current value. After
  choosing, the cell shows `<short name> - <start>–<end>` (for example
  `F - 08:00–15:30`), `08:00–14:00` for a free time, `Krank (K)` or `Urlaub (U)`;
  `—` means off. A change saves immediately, and an error resets the cell. Escape
  and a click beside the menu close it.
- **Under the shift sits the free text of the cell** (`where_people_duty_entries.note`,
  `migrations/1936_people_duty_note.sql`, at most 200 characters): one line per
  filled cell, empty until someone writes — `Tausch mit Kollegen`, `fährt den Bus`,
  `Einweisung`. It is saved on blur/Enter as part of the cell (`PUT …/duty` with the
  kind and the shift of the cell), it stays with the day when the shift changes and it
  goes away with the cell. The note belongs to the cell, not to the shift, so the
  generator leaves it alone when it writes a shift over a day.
- **freie Eingabe …** opens `web/DutyFreeTimeDialog.svelte` for that one day:
  `Von (24 h)`, `Bis (24 h)` and `Pause (Min.)`, typed as in the shift form
  (`8`, `830`, `8.30` become `08:00`, `08:30`). The window shows the net hours
  below the fields and refuses an unreadable time, equal start and end, and a
  break that is not shorter than the window. An end before the start runs past
  midnight. The cell then carries `kind = free` with its own `starts_at`,
  `ends_at` and `break_minutes` (`migrations/1920_people_duty_free_time.sql`) and
  counts as work in the hours of the month and in the day's `Eingeteilt`; opening
  the dialog again shows the times of that cell. After `Übernehmen` the board asks
  **Tage in Folge** for the free time just like for a shift, so the typed window is
  written onto several days in a row (`POST …/duty/series` with `kind = free` and
  its three values).
- Choosing a **shift** or finishing **freie Eingabe …** first asks **Tage in Folge**
  (`web/DutySeriesDialog.svelte`):
  one number, the selected day included, and a live preview of the range
  (`Mo., 16.11.2026 – Fr., 20.11.2026`). `1` writes the single cell as before;
  `2` to `366` write the same shift or the same time window onto that many
  consecutive days in one call and
  replace whatever planned there. `Krank (K)`, `Urlaub (U)` and clearing still
  write the one cell.
- A series keeps the **planning rules** (`src/duty_rules.rs`, nothing is written the
  frontend could not see): it stops at the first day that breaks one, so the days
  stay in a row — Sunday work without `sunday_work_allowed`, a public holiday
  without `holiday_work_allowed`, a day already planned
  as sick or vacation with `respect_absence`, a shift longer than `max_daily_hours`,
  more than `max_consecutive_days` in a row (the days before the series count),
  less than `min_rest_hours` to the shift of the day before or after, less than
  24 + `min_rest_hours` around the compensation day a worked Sunday asks for
  (`compensation_rest`, while `extended_sunday_rest` is set),
  `max_weekly_hours` in that calendar week, fewer days off left than
  `min_days_off_per_week`, or a position the shift excludes
  (`excluded_position`). The answer names the day and the rule
  (`{ day, reason }`), and the board reports it in German
  (`6 von 7 Tagen eingetragen — So., 22.11.2026: höchstens 6 Tage in Folge.`).
  A single cell placed by hand is the planner's decision and stays unchecked. The
  apprentice rule (`apprentice_needs_company`) is no stop for a series either: what
  matters is the print, where the check names every apprentice left alone.
  Messages under the dates — a stop of a series, a refusal of the API — clear
  themselves after **10 seconds** (`DutyPlanPage.svelte`), so the board keeps its
  height.
- One entry per employee and day (`where_people_duty_entries`,
  `migrations/1902_people_duty_entries.sql`): saving again replaces the cell,
  clearing deletes the entry. `kind` distinguishes `shift`, `sick`, `vacation` and
  `free`; only a shift carries a `shift_id`, only a free time its three time
  columns (`migrations/1904_people_duty_kind.sql`,
  `migrations/1920_people_duty_free_time.sql`).
  Whoever deletes a shift or removes an employee loses the associated plan entries
  (foreign key with `ON DELETE CASCADE`).
- The footer counts per day how many employees are in a **shift or a free time** —
  sick and vacation do not count; Sundays are highlighted.
- Reading with `personal.personal-lesen`, writing with
  `personal.personal-editieren`. API: `GET /api/v1/people/duty?from=&to=`
  (at most 400 days, otherwise `422`), `PUT /api/v1/people/duty` with
  `{ employee_id, day, kind, shift_id }` — without `kind` (or `kind: null`) the
  cell is cleared, `kind: "shift"` requires a `shift_id` (otherwise `422`),
  `kind: "free"` needs `starts_at`, `ends_at` as `HH:MM` and an optional
  `break_minutes` (0–720, shorter than the window) — and
  `POST /api/v1/people/duty/series` with
  `{ employee_id, day, days, kind, shift_id }` for `days` days in a row (1–366,
  the first day included; a series always needs a kind and answers with
  `{ entries, requested, blocked }`; free times carry their clocks here too,
  unknown shifts `404`, everything else `422`).

## How the plan is generated

`src/generate.rs` builds a period out of everything the module knows. The demand of
a day is the stored one (`where_people_duty_demand`); a day without it uses what the
disposition asks for (`dispo-demand`), so an imported PeM month is enough to plan it.

- **Order**: the days in order. Inside a day the seats of the rows are filled in
  **rounds** — one employee per row and round, in catalogue order — so the early and
  the late shift of a position are staffed before one of them gets a second person.
  That is the **counter shift** (`Gegenschicht`): if one Oberbeleuchter works late,
  the other works early or is off. Inside a row the named positions come before the
  `nicht relevant` row.
- **Who gets the row**: inside a day with both bands the employees whose running
  block is the band of the row come first — they stay — and the free ones are left
  for the other band, so the rotation does not close a row that somebody else could
  have filled. After that the free employees are ranked — fewest hours booked in this
  run first (the history counts too, so the boundary to the days before is smooth),
  then a position that the band does not carry yet, then the employee whose running
  block is that band, then the lower id. Apprentices come after the regular
  employees and only join a day that already has a colleague
  (`apprentice_needs_company`); an `apprentice_only` shift takes nobody else. An
  employee with **`nicht automatisch disponierbar`**
  (`where_employees.auto_plan = false`) is never chosen — the row then stays a gap
  (`no_employee`); cells the planner set by hand keep their employee and cover the
  demand as before.
- **The rotation decides before the hours**: an employee in a running block can only
  take that band (`fits`), so a day of a scarce position may stay open where a
  colleague would have had to switch — that is the price of the rhythm and shows up
  as a gap instead of a wild early/late change. It is a rhythm, not a law: in the
  last `RELAX_DAYS` (3) days of a block the plan may switch the band when the row
  cannot be staffed otherwise (`Record::block_ends_soon`) — the rest period then has
  to hold as always, so an early-to-late switch at the end of a block is the common
  case, a late-to-early one only with a free day in between.
- **No band**: a `Zwischenschicht` and a weekend shift are no band of their own —
  they neither end a block nor start one (`rotation_band`). The weekend shift is the
  only late shift a Saturday or Sunday has, so it may be worked inside an early
  block; hours, days in a row and the rest period still count, so the next working
  day stays guarded.
- **Emergency Zwischenschicht**: rows that are still open after all rounds get a
  `Zwischenschicht` standing in (`between_shifts`). It qualifies when it covers at
  least half of the row's window — a `ZS2` (10:00–18:00) serves an early row whose
  own shift everybody of that band is blocked from, while a `ZS1` ending at 17:00 is
  no stand-in for an evening that runs to 23:00. Candidates are tried by overlap.
- **House without a foreman**: a demand row for `Meister` whose location is marked
  `needs_foreman = false` (`migrations/1934_people_location_foreman.sql`) is skipped
  before it becomes a job — no cell, no gap. The same holds for a demand the planner
  entered by hand; the setting is what the house says, not what the flag staffing says.
- **House and sub-department**: a demand row of a house is only filled by employees
  who work in the sub-department of that house. The link is maintained on the location
  (`where_people_locations."department"`, `migrations/1929_people_demand_location.sql`)
  under `Stammdaten → Abteilungen und Positionen → Standorte`: `Podium` → `Podium`,
  `Oberes Foyer` → `Großes Haus`. Since `migrations/1942_people_subdepartments.sql` the
  binding names a sub-department, and **only the employee's sub-department** is compared
  with it — the department above never opens a house, so somebody without a
  sub-department is planned by hand only. A location without a binding — and every row
  without a location — takes anybody. Nobody in that sub-department on the staff is
  reported as the gap reason `subdepartment`.
- **Existing cells count by sub-department too**: a cell that stays covers its row only
  when position *and* sub-department match, so a hand-placed employee of another crew
  leaves the row open.
- **Who fits** — the same rules the series checks: `min_rest_hours` to the previous
  cell, the 24 hours around the compensation day a worked Sunday asks for when
  `extended_sunday_rest` is set,
  `max_daily_hours`, `max_weekly_hours`, `max_consecutive_days`,
  `min_days_off_per_week`, the agreed `work_days` of the personnel master data,
  `sunday_work_allowed`, the positions the shift excludes, the apprentice rule (an
  apprentice gets apprentice shifts only, nobody else gets theirs) and the weekend
  rule (no regular late shift on Saturday or Sunday). A public holiday without
  `holiday_work_allowed` closes the whole day. The rules hold for every employee, not
  only for the ones already booked in the run.
- **Blocks**: `min_consecutive_days` is why the rank starts with `continues` — an
  employee whose running block is shorter than the minimum comes first, so a single
  worked day between two days off is avoided where the plan has the choice. A `frei`
  cell ends the block — minimum and maximum count from the next worked day on — is
  never written as a worked day, costs 0 hours and is not counted in the fairness of
  the run. The rotation (`shift_rotation_days`) is untouched by it: the employee
  keeps the band they worked before the free day.
- **What stays**: cells that are already planned cover their part of the demand — a
  cell of a named position serves its row, whatever is left serves the
  `nicht relevant` row, and only within the same house — and they count for rest
  time, weekly hours and fairness like a generated one, so a second run on the same
  period writes nothing. Sick, vacation and free time are never touched, not even
  with `overwrite`; `overwrite` only clears the worked shift cells of the period and
  rebuilds them. Nobody is planned onto a day that already has a cell — a **free day
  of the plan** (`FT`, `SoFT`) included: it is a cell like any other, so a day set
  aside by hand stays free, and a person that day counts as taken. Only `overwrite`
  takes them: it lets the generator lay the month out again, free days included, and
  the run fills them at the end.
- **Free days** (`FT`, `SoFT`): after the demand and the hours balance every day of
  the period without a cell becomes a free day, so the calendar is complete for every
  active employee — the shift whose short name is `FT` (band `frei`). A day that is a
  **Sunday or a public holiday** takes `SoFT` (`Sonntagsausgleich`) instead: a free day
  there would not be a day off the plan granted, it is what the calendar gives anyway.
  A worked Sunday gives its compensation (`compensate_sunday`): the first free day after
  it within 14 days is written as `SoFT` as well, so a compensation day is never pulled
  back to `FT`. Both are no work — no hours in the `Ist`, no demand — and the report
  names their number as `free_days` (`n freie Tage` in the result line).
- **Hours balance** (`weight of the month`): after the demand is served the
  generator closes what an employee is short of their contract. The target is
  `weekly_hours / work_days` per working day of the period (Monday to Friday, plus
  Saturday from six working days, plus Sunday from seven) — the same formula the
  board uses for `Soll`. Carried in is the balance from `balance_from` (1 January
  when empty): booked minutes minus owed minutes up to the day before the period,
  with sick and vacation days counting as `sick_day_hours` / `vacation_day_hours`.
  A deficit is filled with **extra shifts** on days the employee is free and the
  rules allow another one (`fits`, `max_weekly_hours`, days in a row, days off,
  Sunday, holidays, the apprentice rule, the weekend rule), the rest period to the
  next planned cell included — an extra that would break it is not written. The shift is chosen so that the deficit
  ends closest to zero: the smallest one that covers it, otherwise the longest one;
  `zwischen` and free shifts are no extra work, a working day takes no weekend shift
  and a weekend takes nothing else, and where the person normally works
  wins. An extra shift sees the **whole** plan of the employee, not only the days
  before it: the days already planned after it count for the run of working days and
  for the hours of its week (`run_after`, `week_planned`) — otherwise a gap filled
  in the middle would turn two planned runs into seven days in a row and the week
  over its limit. An extra shift stays in the band of the running block — and where there is  none, in the band of the next planned cell of the employee, so an extra on Monday  does not start a block that the cell of the Wednesday contradicts (`extra_band`). A
  remainder stays as a few overtime hours — a short deficit is not left open for the
  sake of exact zero. The report names the result as `extra` (`n Extraschichten` in
  the result line). Employees without `weekly_hours` or `work_days` have no target
  and get no extra shift, and `nicht automatisch disponierbar` applies here too.
- **What it does not do**: it reports what it cannot fill and never forces a rule.
  A gap is `{ day, shift_id, position, missing, reason }` with `no_employee`,
  `subdepartment` or `holiday`, and the window lists them per day. At most 92 days per
  call; an end before the start or a longer period answers `422`. Writing needs
  `personal.personal-editieren`.

## Checking a written plan

`GET /api/v1/people/duty/audit?from=&to=` (`src/duty_audit.rs`, personnel right)
checks the plan that stands in the database the way the series check would, and
reports every cell that breaks a rule instead of stopping at the first one:
`{ checked, violations: [{ employee_id, day, reason }] }`. The days around the
period are read as well (a month each way), so a rest period, a run of days or the
weekly hours across the edge of the period count; reported are only the cells of the
period. The cell itself is no neighbour of its own check.

`reason` is one of `min_rest_hours`, `compensation_rest` (less than 24 +
`min_rest_hours` around the compensation day a worked Sunday asks for),
`max_daily_hours`, `max_weekly_hours`,
`max_consecutive_days`, `min_days_off_per_week`, `sunday`, `holiday`, `absence`,
`apprentice_alone` (an apprentice without a non-apprentice in the same shift, the
apprentice shifts excepted — nobody else may enter them), `apprentice_only` (an
apprentice shift for somebody else), `weekend` (a regular late shift on Saturday or
Sunday) and
`excluded_position`. A free day (`FT`, `SoFT`) is no work: it counts no minutes and
is no cell of the plan.

## Printing the plan

The board has a `Drucken` button right of `Import / Export`, next to `Dienstplan
generieren`. It first runs the
check of the month above; a broken rule opens the window of the rule check with the
days and the reasons (`Abbrechen` / `Trotzdem drucken`) and the print window only
opens when the plan is clean or the planner wants it anyway. A quiet plan goes
straight to the print window (`web/DutyPrintDialog.svelte`).

The window prints **one month** — the month of the board, from its first to its last
day — and asks for the paper only: **DIN A3 quer** by default, also `A3 hoch`,
`A4 quer` and `A4 hoch`. It shows the pages as a preview and hands them to the print
dialog of the system, which is where printer, copies, duplex and margins are chosen;
the pages go into an iframe of their own, so only the sheets reach the printer and
not the board. The window keeps a light colour scheme of its own — it is paper, and a
dark session must not turn its menu white on white.

A page carries **one week, and the week begins on Monday**: the first page starts on
the 1st of the month and ends with the Sunday of that week (a month whose 1st is a
Sunday starts with that single day), the following pages run Monday to Sunday, the
last one as far as the month goes. A month becomes three to six sheets. Every page is
built twice: the row of the dates with the disposition of every location below it
(`Standort` in the first column, the events of the day with their **time in bold** and
their title), an **empty row**, the row of the dates again and then the staff **in the
order of the board** — a **thick line** wherever the position changes, so `Meister`,
`Stellwerker` and `Oberbeleuchter` stand apart from each other. The name column of a row
carries **the tint of the board**: the colour of the employee's sub-department, the
department's when it has none (`.blatt .personal tbody tr th.wer`, mixed at 30 % on white
like the board). Each row has two
lines: the name on the left and the hours **right-aligned** under their labels
(`Soll` | `Ist` | `+/-` in the head of the column), then
`Position · Abteilung · Unterabteilung` in the smaller line — `personalZeileText` in
`web/roster-export.ts` writes it, so the print, the XLSX and the PDF carry the same
line, and a part that is empty is dropped (`Meister · Großes Haus`). The line runs at
`ZEILE2_FAKTOR` (0.7) of the row font, so the three parts fit the first column, and it
ends in an ellipsis where a longer combination still overflows — the row keeps its
height. The days follow in the colour of the shift with the **short name and the
time on one line** in the regular weight — `F - 08:00–15:30`, `07:30–16:00` for a free
time window, `Krank` without a time —
and the tint of the board behind the cell: the colour of the shift, `Krank` and `Urlaub`
from the settings, and the violet `FARBE_ZEITFENSTER` for a free time window, which
`web/roster-report.ts` exports so that board and paper can never drift apart —
and the **free text of the cell on the second line**, which the sheet reserves for it
(the note is dropped when a row is shorter than 8 mm, like a second line of eleven
points). The base font is
**11 pt**; a row never grows beyond 9.2 mm — two lines of eleven points need about
that — so the whole staff fits the height of one page, and a full house on small paper
falls back to the size the rows can carry. The footer carries the legend of the shifts
and the number of the page.

**The disposition keeps every event.** Each event of a day gets its own line, and a
title too long for the column wraps to a second line that steps in (`umbrechen`), so
a day with five events shows five — the row grows with the busiest day of its
location instead of squeezing the rest away. The block carries a font of its own
(9 pt) that shrinks to 4,6 pt where the calendar is full, but it never takes the room
the staff needs: it is capped at 40 % of the sheet. `blattmass(bericht, seite)` in
`web/roster-export.ts` computes that once — margins, both fonts, the height of
every dispo row and the wrapped lines themselves — and the preview and the PDF both
read it, so a printed page and an exported one break their lines at the same place.

**Import / Export** sits left of `Drucken` and opens `web/DutyExchangeDialog.svelte`.
The window asks for the month and the year first; both blocks below work on that
month, whatever the board shows.

**Export** hands the month out in three scopes: `Wie Druckansicht`, `Nur Dispo inkl.
Standorte` and `Nur Dienste` — as CSV, XLSX or PDF (`web/roster-export.ts`, no
library). The print scope writes the printed sheet itself: `blaetter()` in the export
module builds every printed page once — the head with the month, the week and the day
of the print, the row of the dates, the disposition of every location, the empty row,
the row of the dates with `Soll` | `Ist` | `+/-`, the staff with the two lines of its
first column and the footer with the legend and the number of the page — and CSV and
XLSX write exactly that grid. The first column of a page is the first block of the
sheet (name, then the three columns of the hours); the day columns follow like the
seven columns of the paper, and a cell that carries two lines on paper keeps them in
the file.

- **CSV**: one block per page, `;` and decimal comma, two-line cells quoted, UTF-8 with
  BOM — what a German Excel expects.
- **XLSX**: one sheet per page, with the look of the sheet: the grey head, the tinted
  shift cells, the tinted name column of the staff (sub-department, else department),
  `frei` in grey, the bold time of an event in the disposition, the borders
  and the thick line between two positions, the row heights and column widths of the
  paper, no grid, and
  the page set up to print on the same paper.
- **PDF**: the printed sheets themselves, drawn by hand (Helvetica, WinAnsi), one page
  per week — for `Nur Dispo` and `Nur Dienste` the same sheet with that block alone; the
  row tint of the name column is drawn as its own rectangle, so a PDF carries the
  sub-department colours like the paper.

## Exchanging a month

`Import / Export` (left of `Drucken`) is the one window for both directions
(`web/DutyExchangeDialog.svelte`). The export hands a month out, the import takes
files back in — the disposition and the plan as the module writes them, and the xlsx
disposition of PeM. All three scopes of the import ask the same four questions:

- **which month** — the month and the year at the top of the window, used by the export
  and the import alike;
- **Vorschau zeigen, bevor geschrieben wird** — `Datei prüfen` reads the file and shows
  what it carries (rows, days, locations or people, the entries the month already holds,
  the warnings) before anything is written; without the switch the same click writes;
- **Einträge dieses Monats ersetzen** — ticked by default, because a sheet is a whole month:
  the month is emptied before the file is written. Without the tick the rows are added to
  what is there (the plan merges cell by cell, the disposition appends) — and appending a
  sheet that already stands in the month is refused with a `409`, so a second click cannot
  double a month;
- **Zeilen ohne Uhrzeit ebenfalls übernehmen** — for the disposition only: a cell like
  `i.A. an die Vorstellung` carries no time and is dropped when the switch is off.

`POST /api/v1/people/duty/exchange/{dispo|plan}/scan?month=&year=&only_with_time=`
reads a file without writing and answers `rows`, `days`, `groups` (the locations of a
disposition, the people of a plan), `warnings`, `existing_entries` and
`already_imported`.

The two exchange sheets are written by `web/roster-exchange.ts`, the import of the
module reads them back, and a file written on one machine lands on the next without a
hand on it. Both are `;`-separated CSV (UTF-8 with BOM) or XLSX with one sheet; the
reader recognises the format by its first bytes (`PK\x03\x04` is XLSX, everything else
CSV, `,` accepted as separator too) and takes the dates as `TT.MM.JJJJ` or
`JJJJ-MM-TT`.

| Sheet | Columns |
| --- | --- |
| `Nur Dispo inkl. Standorte` | `Standort; Datum; Beginn; Titel; Kennzeichen; Ignoriert` |
| `Nur Dienste` | `Personalnummer; Name; Datum; Art; Schicht; Beginn; Ende; Pause; Notiz` |

- The disposition carries **one row per event**, in the order of the locations of the
  master data, then by day and by position inside the day. `Ignoriert` is `ja` for an
  event the planner set aside and empty otherwise; a sheet without that column imports
  every event as planned.
- The plan carries **one row per cell** — `Art` is `Schicht`, `Zeitfenster`, `Krank` or
  `Urlaub`, `Schicht` holds the short name of the shift (also for the free shifts
  `FT`, `FT-W`, `SoFT`), `Beginn`/`Ende`/`Pause` only the window of a `Zeitfenster`,
  and `Notiz` the free text of the cell. It is written for **everybody the plan may hold
  a cell for**, inactive people included — an import with `ersetzen` clears the month,
  and a missing person would lose their days. An employee without a personnel number
  cannot come back and is named in the window instead of being written.
- **A file of another month is laid onto the chosen one**: `exchange::day_in_month`
  keeps the day of the month, so the disposition of December becomes the disposition of
  January, and the window names that move once (`Die Tage der Datei lagen im Dezember
  2026; geschrieben wurde der Januar 2027 (286 Zeilen).`). A day the chosen month does
  not have — the 31st of February — and every other row that does not fit — an unknown
  personnel number, an unknown shift, a window without start or end — is skipped and
  named with its line in `warnings`; the window shows `rows` and `written` and the
  warnings below the buttons. `POST /api/v1/people/duty/exchange/dispo` and
  `…/exchange/plan` (right `personal.personal-editieren`) take `replace` and — for the
  disposition — `only_with_time`; locations the disposition names are created.


## Sending the plan as a calendar

Below the import part of `Import / Export` sits the block `ICS`
(`web/DutyExchangeDialog.svelte`): `Persönlicher Dienstplan senden` opens the list of the
people of the view that have an e-mail address, a checkmark chooses them, `Senden` sends
each one their own calendar. The mail goes to the address in the personnel file — the
sender is the e-mail account of the signed-in user (`"User_ITPrefs"`: `"Emailadress"`,
`"SMTPServer"`, `"SMTPPort"`, `"EmailPassword"`, signature and BCC), which lives in the
installation database and is reached through `AppState::installation_pool()`. An account
without a server or an address is refused before the first message:
`422 Für diesen Benutzer ist kein eMail-Konto eingerichtet.`

`POST /api/v1/people/duty/ical/send` takes `{ "from", "to", "employee_ids" }` and answers
`{ "sent", "skipped": [{ "name", "reason" }] }`; only the people the dispo rights of the
session cover are reached (`personaldispo.personaldispo-lesen`). Who gets nothing is named
with the reason — `keine eMail-Adresse`, `keine Schicht im Zeitraum`, or what the server
answered.

- The calendar carries **one event per shift cell** of the month: `SUMMARY` is the name of
  the shift, `DTSTART`/`DTEND` its real times (a shift that ends before it starts runs past
  midnight), a free day (`band` `frei`) is a whole day (`DTSTART;VALUE=DATE`). The
  `UID` is derived from employee, day and shift, so sending the same month again corrects
  the entries instead of doubling them.
- The body repeats the month as lines — `02.11. - Spätschicht - 15:00 - 23:00` — and
  carries the same list as the description of every event. The file is
  `dienstplan-JJJJ-MM.ics`, `text/calendar; method=PUBLISH`.
- One message per person, sent through `lettre`
  (`crates/where-module-people/src/mail.rs`): port 465 speaks TLS from the first byte, any
  other port starts plain and switches with `STARTTLS` when the server offers it; a
  password is used only when one is stored, so a relay that wants none stays untouched.

A shift cell carries no times of its own — they come from the shift — and the calendar
takes them from there (`ical::clock_of`), so an entry shows the same window the board
shows.

## Exchanging master data

The three master-data pages carry the same `Import / Export`
(`web/MasterExchangeDialog.svelte`) and write CSV that the module reads back unchanged,
so an installation can be set up from a sheet of another one:

| Page | Area | Columns |
| --- | --- | --- |
| `Stammdaten → Abteilungen und Positionen` | `lists` | `Liste; Name; Abteilung; Vorarbeiter` |
| `Stammdaten → Personal` | `employees` | `Personalnummer; Anrede; Titel; Vorname; Nachname; Geburtsdatum; Geburtsort; Nationalität; Familienstand; Straße; PLZ; Ort; Land; E-Mail; Telefon; Mobil; Eintritt; Austritt; Beschäftigung; Wochenstunden; Arbeitstage; Urlaubstage; Position; Abteilung; Unterabteilung; Standort; Kostenstelle; Vorgesetzter; Sozialversicherungsnummer; Steuer-ID; Notiz; Aktiv; Azubi; Automatisch disponierbar; Qualifikationen` |
| `Stammdaten → Dienstplan` | `shifts` | `Kürzel; Name; Beginn; Ende; Pause; Band; Nur Azubis; Farbe; Aktiv; Ausgeschlossene Positionen` |

- `Liste` is `Abteilungen`, `Unterabteilungen`, `Positionen` or `Standorte`; `Abteilung`
  names the department of a sub-department and the sub-department a location belongs to;
  `Vorarbeiter` (`ja`/`nein`) only means something for a location. Everything else in that
  sheet stays empty.
- The reader needs `Liste` and `Name`, `Vorname` and `Nachname`, `Name`, `Beginn` and
  `Ende`; every other column may be missing, and an empty cell keeps the field empty.
  Dates are `JJJJ-MM-TT` or `TT.MM.JJJJ`, hours take a comma or a full stop as the
  decimal mark, and the qualifications are separated by `,` or `;`.
- **The key is what finds a record again**: the name of a list entry (inside its list),
  the personnel number of an employee — or last and first name when there is none — and
  the short name of a shift, or its name when it has no short name.
- `Vorhandene Einträge überschreiben` is off by default: without it the import only
  creates what the file brings and leaves every record it finds alone. With it, a record
  the file names again is overwritten. **Nothing is ever deleted** — a record missing from
  the file stays.
- `Vorschau zeigen, bevor geschrieben wird` is on: `Datei prüfen` reads the file and shows
  `rows`, `created`, `changed` and `unchanged` plus every row it skipped with its line
  before anything is written.

`POST /api/v1/people/masterdata/{lists|employees|shifts}/scan?replace=` reads a file
without writing and answers the same report; `POST …/masterdata/{area}` writes it. Both
take the file as `application/octet-stream` (CSV or XLSX, recognised by its first bytes).
Reading a file needs `personal.personal-lesen`, writing `personal.personal-editieren`;
the lists and the shifts are maintained by the administration in their masks, so their
files ask for the administrative right as well, and the employee sheet — birth dates,
addresses, social security numbers — needs `personal.persoenliche-daten-sehen`.

The sheets are written in `web/master-data-exchange.ts`, read by
`crates/where-module-people/src/masterdata.rs`.