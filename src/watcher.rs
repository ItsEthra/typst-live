use crate::state::ServerState;
use eyre::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{process::Command, time::Instant};

pub async fn setup_watching_typst(state: Arc<ServerState>) -> Result<RecommendedWatcher> {
    let mut last_update = Instant::now();

    if !state.args.no_recompile {
        match Command::new("typst")
            .arg("watch")
            .arg(&state.args.filename)
            .arg("output.pdf")
            .spawn()
        {
            Ok(child) => {
                _ = state.tokio.spawn(async move {
                    match child.wait_with_output().await {
                        Ok(out) if !out.status.success() => {
                            println!("[ERR] Typst exited with error code: {}", out.status)
                        }
                        Err(err) => println!("[ERR] Typst exited with error: {err:?}"),
                        _ => {}
                    }
                })
            }
            Err(err) => println!("[ERR] Failed to spawn the typst {err:?}"),
        }
    }

    let mut watcher = notify::recommended_watcher(move |e: Result<Event, _>| match e {
        Ok(e) if matches!(e.kind, EventKind::Modify(_)) => {
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
        Err(err) => println!("[ERR] {err}"),
        _ => {}
    })?;
    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;

    Ok(watcher)
}
