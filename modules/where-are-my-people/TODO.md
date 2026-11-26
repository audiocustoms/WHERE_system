# TODO — WHERE - are my people?

Open work on the personnel module. Nothing on this list is implemented yet. Each
item states what is wanted, what it touches and the questions that have to be
answered before the work starts.

## 1. Lock the schedule after print / export

A plan that has left the house must not change silently. Printing or exporting a
period locks it: from then on only the administration and holders of a dedicated
right may change the cells of that period, and every such change needs a reason,
which is written to a log.

**Work**

- [ ] Print / PDF export of the board (`web/SchedulePage.svelte`): the shown week
      or month with dates, employees, shifts, note rows and staffing demand, in the
      layout of the board itself (print CSS, no second template).
- [ ] Migration (next free number `1943_…`): table `where_people_duty_locks` with
      `from_date`, `to_date`, `locked_at`, `locked_by`, `kind` (`print` / `export`)
      and `file_name`; one row per printed period.
- [ ] Right `personal.dienstplan-nach-export-aendern` ("change a plan that was
      printed") in `where-api/src/authz.rs`, granted to `Administratoren`; the
      regular planning right stays `personal.personal-editieren`.
- [ ] Enforcement on the server, not only in the UI: `ScheduleRepository::set` and
      `set_series` answer `409` for a cell inside a locked period unless the caller
      holds the right.
- [ ] Reason: `PUT /api/v1/people/schedule` and `POST /api/v1/people/schedule/series` get
      an optional `reason`; inside a locked period the API requires it.
- [ ] Change log: table `where_people_duty_changes` (`employee_id`, `day`,
      `old_kind`, `old_shift_id`, `new_kind`, `new_shift_id`, `reason`, `user`,
      `changed_at`), written in the same transaction as the change, plus the same
      line in the log file `data/logs/schedule-lock.log`.
- [ ] Board: banner "Zeitraum gesperrt am … durch …", a `Grund` field in the shift
      menu for locked days, and `Sperre aufheben` in the footer for administrators.
- [ ] Unlocking is itself a logged change with its own reason.
- [ ] Documentation: module README (locking, right, log file), migration counter.

**Open questions**

- Does the lock cover the free-text note rows and the staffing demand of the
  period as well?
- Lock per printed period, or one continuous lock from the first print onwards?
- Is a second print of the same period refused, or allowed without a new lock?
- How long is the change log kept — and is the log file rotated?

**Acceptance**

- Printing a week locks it: a planner without the right gets `409` for every cell
  of that week, also when calling the API directly.
- A holder of the right can change a cell with a reason; the change then appears in
  the change log and in `data/logs/schedule-lock.log`.
- Without a reason nothing is written.

## 2. Import manager for dates

Bring schedule dates in from a file instead of clicking them cell by cell — the
way `Datei → Import-Manager` already reads articles and addresses
(`apps/web/src/lib/import/ImportDialog.svelte`, `POST /api/v1/import/articles`).

**Work**

- [ ] New target in `apps/web/src/lib/import/importFields.ts` (key `duty`, label
      `Dienstplan`) with the fields employee, day, kind (`Schicht`, `Krank`,
      `Urlaub`) and shift (short name or name).
- [ ] Endpoint in the module: `POST /api/v1/people/schedule/import` with rows already
      shaped the way the board sends them; answer as `ImportReport`
      (`{ created, failed: [{ row, message }] }`) so the dialog can name the
      rejected lines.
- [ ] One row at a time, each in its own transaction, like the core import: one
      broken line does not lose the file.
- [ ] Employee matched by personnel number first, then by last and first name;
      unknown and ambiguous names are rejected rows.
- [ ] Shift matched by short name, then by name; an unknown shift is a rejected row
      — no silent creation, shifts are maintained in the duty-plan settings.
- [ ] Imported rows pass the same checks as the board: `duty_rules.rs` (Sunday,
      rest, hours, days off), the apprentice rule and the print lock.
- [ ] Dry run: the dialog shows how many rows would be written and which ones would
      fail before anything is stored.
- [ ] Documentation: module README (target, columns, matching, report).

**Open questions**

- "Dates" could also mean **holidays / a calendar** for the tenant. That exists
  since `migrations/1916_people_holidays.sql` (public holidays and school holidays
  are imported from OpenHolidays), so this item is about schedule entries — unless the
  calendar itself should become importable as well.
- Do imported cells overwrite existing ones, or only fill empty cells?
- Which date format does the file use (`TT.MM.JJJJ`, `JJJJ-MM-TT`, Excel serial
  numbers)?
- Does an import lock the period like a print, or only write the change log?
- Which right gates the import — `personal.personal-editieren` only, or a separate
  one?
