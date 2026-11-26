# Building WHERE

Two things can be built from this repository: the development stack (API plus SPA) and
the installers of the products. Everything below assumes the repository root as the
working directory.

## Requirements

| What | Version | Needed for |
| --- | --- | --- |
| Rust | 1.85 or newer (edition 2024) | everything |
| Node.js | 20 or newer, 22 recommended | SPA, tools, Tauri CLI |
| Windows | Visual Studio Build Tools with the C++ workload, WebView2 runtime | desktop build |
| PostgreSQL | 13 or newer | only if you do not use the embedded development database |

Tauri downloads NSIS and WiX by itself on the first bundle. A Linux or macOS installer is
not supported yet — see [Roadmap](README.md#roadmap).

## Development stack

The preview needs no database installation: it starts a PostgreSQL build for the browser
(PGlite), registers the modules, builds the API and serves the SPA.

```bash
npm install --prefix tools
node tools/preview.mjs --no-open
```

| Address | What |
| --- | --- |
| <http://localhost:5173> | SPA (Vite dev server, `/api` proxied to the API) |
| <http://127.0.0.1:8090/api/v1/> | API |
| <http://127.0.0.1:8090/swagger-ui> | OpenAPI |
| <http://127.0.0.1:8090/health> | health check |

`node tools/preview.mjs --stop` stops everything, `--modules go,people` installs another
set of modules, `--no-modules` none. Logs are written to `data/logs/`. A fresh database
starts with the initial administrator (password change on the first login); `data/` is
ignored by git, so every checkout keeps its own database.

## Backend on its own

```bash
cp .env.example .env        # point DATABASE_URL at your PostgreSQL
cargo run -p where-server   # applies the migrations on start
```

The API answers on `127.0.0.1:8090`. `--config <path>`, `WHERE_CONFIG` or the per-user
configuration file change the settings; environment variables override the file.

## Frontend on its own

```bash
cd apps/web
npm install
npm run dev                 # http://localhost:5173, /api proxied to 127.0.0.1:8090
```

The SPA is a static build (`adapter-static`, `ssr = false`) — web, desktop and the
Android shell use the same artefact. Module front ends live in `modules/<product>/web` and
are mounted by thin route files under `apps/web/src/routes`; the `$core` and `$modules`
aliases in `vite.config.ts` and `tsconfig.json` point there.

## Checks

```bash
cargo fmt --all --check
cargo test -p where-api -p where-module-people -p where-server

cd apps/web
npm run check               # svelte-check
npm run build
```

`cargo test --workspace` also builds the desktop shell, whose build script fetches
PostgreSQL binaries — it needs network access on a clean machine and is not part of CI.

## Windows installers

```bash
cd apps/desktop
npm install
npm run installer          # WHERE - is my stuff?    de.where.warehouse
npm run installer:people   # WHERE - are my people?  de.where.are-my-people
```

One run does three things: build the SPA (`beforeBuildCommand`), build `where-server` in
release mode and copy it into `src-tauri/binaries` as the sidecar, then bundle with Tauri.
The installers land in `target/release/bundle/nsis/`.

What an installation is, is decided by the bundle identifier alone:

| | `WHERE - is my stuff?` | `WHERE - are my people?` |
| --- | --- | --- |
| Bundle identifier | `de.where.warehouse` | `de.where.are-my-people` |
| Tauri configuration | `src-tauri/tauri.conf.json` | `src-tauri/tauri.people.conf.json` |
| Module registered at the first start | `stuff` (core) | `people` |
| Data directory | `%LOCALAPPDATA%\where-data` | the same |
| Embedded PostgreSQL / API | `5433` / `8091` | the same |
| First view | the dashboard (`/`) | the schedule (`/people/schedule`) |

Both products share one data directory and therefore one database. Whichever starts first
creates it and is recorded in `where_installation.database_owner`; the other one joins.
Because they share the embedded instance, only one of them runs at a time. The NSIS
uninstaller deletes `%LOCALAPPDATA%\<identifier>` when "delete app data" is ticked — which
is exactly why the data directory is called `where-data` and not after a bundle
identifier. An earlier version kept its data in `%LOCALAPPDATA%\de.where.lagerlogistik`;
that folder is moved to `where-data` on the first start.

## Central server

The API hands out `/api/v1`, `/swagger-ui`, `/health` and `/ready` — no SPA. On a server,
a web server serves `apps/web/build` and proxies the loopback API, so everything lives on
one origin and neither CORS nor `VITE_API_BASE` is needed.

```bash
cargo build --release -p where-server
npm --prefix apps/web ci && npm --prefix apps/web run build

install -D -m 755 target/release/where-server /opt/where/where-server
rsync -a --delete apps/web/build/ /srv/where/web/
```

`/etc/where/config.toml` — mode `640`, the file holds the database password:

```toml
product = "stuff"                       # or "people" — a module may stand alone
bind = "127.0.0.1:8090"
database_url = "postgres://where:<password>@127.0.0.1:5432/where"
run_migrations = true
file_storage_dir = "/var/lib/where/files"
```

The service needs `WHERE_DATA_DIR` set, because a service user has no reliable home
directory to fall back to:

```ini
[Service]
User=where
WorkingDirectory=/var/lib/where
Environment=WHERE_DATA_DIR=/var/lib/where
ExecStart=/opt/where/where-server --config /etc/where/config.toml
Restart=always
```

Workstations of that server run in `mode = "client"` and only need `api_base` — they
draw the window and talk to the API, no local database.

## Configuration

Layers: built-in defaults, then the TOML file, then environment variables.

| Key / variable | Meaning |
| --- | --- |
| `mode` / `WHERE_MODE` | `standalone` (embedded database) or `client` (API elsewhere) |
| `api_base` / `WHERE_API_BASE` | API address, required in `client` mode |
| `database_url` / `DATABASE_URL` | PostgreSQL connection of the API |
| `bind` / `BIND_ADDR` | listen address, default `127.0.0.1:8090` |
| `max_connections` / `DB_MAX_CONNECTIONS` | connection pool size |
| `run_migrations` / `RUN_MIGRATIONS` | apply the embedded migrations on start |
| `file_storage_dir` / `FILE_STORAGE_DIR` | absolute path for uploaded files |
| `cors_allowed_origins` / `CORS_ALLOWED_ORIGINS` | allowed browser origins |
| `WHERE_DATA_DIR` | base directory for data, binaries and `config.toml` |
| `WHERE_PRODUCT` | product of this installation (`stuff`, `people`) |
| `DB_SHARED_BACKEND` | `1` for poolers and the PGlite development database |

## Troubleshooting

- **Ports.** 8090 for the API, 5173 for the Vite dev server, 5432–5434 for the PGlite
  instances of the preview. A second stack on the same machine needs other ports.
- **Migrations.** Applied on start unless `run_migrations` is `false`. `data/logs/db.log`
  names the schema version; `node tools/verify-schema.mjs` checks the SQL against a real
  PostgreSQL engine.
- **Empty window.** The desktop bundle needs a built SPA — `npm --prefix apps/web run
  build`, or use `npm run installer`, which builds it.
- **No installer output.** Check that the sidecar was copied: `npm run prepare:server`
  prints its path, and `cargo build -p where-server --release` must have run first.
