use anyhow::{bail, Result};
use axum::{routing::get, Router};
use clap::Parser;
use log::{error, info, warn, LevelFilter};
use std::{
    env, ffi::OsString, fs, future::IntoFuture, iter::repeat_with, path::PathBuf, sync::Arc,
};
use tokio::{net::TcpListener, signal::ctrl_c, sync::Notify};
use tokio_util::sync::CancellationToken;

mod routes;
mod watcher;

#[derive(Parser)]
/// hot reloading for typst.
struct Args {
    /// do not open browser tab when launched.
    #[arg(long, short = 'T')]
    no_browser_tab: bool,
    /// turns off recompilation, just listens to file changes and updates the webpage.
    #[arg(long, short = 'R')]
    no_recompile: bool,
    /// specifies file to recompile when changes are made. If `--no-recompile` is used it should be pdf file.
    filename: PathBuf,
    /// specifies the listen address.
    #[arg(long, short = 'A', default_value = "127.0.0.1")]
    address: String,
    /// specifies the port to listen on.
    #[arg(long, short = 'P', default_value = "5599")]
    port: u16,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    remaining: Vec<OsString>,
}

struct ServerState {
    args: Args,
    changed: Notify,
    scratch: PathBuf,
    shutdown: CancellationToken,
}

async fn make_listener(address: &str, preferred_port: u16) -> Result<TcpListener> {
    for port in [preferred_port].into_iter().chain(49152..65535) {
        let listener = TcpListener::bind((address, port)).await;
        if listener.is_ok() {
            return listener.map_err(Into::into);
        }
    }

    bail!("Couldn't find a port to bind to")
}

async fn run(state: Arc<ServerState>) -> Result<()> {
    let router = Router::new()
        .route("/", get(routes::root))
        .route("/target.pdf", get(routes::target))
        .route("/listen", get(routes::listen))
        .with_state(state.clone());
    let listener = make_listener(&state.args.address, state.args.port).await?;

    let url = format!("http://{}", listener.local_addr()?);
    info!("Server is listening on {url}",);

    if !state.args.no_browser_tab && open::that_detached(&url).is_err() {
        warn!("Could not open the preview in your browser. Open URL manually: {url}");
    }

    tokio::select! {
        x = axum::serve(listener, router).into_future() => x.map_err(Into::into),
        _ = state.shutdown.clone().cancelled_owned() => Ok(()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .format_timestamp(None)
        .format_target(false)
        .init();

    let args: Args = Args::parse();

    if args.no_recompile && !args.filename.ends_with(".pdf") {
        error!("When using --no-recompile option, filename must be pdf file");
        return Ok(());
    }

    if fs::metadata(&args.filename).is_err() {
        error!("File {:?} doesn't exist", args.filename);
        return Ok(());
    }

    let mut scratch = env::temp_dir().join("typst-live");
    scratch.set_extension(
        repeat_with(fastrand::alphanumeric)
            .take(6)
            .collect::<String>()
            + ".pdf",
    );

    if !args.no_recompile {
        info!("Using a scratch file at {scratch:?}");
        fs::write(&scratch, b"")?;
    }

    let state = Arc::new(ServerState {
        shutdown: CancellationToken::new(),
        changed: Notify::new(),
        scratch,
        args,
    });

    tokio::spawn(graceful_shutdown(state.clone()));

    let _watcher = watcher::setup_watching_typst(state.clone()).await?;
    run(state.clone()).await?;

    fs::remove_file(&state.scratch)?;
    Ok(())
}

async fn graceful_shutdown(state: Arc<ServerState>) {
    ctrl_c().await.unwrap();
    state.shutdown.cancel();
}
