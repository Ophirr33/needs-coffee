use config::Config;
use errors::{OResult, OpaqueError};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use resource::SiteResources;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn serve(
    config: &Config,
    build_dir: &Path,
    static_dir: &Path,
    metadata_file: &Path,
) -> OResult<()> {
    let mut config = Config::new(config.timings.clone());
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(1))?;
    watcher.watch(static_dir, RecursiveMode::NonRecursive)?;
    let resources = SiteResources::read_resources(&static_dir, &config)?;
    resources.build_all(build_dir, false)?;
    config = Config::new(resources.timings());
    loop {
        match rx.recv()? {
            DebouncedEvent::Error(e, _) => {
                error!("File watch error {:?} quitting", e);
                return Err(OpaqueError::from(e));
            }
            DebouncedEvent::Rescan | DebouncedEvent::Chmod(_) => {}
            _ => {
                info!("Detected changes, rebuilding files");
                let resources = SiteResources::read_resources(&static_dir, &config)?;
                if let Err(e) = resources.build_all(build_dir, false) {
                    eprintln!("Could not build due to {}", e);
                    continue;
                }
                let updated_config = Config::new(resources.timings());
                if config != updated_config {
                    updated_config.to_file(&metadata_file)?;
                    config = updated_config;
                }
            }
        }
    }
}
