//! MCP types and data structures

use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Log source type
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LogSourceType {
    Local,
    Remote,
}

/// Log source information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LogSource {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Source type (local/remote)
    #[serde(rename = "type")]
    pub source_type: LogSourceType,
    /// Connection status (for remote sources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// File path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// File size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Last activity timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<String>,
    /// Total bytes received (for remote sources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_received: Option<u64>,
}

/// Log entry returned by MCP tools
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LogEntryResult {
    /// Line number in the file (1-indexed)
    pub line_number: usize,
    /// Raw content
    pub content: String,
    /// Detected log level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Parsed timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Log level distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct LevelDistribution {
    #[serde(rename = "TRACE", default)]
    pub trace: usize,
    #[serde(rename = "DEBUG", default)]
    pub debug: usize,
    #[serde(rename = "INFO", default)]
    pub info: usize,
    #[serde(rename = "WARN", default)]
    pub warn: usize,
    #[serde(rename = "ERROR", default)]
    pub error: usize,
    #[serde(rename = "FATAL", default)]
    pub fatal: usize,
}

/// Log statistics summary
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LogStatistics {
    /// Total number of lines
    pub total_lines: usize,
    /// Level distribution
    pub level_distribution: LevelDistribution,
    /// Error rate percentage
    pub error_rate: String,
    /// Time range of logs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
}

/// Time range
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimeRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

/// Search match result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchMatch {
    /// Line number
    pub line_number: usize,
    /// Line content
    pub content: String,
    /// Detected log level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Context lines before the match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_before: Option<Vec<String>>,
    /// Context lines after the match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_after: Option<Vec<String>>,
}

/// Error group for analysis
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ErrorGroup {
    /// Error pattern or message
    pub pattern: String,
    /// Number of occurrences
    pub count: usize,
    /// First occurrence timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_occurrence: Option<String>,
    /// Last occurrence timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_occurrence: Option<String>,
    /// Sample log entries
    pub sample_entries: Vec<LogEntryResult>,
}

/// Bookmark entry information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BookmarkEntry {
    /// Line number
    pub line_number: usize,
    /// Line content
    pub content: String,
    /// Log level if detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Timestamp if detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Optional note/comment for this bookmark
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Log frequency data point
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FrequencyDataPoint {
    /// Time bucket (ISO 8601 timestamp)
    pub timestamp: String,
    /// Count of entries in this bucket
    pub count: usize,
    /// Count by level in this bucket
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_level: Option<LevelDistribution>,
}

/// Timeline analysis result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineAnalysis {
    /// Frequency data points
    pub data_points: Vec<FrequencyDataPoint>,
    /// Bucket size in seconds
    pub bucket_size_seconds: u64,
    /// Peak activity timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_timestamp: Option<String>,
    /// Peak count
    pub peak_count: usize,
}

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Port for SSE server
    pub port: u16,
    /// Bind address
    pub bind_address: String,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: 12600,
            bind_address: "127.0.0.1".to_string(),
        }
    }
}
