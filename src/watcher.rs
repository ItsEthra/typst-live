use crate::ServerState;
use anyhow::{bail, Result};
use log::{debug, error};
use notify::{event::AccessKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::Path, sync::Arc, time::Duration};
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
    let watched_file = watchpath.canonicalize().unwrap_or(watchpath.clone());
    let watched_name = watched_file.file_name().map(|x| x.to_os_string());
    let watched_parent = watched_file
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .or_else(|| watchpath.parent().map(|p| p.to_path_buf()));

    let mut watcher = notify::recommended_watcher(move |e: Result<Event, _>| match e {
        Ok(e) => {
            debug!("Watch event: kind={:?}, paths={:?}", e.kind, e.paths);

            let relevant_kind = matches!(e.kind, EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any)
                && !matches!(e.kind, EventKind::Access(AccessKind::Read));

            if relevant_kind
                && e.paths.iter().any(|p| matches_watched_file(p, watched_name.as_deref(), watched_parent.as_deref()))
                && last_update.elapsed() > Duration::from_millis(100)
            {
                debug!("File has changed, notifying waiters");

                last_update = Instant::now();
                state.changed.notify_waiters();
            }
        }
        Err(err) => error!("{err}"),
    })?;
    watcher.watch(watchpath.parent().unwrap(), RecursiveMode::NonRecursive)?;

    Ok(watcher)
}

fn matches_watched_file(
    path: &Path,
    watched_name: Option<&std::ffi::OsStr>,
    watched_parent: Option<&Path>,
) -> bool {
    let Some(watched_name) = watched_name else {
        return false;
    };

    if path.file_name() != Some(watched_name) {
        return false;
    }

    let Some(watched_parent) = watched_parent else {
        return true;
    };

    let event_parent = path.parent().unwrap_or(path);
    let event_parent = event_parent
        .canonicalize()
        .unwrap_or_else(|_| event_parent.to_path_buf());

    event_parent == watched_parent
}
