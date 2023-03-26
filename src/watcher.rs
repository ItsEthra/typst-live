use crate::state::ServerState;
use eyre::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::Path, process::Stdio, sync::Arc, time::Duration};
use tokio::{process::Command, time::Instant};
use tracing::{debug, error};

pub async fn setup_watching_typst(state: Arc<ServerState>) -> Result<RecommendedWatcher> {
    let mut last_update = Instant::now();

    if !state.args.no_recompile {
        match Command::new("typst")
            .arg("--watch")
            .arg(&state.args.filename)
            .arg("output.pdf")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                _ = state.tokio.spawn(async move {
                    match child.wait_with_output().await {
                        Ok(out) if !out.status.success() => {
                            error!("Typst exited with error code: {}", out.status)
                        }
                        Err(err) => error!("Typst exited with error: {err:?}"),
                        _ => {}
                    }
                })
            }
            Err(err) => error!("Failed to spawn the typst {err:?}"),
        }
    }

    let mut watcher = notify::recommended_watcher(move |e: Result<Event, _>| match e {
        Ok(e) if matches!(e.kind, EventKind::Modify(_)) => {
            debug!("Received notify event");

            let ending = if state.args.no_recompile {
                &state.args.filename
            } else {
                "output.pdf"
            };

            if e.paths.iter().any(|p| p.ends_with(ending))
                && last_update.elapsed() > Duration::from_millis(100)
            {
                last_update = Instant::now();
                state.changed.notify_waiters();
            }
        }
        Err(err) => error!("Error: {err}"),
        _ => {}
    })?;
    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;

    Ok(watcher)
}
