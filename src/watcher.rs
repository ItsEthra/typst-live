use crate::state::ServerState;
use eyre::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::Path, process::Stdio, sync::Arc, time::Duration};
use tokio::{process::Command, time::Instant};
use tracing::{debug, error};

pub async fn setup_watching_typst(state: Arc<ServerState>) -> Result<RecommendedWatcher> {
    let mut last_update = Instant::now();
    let typstname = state.typstname.read().await.clone();

    match Command::new("typst")
        .arg("--watch")
        .arg(&typstname)
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

    let mut watcher = notify::recommended_watcher(move |e: Result<Event, _>| match e {
        Ok(e) if matches!(e.kind, EventKind::Modify(_)) => {
            debug!("Received notify event");

            if e.paths.iter().any(|p| p.ends_with("output.pdf"))
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
