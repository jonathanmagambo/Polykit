//! File watching for incremental rebuilds.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use notify::Config as NotifyConfig;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::{Error, Result};
use crate::path_utils;

pub struct WatcherConfig {
    pub debounce_ms: u64,
    pub packages_dir: PathBuf,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 300,
            packages_dir: PathBuf::from("./packages"),
        }
    }
}

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    receiver: std::sync::mpsc::Receiver<notify::Result<Event>>,
    config: WatcherConfig,
}

impl FileWatcher {
    pub fn new(config: WatcherConfig) -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let notify_config = NotifyConfig::default();

        let watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.send(res) {
                    eprintln!("Failed to send watcher event: {}", e);
                }
            },
            notify_config,
        )
        .map_err(|e| Error::Adapter {
            package: "watcher".to_string(),
            message: format!("Failed to create watcher: {}", e),
        })?;

        let mut file_watcher = Self {
            watcher,
            receiver: rx,
            config,
        };

        file_watcher.watch_packages_dir()?;

        Ok(file_watcher)
    }

    fn watch_packages_dir(&mut self) -> Result<()> {
        self.watcher
            .watch(&self.config.packages_dir, RecursiveMode::Recursive)
            .map_err(|e| Error::Adapter {
                package: "watcher".to_string(),
                message: format!("Failed to watch directory: {}", e),
            })?;
        Ok(())
    }

    pub fn next_event(&mut self) -> Result<Option<Event>> {
        match self.receiver.try_recv() {
            Ok(Ok(event)) => Ok(Some(event)),
            Ok(Err(e)) => Err(Error::Adapter {
                package: "watcher".to_string(),
                message: format!("Watcher error: {}", e),
            }),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(Error::Adapter {
                package: "watcher".to_string(),
                message: "Watcher channel disconnected".to_string(),
            }),
        }
    }

    pub fn wait_for_event(&mut self) -> Result<Event> {
        self.receiver
            .recv()
            .map_err(|_| Error::Adapter {
                package: "watcher".to_string(),
                message: "Watcher channel disconnected".to_string(),
            })?
            .map_err(|e| Error::Adapter {
                package: "watcher".to_string(),
                message: format!("Watcher error: {}", e),
            })
    }

    pub fn get_affected_packages(&self, event: &Event) -> HashSet<String> {
        let mut affected = HashSet::new();

        match &event.kind {
            EventKind::Any | EventKind::Other => {
                for path in &event.paths {
                    if let Some(package_name) =
                        Self::file_to_package(path, &self.config.packages_dir)
                    {
                        affected.insert(package_name);
                    }
                }
            }
            _ => {
                for path in &event.paths {
                    if let Some(package_name) =
                        Self::file_to_package(path, &self.config.packages_dir)
                    {
                        affected.insert(package_name);
                    }
                }
            }
        }

        affected
    }

    fn file_to_package(file_path: &Path, packages_dir: &Path) -> Option<String> {
        path_utils::file_to_package(file_path, packages_dir)
    }
}
