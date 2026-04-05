use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tracing::{info, warn};

/// Watches a folder for new/modified files and notifies via callback.
/// Works with any sync tool (Syncthing, Dropbox, OneDrive, manual copies).
pub struct FolderWatcher {
    watcher: RecommendedWatcher,
    _rx: mpsc::Receiver<Result<Event, notify::Error>>,
}

/// Events emitted when files change in a watched folder.
#[derive(Debug, Clone)]
pub enum FolderEvent {
    /// New or modified files detected
    FilesChanged { paths: Vec<PathBuf> },
    /// Files were removed
    FilesRemoved { paths: Vec<PathBuf> },
}

impl FolderWatcher {
    /// Start watching a folder. Returns a receiver for change events.
    pub fn watch(
        folder_path: &Path,
    ) -> Result<(Self, mpsc::Receiver<FolderEvent>), notify::Error> {
        let (notify_tx, notify_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let _ = notify_tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(5)),
        )?;

        watcher.watch(folder_path, RecursiveMode::Recursive)?;

        info!("Watching folder for changes: {}", folder_path.display());

        // Spawn a thread to process raw notify events into FolderEvents
        let event_tx_clone = event_tx.clone();
        std::thread::spawn(move || {
            // Batch events over 2 seconds to avoid rapid-fire processing
            let mut pending_changed: Vec<PathBuf> = Vec::new();
            let mut pending_removed: Vec<PathBuf> = Vec::new();
            let batch_interval = Duration::from_secs(2);

            loop {
                match notify_rx.recv_timeout(batch_interval) {
                    Ok(Ok(event)) => {
                        let dominated_paths: Vec<PathBuf> = event
                            .paths
                            .into_iter()
                            .filter(|p| {
                                // Skip hidden files and .mim directory
                                !p.components().any(|c| {
                                    c.as_os_str()
                                        .to_string_lossy()
                                        .starts_with('.')
                                })
                            })
                            .filter(|p| p.is_file() || !p.exists())
                            .collect();

                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                pending_changed.extend(dominated_paths);
                            }
                            EventKind::Remove(_) => {
                                pending_removed.extend(dominated_paths);
                            }
                            _ => {}
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("Watch error: {}", e);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Flush batch
                        if !pending_changed.is_empty() {
                            pending_changed.sort();
                            pending_changed.dedup();
                            let _ = event_tx_clone.send(FolderEvent::FilesChanged {
                                paths: std::mem::take(&mut pending_changed),
                            });
                        }
                        if !pending_removed.is_empty() {
                            pending_removed.sort();
                            pending_removed.dedup();
                            let _ = event_tx_clone.send(FolderEvent::FilesRemoved {
                                paths: std::mem::take(&mut pending_removed),
                            });
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok((
            Self {
                watcher,
                _rx: mpsc::channel().1, // placeholder
            },
            event_rx,
        ))
    }

    /// Stop watching.
    pub fn unwatch(&mut self, folder_path: &Path) {
        let _ = self.watcher.unwatch(folder_path);
    }
}
