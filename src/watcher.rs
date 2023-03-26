use crate::state::ServerState;
use eyre::Result;
use notify::{Event, EventKind, INotifyWatcher, RecursiveMode, Watcher};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{process::Command, time::Instant};
use tracing::{debug, error};

pub fn setup_watch(state: Arc<ServerState>) -> Result<INotifyWatcher> {
    let mut last_update = Instant::now();

    let mut watcher = notify::recommended_watcher(move |e: Result<Event, _>| match e {
        Ok(e) if matches!(e.kind, EventKind::Modify(_)) => {
            debug!("Received notify event");

            let typstname = state.typstname.blocking_read().clone();

            if e.paths.iter().any(|p| p.ends_with(&typstname))
                && last_update.elapsed() > Duration::from_millis(100)
            {
                last_update = Instant::now();
                debug!("{typstname} was updated, recompiling");

                state.tokio.spawn(compile_typst(typstname, state.clone()));
            }
        }
        Err(err) => eprintln!("Error: {err}"),
        _ => {}
    })?;
    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;

    Ok(watcher)
}

pub async fn compile_typst(typstname: String, state: Arc<ServerState>) {
    let result = Command::new("typst")
        .arg(typstname)
        .arg("output.pdf")
        .output()
        .await;

    match result {
        Ok(out) => {
            if out.status.success() {
                state.changed.notify_waiters();
            } else {
                error!("Typst exited with {} status code", out.status);
            }
        }
        Err(err) => error!("Error running typst: {err:?}"),
    }
}
