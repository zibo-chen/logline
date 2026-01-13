//! Log buffer management with support for large files

use crate::log_entry::{LogEntry, LogLevel};
use std::collections::VecDeque;
use std::ops::Range;

/// Configuration for the log buffer
#[derive(Debug, Clone)]
pub struct LogBufferConfig {
    /// Maximum number of lines to keep in memory
    pub max_lines: usize,
    /// Whether to automatically trim old entries when limit is reached
    pub auto_trim: bool,
}

impl Default for LogBufferConfig {
    fn default() -> Self {
        Self {
            max_lines: 100_000,
            auto_trim: true,
        }
    }
}

/// Buffer for storing log entries with efficient operations
pub struct LogBuffer {
    /// All log entries
    entries: VecDeque<LogEntry>,
    /// Configuration
    config: LogBufferConfig,
    /// Total lines ever added (including trimmed ones)
    total_lines_added: usize,
    /// First line number in the buffer (1-indexed)
    first_line_number: usize,
}

impl LogBuffer {
    /// Create a new log buffer with default config
    pub fn new() -> Self {
        Self::with_config(LogBufferConfig::default())
    }

    /// Create a new log buffer with custom config
    pub fn with_config(config: LogBufferConfig) -> Self {
        Self {
            entries: VecDeque::with_capacity(config.max_lines.min(10_000)),
            config,
            total_lines_added: 0,
            first_line_number: 1,
        }
    }

    /// Add a single log entry
    pub fn push(&mut self, entry: LogEntry) {
        self.total_lines_added += 1;

        if self.config.auto_trim && self.entries.len() >= self.config.max_lines {
            self.entries.pop_front();
            self.first_line_number += 1;
        }

        self.entries.push_back(entry);
    }

    /// Add multiple log entries
    pub fn extend(&mut self, entries: impl IntoIterator<Item = LogEntry>) {
        for entry in entries {
            self.push(entry);
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.first_line_number = self.total_lines_added + 1;
    }

    /// Get number of entries currently in buffer
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if buffer is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get total lines ever added
    pub fn total_lines(&self) -> usize {
        self.total_lines_added
    }

    /// Get entry by index (0-indexed into current buffer)
    pub fn get(&self, index: usize) -> Option<&LogEntry> {
        self.entries.get(index)
    }

    /// Get entry by line number (1-indexed from file start)
    #[allow(dead_code)]
    pub fn get_by_line_number(&self, line_number: usize) -> Option<&LogEntry> {
        if line_number < self.first_line_number {
            return None;
        }
        let index = line_number - self.first_line_number;
        self.entries.get(index)
    }

    /// Get a range of entries
    #[allow(dead_code)]
    pub fn get_range(&self, range: Range<usize>) -> impl Iterator<Item = &LogEntry> {
        let start = range.start.min(self.entries.len());
        let end = range.end.min(self.entries.len());
        self.entries.range(start..end)
    }

    /// Get entries within a line number range
    #[allow(dead_code)]
    pub fn get_line_range(&self, start_line: usize, end_line: usize) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.line_number >= start_line && e.line_number <= end_line)
            .collect()
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }

    /// Get first line number in buffer
    #[allow(dead_code)]
    pub fn first_line_number(&self) -> usize {
        self.first_line_number
    }

    /// Get last line number in buffer
    pub fn last_line_number(&self) -> usize {
        self.entries
            .back()
            .map(|e| e.line_number)
            .unwrap_or(self.first_line_number.saturating_sub(1))
    }

    /// Filter entries by log level
    #[allow(dead_code)]
    pub fn filter_by_level(&self, levels: &[LogLevel]) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.level.map(|l| levels.contains(&l)).unwrap_or(true))
            .collect()
    }

    /// Search entries matching a query
    #[allow(dead_code)]
    pub fn search(
        &self,
        query: &str,
        case_sensitive: bool,
        use_regex: bool,
    ) -> Vec<(usize, &LogEntry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.matches(query, case_sensitive, use_regex))
            .collect()
    }

    /// Find indices of entries matching a query
    #[allow(dead_code)]
    pub fn search_indices(&self, query: &str, case_sensitive: bool, use_regex: bool) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.matches(query, case_sensitive, use_regex))
            .map(|(i, _)| i)
            .collect()
    }

    /// Toggle bookmark on entry at index
    #[allow(dead_code)]
    pub fn toggle_bookmark(&mut self, index: usize) -> bool {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.bookmarked = !entry.bookmarked;
            entry.bookmarked
        } else {
            false
        }
    }

    /// Toggle bookmarks on multiple entries
    /// Returns the number of entries affected
    pub fn toggle_bookmarks(&mut self, indices: &[usize]) -> usize {
        if indices.is_empty() {
            return 0;
        }

        // Determine whether to add or remove bookmarks
        // If any selected line is not bookmarked, add bookmarks to all
        // If all are bookmarked, remove bookmarks from all
        let all_bookmarked = indices
            .iter()
            .filter_map(|&idx| self.entries.get(idx))
            .all(|e| e.bookmarked);

        let new_state = !all_bookmarked;
        let mut count = 0;

        for &idx in indices {
            if let Some(entry) = self.entries.get_mut(idx) {
                entry.bookmarked = new_state;
                count += 1;
            }
        }

        count
    }

    /// Get all bookmarked entries
    #[allow(dead_code)]
    pub fn bookmarked_entries(&self) -> Vec<(usize, &LogEntry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.bookmarked)
            .collect()
    }

    /// Get memory usage estimate in bytes
    pub fn memory_usage(&self) -> usize {
        self.entries
            .iter()
            .map(|e| std::mem::size_of::<LogEntry>() + e.content.len())
            .sum()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Filtered view of log buffer
#[allow(dead_code)]
pub struct FilteredLogView {
    /// Indices of entries that match the filter
    pub indices: Vec<usize>,
    /// Active log level filters
    pub level_filter: Vec<LogLevel>,
    /// Search query
    pub search_query: String,
    /// Case sensitive search
    pub case_sensitive: bool,
    /// Use regex for search
    pub use_regex: bool,
}

#[allow(dead_code)]
impl FilteredLogView {
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            level_filter: vec![
                LogLevel::Trace,
                LogLevel::Debug,
                LogLevel::Info,
                LogLevel::Warn,
                LogLevel::Error,
                LogLevel::Fatal,
            ],
            search_query: String::new(),
            case_sensitive: false,
            use_regex: false,
        }
    }

    /// Update the filtered view based on current filters
    pub fn update(&mut self, buffer: &LogBuffer) {
        self.indices = buffer
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                // Level filter
                let level_match = entry
                    .level
                    .map(|l| self.level_filter.contains(&l))
                    .unwrap_or(true);

                // Search filter
                let search_match = self.search_query.is_empty()
                    || entry.matches(&self.search_query, self.case_sensitive, self.use_regex);

                level_match && search_match
            })
            .map(|(i, _)| i)
            .collect();
    }

    /// Get filtered entry count
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Check if filtered view is empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Get entry at filtered index
    pub fn get<'a>(&self, filtered_index: usize, buffer: &'a LogBuffer) -> Option<&'a LogEntry> {
        self.indices
            .get(filtered_index)
            .and_then(|&i| buffer.get(i))
    }

    /// Get original buffer index from filtered index
    pub fn original_index(&self, filtered_index: usize) -> Option<usize> {
        self.indices.get(filtered_index).copied()
    }

    /// Toggle a log level in the filter
    pub fn toggle_level(&mut self, level: LogLevel) {
        if let Some(pos) = self.level_filter.iter().position(|&l| l == level) {
            self.level_filter.remove(pos);
        } else {
            self.level_filter.push(level);
        }
    }

    /// Check if a level is enabled
    pub fn is_level_enabled(&self, level: LogLevel) -> bool {
        self.level_filter.contains(&level)
    }
}

impl Default for FilteredLogView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_push() {
        let mut buffer = LogBuffer::new();
        buffer.push(LogEntry::new(1, "Test line".to_string(), 0));
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.total_lines(), 1);
    }

    #[test]
    fn test_buffer_auto_trim() {
        let config = LogBufferConfig {
            max_lines: 3,
            auto_trim: true,
        };
        let mut buffer = LogBuffer::with_config(config);

        for i in 1..=5 {
            buffer.push(LogEntry::new(i, format!("Line {}", i), 0));
        }

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.first_line_number(), 3);
        assert_eq!(buffer.get(0).unwrap().line_number, 3);
    }

    #[test]
    fn test_search() {
        let mut buffer = LogBuffer::new();
        buffer.push(LogEntry::new(1, "Hello world".to_string(), 0));
        buffer.push(LogEntry::new(2, "Error occurred".to_string(), 0));
        buffer.push(LogEntry::new(3, "Hello again".to_string(), 0));

        let results = buffer.search("Hello", false, false);
        assert_eq!(results.len(), 2);
    }
}
