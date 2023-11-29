use argh::FromArgs;
use axum::{routing::get, Router};
use eyre::Result;
use state::ServerState;
use std::{fs, future::IntoFuture, sync::Arc};
use tokio::{net::TcpListener, runtime::Runtime, signal::ctrl_c, sync::Notify};

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
    #[argh(option, short = 'A', default = "String::from(\"127.0.0.1\")")]
    /// specifies the listen address. Defaults to 127.0.0.1
    address: String,
    #[argh(option, short = 'P', default = "5599")]
    /// specifies the port to listen on. Defaults to 5599
    port: u16,
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

    if open::that_detached(&url).is_err() {
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

    let args: Args = argh::from_env();

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

    if let Err(e) = fs::remove_file("output.pdf") {
        println!("[WARN] Failed to remove `output.pdf`. {e}");
    }

    state.shutdown.notify_waiters();
}
