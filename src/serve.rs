use errors::{OpaqueError, OResult};
use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::Path;

fn serve(static_dir: &Path) -> OResult<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(1))?;
    watcher.watch(static_dir, RecursiveMode::NonRecursive)?;
    Ok(())
}
