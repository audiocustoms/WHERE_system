#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Desktop shell.
//!
//! The UI is the same static SvelteKit build the Axum server serves; Tauri only
//! provides the window, printing and file-system access. It talks to the API
//! over HTTP, so there is no Tauri IPC layer to maintain.
//!
//! Two deployment shapes are supported by the same binary:
//!
//! - **single workstation** (`mode = "standalone"`): an embedded PostgreSQL and
//!   the API run locally and are stopped together with the window,
//! - **workstation of a central server** (`mode = "client"`): the window only
//!   talks to `api_base`, nothing runs locally.

mod runtime;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use where_server::{Mode, Product, Settings};

/// The local stack, kept alive for the lifetime of the window.
struct Local(Mutex<Option<runtime::Standalone>>);

fn main() {
    // The bundle this binary was built as names the product — the same source the
    // window title and the module the installation registers.
    let context = tauri::generate_context!();
    let product = product_of(&context);

    init_logging();

    let settings = match load_settings(product) {
        Ok(settings) => settings,
        Err(error) => {
            fatal(&format!("{error:#}"));
            return;
        }
    };

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--self-test") {
        std::process::exit(self_test(&settings));
    }
    if args.iter().any(|arg| arg == "--serve") {
        std::process::exit(serve(&settings));
    }

    if let Err(error) = open_window(settings, product, context) {
        fatal(&format!("{error:#}"));
    }
}

/// Runs the local stack without a window until Ctrl+C.
///
/// Used for smoke tests and for installations that only need the API.
fn serve(settings: &Settings) -> i32 {
    match runtime::Standalone::start(settings, None) {
        Ok(stack) => {
            println!("api: {}", stack.api_base());
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
            let _ = runtime.block_on(tokio::signal::ctrl_c());
            if let Some(error) = stack.stop() {
                eprintln!("runtime error: {error}");
                return 1;
            }
            0
        }
        Err(error) => {
            eprintln!("start failed: {error:#}");
            1
        }
    }
}

/// Writes the log to the data directory; a packaged build has no console.
fn init_logging() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,where_desktop=debug,where_api=info,where_persistence=info,sqlx=error")
    });

    let dir = Settings::data_root();
    let _ = std::fs::create_dir_all(&dir);
    let log_file = dir.join("desktop.log");
    // Keep the log small: the demo dataset is executed as one large statement
    // and would otherwise grow the file by a megabyte per start.
    if let Ok(meta) = std::fs::metadata(&log_file)
        && meta.len() > 4 * 1024 * 1024
    {
        let _ = std::fs::remove_file(&log_file);
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file);

    match file {
        Ok(file) => {
            let _ = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(file))
                .with_env_filter(filter)
                .try_init();
        }
        Err(_) => {
            let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
        }
    }
}

/// The product this window belongs to: the identifier of the bundle it was built as.
///
/// Both desktop applications share this code; only `tauri.conf.json` tells them
/// apart, and Tauri embeds it at build time. They also share data directory and
/// database — the product only decides which module the installation registers.
fn product_of(context: &tauri::Context<tauri::Wry>) -> Product {
    Product::from_identifier(&context.config().identifier).unwrap_or_default()
}

/// Loads the settings and makes sure the embedded instance has a password.
fn load_settings(product: Product) -> Result<Settings> {
    let path = config_argument()?.unwrap_or_else(Settings::config_path);
    let mut settings = if path.is_file() {
        Settings::load_from(&path)?
    } else {
        Settings::for_product(product)
    };

    // The bundle decides which product registers itself, unless `WHERE_PRODUCT` says
    // otherwise — the test harness uses that to run one product from either binary.
    if std::env::var_os("WHERE_PRODUCT").is_none() {
        settings.product = product;
    }

    if settings.mode == Mode::Standalone && settings.embedded.password.is_empty() {
        settings.embedded.password = uuid::Uuid::new_v4().simple().to_string();
        settings.save_to(&path)?;
        tracing::info!(
            config = %path.display(),
            product = settings.product.key(),
            "created the workstation configuration"
        );
    }
    Ok(settings)
}

fn open_window(
    settings: Settings,
    product: Product,
    context: tauri::Context<tauri::Wry>,
) -> Result<()> {
    let client_base = settings.api_base.clone();
    let standalone = settings.mode == Mode::Standalone;

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .manage(Local(Mutex::new(None)))
        .setup(move |app| {
            let splash =
                WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash.html".into()))
                    .title("WHERE startet …")
                    .inner_size(460.0, 240.0)
                    .resizable(false)
                    .decorations(false)
                    .always_on_top(true)
                    .center()
                    .build()?;

            let handle = app.handle().clone();
            let resources = app.path().resource_dir().ok();
            std::thread::spawn(move || {
                let base = if standalone {
                    match runtime::Standalone::start(&settings, resources) {
                        Ok(stack) => {
                            let base = stack.api_base().to_string();
                            if let Some(state) = handle.try_state::<Local>() {
                                *state.0.lock().unwrap() = Some(stack);
                            }
                            base
                        }
                        Err(error) => {
                            let message = format!("{error:#}");
                            tracing::error!("{message}");
                            let script = format!(
                                "window.__zeigeFehler({});",
                                serde_json::to_string(&message).unwrap_or_default()
                            );
                            let _ = splash.eval(&script);
                            return;
                        }
                    }
                } else {
                    client_base.unwrap_or_else(|| "http://127.0.0.1:8090".to_string())
                };

                if let Err(error) = open_main(&handle, &base, product) {
                    let message = format!("{error:#}");
                    tracing::error!("{message}");
                    let script = format!(
                        "window.__zeigeFehler({});",
                        serde_json::to_string(&message).unwrap_or_default()
                    );
                    let _ = splash.eval(&script);
                    return;
                }
                let _ = splash.close();
            });
            Ok(())
        });

    let app = builder
        .build(context)
        .context("building the desktop window")?;

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<Local>() {
                if let Some(stack) = state.0.lock().unwrap().take() {
                    if let Some(error) = stack.stop() {
                        tracing::error!("local stack stopped with: {error}");
                    }
                }
            }
        }
    });
    Ok(())
}

/// Opens the real window once the API answers.
fn open_main(handle: &tauri::AppHandle, api_base: &str, product: Product) -> Result<()> {
    let script = format!(
        "window.__WHERE_API_BASE__ = {}; window.__WHERE_PRODUCT__ = {};",
        serde_json::to_string(api_base).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::to_string(product.label()).unwrap_or_else(|_| "\"\"".to_string())
    );
    WebviewWindowBuilder::new(handle, "main", WebviewUrl::App("index.html".into()))
        .title(product.label())
        .inner_size(1280.0, 800.0)
        .min_inner_size(900.0, 600.0)
        .initialization_script(&script)
        .build()
        .with_context(|| format!("opening the main window for {api_base}"))?;
    Ok(())
}

/// Reads `--config <path>` / `--config=<path>`.
fn config_argument() -> Result<Option<PathBuf>> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--config=") {
            return Ok(Some(PathBuf::from(value)));
        }
        if arg == "--config" {
            let value = args.next().context(
                "--config needs a path, e.g. --config C:\\ProgramData\\WHERE\\config.toml",
            )?;
            return Ok(Some(PathBuf::from(value)));
        }
    }
    Ok(None)
}

/// Reports an unrecoverable start-up problem without a window.
fn fatal(message: &str) {
    tracing::error!("{message}");
    eprintln!("WHERE: {message}");
    let log = Settings::data_root().join("desktop.log");
    let _ = std::fs::create_dir_all(Settings::data_root());
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .and_then(|mut file| {
            use std::io::Write;
            writeln!(file, "WHERE: {message}")
        });
}

/// Starts the local stack, calls the API and stops everything again.
///
/// Exists so a packaged build can be checked without driving the window: it
/// prints nothing (the release binary has no console) and writes the outcome to
/// `%LOCALAPPDATA%\where-data\self-test.log`.
fn self_test(settings: &Settings) -> i32 {
    let log = Settings::data_root().join("self-test.log");
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("product: {}", settings.product.label()));
    lines.push(format!("mode: {:?}", settings.mode));
    lines.push(format!("bind: {}", settings.bind));
    lines.push(format!("data: {}", settings.embedded.data_dir.display()));

    if settings.mode == Mode::Client {
        return client_self_test(settings, lines, &log);
    }

    let mut code = 0;
    match runtime::Standalone::start(settings, None) {
        Ok(stack) => {
            lines.push(format!("api: {}", stack.api_base()));
            lines.push("started: ok".to_string());

            let address: std::net::SocketAddr = match settings.socket_addr() {
                Ok(address) => address,
                Err(error) => {
                    lines.push(format!("address: {error:#}"));
                    return 1;
                }
            };
            for path in [
                "/health",
                "/api/v1/articles?limit=1",
                "/api/v1/addresses?limit=1",
            ] {
                match http_get(address, path) {
                    Ok((status, body)) => {
                        let total = body
                            .split("\"total\":")
                            .nth(1)
                            .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
                            .unwrap_or("");
                        if total.is_empty() {
                            lines.push(format!("GET {path}: {status}"));
                        } else {
                            lines.push(format!("GET {path}: {status}, total {total}"));
                        }
                    }
                    Err(error) => lines.push(format!("GET {path}: failed — {error:#}")),
                }
            }

            if let Some(error) = stack.stop() {
                lines.push(format!("runtime error: {error}"));
                code = 1;
            } else {
                lines.push("stopped: ok".to_string());
            }
        }
        Err(error) => {
            lines.push(format!("started: failed — {error:#}"));
            code = 1;
        }
    }

    let _ = std::fs::create_dir_all(Settings::data_root());
    let _ = std::fs::write(&log, lines.join("\n") + "\n");
    for line in &lines {
        println!("{line}");
    }
    code
}

/// Checks the configured central server instead of starting anything locally.
fn client_self_test(settings: &Settings, mut lines: Vec<String>, log: &std::path::Path) -> i32 {
    let base = settings
        .api_base
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:8090".to_string());
    lines.push(format!("api: {base}"));

    let mut code = 0;
    match server_address(&base) {
        Ok(address) => match http_get(address, "/health") {
            Ok((status, _)) => lines.push(format!("GET /health: {status}")),
            Err(error) => {
                lines.push(format!("GET /health: failed — {error:#}"));
                code = 1;
            }
        },
        Err(error) => {
            lines.push(format!("api_base: {error:#}"));
            code = 1;
        }
    }

    let _ = std::fs::create_dir_all(Settings::data_root());
    let _ = std::fs::write(log, lines.join("\n") + "\n");
    for line in &lines {
        println!("{line}");
    }
    code
}

/// Turns `http://host:port/…` into a socket address; port 8090 by default.
fn server_address(base: &str) -> Result<std::net::SocketAddr> {
    let without_scheme = base.split_once("://").map(|(_, rest)| rest).unwrap_or(base);
    let authority = without_scheme.split('/').next().unwrap_or_default();
    let candidate = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:8090")
    };
    candidate
        .parse()
        .with_context(|| format!("{base} is not a usable API address"))
}

/// Minimal HTTP GET without a client dependency; returns the status line.
fn http_get(address: std::net::SocketAddr, path: &str) -> Result<(String, String)> {
    use std::io::{Read, Write};

    let mut stream = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .with_context(|| format!("connecting to {address}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .context("setting the read timeout")?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .context("sending the request")?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .context("reading the response")?;
    let status = response
        .lines()
        .next()
        .context("empty response")?
        .trim()
        .to_string();
    if !status.contains(" 200") {
        anyhow::bail!("{status}");
    }
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    Ok((status, body))
}
