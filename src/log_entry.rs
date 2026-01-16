//! Log entry data structures and parsing

use chrono::{DateTime, Local, NaiveDateTime};
use regex::Regex;
use std::sync::LazyLock;

/// Log severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    /// Parse log level from string
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.to_uppercase();
        match s.as_str() {
            "TRACE" | "TRC" | "T" => Some(LogLevel::Trace),
            "DEBUG" | "DBG" | "D" => Some(LogLevel::Debug),
            "INFO" | "INF" | "I" => Some(LogLevel::Info),
            "WARN" | "WARNING" | "WRN" | "W" => Some(LogLevel::Warn),
            "ERROR" | "ERR" | "E" => Some(LogLevel::Error),
            "FATAL" | "CRITICAL" | "CRIT" | "F" => Some(LogLevel::Fatal),
            _ => None,
        }
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    /// Get color for this log level (egui color)
    pub fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Trace => egui::Color32::from_rgb(128, 128, 128), // Gray
            LogLevel::Debug => egui::Color32::from_rgb(100, 149, 237), // Cornflower blue
            LogLevel::Info => egui::Color32::from_rgb(60, 120, 180),   // Medium blue
            LogLevel::Warn => egui::Color32::from_rgb(255, 193, 7),    // Amber
            LogLevel::Error => egui::Color32::from_rgb(244, 67, 54),   // Red
            LogLevel::Fatal => egui::Color32::from_rgb(156, 39, 176),  // Purple
        }
    }
}

/// A single log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Line number in the original file (1-indexed)
    pub line_number: usize,
    /// Raw text content
    pub content: String,
    /// Detected log level
    pub level: Option<LogLevel>,
    /// Parsed timestamp (if detected)
    #[allow(dead_code)]
    pub timestamp: Option<DateTime<Local>>,
    /// Whether this entry is bookmarked
    pub bookmarked: bool,
    /// Byte offset in file where this line starts
    #[allow(dead_code)]
    pub byte_offset: u64,
    /// Grok parsed fields (field name -> value)
    pub grok_fields: Option<std::collections::HashMap<String, String>>,
    /// Formatted display text (from grok template)
    pub formatted_content: Option<String>,
    /// Formatted display segments with style info
    pub formatted_segments: Option<Vec<FormattedSegment>>,
}

/// A segment of formatted text with styling information
#[derive(Debug, Clone)]
pub struct FormattedSegment {
    /// The text content
    pub text: String,
    /// Optional color (RGB)
    pub color: Option<(u8, u8, u8)>,
}

impl LogEntry {
    /// Create a new log entry from raw text
    pub fn new(line_number: usize, content: String, byte_offset: u64) -> Self {
        let level = Self::detect_level(&content);
        let timestamp = Self::detect_timestamp(&content);

        Self {
            line_number,
            content,
            level,
            timestamp,
            bookmarked: false,
            byte_offset,
            grok_fields: None,
            formatted_content: None,
            formatted_segments: None,
        }
    }

    /// Set grok parsed fields and optionally formatted content
    pub fn set_grok_fields(&mut self, fields: std::collections::HashMap<String, String>) {
        self.grok_fields = Some(fields);
    }

    /// Clear grok parsed fields and formatted content
    pub fn clear_grok_fields(&mut self) {
        self.grok_fields = None;
        self.formatted_content = None;
        self.formatted_segments = None;
    }

    /// Get the display content (formatted if available, otherwise original)
    pub fn display_content(&self) -> &str {
        self.formatted_content.as_deref().unwrap_or(&self.content)
    }

    /// Detect log level from content
    fn detect_level(content: &str) -> Option<LogLevel> {
        // Common log level patterns
        static LEVEL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)\b(TRACE|DEBUG|DBG|INFO|INF|WARN|WARNING|WRN|ERROR|ERR|FATAL|CRITICAL|CRIT)\b").unwrap()
        });

        LEVEL_REGEX
            .find(content)
            .and_then(|m| LogLevel::from_str(m.as_str()))
    }

    /// Detect timestamp from content
    fn detect_timestamp(content: &str) -> Option<DateTime<Local>> {
        // Common timestamp patterns
        static TIMESTAMP_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
            vec![
                // ISO 8601: 2024-01-15T10:30:45.123
                (
                    Regex::new(r"(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{3})?)").unwrap(),
                    "%Y-%m-%dT%H:%M:%S%.3f",
                ),
                // Common: 2024-01-15 10:30:45
                (
                    Regex::new(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})").unwrap(),
                    "%Y-%m-%d %H:%M:%S",
                ),
                // Date with slash: 2024/01/15 10:30:45
                (
                    Regex::new(r"(\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2})").unwrap(),
                    "%Y/%m/%d %H:%M:%S",
                ),
                // Time only: 10:30:45.123
                (
                    Regex::new(r"(\d{2}:\d{2}:\d{2}(?:\.\d{3})?)").unwrap(),
                    "%H:%M:%S%.3f",
                ),
            ]
        });

        for (regex, format) in TIMESTAMP_PATTERNS.iter() {
            if let Some(cap) = regex.captures(content) {
                if let Some(m) = cap.get(1) {
                    // Try parsing with the format
                    if let Ok(dt) = NaiveDateTime::parse_from_str(m.as_str(), format) {
                        return Some(DateTime::from_naive_utc_and_offset(
                            dt,
                            *Local::now().offset(),
                        ));
                    }
                    // Try parsing date only formats
                    if let Ok(dt) = NaiveDateTime::parse_from_str(
                        &format!("{} 00:00:00", m.as_str()),
                        "%Y-%m-%d %H:%M:%S",
                    ) {
                        return Some(DateTime::from_naive_utc_and_offset(
                            dt,
                            *Local::now().offset(),
                        ));
                    }
                }
            }
        }
        None
    }

    /// Check if content matches a search query
    #[allow(dead_code)]
    pub fn matches(&self, query: &str, case_sensitive: bool, use_regex: bool) -> bool {
        if use_regex {
            let regex = if case_sensitive {
                Regex::new(query)
            } else {
                Regex::new(&format!("(?i){}", query))
            };
            regex.map(|r| r.is_match(&self.content)).unwrap_or(false)
        } else if case_sensitive {
            self.content.contains(query)
        } else {
            self.content.to_lowercase().contains(&query.to_lowercase())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_detection() {
        assert_eq!(
            LogEntry::detect_level("[INFO] Hello world"),
            Some(LogLevel::Info)
        );
        assert_eq!(
            LogEntry::detect_level("2024-01-15 ERROR: Something failed"),
            Some(LogLevel::Error)
        );
        assert_eq!(
            LogEntry::detect_level("[warn] Warning message"),
            Some(LogLevel::Warn)
        );
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("WRN"), Some(LogLevel::Warn));
    }
}
