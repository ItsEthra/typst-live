use axum::{routing::get, Router};
use clap::Parser;
use eyre::Result;
use state::ServerState;
use std::{ffi::OsString, fs, future::IntoFuture, sync::Arc};
use tokio::{net::TcpListener, runtime::Runtime, signal::ctrl_c, sync::Notify};

mod routes;
mod state;
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
    filename: String,
    /// specifies the listen address. Defaults to 127.0.0.1
    #[arg(long, short = 'A', default_value = "127.0.0.1")]
    address: String,
    /// specifies the port to listen on. Defaults to 5599
    #[arg(long, short = 'P', default_value = "5599")]
    port: u16,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    remaining: Vec<OsString>,
}

async fn run(state: Arc<ServerState>) -> Result<()> {
    let router = Router::new()
        .route("/", get(routes::root))
        .route("/target.pdf", get(routes::target))
        .route("/listen", get(routes::listen))
        .with_state(state.clone());

    let addr = format!("{}:{}", state.args.address, state.args.port);
    let listener = TcpListener::bind(&addr).await?;

    let url = format!("http://{}", listener.local_addr()?);
    println!("[INFO] Server is listening on {url}",);

    if !state.args.no_browser_tab && open::that_detached(&url).is_err() {
        println!("[WARN] Could not open the preview in your browser. Open URL manually: {url}");
    }

    let fut = axum::serve(listener, router).into_future();
    tokio::select! {
        _ = fut => {},
        _ = state.shutdown.notified() => {},
    };

    Ok(())
}

fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    std::env::set_var("RUST_LOG", "hyper=error,debug");

    let args: Args = Args::parse();

    if args.no_recompile && !args.filename.ends_with(".pdf") {
        println!("[ERR] When using --no-recompile option, filename must be pdf file");
        return Ok(());
    }

    if fs::metadata(&args.filename).is_err() {
        println!("[ERR] File `{}` doesn't exist", args.filename);
        return Ok(());
    }

    if fs::metadata("output.pdf").is_ok() && !args.no_recompile {
        println!("[WARN] Remove or save `output.pdf` as it will be overwritten. Exiting");
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

    let watcher = state
        .tokio
        .block_on(watcher::setup_watching_typst(state.clone()))?;
    state.tokio.block_on(run(state.clone()))?;
    drop(watcher);

    Ok(())
}

async fn graceful_shutdown(state: Arc<ServerState>) {
    ctrl_c().await.unwrap();

    // Reset to prevent ^C from appearing.
    print!("\r");

    if !state.args.no_recompile {
        if let Err(e) = fs::remove_file("output.pdf") {
            println!("[WARN] Failed to remove `output.pdf`. {e}");
        }
    }

    state.shutdown.notify_waiters();
}
