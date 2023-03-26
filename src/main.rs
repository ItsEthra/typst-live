use axum::{routing::get, Router, Server};
use eyre::Result;
use state::ServerState;
use std::{env, fs, sync::Arc};
use tokio::{runtime::Runtime, signal::ctrl_c, sync::Notify};
use tracing::{info, warn};
use watcher::compile_typst;

mod routes;
mod state;
mod watcher;

async fn run(state: Arc<ServerState>) -> Result<()> {
    let router = Router::new()
        .route("/", get(routes::root))
        .route("/target.pdf", get(routes::target))
        .route("/listen", get(routes::listen))
        .with_state(state.clone());

    let server = Server::bind(&"0.0.0.0:5599".parse()?).serve(router.into_make_service());
    println!("Server is listening on http://{}/", server.local_addr());

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

    let Some(file) = env::args().nth(1) else {
        println!("Usage: ./typst-live <file.typ>");
        return Ok(());
    };

    if fs::metadata(&file).is_err() {
        println!("File `{file}` doesn't exist");
        return Ok(());
    }

    if fs::metadata("output.pdf").is_ok() {
        warn!("Remove or save `output.pdf` as it will be overwritten. Exiting");
        return Ok(());
    }

    let tokio = Runtime::new()?;
    let state = Arc::new(ServerState {
        typstname: file.clone().into(),
        shutdown: Notify::new(),
        changed: Notify::new(),
        tokio,
    });

    state.tokio.spawn(graceful_shutdown(state.clone()));

    // Compile first time to create `output.pdf`
    state.tokio.block_on(compile_typst(file, state.clone()));

    let _watcher = watcher::setup_watch(state.clone())?;
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
