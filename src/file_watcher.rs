//! File watching and monitoring module

use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Receiver};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Events sent from the file watcher
#[derive(Debug, Clone)]
pub enum FileWatchEvent {
    /// File content was modified
    Modified,
    /// File was deleted or renamed
    Removed,
    /// File was recreated (log rotation)
    Recreated,
    /// Error occurred while watching
    #[allow(dead_code)]
    Error(String),
}

/// File watcher that monitors a log file for changes
pub struct FileWatcher {
    /// Path to the watched file
    path: PathBuf,
    /// The actual watcher instance
    _watcher: RecommendedWatcher,
    /// Channel receiver for file events
    #[allow(dead_code)]
    event_rx: Receiver<FileWatchEvent>,
    /// Flag to indicate if watching is active
    is_active: Arc<AtomicBool>,
}

impl FileWatcher {
    /// Create a new file watcher for the given path
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let (tx, rx) = bounded::<FileWatchEvent>(100);
        let is_active = Arc::new(AtomicBool::new(true));

        let path_clone = path.clone();
        let tx_clone = tx.clone();
        let is_active_clone = is_active.clone();

        // Create the watcher with a custom event handler
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if !is_active_clone.load(Ordering::Relaxed) {
                    return;
                }

                match res {
                    Ok(event) => {
                        // Only process events for our specific file
                        if !event.paths.iter().any(|p| p == &path_clone) {
                            return;
                        }

                        let watch_event = match event.kind {
                            EventKind::Modify(_) => Some(FileWatchEvent::Modified),
                            EventKind::Remove(_) => Some(FileWatchEvent::Removed),
                            EventKind::Create(_) => Some(FileWatchEvent::Recreated),
                            _ => None,
                        };

                        if let Some(evt) = watch_event {
                            let _ = tx_clone.try_send(evt);
                        }
                    }
                    Err(e) => {
                        let _ = tx_clone.try_send(FileWatchEvent::Error(e.to_string()));
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )
        .context("Failed to create file watcher")?;

        let mut file_watcher = Self {
            path,
            _watcher: watcher,
            event_rx: rx,
            is_active,
        };

        // Start watching the file's parent directory
        file_watcher.start_watching()?;

        Ok(file_watcher)
    }

    /// Start watching the file
    fn start_watching(&mut self) -> Result<()> {
        // Watch the parent directory to catch file recreation
        let parent = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        self._watcher
            .watch(&parent, RecursiveMode::NonRecursive)
            .context("Failed to start watching directory")?;

        Ok(())
    }

    /// Get the event receiver
    #[allow(dead_code)]
    pub fn events(&self) -> &Receiver<FileWatchEvent> {
        &self.event_rx
    }

    /// Try to receive an event without blocking
    #[allow(dead_code)]
    pub fn try_recv(&self) -> Option<FileWatchEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Check if there are pending events
    #[allow(dead_code)]
    pub fn has_events(&self) -> bool {
        !self.event_rx.is_empty()
    }

    /// Get the watched path
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Stop watching
    pub fn stop(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }

    /// Check if watcher is active
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Manager for watching multiple files
#[allow(dead_code)]
pub struct MultiFileWatcher {
    watchers: Vec<(PathBuf, FileWatcher)>,
}

#[allow(dead_code)]
impl MultiFileWatcher {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
        }
    }

    /// Add a file to watch
    pub fn add(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        // Check if already watching
        if self.watchers.iter().any(|(p, _)| p == &path) {
            return Ok(());
        }

        let watcher = FileWatcher::new(&path)?;
        self.watchers.push((path, watcher));
        Ok(())
    }

    /// Remove a file from watching
    pub fn remove(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        self.watchers.retain(|(p, _)| p != path);
    }

    /// Get events for a specific file
    pub fn try_recv(&self, path: impl AsRef<Path>) -> Option<FileWatchEvent> {
        let path = path.as_ref();
        self.watchers
            .iter()
            .find(|(p, _)| p == path)
            .and_then(|(_, w)| w.try_recv())
    }

    /// Get all pending events
    pub fn drain_events(&self) -> Vec<(PathBuf, FileWatchEvent)> {
        let mut events = Vec::new();
        for (path, watcher) in &self.watchers {
            while let Some(event) = watcher.try_recv() {
                events.push((path.clone(), event));
            }
        }
        events
    }

    /// Get number of watched files
    pub fn len(&self) -> usize {
        self.watchers.len()
    }

    /// Check if any files are being watched
    pub fn is_empty(&self) -> bool {
        self.watchers.is_empty()
    }
}

impl Default for MultiFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_file_watcher_creation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.log");
        File::create(&file_path).unwrap();

        let watcher = FileWatcher::new(&file_path);
        assert!(watcher.is_ok());
    }
}
