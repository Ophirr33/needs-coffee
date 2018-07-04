use config::Config;
use errors::{OpaqueError, OResult};
use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode};
use resource::SiteResources;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::Path;

pub fn serve(config: &Config, build_dir: &Path, static_dir: &Path) -> OResult<()> {
    let mut config = Config::new(config.timings.clone());
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(1))?;
    watcher.watch(static_dir, RecursiveMode::NonRecursive)?;
    loop {
        match rx.recv()? {
            DebouncedEvent::Error(e, _) => {
                error!("File watch error {:?} quitting", e);
                return Err(OpaqueError::from(e));
            },
            DebouncedEvent::Rescan | DebouncedEvent::Chmod(_) => {},
            _ => {
                info!("Detected changes, rebuilding files");
                let resources = SiteResources::read_resources(&static_dir, &config)?;
                resources.build_all(build_dir, false)?;
                config = Config::new(resources.timings());
            }
        }
    }
}
