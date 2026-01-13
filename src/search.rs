//! Search and filter functionality

use crate::log_buffer::LogBuffer;
use crate::log_entry::LogLevel;
use regex::Regex;
use std::collections::HashSet;

/// Search configuration
#[derive(Debug, Clone, Default)]
pub struct SearchConfig {
    /// Search query string
    pub query: String,
    /// Case sensitive search
    pub case_sensitive: bool,
    /// Use regex
    pub use_regex: bool,
    /// Whole word match
    pub whole_word: bool,
}

impl SearchConfig {
    /// Check if the search is active (non-empty query)
    pub fn is_active(&self) -> bool {
        !self.query.is_empty()
    }

    /// Build a regex from the config
    pub fn build_regex(&self) -> Option<Regex> {
        if self.query.is_empty() {
            return None;
        }

        let pattern = if self.use_regex {
            if self.whole_word {
                format!(r"\b{}\b", self.query)
            } else {
                self.query.clone()
            }
        } else {
            let escaped = regex::escape(&self.query);
            if self.whole_word {
                format!(r"\b{}\b", escaped)
            } else {
                escaped
            }
        };

        let pattern = if self.case_sensitive {
            pattern
        } else {
            format!("(?i){}", pattern)
        };

        Regex::new(&pattern).ok()
    }
}

/// Search result with match information
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Line index in buffer
    pub buffer_index: usize,
    /// Line number in file
    #[allow(dead_code)]
    pub line_number: usize,
    /// Start positions of matches within the line
    #[allow(dead_code)]
    pub match_positions: Vec<(usize, usize)>,
}

/// Search engine for log content
pub struct SearchEngine {
    /// Search configuration
    pub config: SearchConfig,
    /// Cached search results
    results: Vec<SearchMatch>,
    /// Current result index
    current_index: Option<usize>,
    /// Whether results need refresh
    dirty: bool,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            config: SearchConfig::default(),
            results: Vec::new(),
            current_index: None,
            dirty: true,
        }
    }

    /// Set the search query
    pub fn set_query(&mut self, query: String) {
        if self.config.query != query {
            self.config.query = query;
            self.dirty = true;
            self.current_index = None;
        }
    }

    /// Set case sensitivity
    pub fn set_case_sensitive(&mut self, case_sensitive: bool) {
        if self.config.case_sensitive != case_sensitive {
            self.config.case_sensitive = case_sensitive;
            self.dirty = true;
        }
    }

    /// Set regex mode
    pub fn set_use_regex(&mut self, use_regex: bool) {
        if self.config.use_regex != use_regex {
            self.config.use_regex = use_regex;
            self.dirty = true;
        }
    }

    /// Set whole word match
    pub fn set_whole_word(&mut self, whole_word: bool) {
        if self.config.whole_word != whole_word {
            self.config.whole_word = whole_word;
            self.dirty = true;
        }
    }

    /// Execute search on buffer
    pub fn search(&mut self, buffer: &LogBuffer) {
        self.results.clear();

        if !self.config.is_active() {
            self.dirty = false;
            return;
        }

        let regex = match self.config.build_regex() {
            Some(r) => r,
            None => {
                self.dirty = false;
                return;
            }
        };

        for (idx, entry) in buffer.iter().enumerate() {
            let matches: Vec<(usize, usize)> = regex
                .find_iter(&entry.content)
                .map(|m| (m.start(), m.end()))
                .collect();

            if !matches.is_empty() {
                self.results.push(SearchMatch {
                    buffer_index: idx,
                    line_number: entry.line_number,
                    match_positions: matches,
                });
            }
        }

        // Set current index to first result
        if !self.results.is_empty() && self.current_index.is_none() {
            self.current_index = Some(0);
        }

        self.dirty = false;
    }

    /// Update search if dirty
    #[allow(dead_code)]
    pub fn update_if_dirty(&mut self, buffer: &LogBuffer) {
        if self.dirty {
            self.search(buffer);
        }
    }

    /// Mark results as needing refresh
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get number of results
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Get current result index (1-indexed for display)
    pub fn current_result_number(&self) -> Option<usize> {
        self.current_index.map(|i| i + 1)
    }

    /// Get current match
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.current_index.and_then(|i| self.results.get(i))
    }

    /// Move to next result
    pub fn next(&mut self) -> Option<&SearchMatch> {
        if self.results.is_empty() {
            return None;
        }

        self.current_index = Some(match self.current_index {
            Some(i) => (i + 1) % self.results.len(),
            None => 0,
        });

        self.current_match()
    }

    /// Move to previous result
    pub fn previous(&mut self) -> Option<&SearchMatch> {
        if self.results.is_empty() {
            return None;
        }

        self.current_index = Some(match self.current_index {
            Some(0) => self.results.len() - 1,
            Some(i) => i - 1,
            None => self.results.len() - 1,
        });

        self.current_match()
    }

    /// Jump to result near a specific line
    #[allow(dead_code)]
    pub fn jump_to_line(&mut self, line: usize) -> Option<&SearchMatch> {
        if self.results.is_empty() {
            return None;
        }

        // Find the closest result
        let idx = self
            .results
            .iter()
            .enumerate()
            .min_by_key(|(_, m)| m.line_number.abs_diff(line))
            .map(|(i, _)| i);

        self.current_index = idx;
        self.current_match()
    }

    /// Get all results
    #[allow(dead_code)]
    pub fn results(&self) -> &[SearchMatch] {
        &self.results
    }

    /// Check if a buffer index has matches
    #[allow(dead_code)]
    pub fn has_match(&self, buffer_index: usize) -> bool {
        self.results.iter().any(|m| m.buffer_index == buffer_index)
    }

    /// Check if a buffer index is the current match
    pub fn is_current_match(&self, buffer_index: usize) -> bool {
        self.current_match()
            .map(|m| m.buffer_index == buffer_index)
            .unwrap_or(false)
    }

    /// Clear search
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.config.query.clear();
        self.results.clear();
        self.current_index = None;
        self.dirty = false;
    }

    /// Check if search is active
    pub fn is_active(&self) -> bool {
        self.config.is_active()
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter configuration for log levels and other criteria
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Enabled log levels
    pub levels: HashSet<LogLevel>,
    /// Text to exclude (inverted filter)
    pub exclude_patterns: Vec<String>,
    /// Only show bookmarked lines
    pub bookmarks_only: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        let mut levels = HashSet::new();
        levels.insert(LogLevel::Trace);
        levels.insert(LogLevel::Debug);
        levels.insert(LogLevel::Info);
        levels.insert(LogLevel::Warn);
        levels.insert(LogLevel::Error);
        levels.insert(LogLevel::Fatal);

        Self {
            levels,
            exclude_patterns: Vec::new(),
            bookmarks_only: false,
        }
    }
}

impl FilterConfig {
    /// Check if a level is enabled
    pub fn is_level_enabled(&self, level: LogLevel) -> bool {
        self.levels.contains(&level)
    }

    /// Toggle a level
    pub fn toggle_level(&mut self, level: LogLevel) {
        if self.levels.contains(&level) {
            self.levels.remove(&level);
        } else {
            self.levels.insert(level);
        }
    }

    /// Enable all levels
    pub fn enable_all_levels(&mut self) {
        self.levels.insert(LogLevel::Trace);
        self.levels.insert(LogLevel::Debug);
        self.levels.insert(LogLevel::Info);
        self.levels.insert(LogLevel::Warn);
        self.levels.insert(LogLevel::Error);
        self.levels.insert(LogLevel::Fatal);
    }

    /// Disable all levels
    #[allow(dead_code)]
    pub fn disable_all_levels(&mut self) {
        self.levels.clear();
    }

    /// Show only errors and warnings
    pub fn errors_and_warnings_only(&mut self) {
        self.levels.clear();
        self.levels.insert(LogLevel::Warn);
        self.levels.insert(LogLevel::Error);
        self.levels.insert(LogLevel::Fatal);
    }

    /// Add exclude pattern
    pub fn add_exclude(&mut self, pattern: String) {
        if !pattern.is_empty() && !self.exclude_patterns.contains(&pattern) {
            self.exclude_patterns.push(pattern);
        }
    }

    /// Remove exclude pattern
    pub fn remove_exclude(&mut self, index: usize) {
        if index < self.exclude_patterns.len() {
            self.exclude_patterns.remove(index);
        }
    }

    /// Check if any filters are active
    pub fn is_filtering(&self) -> bool {
        self.levels.len() < 6 || !self.exclude_patterns.is_empty() || self.bookmarks_only
    }
}

/// Combined filter that applies both search and level filtering
pub struct LogFilter {
    /// Search engine
    pub search: SearchEngine,
    /// Filter configuration
    pub filter: FilterConfig,
    /// Filtered indices (cache)
    filtered_indices: Vec<usize>,
    /// Whether filter needs refresh
    dirty: bool,
}

impl LogFilter {
    pub fn new() -> Self {
        Self {
            search: SearchEngine::new(),
            filter: FilterConfig::default(),
            filtered_indices: Vec::new(),
            dirty: true,
        }
    }

    /// Apply filters to buffer and return filtered indices
    pub fn apply(&mut self, buffer: &LogBuffer) -> &[usize] {
        if !self.dirty {
            return &self.filtered_indices;
        }

        self.filtered_indices.clear();

        // Build exclude regexes
        let exclude_regexes: Vec<Regex> = self
            .filter
            .exclude_patterns
            .iter()
            .filter_map(|p| Regex::new(&format!("(?i){}", regex::escape(p))).ok())
            .collect();

        for (idx, entry) in buffer.iter().enumerate() {
            // Level filter
            if let Some(level) = entry.level {
                if !self.filter.is_level_enabled(level) {
                    continue;
                }
            }

            // Bookmarks filter
            if self.filter.bookmarks_only && !entry.bookmarked {
                continue;
            }

            // Exclude patterns
            let excluded = exclude_regexes.iter().any(|r| r.is_match(&entry.content));
            if excluded {
                continue;
            }

            // Search filter (if active)
            if self.search.is_active() {
                if let Some(regex) = self.search.config.build_regex() {
                    if !regex.is_match(&entry.content) {
                        continue;
                    }
                }
            }

            self.filtered_indices.push(idx);
        }

        self.dirty = false;
        &self.filtered_indices
    }

    /// Mark as needing refresh
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.search.mark_dirty();
    }

    /// Get filtered line count
    #[allow(dead_code)]
    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get original buffer index from filtered index
    #[allow(dead_code)]
    pub fn buffer_index(&self, filtered_index: usize) -> Option<usize> {
        self.filtered_indices.get(filtered_index).copied()
    }

    /// Check if filtering is active
    pub fn is_filtering(&self) -> bool {
        self.filter.is_filtering() || self.search.is_active()
    }
}

impl Default for LogFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_entry::LogEntry;

    #[test]
    fn test_search_basic() {
        let mut buffer = LogBuffer::new();
        buffer.push(LogEntry::new(1, "Hello world".to_string(), 0));
        buffer.push(LogEntry::new(2, "Test message".to_string(), 0));
        buffer.push(LogEntry::new(3, "Hello again".to_string(), 0));

        let mut engine = SearchEngine::new();
        engine.set_query("Hello".to_string());
        engine.search(&buffer);

        assert_eq!(engine.result_count(), 2);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut buffer = LogBuffer::new();
        buffer.push(LogEntry::new(1, "HELLO world".to_string(), 0));
        buffer.push(LogEntry::new(2, "hello there".to_string(), 0));

        let mut engine = SearchEngine::new();
        engine.set_query("hello".to_string());
        engine.set_case_sensitive(false);
        engine.search(&buffer);

        assert_eq!(engine.result_count(), 2);
    }

    #[test]
    fn test_search_navigation() {
        let mut buffer = LogBuffer::new();
        for i in 1..=5 {
            buffer.push(LogEntry::new(i, format!("Match {}", i), 0));
        }

        let mut engine = SearchEngine::new();
        engine.set_query("Match".to_string());
        engine.search(&buffer);

        assert_eq!(engine.current_result_number(), Some(1));

        engine.next();
        assert_eq!(engine.current_result_number(), Some(2));

        engine.previous();
        assert_eq!(engine.current_result_number(), Some(1));

        engine.previous(); // Wrap around
        assert_eq!(engine.current_result_number(), Some(5));
    }
}
