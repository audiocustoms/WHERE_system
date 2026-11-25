//! Local runtime of the single-workstation installation.
//!
//! Starts an embedded PostgreSQL in the per-user data directory, then the API
//! on the loopback interface, and stops both again when the window closes.
//! The central-server installation does not use this module at all.

use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use postgresql_embedded::{PostgreSQL, Settings as PgSettings, V18};
use where_server::Settings;

/// Handle of the running local stack.
pub struct Standalone {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<std::thread::JoinHandle<()>>,
    state: Arc<Mutex<Option<String>>>,
    api_base: String,
}

impl Standalone {
    /// Starts the embedded database and the API, then waits until the API
    /// accepts connections.
    ///
    /// `resources` is the resource directory of the bundle; the demo dataset is
    /// looked up there and next to the executable.
    pub fn start(settings: &Settings, resources: Option<std::path::PathBuf>) -> Result<Self> {
        let api_base = format!("http://{}", settings.bind);
        let api_addr: SocketAddr = settings.socket_addr()?;
        let state = Arc::new(Mutex::new(None));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let (ready_tx, ready_rx) = channel::<Result<()>>();

        let thread_settings = settings.clone();
        let thread_state = Arc::clone(&state);
        let join = std::thread::Builder::new()
            .name("where-standalone".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready_tx.send(Err(anyhow!("tokio runtime: {error}")));
                        return;
                    }
                };
                let outcome =
                    runtime.block_on(run_stack(thread_settings, resources, shutdown_rx, ready_tx));
                if let Err(error) = outcome {
                    let message = format!("{error:#}");
                    tracing::error!("{message}");
                    *thread_state.lock().unwrap() = Some(message);
                }
            })
            .context("spawning the local stack thread")?;

        match ready_rx.recv_timeout(Duration::from_secs(300)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = join.join();
                return Err(error);
            }
            Err(RecvTimeoutError::Timeout) => {
                return Err(anyhow!(
                    "the embedded database did not become ready within five minutes"
                ));
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = join.join();
                let detail = state
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| "no reason reported".to_string());
                return Err(anyhow!("the local stack stopped during start-up: {detail}"));
            }
        }

        wait_for_api(api_addr, Duration::from_secs(60), &state)
            .with_context(|| format!("waiting for the API on {api_addr}"))?;

        Ok(Self {
            shutdown: Some(shutdown_tx),
            join: Some(join),
            state,
            api_base,
        })
    }

    /// Address the web view should talk to.
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// Stops the API and the database.
    ///
    /// Returns the error the stack reported, if it stopped on its own.
    pub fn stop(mut self) -> Option<String> {
        if let Some(sender) = self.shutdown.take() {
            let _ = sender.send(());
        }
        if let Some(join) = self.join.take() {
            let (done_tx, done_rx) = channel();
            std::thread::spawn(move || {
                let _ = join.join();
                let _ = done_tx.send(());
            });
            if done_rx.recv_timeout(Duration::from_secs(25)).is_err() {
                tracing::warn!("local stack did not stop within 25 s");
            }
        }
        self.state.lock().unwrap().clone()
    }
}

impl Drop for Standalone {
    fn drop(&mut self) {
        if let Some(sender) = self.shutdown.take() {
            let _ = sender.send(());
        }
    }
}

async fn run_stack(
    settings: Settings,
    resources: Option<std::path::PathBuf>,
    shutdown: tokio::sync::oneshot::Receiver<()>,
    ready: std::sync::mpsc::Sender<Result<()>>,
) -> Result<()> {
    let embedded = &settings.embedded;
    std::fs::create_dir_all(Settings::data_root())
        .with_context(|| format!("creating {}", Settings::data_root().display()))?;
    tracing::info!(
        data_dir = %embedded.data_dir.display(),
        port = embedded.port,
        "starting the embedded database"
    );

    let mut database = PostgreSQL::new(PgSettings {
        // The `bundled` archive decides the effective version; the requirement
        // only documents the generation the schema was verified against.
        version: (*V18).clone(),
        host: "127.0.0.1".to_string(),
        port: embedded.port,
        username: embedded.username.clone(),
        password: embedded.password.clone(),
        data_dir: embedded.data_dir.clone(),
        installation_dir: embedded.installation_dir.clone(),
        password_file: Settings::data_root().join("pgpass"),
        temporary: false,
        // `initdb` needs more than the crate's five second default.
        timeout: Some(Duration::from_secs(180)),
        ..Default::default()
    });

    stop_stale_database(embedded);
    ensure_installation(embedded);

    database.setup().await.context("preparing PostgreSQL")?;
    database.start().await.context("starting PostgreSQL")?;
    tracing::info!("embedded database is running");

    if !database
        .database_exists(&embedded.database)
        .await
        .context("checking the database")?
    {
        database
            .create_database(&embedded.database)
            .await
            .context("creating the database")?;
        tracing::info!(database = %embedded.database, "database created");
        if settings.product == where_server::Product::Stuff {
            load_demo_dataset(&settings, resources.as_deref()).await?;
        } else {
            // Only the warehouse application ships a demonstration dataset.
            tracing::info!(
                product = settings.product.key(),
                "starting with an empty database"
            );
        }
    }

    let serve = where_server::serve_until(settings.clone(), async move {
        let _ = shutdown.await;
        tracing::info!("local stack: shutdown requested");
    });

    let _ = ready.send(Ok(()));
    let result = serve.await;

    if let Err(error) = database.stop().await {
        tracing::warn!("stopping PostgreSQL: {error}");
    }
    result
}

/// Removes a PostgreSQL installation directory that was never extracted completely.
///
/// `postgresql_embedded` treats an existing installation directory as installed and
/// skips the extraction, so an emptied `pg-bin` would leave `initdb` missing and the
/// start-up would fail with a bare file-not-found. Removing the incomplete directory
/// makes the next `setup` extract the bundled archive again.
fn ensure_installation(embedded: &where_server::Embedded) {
    let dir = &embedded.installation_dir;
    if !dir.is_dir() {
        return;
    }
    let bin = dir.join("bin");
    if bin.join("initdb.exe").is_file() || bin.join("initdb").is_file() {
        return;
    }
    tracing::warn!(
        installation_dir = %dir.display(),
        "the PostgreSQL installation is incomplete, removing it"
    );
    if let Err(error) = std::fs::remove_dir_all(dir) {
        tracing::error!("removing the incomplete installation failed: {error}");
    }
}

/// Stops a database that a killed application left behind, or removes its
/// stale process file.
///
/// Closing the window shuts PostgreSQL down, but a crash, the task manager or a
/// forced end leaves the postmaster running (and the port bound) or a
/// `postmaster.pid` behind that no longer belongs to a process. `pg_ctl status`
/// tells the two apart: only a running server is stopped, a stale file is
/// removed so the next start is not blocked.
fn stop_stale_database(embedded: &where_server::Embedded) {
    let pid_file = embedded.data_dir.join("postmaster.pid");
    if !pid_file.is_file() {
        return;
    }
    let pg_ctl = embedded.installation_dir.join("bin").join("pg_ctl.exe");
    if !pg_ctl.is_file() {
        return;
    }

    let running = std::process::Command::new(&pg_ctl)
        .arg("-D")
        .arg(&embedded.data_dir)
        .arg("status")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !running {
        match std::fs::remove_file(&pid_file) {
            Ok(()) => tracing::warn!("removed a stale postmaster.pid of a previous run"),
            Err(error) => tracing::warn!("removing the stale postmaster.pid failed: {error}"),
        }
        return;
    }

    tracing::warn!("a database of a previous run is still active, stopping it");
    let outcome = std::process::Command::new(&pg_ctl)
        .arg("-D")
        .arg(&embedded.data_dir)
        .args(["-m", "fast", "-w", "-t", "30", "stop"])
        .output();
    match outcome {
        Ok(output) if output.status.success() => tracing::info!("previous database stopped"),
        Ok(output) => {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            tracing::warn!("stopping the previous database failed: {message}");
            let _ = std::fs::remove_file(&pid_file);
        }
        Err(error) => tracing::warn!("calling pg_ctl failed: {error}"),
    }
}

/// Loads the bundled demo dataset into the freshly created database.
///
/// The installer ships `demo.sql`, a dump of a seeded database (13 addresses,
/// 40 articles with stock, prices, suppliers and a pyro record). Without the
/// file — a source checkout, or an installation without resources — the schema
/// is created by the migrations alone and the database starts empty.
async fn load_demo_dataset(settings: &Settings, resources: Option<&std::path::Path>) -> Result<()> {
    let Some(path) = demo_script(resources) else {
        tracing::info!("no demo dataset found, starting with an empty database");
        return Ok(());
    };
    let script = std::fs::read_to_string(&path)
        .with_context(|| format!("reading the demo dataset {}", path.display()))?;
    let url = settings
        .resolved_database_url()
        .context("no database configured")?;
    where_server::apply_script(&url, script)
        .await
        .with_context(|| format!("loading the demo dataset {}", path.display()))?;
    tracing::info!(file = %path.display(), "demo dataset loaded");
    Ok(())
}

/// Looks for `demo.sql` in the resource directory and next to the executable.
fn demo_script(resources: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(dir) = resources {
        candidates.push(dir.join("demo.sql"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("demo.sql"));
        candidates.push(dir.join("resources").join("demo.sql"));
    }
    // Development builds run from `target/<profile>`; the dataset stays in the
    // crate so a `cargo run` behaves like the packaged application.
    if cfg!(debug_assertions) {
        candidates.push(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("demo.sql"),
        );
    }
    candidates.into_iter().find(|path| path.is_file())
}

/// Waits until something accepts TCP connections on `address`.
///
/// A successful connect is enough: the API binds the listener only after the
/// database answered and the migrations ran. `state` carries the reason a stack
/// stopped on its own, so a start-up failure is reported instead of a timeout.
pub fn wait_for_api(
    address: SocketAddr,
    timeout: Duration,
    state: &Mutex<Option<String>>,
) -> Result<()> {
    let started = Instant::now();
    loop {
        if TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok() {
            return Ok(());
        }
        if let Some(detail) = state.lock().unwrap().clone() {
            return Err(anyhow!("the local stack stopped during start-up: {detail}"));
        }
        if started.elapsed() > timeout {
            return Err(anyhow!("no answer within {timeout:?}"));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
