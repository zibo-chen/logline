//! MCP Tools implementation for log analysis
//!
//! Provides various tools for AI assistants to analyze logs:
//! - list_log_sources: List available log sources
//! - get_log_entries: Read log entries with pagination
//! - search_logs: Search logs with regex/keyword
//! - get_log_statistics: Get statistical summary
//! - analyze_errors: Analyze error patterns
//! - advanced_filter: Filter logs with multiple conditions
//! - list_bookmarks: List all bookmarked lines
//! - manage_bookmarks: Add/remove bookmarks
//! - analyze_timeline: Analyze log frequency over time

use crate::log_entry::{LogEntry, LogLevel};
use crate::mcp::types::*;
use crate::remote_server::{ConnectionStatus, RemoteStream};

use chrono::{DateTime, Local, TimeZone};
use regex::Regex;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

// ============================================================================
// Tool Parameter Structures
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListLogSourcesParams {
    /// Filter by source type: "local", "remote", or "all"
    #[serde(default = "default_source_type")]
    pub source_type: String,
    /// Filter by status: "online", "offline", or "all" (for remote sources)
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_source_type() -> String {
    "all".to_string()
}

fn default_status() -> String {
    "all".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLogEntriesParams {
    /// Log source ID (from list_log_sources)
    pub source_id: String,
    /// Starting line number (1-indexed, default: 1)
    #[serde(default = "default_start_line")]
    pub start_line: usize,
    /// Number of entries to return (default: 100, max: 1000)
    #[serde(default = "default_count")]
    pub count: usize,
    /// Log levels to include (e.g., ["ERROR", "WARN"]). Empty means all levels.
    #[serde(default)]
    pub levels: Vec<String>,
    /// If true, read from the end of file
    #[serde(default)]
    pub from_end: bool,
}

fn default_start_line() -> usize {
    1
}

fn default_count() -> usize {
    100
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchLogsParams {
    /// Log source ID
    pub source_id: String,
    /// Search query (keyword or regex pattern)
    pub query: String,
    /// Whether the query is a regex pattern
    #[serde(default)]
    pub is_regex: bool,
    /// Case sensitive search
    #[serde(default)]
    pub case_sensitive: bool,
    /// Log levels to search in (empty means all)
    #[serde(default)]
    pub levels: Vec<String>,
    /// Number of context lines before each match
    #[serde(default)]
    pub context_lines: usize,
    /// Maximum number of results (default: 50)
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_results() -> usize {
    50
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLogStatisticsParams {
    /// Log source ID
    pub source_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeErrorsParams {
    /// Log source ID
    pub source_id: String,
    /// Maximum number of error groups to return
    #[serde(default = "default_max_groups")]
    pub max_groups: usize,
    /// Include context lines around errors
    #[serde(default = "default_include_context")]
    pub include_context: bool,
}

fn default_max_groups() -> usize {
    20
}

fn default_include_context() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AdvancedFilterParams {
    /// Log source ID
    pub source_id: String,
    /// Filter conditions (all must match - AND logic)
    pub conditions: Vec<FilterConditionParam>,
    /// Maximum number of results (default: 100)
    #[serde(default = "default_count")]
    pub max_results: usize,
    /// Start time filter (ISO 8601 format)
    #[serde(default)]
    pub start_time: Option<String>,
    /// End time filter (ISO 8601 format)
    #[serde(default)]
    pub end_time: Option<String>,
    /// Include context lines around matches
    #[serde(default)]
    pub context_lines: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FilterConditionParam {
    /// Field to filter: "content", "level"
    pub field: String,
    /// Operator: "contains", "not_contains", "equals", "not_equals", "regex", "starts_with", "ends_with"
    pub operator: String,
    /// Value for the filter
    pub value: String,
    /// Case sensitive (default: false)
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBookmarksParams {
    /// Log source ID
    pub source_id: String,
    /// Include context lines around bookmarks
    #[serde(default)]
    #[allow(dead_code)]
    pub context_lines: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ManageBookmarksParams {
    /// Log source ID
    pub source_id: String,
    /// Action: "add", "remove", "toggle", "clear_all"
    pub action: String,
    /// Line numbers to bookmark (for add/remove/toggle)
    #[serde(default)]
    pub line_numbers: Vec<usize>,
    /// Optional note for the bookmarks
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeTimelineParams {
    /// Log source ID
    pub source_id: String,
    /// Time bucket size in seconds (default: 60)
    #[serde(default = "default_bucket_size")]
    pub bucket_size_seconds: u64,
    /// Log levels to include (empty means all)
    #[serde(default)]
    pub levels: Vec<String>,
    /// Start time filter (ISO 8601 format)
    #[serde(default)]
    pub start_time: Option<String>,
    /// End time filter (ISO 8601 format)
    #[serde(default)]
    pub end_time: Option<String>,
}

fn default_bucket_size() -> u64 {
    60
}

// ============================================================================
// Tool Response Structures
// ============================================================================

/// Response for list_log_sources tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListLogSourcesResponse {
    /// List of available log sources
    pub sources: Vec<LogSource>,
    /// Total number of sources
    pub total_count: usize,
    /// Number of online remote sources
    pub online_count: usize,
}

/// Response for get_log_entries tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetLogEntriesResponse {
    /// Log entries
    pub entries: Vec<LogEntryResult>,
    /// Total lines in the source
    pub total_lines: usize,
    /// Whether there are more entries
    pub has_more: bool,
    /// Next line number for pagination
    pub next_line: usize,
}

/// Response for search_logs tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchLogsResponse {
    /// Search matches
    pub matches: Vec<SearchMatch>,
    /// Total number of matches found
    pub total_matches: usize,
    /// The query that was used
    pub query: String,
    /// Whether regex was used
    pub is_regex: bool,
}

/// Response for get_log_statistics tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetLogStatisticsResponse {
    /// Statistics summary
    pub statistics: LogStatistics,
}

/// Response for analyze_errors tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalyzeErrorsResponse {
    /// Grouped error patterns
    pub error_groups: Vec<ErrorGroup>,
    /// Total number of errors
    pub total_errors: usize,
    /// Number of unique error patterns
    pub unique_patterns: usize,
}

/// Response for advanced_filter tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct AdvancedFilterResponse {
    /// Matching entries
    pub entries: Vec<LogEntryResult>,
    /// Total matches found
    pub total_matches: usize,
    /// Filters applied summary
    pub filters_applied: Vec<String>,
}

/// Response for list_bookmarks tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListBookmarksResponse {
    /// Bookmarked entries
    pub bookmarks: Vec<BookmarkEntry>,
    /// Total number of bookmarks
    pub total_bookmarks: usize,
}

/// Response for manage_bookmarks tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct ManageBookmarksResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Action performed
    pub action: String,
    /// Number of bookmarks affected
    pub affected_count: usize,
    /// Message describing what happened
    pub message: String,
}

/// Response for analyze_timeline tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalyzeTimelineResponse {
    /// Timeline analysis
    pub timeline: TimelineAnalysis,
    /// Summary statistics
    pub summary: String,
}

// ============================================================================
// Shared State for Tools
// ============================================================================

/// Shared state accessible by all MCP tools
pub struct LoglineToolState {
    /// Remote server reference for getting stream info
    pub remote_streams: Arc<RwLock<Vec<RemoteStream>>>,
    /// Local files being watched
    pub local_files: Arc<RwLock<Vec<PathBuf>>>,
    /// Cache directory for remote logs
    #[allow(dead_code)]
    pub cache_dir: PathBuf,
    /// Bookmarks storage: source_id -> line_numbers with optional notes
    bookmarks: Arc<RwLock<HashMap<String, HashMap<usize, Option<String>>>>>,
}

impl LoglineToolState {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            remote_streams: Arc::new(RwLock::new(Vec::new())),
            local_files: Arc::new(RwLock::new(Vec::new())),
            cache_dir,
            bookmarks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update remote streams from server
    pub fn update_remote_streams(&self, streams: Vec<RemoteStream>) {
        let mut guard = self.remote_streams.write().unwrap();
        *guard = streams;
    }

    /// Add a local file
    pub fn add_local_file(&self, path: PathBuf) {
        let mut guard = self.local_files.write().unwrap();
        if !guard.contains(&path) {
            guard.push(path);
        }
    }

    /// Get all log sources
    fn get_sources(&self, source_type: &str, status: &str) -> Vec<LogSource> {
        let mut sources = Vec::new();

        // Add local files
        if source_type == "all" || source_type == "local" {
            let local_files = self.local_files.read().unwrap();
            for path in local_files.iter() {
                let size = fs::metadata(path).ok().map(|m| m.len());
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                sources.push(LogSource {
                    id: format!("local:{}", path.display()),
                    name,
                    source_type: LogSourceType::Local,
                    status: None,
                    path: Some(path.clone()),
                    size,
                    last_activity: None,
                    bytes_received: None,
                });
            }
        }

        // Add remote streams
        if source_type == "all" || source_type == "remote" {
            let remote_streams = self.remote_streams.read().unwrap();
            for stream in remote_streams.iter() {
                let stream_status = match stream.status {
                    ConnectionStatus::Online => "online",
                    ConnectionStatus::Offline => "offline",
                };

                // Filter by status
                if status != "all" && status != stream_status {
                    continue;
                }

                sources.push(LogSource {
                    id: stream.stream_id.clone(),
                    name: stream.project_name.clone(),
                    source_type: LogSourceType::Remote,
                    status: Some(stream_status.to_string()),
                    path: Some(stream.cache_path.clone()),
                    size: fs::metadata(&stream.cache_path).ok().map(|m| m.len()),
                    last_activity: Some(format!("{:?}", stream.last_activity)),
                    bytes_received: Some(stream.bytes_received),
                });
            }
        }

        sources
    }

    /// Get file path for a source ID
    fn get_source_path(&self, source_id: &str) -> Option<PathBuf> {
        if let Some(stripped) = source_id.strip_prefix("local:") {
            return Some(PathBuf::from(stripped));
        }

        // Check remote streams
        let remote_streams = self.remote_streams.read().unwrap();
        for stream in remote_streams.iter() {
            if stream.stream_id == source_id {
                return Some(stream.cache_path.clone());
            }
        }

        None
    }

    /// Read log entries from a file
    fn read_log_entries(
        &self,
        path: &PathBuf,
        start_line: usize,
        count: usize,
        levels: &[String],
        from_end: bool,
    ) -> Result<(Vec<LogEntryResult>, usize, bool), String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let level_filter: Vec<LogLevel> = levels
            .iter()
            .filter_map(|l| LogLevel::from_str(l))
            .collect();

        let entries: Vec<LogEntryResult> = if from_end {
            // Read from end
            let start = total_lines.saturating_sub(count);
            lines[start..]
                .iter()
                .enumerate()
                .filter_map(|(i, line)| {
                    let entry = LogEntry::new(start + i + 1, line.to_string(), 0);
                    if !level_filter.is_empty() {
                        if let Some(level) = entry.level {
                            if !level_filter.contains(&level) {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                    Some(entry_to_result(&entry))
                })
                .take(count)
                .collect()
        } else {
            // Read from start_line
            let start = start_line.saturating_sub(1);
            lines[start..]
                .iter()
                .enumerate()
                .filter_map(|(i, line)| {
                    let entry = LogEntry::new(start + i + 1, line.to_string(), 0);
                    if !level_filter.is_empty() {
                        if let Some(level) = entry.level {
                            if !level_filter.contains(&level) {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                    Some(entry_to_result(&entry))
                })
                .take(count)
                .collect()
        };

        let has_more = if from_end {
            start_line > 1
        } else {
            start_line + count <= total_lines
        };

        Ok((entries, total_lines, has_more))
    }

    /// Search logs
    fn search_logs(
        &self,
        path: &PathBuf,
        query: &str,
        is_regex: bool,
        case_sensitive: bool,
        levels: &[String],
        context_lines: usize,
        max_results: usize,
    ) -> Result<Vec<SearchMatch>, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Build search regex
        let pattern = if is_regex {
            query.to_string()
        } else {
            regex::escape(query)
        };
        let pattern = if case_sensitive {
            pattern
        } else {
            format!("(?i){}", pattern)
        };
        let regex = Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;

        let level_filter: Vec<LogLevel> = levels
            .iter()
            .filter_map(|l| LogLevel::from_str(l))
            .collect();

        let mut matches = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if matches.len() >= max_results {
                break;
            }

            if !regex.is_match(line) {
                continue;
            }

            let entry = LogEntry::new(i + 1, line.to_string(), 0);

            // Level filter
            if !level_filter.is_empty() {
                if let Some(level) = entry.level {
                    if !level_filter.contains(&level) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Get context
            let context_before = if context_lines > 0 {
                let start = i.saturating_sub(context_lines);
                Some(lines[start..i].iter().map(|s| s.to_string()).collect())
            } else {
                None
            };

            let context_after = if context_lines > 0 {
                let end = (i + 1 + context_lines).min(total_lines);
                Some(lines[i + 1..end].iter().map(|s| s.to_string()).collect())
            } else {
                None
            };

            matches.push(SearchMatch {
                line_number: i + 1,
                content: line.to_string(),
                level: entry.level.map(|l| l.as_str().to_string()),
                timestamp: entry.timestamp.map(|t| t.to_rfc3339()),
                context_before,
                context_after,
            });
        }

        Ok(matches)
    }

    /// Get log statistics
    fn get_statistics(&self, path: &PathBuf) -> Result<LogStatistics, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let mut distribution = LevelDistribution::default();
        let mut first_timestamp: Option<chrono::DateTime<chrono::Local>> = None;
        let mut last_timestamp: Option<chrono::DateTime<chrono::Local>> = None;

        for (i, line) in lines.iter().enumerate() {
            let entry = LogEntry::new(i + 1, line.to_string(), 0);

            if let Some(level) = entry.level {
                match level {
                    LogLevel::Trace => distribution.trace += 1,
                    LogLevel::Debug => distribution.debug += 1,
                    LogLevel::Info => distribution.info += 1,
                    LogLevel::Warn => distribution.warn += 1,
                    LogLevel::Error => distribution.error += 1,
                    LogLevel::Fatal => distribution.fatal += 1,
                }
            }

            if let Some(ts) = entry.timestamp {
                if first_timestamp.is_none() {
                    first_timestamp = Some(ts);
                }
                last_timestamp = Some(ts);
            }
        }

        let error_count = distribution.error + distribution.fatal;
        let error_rate = if total_lines > 0 {
            format!("{:.2}%", (error_count as f64 / total_lines as f64) * 100.0)
        } else {
            "0%".to_string()
        };

        Ok(LogStatistics {
            total_lines,
            level_distribution: distribution,
            error_rate,
            time_range: Some(TimeRange {
                start: first_timestamp.map(|t| t.to_rfc3339()),
                end: last_timestamp.map(|t| t.to_rfc3339()),
            }),
        })
    }

    /// Analyze errors
    fn analyze_errors(
        &self,
        path: &PathBuf,
        max_groups: usize,
        _include_context: bool,
    ) -> Result<Vec<ErrorGroup>, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();

        // Group errors by simplified pattern
        let mut error_groups: HashMap<String, Vec<(usize, LogEntry)>> = HashMap::new();

        for (i, line) in lines.iter().enumerate() {
            let entry = LogEntry::new(i + 1, line.to_string(), 0);

            if let Some(level) = entry.level {
                if level == LogLevel::Error || level == LogLevel::Fatal {
                    // Simplify error message to group similar errors
                    let pattern = simplify_error_pattern(line);
                    error_groups.entry(pattern).or_default().push((i, entry));
                }
            }
        }

        // Convert to ErrorGroup and sort by count
        let mut groups: Vec<ErrorGroup> = error_groups
            .into_iter()
            .map(|(pattern, entries)| {
                let count = entries.len();
                let first_occurrence = entries
                    .first()
                    .and_then(|(_, e)| e.timestamp.map(|t| t.to_rfc3339()));
                let last_occurrence = entries
                    .last()
                    .and_then(|(_, e)| e.timestamp.map(|t| t.to_rfc3339()));

                // Sample entries (up to 3)
                let sample_entries: Vec<LogEntryResult> = entries
                    .iter()
                    .take(3)
                    .map(|(_, e)| entry_to_result(e))
                    .collect();

                ErrorGroup {
                    pattern,
                    count,
                    first_occurrence,
                    last_occurrence,
                    sample_entries,
                }
            })
            .collect();

        groups.sort_by(|a, b| b.count.cmp(&a.count));
        groups.truncate(max_groups);

        Ok(groups)
    }

    /// Apply advanced filters to log entries
    fn apply_advanced_filter(
        &self,
        path: &PathBuf,
        conditions: &[FilterConditionParam],
        max_results: usize,
        start_time: Option<&str>,
        end_time: Option<&str>,
        _context_lines: usize,
    ) -> Result<(Vec<LogEntryResult>, Vec<String>), String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut filters_applied = Vec::new();

        // Parse time filters
        let start_dt = start_time.and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Local))
        });
        let end_dt = end_time.and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Local))
        });

        if start_dt.is_some() || end_dt.is_some() {
            filters_applied.push(format!(
                "Time range: {} to {}",
                start_time.unwrap_or("*"),
                end_time.unwrap_or("*")
            ));
        }

        // Build condition matchers
        let mut condition_matchers: Vec<Box<dyn Fn(&str, &LogEntry) -> bool>> = Vec::new();

        for cond in conditions {
            let field = cond.field.to_lowercase();
            let operator = cond.operator.to_lowercase();
            let value = cond.value.clone();
            let case_sensitive = cond.case_sensitive;

            filters_applied.push(format!("{} {} '{}'", field, operator, value));

            match (field.as_str(), operator.as_str()) {
                ("content", "contains") => {
                    condition_matchers.push(Box::new(move |line: &str, _: &LogEntry| {
                        if case_sensitive {
                            line.contains(&value)
                        } else {
                            line.to_lowercase().contains(&value.to_lowercase())
                        }
                    }));
                }
                ("content", "not_contains") => {
                    condition_matchers.push(Box::new(move |line: &str, _: &LogEntry| {
                        if case_sensitive {
                            !line.contains(&value)
                        } else {
                            !line.to_lowercase().contains(&value.to_lowercase())
                        }
                    }));
                }
                ("content", "regex") => {
                    let pattern = if case_sensitive {
                        value.clone()
                    } else {
                        format!("(?i){}", value)
                    };
                    let regex =
                        Regex::new(&pattern).map_err(|e| format!("Invalid regex: {}", e))?;
                    condition_matchers.push(Box::new(move |line: &str, _: &LogEntry| {
                        regex.is_match(line)
                    }));
                }
                ("content", "starts_with") => {
                    condition_matchers.push(Box::new(move |line: &str, _: &LogEntry| {
                        if case_sensitive {
                            line.starts_with(&value)
                        } else {
                            line.to_lowercase().starts_with(&value.to_lowercase())
                        }
                    }));
                }
                ("content", "ends_with") => {
                    condition_matchers.push(Box::new(move |line: &str, _: &LogEntry| {
                        if case_sensitive {
                            line.ends_with(&value)
                        } else {
                            line.to_lowercase().ends_with(&value.to_lowercase())
                        }
                    }));
                }
                ("level", "equals") => {
                    let target_level = LogLevel::from_str(&value);
                    condition_matchers.push(Box::new(move |_: &str, entry: &LogEntry| {
                        entry.level == target_level
                    }));
                }
                ("level", "not_equals") => {
                    let target_level = LogLevel::from_str(&value);
                    condition_matchers.push(Box::new(move |_: &str, entry: &LogEntry| {
                        entry.level != target_level
                    }));
                }
                _ => {
                    return Err(format!(
                        "Unsupported filter: {} {} '{}'",
                        field, operator, value
                    ))
                }
            }
        }

        let mut results = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if results.len() >= max_results {
                break;
            }

            let entry = LogEntry::new(i + 1, line.to_string(), 0);

            // Check time filter
            if let Some(ts) = entry.timestamp {
                if let Some(start) = start_dt {
                    if ts < start {
                        continue;
                    }
                }
                if let Some(end) = end_dt {
                    if ts > end {
                        continue;
                    }
                }
            }

            // Check all conditions (AND logic)
            let matches_all = condition_matchers
                .iter()
                .all(|matcher| matcher(line, &entry));

            if matches_all {
                let result = entry_to_result(&entry);
                results.push(result);
            }
        }

        Ok((results, filters_applied))
    }

    /// Get bookmarks for a source
    fn get_bookmarks(&self, source_id: &str) -> HashMap<usize, Option<String>> {
        let bookmarks = self.bookmarks.read().unwrap();
        bookmarks.get(source_id).cloned().unwrap_or_default()
    }

    /// Manage bookmarks for a source
    fn manage_bookmarks(
        &self,
        source_id: &str,
        action: &str,
        line_numbers: &[usize],
        note: Option<&str>,
    ) -> (usize, String) {
        let mut bookmarks = self.bookmarks.write().unwrap();
        let source_bookmarks = bookmarks.entry(source_id.to_string()).or_default();

        match action {
            "add" => {
                let mut added = 0;
                for &line in line_numbers {
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        source_bookmarks.entry(line)
                    {
                        e.insert(note.map(String::from));
                        added += 1;
                    }
                }
                (added, format!("Added {} bookmark(s)", added))
            }
            "remove" => {
                let mut removed = 0;
                for &line in line_numbers {
                    if source_bookmarks.remove(&line).is_some() {
                        removed += 1;
                    }
                }
                (removed, format!("Removed {} bookmark(s)", removed))
            }
            "toggle" => {
                let mut toggled = 0;
                for &line in line_numbers {
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        source_bookmarks.entry(line)
                    {
                        e.insert(note.map(String::from));
                    } else {
                        source_bookmarks.remove(&line);
                    }
                    toggled += 1;
                }
                (toggled, format!("Toggled {} bookmark(s)", toggled))
            }
            "clear_all" => {
                let count = source_bookmarks.len();
                source_bookmarks.clear();
                (count, format!("Cleared all {} bookmark(s)", count))
            }
            _ => (0, format!("Unknown action: {}", action)),
        }
    }

    /// Analyze log timeline (frequency over time)
    fn analyze_log_timeline(
        &self,
        path: &PathBuf,
        bucket_size_seconds: u64,
        levels: &[String],
        start_time: Option<&str>,
        end_time: Option<&str>,
    ) -> Result<TimelineAnalysis, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();

        // Parse time filters
        let start_dt = start_time.and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Local))
        });
        let end_dt = end_time.and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Local))
        });

        let level_filter: HashSet<LogLevel> = levels
            .iter()
            .filter_map(|l| LogLevel::from_str(l))
            .collect();

        // Collect entries with timestamps
        let mut entries_with_ts: Vec<(DateTime<Local>, LogEntry)> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let entry = LogEntry::new(i + 1, line.to_string(), 0);

            if let Some(ts) = entry.timestamp {
                // Apply time filter
                if let Some(start) = start_dt {
                    if ts < start {
                        continue;
                    }
                }
                if let Some(end) = end_dt {
                    if ts > end {
                        continue;
                    }
                }

                // Apply level filter
                if !level_filter.is_empty() {
                    if let Some(level) = entry.level {
                        if !level_filter.contains(&level) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                entries_with_ts.push((ts, entry));
            }
        }

        if entries_with_ts.is_empty() {
            return Ok(TimelineAnalysis {
                data_points: Vec::new(),
                bucket_size_seconds,
                peak_timestamp: None,
                peak_count: 0,
            });
        }

        // Sort by timestamp
        entries_with_ts.sort_by_key(|(ts, _)| *ts);

        // Create buckets
        let mut buckets: HashMap<i64, (usize, LevelDistribution)> = HashMap::new();

        for (ts, entry) in &entries_with_ts {
            let bucket_start =
                (ts.timestamp() / bucket_size_seconds as i64) * bucket_size_seconds as i64;
            let (count, dist) = buckets
                .entry(bucket_start)
                .or_insert((0, LevelDistribution::default()));
            *count += 1;

            if let Some(level) = entry.level {
                match level {
                    LogLevel::Trace => dist.trace += 1,
                    LogLevel::Debug => dist.debug += 1,
                    LogLevel::Info => dist.info += 1,
                    LogLevel::Warn => dist.warn += 1,
                    LogLevel::Error => dist.error += 1,
                    LogLevel::Fatal => dist.fatal += 1,
                }
            }
        }

        // Convert to data points
        let mut data_points: Vec<FrequencyDataPoint> = buckets
            .into_iter()
            .map(|(bucket_start, (count, dist))| {
                let ts = Local.timestamp_opt(bucket_start, 0).unwrap();
                FrequencyDataPoint {
                    timestamp: ts.to_rfc3339(),
                    count,
                    by_level: Some(dist),
                }
            })
            .collect();

        data_points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Find peak
        let (peak_timestamp, peak_count) = data_points
            .iter()
            .max_by_key(|dp| dp.count)
            .map(|dp| (Some(dp.timestamp.clone()), dp.count))
            .unwrap_or((None, 0));

        Ok(TimelineAnalysis {
            data_points,
            bucket_size_seconds,
            peak_timestamp,
            peak_count,
        })
    }
}

// ============================================================================
// MCP Tool Handler
// ============================================================================

/// Logline MCP Server - provides log analysis tools for AI assistants
#[derive(Clone)]
pub struct LoglineTools {
    state: Arc<LoglineToolState>,
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

#[tool_router]
impl LoglineTools {
    pub fn new(state: Arc<LoglineToolState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    /// List all available log sources (local files and remote streams)
    #[tool(
        name = "list_log_sources",
        description = "List all available log sources including local files and remote agent streams. Returns source IDs that can be used with other tools."
    )]
    fn list_log_sources(
        &self,
        Parameters(params): Parameters<ListLogSourcesParams>,
    ) -> Result<Json<ListLogSourcesResponse>, String> {
        let sources = self.state.get_sources(&params.source_type, &params.status);
        let online_count = sources
            .iter()
            .filter(|s| s.status.as_deref() == Some("online"))
            .count();

        Ok(Json(ListLogSourcesResponse {
            total_count: sources.len(),
            online_count,
            sources,
        }))
    }

    /// Get log entries from a source with pagination and filtering
    #[tool(
        name = "get_log_entries",
        description = "Read log entries from a source. Supports pagination with start_line and count parameters. Can filter by log levels (ERROR, WARN, INFO, DEBUG, TRACE, FATAL)."
    )]
    fn get_log_entries(
        &self,
        Parameters(params): Parameters<GetLogEntriesParams>,
    ) -> Result<Json<GetLogEntriesResponse>, String> {
        let count = params.count.min(1000); // Max 1000 entries

        let path = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let (entries, total_lines, has_more) = self.state.read_log_entries(
            &path,
            params.start_line,
            count,
            &params.levels,
            params.from_end,
        )?;

        let next_line = if params.from_end {
            params.start_line.saturating_sub(count)
        } else {
            params.start_line + entries.len()
        };

        Ok(Json(GetLogEntriesResponse {
            entries,
            total_lines,
            has_more,
            next_line,
        }))
    }

    /// Search logs with keyword or regex pattern
    #[tool(
        name = "search_logs",
        description = "Search logs for patterns. Supports both keyword and regex search. Can filter by log levels and return context lines around matches."
    )]
    fn search_logs(
        &self,
        Parameters(params): Parameters<SearchLogsParams>,
    ) -> Result<Json<SearchLogsResponse>, String> {
        let path = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let matches = self.state.search_logs(
            &path,
            &params.query,
            params.is_regex,
            params.case_sensitive,
            &params.levels,
            params.context_lines,
            params.max_results,
        )?;

        Ok(Json(SearchLogsResponse {
            matches: matches.clone(),
            total_matches: matches.len(),
            query: params.query,
            is_regex: params.is_regex,
        }))
    }

    /// Get log statistics summary
    #[tool(
        name = "get_log_statistics",
        description = "Get statistical summary of a log source including total lines, log level distribution, error rate, and time range."
    )]
    fn get_log_statistics(
        &self,
        Parameters(params): Parameters<GetLogStatisticsParams>,
    ) -> Result<Json<GetLogStatisticsResponse>, String> {
        let path = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let statistics = self.state.get_statistics(&path)?;

        Ok(Json(GetLogStatisticsResponse { statistics }))
    }

    /// Analyze error patterns in logs
    #[tool(
        name = "analyze_errors",
        description = "Analyze error patterns in logs. Groups similar errors together and provides occurrence counts, timestamps, and sample entries."
    )]
    fn analyze_errors(
        &self,
        Parameters(params): Parameters<AnalyzeErrorsParams>,
    ) -> Result<Json<AnalyzeErrorsResponse>, String> {
        let path = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let error_groups =
            self.state
                .analyze_errors(&path, params.max_groups, params.include_context)?;

        let total_errors: usize = error_groups.iter().map(|g| g.count).sum();
        let unique_patterns = error_groups.len();

        Ok(Json(AnalyzeErrorsResponse {
            error_groups,
            total_errors,
            unique_patterns,
        }))
    }

    /// Apply advanced multi-condition filters to logs
    #[tool(
        name = "advanced_filter",
        description = "Filter logs with multiple conditions combined with AND logic. Supports content filters (contains, not_contains, regex, starts_with, ends_with) and level filters. Can also filter by time range using ISO 8601 timestamps."
    )]
    fn advanced_filter(
        &self,
        Parameters(params): Parameters<AdvancedFilterParams>,
    ) -> Result<Json<AdvancedFilterResponse>, String> {
        let path = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let (entries, filters_applied) = self.state.apply_advanced_filter(
            &path,
            &params.conditions,
            params.max_results,
            params.start_time.as_deref(),
            params.end_time.as_deref(),
            params.context_lines,
        )?;

        Ok(Json(AdvancedFilterResponse {
            total_matches: entries.len(),
            entries,
            filters_applied,
        }))
    }

    /// List all bookmarked log entries
    #[tool(
        name = "list_bookmarks",
        description = "List all bookmarked log entries from a source. Bookmarks are used to mark important lines for later reference. Returns bookmarked entries with their line numbers, content, and optional notes."
    )]
    fn list_bookmarks(
        &self,
        Parameters(params): Parameters<ListBookmarksParams>,
    ) -> Result<Json<ListBookmarksResponse>, String> {
        let path = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let content =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let bookmarks_map = self.state.get_bookmarks(&params.source_id);

        let mut bookmarks: Vec<BookmarkEntry> = bookmarks_map
            .iter()
            .filter_map(|(&line_num, note)| {
                if line_num > 0 && line_num <= lines.len() {
                    let line = lines[line_num - 1];
                    let entry = LogEntry::new(line_num, line.to_string(), 0);
                    Some(BookmarkEntry {
                        line_number: line_num,
                        content: line.to_string(),
                        level: entry.level.map(|l| l.as_str().to_string()),
                        timestamp: entry.timestamp.map(|t| t.to_rfc3339()),
                        note: note.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by line number
        bookmarks.sort_by_key(|b| b.line_number);

        Ok(Json(ListBookmarksResponse {
            total_bookmarks: bookmarks.len(),
            bookmarks,
        }))
    }

    /// Manage bookmarks (add, remove, toggle, clear)
    #[tool(
        name = "manage_bookmarks",
        description = "Add, remove, toggle, or clear bookmarks on log entries. Actions: 'add' (add bookmarks to specified lines), 'remove' (remove bookmarks from specified lines), 'toggle' (toggle bookmark state), 'clear_all' (remove all bookmarks)."
    )]
    fn manage_bookmarks(
        &self,
        Parameters(params): Parameters<ManageBookmarksParams>,
    ) -> Result<Json<ManageBookmarksResponse>, String> {
        // Verify source exists
        let _ = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let (affected_count, message) = self.state.manage_bookmarks(
            &params.source_id,
            &params.action,
            &params.line_numbers,
            params.note.as_deref(),
        );

        Ok(Json(ManageBookmarksResponse {
            success: true,
            action: params.action,
            affected_count,
            message,
        }))
    }

    /// Analyze log frequency over time
    #[tool(
        name = "analyze_timeline",
        description = "Analyze log entry frequency over time. Groups entries into time buckets and provides counts per bucket, including breakdown by log level. Useful for identifying activity spikes and patterns."
    )]
    fn analyze_timeline(
        &self,
        Parameters(params): Parameters<AnalyzeTimelineParams>,
    ) -> Result<Json<AnalyzeTimelineResponse>, String> {
        let path = self
            .state
            .get_source_path(&params.source_id)
            .ok_or_else(|| format!("Source not found: {}", params.source_id))?;

        let timeline = self.state.analyze_log_timeline(
            &path,
            params.bucket_size_seconds,
            &params.levels,
            params.start_time.as_deref(),
            params.end_time.as_deref(),
        )?;

        let summary = if timeline.data_points.is_empty() {
            "No log entries with timestamps found in the specified range.".to_string()
        } else {
            let total_entries: usize = timeline.data_points.iter().map(|dp| dp.count).sum();
            let num_buckets = timeline.data_points.len();
            let avg_per_bucket = total_entries as f64 / num_buckets as f64;
            format!(
                "Analyzed {} entries across {} time buckets ({} seconds each). Average: {:.1} entries/bucket. Peak: {} entries{}.",
                total_entries,
                num_buckets,
                params.bucket_size_seconds,
                avg_per_bucket,
                timeline.peak_count,
                timeline.peak_timestamp.as_ref().map(|t| format!(" at {}", t)).unwrap_or_default()
            )
        };

        Ok(Json(AnalyzeTimelineResponse { timeline, summary }))
    }
}

#[tool_handler]
impl ServerHandler for LoglineTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Logline MCP Server - Provides log analysis tools for AI assistants.\n\n\
                Available tools:\n\
                - list_log_sources: List all available log sources\n\
                - get_log_entries: Read log entries with pagination\n\
                - search_logs: Search logs with keyword/regex\n\
                - get_log_statistics: Get statistical summary\n\
                - analyze_errors: Analyze error patterns\n\
                - advanced_filter: Filter logs with multiple conditions\n\
                - list_bookmarks: List all bookmarked entries\n\
                - manage_bookmarks: Add/remove/toggle bookmarks\n\
                - analyze_timeline: Analyze log frequency over time"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert LogEntry to LogEntryResult for serialization
fn entry_to_result(entry: &LogEntry) -> LogEntryResult {
    LogEntryResult {
        line_number: entry.line_number,
        content: entry.content.clone(),
        level: entry.level.map(|l| l.as_str().to_string()),
        timestamp: entry.timestamp.map(|t| t.to_rfc3339()),
    }
}

/// Simplify error message to create grouping pattern
fn simplify_error_pattern(line: &str) -> String {
    // Remove timestamps, numbers, IDs, etc. to group similar errors
    let mut pattern = line.to_string();

    // Remove common timestamp patterns
    let timestamp_re = Regex::new(r"\d{4}[-/]\d{2}[-/]\d{2}[T ]\d{2}:\d{2}:\d{2}").unwrap();
    pattern = timestamp_re
        .replace_all(&pattern, "[TIMESTAMP]")
        .to_string();

    // Replace UUIDs
    let uuid_re =
        Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
            .unwrap();
    pattern = uuid_re.replace_all(&pattern, "[UUID]").to_string();

    // Replace numbers (but keep short ones like error codes)
    let number_re = Regex::new(r"\b\d{5,}\b").unwrap();
    pattern = number_re.replace_all(&pattern, "[NUM]").to_string();

    // Replace IP addresses
    let ip_re = Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap();
    pattern = ip_re.replace_all(&pattern, "[IP]").to_string();

    // Truncate if too long
    if pattern.len() > 200 {
        pattern.truncate(200);
        pattern.push_str("...");
    }

    pattern
}
