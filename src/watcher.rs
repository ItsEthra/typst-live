use crate::ServerState;
use anyhow::{bail, Result};
use log::{debug, error};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{sync::Arc, time::Duration};
use tokio::{process::Command, time::Instant};

pub async fn setup_watching_typst(state: Arc<ServerState>) -> Result<RecommendedWatcher> {
    let mut last_update = Instant::now();

    let shutdown = state.shutdown.clone();
    let watchpath = if !state.args.no_recompile {
        match Command::new("typst")
            .arg("watch")
            .arg(&state.args.filename)
            .arg(&state.scratch)
            .args(&state.args.remaining)
            .spawn()
        {
            Ok(child) => {
                _ = tokio::spawn(async move {
                    match child.wait_with_output().await {
                        Ok(out) if !out.status.success() =>
                            error!("Typst exited with error code: {}", out.status),
                        Err(err) => error!("Typst exited with error: {err:?}"),
                        _ => return,
                    }

                    shutdown.cancel();
                });

                state.scratch.clone()
            }
            Err(err) => bail!("Failed to spawn the typst {err:?}"),
        }
    } else {
        state.args.filename.clone()
    };

    let mut watcher = notify::recommended_watcher(move |e: Result<Event, _>| match e {
        Ok(e) if matches!(e.kind, EventKind::Modify(_)) => {
            let ending = if state.args.no_recompile {
                &state.args.filename
            } else {
                &state.scratch
            };

            if e.paths.iter().any(|p| p.ends_with(ending))
                && last_update.elapsed() > Duration::from_millis(100)
            {
                debug!("File has changed, notifying waiters");

                last_update = Instant::now();
                state.changed.notify_waiters();
            }
        }
        Err(err) => error!("{err}"),
        _ => {}
    })?;
    watcher.watch(watchpath.parent().unwrap(), RecursiveMode::NonRecursive)?;

    Ok(watcher)
}
