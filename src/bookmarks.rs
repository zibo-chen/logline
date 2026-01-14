//! Bookmarks persistence module
//!
//! Manages saving and loading bookmarks for log files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Bookmarks for a specific file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileBookmarks {
    /// Bookmarked line numbers (1-indexed)
    pub lines: HashSet<usize>,
    /// Last modified timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<u64>,
}

/// Global bookmarks storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookmarksStore {
    /// Map of file path to bookmarks
    /// Key is the canonical path string
    pub files: HashMap<String, FileBookmarks>,
}

impl BookmarksStore {
    /// Load bookmarks from disk
    pub fn load() -> Result<Self> {
        let path = Self::storage_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path).context("Failed to read bookmarks file")?;

        let store: Self = toml::from_str(&content).context("Failed to parse bookmarks file")?;

        Ok(store)
    }

    /// Save bookmarks to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::storage_path()?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create bookmarks directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize bookmarks")?;

        std::fs::write(&path, content).context("Failed to write bookmarks file")?;

        Ok(())
    }

    /// Get storage file path
    fn storage_path() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .context("Failed to get data directory")?
            .join("logline");

        Ok(dir.join("bookmarks.toml"))
    }

    /// Get bookmarks for a specific file
    pub fn get_bookmarks(&self, file_path: &Path) -> Option<&FileBookmarks> {
        let key = Self::path_to_key(file_path)?;
        self.files.get(&key)
    }

    /// Set bookmarks for a specific file
    pub fn set_bookmarks(&mut self, file_path: &Path, lines: HashSet<usize>) {
        if let Some(key) = Self::path_to_key(file_path) {
            if lines.is_empty() {
                // Remove entry if no bookmarks
                self.files.remove(&key);
            } else {
                let file_bookmarks = FileBookmarks {
                    lines,
                    last_modified: Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    ),
                };
                self.files.insert(key, file_bookmarks);
            }
        }
    }

    /// Add a bookmark for a file
    #[allow(dead_code)]
    pub fn add_bookmark(&mut self, file_path: &Path, line: usize) {
        if let Some(key) = Self::path_to_key(file_path) {
            let file_bookmarks = self.files.entry(key).or_default();
            file_bookmarks.lines.insert(line);
            file_bookmarks.last_modified = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }
    }

    /// Remove a bookmark for a file
    #[allow(dead_code)]
    pub fn remove_bookmark(&mut self, file_path: &Path, line: usize) {
        if let Some(key) = Self::path_to_key(file_path) {
            if let Some(file_bookmarks) = self.files.get_mut(&key) {
                file_bookmarks.lines.remove(&line);
                file_bookmarks.last_modified = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                );

                // Remove entry if no bookmarks left
                if file_bookmarks.lines.is_empty() {
                    self.files.remove(&key);
                }
            }
        }
    }

    /// Clear all bookmarks for a file
    #[allow(dead_code)]
    pub fn clear_bookmarks(&mut self, file_path: &Path) {
        if let Some(key) = Self::path_to_key(file_path) {
            self.files.remove(&key);
        }
    }

    /// Convert file path to storage key (canonical path string)
    fn path_to_key(file_path: &Path) -> Option<String> {
        // Try to canonicalize the path, but fall back to the original path if it fails
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.to_path_buf());
        Some(canonical.to_string_lossy().to_string())
    }

    /// Clean up old bookmarks (remove entries with no bookmarks)
    #[allow(dead_code)]
    pub fn cleanup(&mut self) {
        self.files
            .retain(|_, bookmarks| !bookmarks.lines.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_bookmarks_store() {
        let mut store = BookmarksStore::default();
        let path = PathBuf::from("/tmp/test.log");

        // Add bookmarks
        store.add_bookmark(&path, 10);
        store.add_bookmark(&path, 20);
        store.add_bookmark(&path, 30);

        // Get bookmarks
        let bookmarks = store.get_bookmarks(&path).unwrap();
        assert_eq!(bookmarks.lines.len(), 3);
        assert!(bookmarks.lines.contains(&10));
        assert!(bookmarks.lines.contains(&20));
        assert!(bookmarks.lines.contains(&30));

        // Remove a bookmark
        store.remove_bookmark(&path, 20);
        let bookmarks = store.get_bookmarks(&path).unwrap();
        assert_eq!(bookmarks.lines.len(), 2);
        assert!(!bookmarks.lines.contains(&20));

        // Clear all bookmarks
        store.clear_bookmarks(&path);
        assert!(store.get_bookmarks(&path).is_none());
    }
}
