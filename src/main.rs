use argh::FromArgs;
use axum::{routing::get, Router, Server};
use eyre::Result;
use state::ServerState;
use std::{fs, sync::Arc};
use tokio::{runtime::Runtime, signal::ctrl_c, sync::Notify};
use tracing::{error, info, warn};

mod routes;
mod state;
mod watcher;

#[derive(FromArgs)]
/// hot reloading for typst.
struct Args {
    /// turns off recompilation, just listens to file changes and updates the webpage.
    #[argh(switch, short = 'R')]
    no_recompile: bool,
    #[argh(positional)]
    /// specifies file to recompile when changes are made. If `--watch` is used it should be pdf file.
    filename: String,
}

async fn run(state: Arc<ServerState>) -> Result<()> {
    let router = Router::new()
        .route("/", get(routes::root))
        .route("/target.pdf", get(routes::target))
        .route("/listen", get(routes::listen))
        .with_state(state.clone());

    let server = Server::bind(&"127.0.0.1:5599".parse()?).serve(router.into_make_service());
    info!("Server is listening on http://{}/", server.local_addr());

    tokio::select! {
        _ = server => {},
        _ = state.shutdown.notified() => {},
    };

    Ok(())
}

fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    std::env::set_var("RUST_LOG", "hyper=error,debug");

    tracing_subscriber::fmt::init();

    let args: Args = argh::from_env();

    if args.no_recompile && !args.filename.ends_with(".pdf") {
        error!("When using --no-recompile option, filename must be pdf file");
        return Ok(());
    }

    if fs::metadata(&args.filename).is_err() {
        error!("File `{}` doesn't exist", args.filename);
        return Ok(());
    }

    if fs::metadata("output.pdf").is_ok() && !args.no_recompile {
        warn!("Remove or save `output.pdf` as it will be overwritten. Exiting");
        return Ok(());
    }

    let tokio = Runtime::new()?;
    let state = Arc::new(ServerState {
        shutdown: Notify::new(),
        changed: Notify::new(),
        tokio,
        args,
    });

    state.tokio.spawn(graceful_shutdown(state.clone()));

    let _watcher = state
        .tokio
        .block_on(watcher::setup_watching_typst(state.clone()))?;
    state.tokio.block_on(run(state.clone()))?;

    Ok(())
}

async fn graceful_shutdown(state: Arc<ServerState>) {
    ctrl_c().await.unwrap();

    // Reset to prevent ^C from appearing.
    print!("\r");

    info!("Exiting typst-live...");
    if let Err(e) = fs::remove_file("output.pdf") {
        warn!("Failed to remove `output.pdf`. {e}");
    }

    state.shutdown.notify_waiters();
}
