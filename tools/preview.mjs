#!/usr/bin/env node
// Local preview: starts every background process the browser preview needs and
// waits until all of them answer.
//
//   databases  PGlite on 5432 (installation) and — when the data directory
//              exists — 5433 / 5434 for the further tenants
//   modules    the modules of DEFAULT_MODULES registered in the installation
//              database (before the api, which takes the single PGlite connection)
//   api        the Axum server on 8090, built first, talking to those databases
//   web        the Vite dev server on 5173
//
// Usage: node tools/preview.mjs [--no-open] [--no-tenants] [--modules people,go]
//                               [--no-modules] [--stop]
//
// Everything runs exactly as in the README section "Local preview — without
// installing PostgreSQL", only started from one place. Ctrl+C stops what this
// script started; a process that was already running keeps running.

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "..");
const logDir = path.join(repo, "data", "logs");
const stateFile = path.join(logDir, "preview.json");

const INSTALLATION = { name: "db", port: 5432, data: "data/pg" };
const TENANTS = [
  { name: "db-m3", port: 5433, data: "data/pg-m3", tenant: 3 },
  { name: "db-m4", port: 5434, data: "data/pg-m4", tenant: 4 },
];
const API = { name: "api", port: 8090 };
const WEB = { name: "vite", port: 5173 };

/** Modules the preview registers so their pages are usable right after it starts. */
const DEFAULT_MODULES = ["people"];

const databaseUrl = (port) => `postgres://postgres:postgres@127.0.0.1:${port}/where`;

const binaryName = (name) => (process.platform === "win32" ? `${name}.exe` : name);

const args = process.argv.slice(2);
const options = {
  open: !args.includes("--no-open"),
  tenants: !args.includes("--no-tenants"),
  modules: modulesFromArgs(args),
  stop: args.includes("--stop"),
  help: args.includes("--help") || args.includes("-h"),
};

/** `--modules people,go`, `--modules=people` or `--no-modules`; default `people`. */
function modulesFromArgs(argv) {
  if (argv.includes("--no-modules")) return [];
  const index = argv.findIndex((arg) => arg === "--modules" || arg.startsWith("--modules="));
  const value =
    index < 0
      ? DEFAULT_MODULES.join(",")
      : argv[index].includes("=")
        ? argv[index].slice("--modules=".length)
        : (argv[index + 1] ?? "");
  return value
    .split(",")
    .map((id) => id.trim().toLowerCase())
    .filter(Boolean);
}

/** `{ name, pid, port }` of everything this script started or took over. */
const running = [];
/** Pids this very run spawned — an adopted process is not ours to end. */
const owning = new Set();
let shuttingDown = false;

// ---------------------------------------------------------------- utilities

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

const say = (text = "") => console.log(text);

/** A line that shows progress dots until `done(result)` closes it. */
function step(text) {
  process.stdout.write(`  ${text} ...`);
  const dots = setInterval(() => process.stdout.write("."), 700);
  return {
    done(result) {
      clearInterval(dots);
      console.log(` ${result}`);
    },
  };
}

function alive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function portFree(port) {
  return new Promise((done) => {
    const socket = net.connect({ port, host: "127.0.0.1" });
    const finish = (free) => {
      socket.destroy();
      done(free);
    };
    socket.once("connect", () => finish(false));
    socket.once("error", () => finish(true));
    socket.setTimeout(600, () => finish(true));
  });
}

async function waitForPort(port, timeoutMs) {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until && !shuttingDown) {
    if (!(await portFree(port))) return true;
    await sleep(300);
  }
  return false;
}

async function waitForHttp(url, timeoutMs) {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until && !shuttingDown) {
    try {
      const answer = await fetch(url, { signal: AbortSignal.timeout(2500) });
      if (answer.ok) return true;
    } catch {
      // not listening yet
    }
    await sleep(400);
  }
  return false;
}

function readState() {
  try {
    return JSON.parse(fs.readFileSync(stateFile, "utf8"));
  } catch {
    return { processes: [] };
  }
}

function writeState() {
  fs.mkdirSync(logDir, { recursive: true });
  fs.writeFileSync(
    stateFile,
    JSON.stringify({ startedAt: new Date().toISOString(), processes: running }, null, 2)
  );
}

/**
 * A process from an earlier run that is still there. Its port has to be taken as
 * well: a recorded pid that no longer listens is a pid Windows handed to some
 * other program in the meantime, and that one must not be touched.
 */
async function adopted(name, port) {
  const known = readState().processes?.find((entry) => entry.name === name);
  if (!known || !alive(known.pid)) return null;
  if (await portFree(port)) return null;
  const entry = { name: known.name, pid: known.pid, port };
  running.push(entry);
  return entry;
}

function start(name, command, commandArgs, cwd, env = {}, port = null) {
  fs.mkdirSync(logDir, { recursive: true });
  const outPath = path.join(logDir, `${name}.log`);
  const errPath = path.join(logDir, `${name}.err.log`);
  fs.appendFileSync(outPath, `\n--- ${new Date().toISOString()} ---\n`);

  // Handed to the child as descriptors instead of piping: the log reaches the
  // file even when this script is killed in the middle of a line.
  const outFd = fs.openSync(outPath, "a");
  const errFd = fs.openSync(errPath, "a");
  const child = spawn(command, commandArgs, {
    cwd,
    env: { ...process.env, ...env },
    stdio: ["ignore", outFd, errFd],
    windowsHide: true,
  });
  fs.closeSync(outFd);
  fs.closeSync(errFd);

  child.on("error", (cause) => {
    console.error(`\n  ! ${name} could not start: ${cause.message}`);
    process.exitCode = 1;
  });
  child.on("exit", (code, signal) => {
    const index = running.findIndex((entry) => entry.pid === child.pid);
    if (index >= 0) running.splice(index, 1);
    if (shuttingDown) return;

    // A missing state file means `--stop` ran: it ends the processes and the
    // watcher below ends this call quietly. Nothing to report here.
    if (!fs.existsSync(stateFile)) return;

    console.error(
      `\n  ! ${name} ended (${signal ?? `exit code ${code}`}) - see data/logs/${name}.err.log`
    );
    if (running.length === 0) {
      say("  every process is gone - this preview ends as well");
      void shutdown(1);
    }
  });

  running.push({ name, pid: child.pid, port });
  owning.add(child.pid);
  writeState();
  return child;
}

/**
 * Ends the given processes, but only those that still own their port. Waits for
 * the ports to fall free so a restart right afterwards does not hit a busy one.
 */
async function endAll(entries) {
  const killed = [];
  for (const entry of [...entries].reverse()) {
    if (!alive(entry.pid)) {
      say(`  ${entry.name.padEnd(8)} was not running any more`);
      continue;
    }
    if (entry.port !== null && (await portFree(entry.port))) {
      say(`  ${entry.name.padEnd(8)} pid ${entry.pid} is another program now - left alone`);
      continue;
    }
    try {
      process.kill(entry.pid);
      killed.push(entry);
      say(`  ${entry.name.padEnd(8)} stopped (pid ${entry.pid})`);
    } catch (cause) {
      say(`  ${entry.name.padEnd(8)} could not be stopped: ${cause.message}`);
    }
  }
  if (killed.length === 0) return;

  const until = Date.now() + 15_000;
  while (Date.now() < until) {
    const busy = [];
    for (const entry of killed) if (!(await portFree(entry.port))) busy.push(entry.port);
    if (busy.length === 0) return;
    await sleep(250);
  }
  const busy = [];
  for (const entry of killed) if (!(await portFree(entry.port))) busy.push(entry.port);
  if (busy.length) say(`  still in use: ${busy.join(", ")}`);
}

function hasDependencies(dir, marker) {
  return fs.existsSync(path.join(dir, marker));
}

function installDependencies(dir, what) {
  const progress = step(`installing ${what} dependencies (npm install)`);
  const npm = process.platform === "win32" ? "npm.cmd" : "npm";
  const result = spawnSync(npm, ["install", "--no-audit", "--no-fund"], {
    cwd: dir,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  progress.done(result.status === 0 ? "ok" : "failed");
  return result.status === 0;
}

function openBrowser(url) {
  const [command, commandArgs] =
    process.platform === "win32"
      ? ["cmd.exe", ["/c", "start", "", url]]
      : process.platform === "darwin"
        ? ["open", [url]]
        : ["xdg-open", [url]];
  spawn(command, commandArgs, { stdio: "ignore", detached: true }).unref();
}

// ---------------------------------------------------------------------- tasks

async function startDatabase(database) {
  const known = await adopted(database.name, database.port);
  if (known) return { state: "runs already", pid: known.pid };

  if (!(await portFree(database.port))) {
    return { state: `port ${database.port} is taken by another program` };
  }
  if (!fs.existsSync(path.join(repo, database.data))) {
    return { state: `skipped, ${database.data} does not exist` };
  }

  const child = start(
    database.name,
    process.execPath,
    [path.join(here, "dev-db.mjs"), "--port", String(database.port), "--data", database.data],
    repo,
    {},
    database.port
  );
  const ready = await waitForPort(database.port, 240_000);
  return ready
    ? { state: "started", pid: child.pid }
    : { state: `no answer on port ${database.port} - see data/logs/${database.name}.err.log` };
}

function appendLog(name, text) {
  fs.mkdirSync(logDir, { recursive: true });
  fs.appendFileSync(
    path.join(logDir, `${name}.log`),
    `\n--- ${new Date().toISOString()} ---\n${text}\n`
  );
}

/** Whether an api from an earlier run still answers on its port (read only). */
async function apiRunsAlready() {
  const known = readState().processes?.find((entry) => entry.name === API.name);
  return Boolean(known && alive(known.pid) && !(await portFree(API.port)));
}

let serverBuild = null;

/** Builds `where-server` and `where-installer` — once per run, both callers share it. */
function buildServer() {
  if (serverBuild) return serverBuild;
  say(`  ${"api".padEnd(26)} building (cargo build -p where-server)`);
  const cargo = spawnSync("cargo", ["build", "-p", "where-server"], {
    cwd: repo,
    stdio: "inherit",
  });
  serverBuild =
    cargo.error || cargo.status !== 0
      ? { ok: false, state: "build failed" }
      : { ok: true, state: "built" };
  return serverBuild;
}

/**
 * Registers the preview modules in the installation database. Runs before the api:
 * PGlite serves one connection at a time, so a running api would block the
 * installer. Modules that are already registered stay as they are, which keeps
 * every further start cheap.
 */
async function installModules() {
  const column = "modules".padEnd(26);

  if (options.modules.length === 0) {
    say(`  ${column} none requested`);
    return { ok: true, state: "none requested" };
  }
  if (await apiRunsAlready()) {
    const state = "skipped, the api holds the database - restart the preview to register";
    say(`  ${column} ${state}`);
    return { ok: true, state };
  }

  const build = buildServer();
  if (!build.ok) {
    say(`  ${column} ${build.state}`);
    return { ok: false, state: build.state };
  }
  const installer = path.join(repo, "target", "debug", binaryName("where-installer"));
  if (!fs.existsSync(installer)) {
    const state = "installer binary missing after the build";
    say(`  ${column} ${state}`);
    return { ok: false, state };
  }

  const registered = [];
  for (const id of options.modules) {
    const progress = step(`${column} ${id}`);
    const result = spawnSync(installer, ["--module", id], {
      cwd: repo,
      env: {
        ...process.env,
        DATABASE_URL: databaseUrl(INSTALLATION.port),
        // The same mode the api runs with — see the README on PGlite.
        DB_SHARED_BACKEND: "1",
        // dev-db.mjs applies the migrations itself.
        RUN_MIGRATIONS: "0",
      },
      encoding: "utf8",
    });
    appendLog("installer", `--module ${id}\n${result.stdout ?? ""}${result.stderr ?? ""}`);
    if (result.status !== 0) {
      progress.done("failed - see data/logs/installer.log");
      return { ok: false, state: `failed (${id})`, modules: registered };
    }
    registered.push(id);
    progress.done("registered");
  }
  return { ok: true, state: "registered", modules: registered };
}

async function startApi(tenantUrls) {
  const label = `${"api".padEnd(26)} port ${API.port}`;

  const known = await adopted(API.name, API.port);
  if (known) {
    say(`  ${label} ... runs already`);
    return { state: "runs already", pid: known.pid };
  }
  if (!(await portFree(API.port))) {
    const state = `port ${API.port} is taken by another program`;
    say(`  ${label} ... ${state}`);
    return { state };
  }

  // The build prints its own progress, so it gets a line of its own.
  const build = buildServer();
  if (!build.ok) {
    say(`  ${label} ... ${build.state}`);
    return { state: build.state };
  }

  const binary = path.join(repo, "target", "debug", binaryName("where-server"));
  if (!fs.existsSync(binary)) {
    const state = "binary missing after the build";
    say(`  ${label} ... ${state}`);
    return { state };
  }

  const progress = step(label);
  const child = start(
    API.name,
    binary,
    [],
    repo,
    {
      DATABASE_URL: databaseUrl(INSTALLATION.port),
      BIND_ADDR: `127.0.0.1:${API.port}`,
      // PGlite shares one engine across connections — see the README.
      DB_SHARED_BACKEND: "1",
      // dev-db.mjs applies the migrations itself.
      RUN_MIGRATIONS: "0",
      TENANT_DATABASES: tenantUrls.join(";"),
    },
    API.port
  );

  const ready = await waitForHttp(`http://127.0.0.1:${API.port}/health`, 120_000);
  const state = ready ? "started" : `no answer on /health - see data/logs/${API.name}.err.log`;
  progress.done(state);
  return { state, pid: child.pid };
}

async function startWeb() {
  const known = await adopted(WEB.name, WEB.port);
  if (known) return { state: "runs already", pid: known.pid };

  if (!(await portFree(WEB.port))) {
    return { state: `port ${WEB.port} is taken by another program` };
  }

  const webDir = path.join(repo, "apps", "web");
  const vite = path.join(webDir, "node_modules", "vite", "bin", "vite.js");
  if (!fs.existsSync(vite)) return { state: "vite is not installed in apps/web" };

  // Straight through node instead of `npm run dev`: no shell in between, so the
  // process this script ends is the dev server itself.
  const child = start(
    WEB.name,
    process.execPath,
    [vite, "dev", "--host", "127.0.0.1", "--port", String(WEB.port), "--strictPort"],
    webDir,
    {},
    WEB.port
  );

  const ready = await waitForHttp(`http://127.0.0.1:${WEB.port}/`, 120_000);
  return ready
    ? { state: "started", pid: child.pid }
    : { state: `no answer on port ${WEB.port} - see data/logs/${WEB.name}.err.log` };
}

// ------------------------------------------------------------------- commands

function printHelp() {
  say(`WHERE preview

  node tools/preview.mjs                start databases, modules, api and web, open the browser
  node tools/preview.mjs --no-open      the same, without opening the browser
  node tools/preview.mjs --no-tenants   only the installation database (port 5432)
  node tools/preview.mjs --no-modules   do not register any module
  node tools/preview.mjs --modules go,people
                                        register these modules (default: ${DEFAULT_MODULES.join(",")})
  node tools/preview.mjs --stop         stop everything an earlier run started

Ports: 5432 / 5433 / 5434 (databases), 8090 (api), 5173 (web)
Logs:  data/logs/<name>.log and <name>.err.log`);
}

async function stopAll() {
  const processes = readState().processes ?? [];
  if (processes.length === 0) {
    say("Nothing to stop: no preview is recorded as running.");
    return;
  }
  // Gone first: a running preview sees the missing file, knows it was asked to
  // stop and ends without reporting every process as a failure.
  fs.rmSync(stateFile, { force: true });
  say("Stopping the preview");
  await endAll(processes);
  say("Done.");
}

async function shutdown(code) {
  shuttingDown = true;
  say("\nStopping the preview");
  await endAll(running);
  fs.rmSync(stateFile, { force: true });
  process.exit(code);
}

async function main() {
  if (options.help) {
    printHelp();
    return;
  }
  if (options.stop) {
    await stopAll();
    return;
  }

  say("WHERE preview");

  // Registered before the first process starts: Ctrl+C while the preview is
  // still coming up must not leave orphans behind.
  const stopRequested = new Promise((done) => {
    const request = () => {
      shuttingDown = true;
      done();
    };
    process.once("SIGINT", request);
    process.once("SIGTERM", request);
  });

  if (!hasDependencies(here, "node_modules/@electric-sql/pglite")) {
    if (!installDependencies(here, "database")) {
      say("\nThe database package is missing - run `npm install` in tools.");
      process.exitCode = 1;
      return;
    }
  }
  if (!hasDependencies(path.join(repo, "apps", "web"), "node_modules/vite")) {
    if (!installDependencies(path.join(repo, "apps", "web"), "web")) {
      say("\nThe web packages are missing - run `npm install` in apps/web.");
      process.exitCode = 1;
      return;
    }
  }

  // 1. Databases
  const databases = options.tenants ? [INSTALLATION, ...TENANTS] : [INSTALLATION];
  const tenantUrls = [];

  for (const database of databases) {
    const label = database.tenant
      ? `${database.name} (tenant ${database.tenant})`
      : `${database.name} (installation)`;
    const progress = step(`${label.padEnd(26)} port ${String(database.port).padEnd(4)}`);
    const result = await startDatabase(database);
    progress.done(result.state);
    if (database.tenant && (result.state === "started" || result.state === "runs already")) {
      tenantUrls.push(
        `${database.tenant}=postgres://postgres:postgres@127.0.0.1:${database.port}/where`
      );
    }
  }

  if (await portFree(INSTALLATION.port)) {
    say("\nThe installation database is not running - the api needs it. Stopping here.");
    await shutdown(1);
    return;
  }

  // 2. Modules — before the api, which would hold the only PGlite connection.
  const modules = await installModules();
  if (!modules.ok) {
    say("\nA module could not be registered - the preview continues without it.");
  }

  // 3. API
  const api = await startApi(tenantUrls);
  if (api.state !== "started" && api.state !== "runs already") {
    say("\nThe api is not running - the web preview needs it. Stopping here.");
    await shutdown(1);
    return;
  }

  // 4. Web
  const webProgress = step(`${"web".padEnd(26)} port ${WEB.port}`);
  const web = await startWeb();
  webProgress.done(web.state);

  if (shuttingDown) {
    await shutdown(0);
    return;
  }

  // 5. Summary
  const webUrl = `http://localhost:${WEB.port}`;
  say("");
  say("WHERE preview is up");
  say("");
  say(`  web         ${webUrl}`);
  say(`  api         http://127.0.0.1:${API.port}/api/v1/`);
  say(`  openapi     http://127.0.0.1:${API.port}/swagger-ui`);
  if (modules.modules?.length) {
    say(`  modules     ${modules.modules.join(", ")}`);
  }
  if (tenantUrls.length) {
    const ids = tenantUrls.map((url) => url.split("=")[0]).join(", ");
    say(`  tenants     ${ids} on their own databases`);
  }
  say("");
  say("  logs        data/logs/<name>.log");
  say("  stop        Ctrl+C   (or: node tools/preview.mjs --stop)");

  if (options.open && web.state !== "failed") openBrowser(webUrl);

  // A second call next to a running preview only reports: the first one owns the
  // processes, so this one must not sit and wait for a Ctrl+C that ends nothing.
  if (owning.size === 0) {
    say("");
    say("Nothing was started - everything was already running. Nothing to watch.");
    return;
  }

  // `--stop` removes the state file before it ends the processes; that is the
  // signal for this call to leave as well, without reporting each one as failed.
  const stoppedElsewhere = new Promise((done) => {
    const timer = setInterval(() => {
      if (!fs.existsSync(stateFile)) {
        clearInterval(timer);
        done();
      }
    }, 400);
  });

  await Promise.race([stopRequested, stoppedElsewhere]);
  if (!fs.existsSync(stateFile)) {
    say("\nStopped with `--stop`.");
    process.exit(0);
  }
  await shutdown(0);
}

await main();
