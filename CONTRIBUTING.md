# Contributing

## This repository is generated

Development happens in a private working repository. This repository receives its state
as a commit and is the place where the products are released. Nobody edits it directly —
a direct change would be overwritten by the next export.

That shapes how a contribution travels:

1. **Open an issue.** Describe what is wrong or missing, ideally with a screenshot and the
   path where it happens. Issues are read, they are the most useful contribution.
2. **Pull requests are not merged here.** A change is ported into the working repository
   and reappears here with the next export. Its author is credited in the commit message
   (`Co-authored-by:`), so the history keeps the attribution.
3. An issue first, please — otherwise the same change is written twice.

Security problems do not belong into an issue; write to the maintainer address in the
repository profile.

## Conventions

- **English** for code, comments, SQL, API paths, page paths, identifiers and
  documentation. **German** only where a user reads it in the application: labels,
  buttons, messages, print output.
- Table and column names of the legacy schema are kept unchanged — the data import
  depends on them. New tables and columns are English.
- LF line endings (`.gitattributes`).
- A feature follows the same chain: migration → domain type → repository → route and
  authorization → OpenAPI → client in `apps/web/src/lib/api.ts` → Svelte page →
  verification against a running instance.
- No explanatory helper texts in the user interface. A screen shows labels and controls;
  text is reserved for the state a user has to act on, and then as short as possible.
- A module's working pages live under their own path and check their own right.
  `/admin/*` is master data and settings and requires the administrative right.
- `Esc` cancels and closes in every window — exactly what the cancel button does,
  including the confirmation it asks for.

## Before you send a change

```bash
cargo fmt --all --check
cargo test -p where-api -p where-module-people -p where-server

cd apps/web
npm run check
npm run build
```

Build instructions, the preview stack and the installer builds are in [BUILD.md](BUILD.md).
