//! Grok pattern parsing for log analysis
//!
//! Provides grok pattern matching capabilities for parsing unstructured log data
//! into structured fields. Includes built-in patterns for common log formats.

use anyhow::{Context, Result};
use grok::Grok;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

/// Pre-processor for log lines before Grok pattern matching
///
/// This allows extracting the actual log content from structured wrappers
/// like JSON lines before applying Grok patterns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreProcessor {
    /// No pre-processing, apply Grok directly to the raw line
    #[default]
    None,
    /// Parse the line as JSON and extract a specific field's value
    /// The field name is the string (e.g., "log" or "message")
    JsonField(String),
}

/// Built-in grok pattern templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinPattern {
    /// Common log format (Apache/Nginx access logs)
    CommonLog,
    /// Combined log format (Apache/Nginx with referrer and user agent)
    CombinedLog,
    /// Syslog format
    Syslog,
    /// Log4j/Logback format
    Log4j,
    /// JSON log format
    JsonLog,
    /// Simple timestamp + level + message
    SimpleLog,
    /// Docker/Container logs
    DockerLog,
    /// Kubernetes logs
    KubernetesLog,
    /// Java exception stack trace
    JavaStackTrace,
    /// Python exception
    PythonException,
    /// Generic timestamp extraction
    Timestamp,
    /// IP address extraction
    IpAddress,
}

impl BuiltinPattern {
    /// Get all available builtin patterns
    pub fn all() -> &'static [BuiltinPattern] {
        &[
            BuiltinPattern::CommonLog,
            BuiltinPattern::CombinedLog,
            BuiltinPattern::Syslog,
            BuiltinPattern::Log4j,
            BuiltinPattern::JsonLog,
            BuiltinPattern::SimpleLog,
            BuiltinPattern::DockerLog,
            BuiltinPattern::KubernetesLog,
            BuiltinPattern::JavaStackTrace,
            BuiltinPattern::PythonException,
            BuiltinPattern::Timestamp,
            BuiltinPattern::IpAddress,
        ]
    }

    /// Get the display name for the pattern
    pub fn display_name(&self) -> &'static str {
        match self {
            BuiltinPattern::CommonLog => "Common Log (CLF)",
            BuiltinPattern::CombinedLog => "Combined Log",
            BuiltinPattern::Syslog => "Syslog",
            BuiltinPattern::Log4j => "Log4j/Logback",
            BuiltinPattern::JsonLog => "JSON Log",
            BuiltinPattern::SimpleLog => "Simple Log",
            BuiltinPattern::DockerLog => "Docker Log",
            BuiltinPattern::KubernetesLog => "Kubernetes Log",
            BuiltinPattern::JavaStackTrace => "Java Stack Trace",
            BuiltinPattern::PythonException => "Python Exception",
            BuiltinPattern::Timestamp => "Timestamp",
            BuiltinPattern::IpAddress => "IP Address",
        }
    }

    /// Get the grok pattern string
    pub fn pattern(&self) -> &'static str {
        match self {
            BuiltinPattern::CommonLog => {
                r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{URIPATHPARAM:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} (?:%{NUMBER:bytes}|-)"#
            }
            BuiltinPattern::CombinedLog => {
                r#"%{IPORHOST:clientip} %{USER:ident} %{USER:auth} \[%{HTTPDATE:timestamp}\] "%{WORD:verb} %{URIPATHPARAM:request} HTTP/%{NUMBER:httpversion}" %{NUMBER:response} (?:%{NUMBER:bytes}|-) "%{DATA:referrer}" "%{DATA:agent}""#
            }
            BuiltinPattern::Syslog => {
                r#"%{SYSLOGTIMESTAMP:timestamp} %{SYSLOGHOST:hostname} %{DATA:program}(?:\[%{POSINT:pid}\])?: %{GREEDYDATA:message}"#
            }
            BuiltinPattern::Log4j => {
                r#"%{TIMESTAMP_ISO8601:timestamp}\s+\[%{DATA:thread}\]\s+%{LOGLEVEL:level}\s+%{DATA:logger}\s+-\s+%{GREEDYDATA:message}"#
            }
            BuiltinPattern::JsonLog => {
                r#"\{.*"timestamp"\s*:\s*"%{TIMESTAMP_ISO8601:timestamp}".*"level"\s*:\s*"%{LOGLEVEL:level}".*"message"\s*:\s*"%{DATA:message}".*\}"#
            }
            BuiltinPattern::SimpleLog => {
                r#"%{TIMESTAMP_ISO8601:timestamp}\s+%{LOGLEVEL:level}\s+%{GREEDYDATA:message}"#
            }
            BuiltinPattern::DockerLog => {
                r#"%{TIMESTAMP_ISO8601:timestamp}\s+%{WORD:stream}\s+%{WORD:flag}\s+%{GREEDYDATA:message}"#
            }
            BuiltinPattern::KubernetesLog => {
                r#"%{TIMESTAMP_ISO8601:timestamp}\s+%{DATA:namespace}\s+%{DATA:pod}\s+%{DATA:container}\s+%{LOGLEVEL:level}\s+%{GREEDYDATA:message}"#
            }
            BuiltinPattern::JavaStackTrace => {
                r#"(?:%{JAVACLASS:class}(?::\s+%{GREEDYDATA:message})?|at\s+%{JAVACLASS:class}\.%{WORD:method}\(%{JAVAFILE:file}(?::%{INT:line})?\))"#
            }
            BuiltinPattern::PythonException => {
                r#"(?:Traceback \(most recent call last\):|File "%{DATA:file}", line %{INT:line}(?:, in %{DATA:function})?|%{WORD:exception_type}(?::\s*%{GREEDYDATA:message})?)"#
            }
            BuiltinPattern::Timestamp => r#"%{TIMESTAMP_ISO8601:timestamp}"#,
            BuiltinPattern::IpAddress => r#"%{IP:ip}(?::%{POSINT:port})?"#,
        }
    }

    /// Get the default display template for this pattern
    pub fn default_template(&self) -> &'static str {
        match self {
            BuiltinPattern::CommonLog => {
                "%{timestamp} %{clientip} %{verb} %{request} -> %{response}"
            }
            BuiltinPattern::CombinedLog => {
                "%{timestamp} %{clientip} %{verb} %{request} -> %{response} [%{agent}]"
            }
            BuiltinPattern::Syslog => "%{timestamp} %{hostname} %{program}: %{message}",
            BuiltinPattern::Log4j => "%{timestamp} [%{level}] %{logger} - %{message}",
            BuiltinPattern::JsonLog => "%{timestamp} [%{level}] %{message}",
            BuiltinPattern::SimpleLog => "%{timestamp} [%{level}] %{message}",
            BuiltinPattern::DockerLog => "%{timestamp} [%{stream}] %{message}",
            BuiltinPattern::KubernetesLog => {
                "%{timestamp} %{namespace}/%{pod} [%{level}] %{message}"
            }
            BuiltinPattern::JavaStackTrace => "", // Keep original for stack traces
            BuiltinPattern::PythonException => "", // Keep original for exceptions
            BuiltinPattern::Timestamp => "",      // Keep original
            BuiltinPattern::IpAddress => "",      // Keep original
        }
    }
}

/// User-defined grok pattern template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPattern {
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Grok pattern string
    pub pattern: String,
    /// Example log line
    pub example: String,
    /// Whether this pattern is enabled
    pub enabled: bool,
    /// Display template for formatting output (e.g., "%{timestamp} [%{level}] %{message}")
    /// If empty, the original content is shown
    pub display_template: String,
    /// Pre-processor to apply before Grok matching (e.g., extract "log" field from JSON)
    #[serde(default)]
    pub pre_processor: PreProcessor,
}

/// Result of parsing a log line with grok
#[derive(Debug, Clone, Default)]
pub struct ParsedFields {
    /// Extracted field name-value pairs
    pub fields: HashMap<String, String>,
}

impl ParsedFields {
    /// Check if any fields were extracted
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

/// Part of a display template
#[derive(Debug, Clone)]
pub enum TemplatePart {
    /// Static text
    Text(String),
    /// Field placeholder %{field:options}
    Placeholder {
        field: String,
        options: Option<String>, // raw options string
    },
}

/// Compiled grok pattern for efficient matching
pub struct CompiledPattern {
    /// Compiled grok pattern
    pattern: grok::Pattern,
    /// Display template for formatting output
    pub display_template: Option<String>,
    /// Parsed display template (cached for performance)
    pub parsed_template: Option<Vec<TemplatePart>>,
}

impl CompiledPattern {
    /// Parse template string into parts
    fn parse_template_str(template: &str) -> Vec<TemplatePart> {
        let mut parts = Vec::new();
        // Regex to match %{field:options}
        // Compiled once using LazyLock
        static RE: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"%\{([^:}]+)(?::([^}]+))?\}").unwrap());

        let mut last_end = 0;
        for cap in RE.captures_iter(template) {
            let match_start = cap.get(0).unwrap().start();
            let match_end = cap.get(0).unwrap().end();

            // Add text before this placeholder
            if match_start > last_end {
                parts.push(TemplatePart::Text(
                    template[last_end..match_start].to_string(),
                ));
            }

            let field_name = &cap[1];
            let options = cap.get(2).map(|m| m.as_str().to_string());

            parts.push(TemplatePart::Placeholder {
                field: field_name.to_string(),
                options,
            });

            last_end = match_end;
        }

        // Add remaining text
        if last_end < template.len() {
            parts.push(TemplatePart::Text(template[last_end..].to_string()));
        }

        parts
    }

    /// Match against a log line and extract fields
    /// Match against a log line and extract fields
    pub fn parse(&self, text: &str) -> Option<ParsedFields> {
        self.pattern.match_against(text).map(|matches| {
            let mut fields = HashMap::new();
            for (name, value) in matches.iter() {
                fields.insert(name.to_string(), value.to_string());
            }
            ParsedFields { fields }
        })
    }

    /// Format a log line and generate styled segments
    /// Returns (plain_text, segments) where segments contain styling info
    pub fn format_with_style(
        &self,
        fields: &HashMap<String, String>,
    ) -> Option<(String, Vec<crate::log_entry::FormattedSegment>)> {
        // Use parsed template if available
        if let Some(parts) = &self.parsed_template {
            if parts.is_empty() {
                return None;
            }
            return Some(Self::apply_parsed_template_with_style(parts, fields));
        }

        let template = self.display_template.as_ref()?;
        if template.is_empty() {
            return None;
        }
        Some(Self::apply_template_with_style(template, fields))
    }

    /// Apply parsed template with style
    fn apply_parsed_template_with_style(
        parts: &[TemplatePart],
        fields: &HashMap<String, String>,
    ) -> (String, Vec<crate::log_entry::FormattedSegment>) {
        use crate::log_entry::FormattedSegment;

        let mut segments = Vec::with_capacity(parts.len());
        let mut result = String::with_capacity(256);

        for part in parts {
            match part {
                TemplatePart::Text(text) => {
                    result.push_str(text);
                    segments.push(FormattedSegment {
                        text: text.clone(),
                        color: None,
                    });
                }
                TemplatePart::Placeholder { field, options } => {
                    let mut value = fields.get(field).map(|s| s.to_string()).unwrap_or_default();

                    if value.is_empty() {
                        continue;
                    }

                    // Parse options
                    let mut color: Option<(u8, u8, u8)> = None;

                    if let Some(opts) = options {
                        for opt in opts.split(',') {
                            let opt = opt.trim();
                            if let Some(color_val) = opt.strip_prefix("color=") {
                                color = Self::parse_color(color_val);
                            } else if let Some(fmt) = opt.strip_prefix("format=") {
                                // Try to format as timestamp
                                value = Self::format_timestamp(&value, fmt).unwrap_or(value);
                            }
                        }
                    }

                    result.push_str(&value);
                    segments.push(FormattedSegment { text: value, color });
                }
            }
        }

        (result, segments)
    }

    /// Apply template with style information
    /// Supports syntax like: %{field:color=red,bold} %{timestamp:format=%Y-%m-%d,color=#00FF00}
    pub fn apply_template_with_style(
        template: &str,
        fields: &HashMap<String, String>,
    ) -> (String, Vec<crate::log_entry::FormattedSegment>) {
        use crate::log_entry::FormattedSegment;

        let mut segments = Vec::new();
        let mut result = String::new();
        let mut last_end = 0;

        // Regex to match %{field:options}
        let re = regex::Regex::new(r"%\{([^:}]+)(?::([^}]+))?\}").unwrap();

        for cap in re.captures_iter(template) {
            let match_start = cap.get(0).unwrap().start();
            let match_end = cap.get(0).unwrap().end();

            // Add text before this placeholder
            if match_start > last_end {
                let text = &template[last_end..match_start];
                result.push_str(text);
                segments.push(FormattedSegment {
                    text: text.to_string(),
                    color: None,
                });
            }

            let field_name = &cap[1];
            let options = cap.get(2).map(|m| m.as_str());

            // Get field value
            let mut value = fields
                .get(field_name)
                .map(|s| s.to_string())
                .unwrap_or_default();

            // Parse options
            let mut color: Option<(u8, u8, u8)> = None;

            if let Some(opts) = options {
                for opt in opts.split(',') {
                    let opt = opt.trim();
                    if let Some(color_val) = opt.strip_prefix("color=") {
                        color = Self::parse_color(color_val);
                    } else if let Some(fmt) = opt.strip_prefix("format=") {
                        // Try to format as timestamp
                        value = Self::format_timestamp(&value, fmt).unwrap_or(value);
                    }
                }
            }

            if !value.is_empty() {
                result.push_str(&value);
                segments.push(FormattedSegment { text: value, color });
            }

            last_end = match_end;
        }

        // Add remaining text
        if last_end < template.len() {
            let text = &template[last_end..];
            result.push_str(text);
            segments.push(FormattedSegment {
                text: text.to_string(),
                color: None,
            });
        }

        (result, segments)
    }

    /// Parse color from string (name or hex)
    fn parse_color(color_str: &str) -> Option<(u8, u8, u8)> {
        // Try hex format first (#RGB or #RRGGBB)
        if let Some(hex) = color_str.strip_prefix('#') {
            if hex.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    return Some((r, g, b));
                }
            } else if hex.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..1].repeat(2), 16),
                    u8::from_str_radix(&hex[1..2].repeat(2), 16),
                    u8::from_str_radix(&hex[2..3].repeat(2), 16),
                ) {
                    return Some((r, g, b));
                }
            }
        }

        // Named colors
        match color_str.to_lowercase().as_str() {
            "red" => Some((244, 67, 54)),
            "green" => Some((76, 175, 80)),
            "blue" => Some((33, 150, 243)),
            "yellow" => Some((255, 235, 59)),
            "orange" => Some((255, 152, 0)),
            "purple" => Some((156, 39, 176)),
            "cyan" => Some((0, 188, 212)),
            "gray" | "grey" => Some((158, 158, 158)),
            "white" => Some((255, 255, 255)),
            "black" => Some((0, 0, 0)),
            _ => None,
        }
    }

    /// Format timestamp using chrono format string
    fn format_timestamp(value: &str, format: &str) -> Option<String> {
        use chrono::{DateTime, NaiveDateTime};

        // Try parsing as various timestamp formats
        // ISO 8601
        if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
            return Some(dt.format(format).to_string());
        }

        // Try common formats
        let formats = vec![
            "%Y-%m-%dT%H:%M:%S%.3fZ",
            "%Y-%m-%d %H:%M:%S",
            "%d/%b/%Y:%H:%M:%S %z",
        ];

        for parse_fmt in formats {
            if let Ok(dt) = NaiveDateTime::parse_from_str(value, parse_fmt) {
                return Some(dt.format(format).to_string());
            }
        }

        None
    }

    /// Apply a template string, replacing %{field} placeholders with values
    #[allow(dead_code)]
    pub fn apply_template_simple(template: &str, fields: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in fields {
            let placeholder = format!("%{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        // Remove any remaining unmatched placeholders
        let re = regex::Regex::new(r"%\{[^}]+\}").unwrap();
        re.replace_all(&result, "").to_string()
    }
}

/// Grok parser for log analysis
pub struct GrokParser {
    /// Grok instance with loaded patterns
    grok: Grok,
    /// Currently active compiled pattern
    active_pattern: Option<Arc<CompiledPattern>>,
    /// Fallback access-log pattern for mixed logs
    fallback_access_pattern: Option<Arc<CompiledPattern>>,
    /// Active pattern name for display
    active_pattern_name: Option<String>,
    /// Custom user patterns
    custom_patterns: Vec<CustomPattern>,
    /// Custom pattern definitions (name -> pattern)
    custom_definitions: HashMap<String, String>,
    /// Pre-processor to apply before Grok matching
    pre_processor: PreProcessor,
}

impl Default for GrokParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GrokParser {
    /// Create a new grok parser with default patterns
    pub fn new() -> Self {
        let grok = Grok::default();
        let fallback_access_pattern = Self::compile_builtin(&grok, BuiltinPattern::CombinedLog);

        Self {
            grok,
            active_pattern: None,
            fallback_access_pattern,
            active_pattern_name: None,
            custom_patterns: Vec::new(),
            custom_definitions: HashMap::new(),
            pre_processor: PreProcessor::None,
        }
    }

    fn compile_builtin(grok: &Grok, pattern: BuiltinPattern) -> Option<Arc<CompiledPattern>> {
        let compiled = grok.compile(pattern.pattern(), false).ok()?;
        let template = pattern.default_template().to_string();
        let parsed_template = if !template.is_empty() {
            Some(CompiledPattern::parse_template_str(&template))
        } else {
            None
        };
        Some(Arc::new(CompiledPattern {
            pattern: compiled,
            display_template: Some(template),
            parsed_template,
        }))
    }

    /// Add a custom pattern definition
    pub fn add_pattern_definition(&mut self, name: &str, pattern: &str) {
        self.grok.add_pattern(name, pattern);
        self.custom_definitions
            .insert(name.to_string(), pattern.to_string());
    }

    /// Set the active pattern from a builtin
    pub fn set_builtin_pattern(&mut self, pattern: BuiltinPattern) -> Result<()> {
        let pattern_str = pattern.pattern();
        let compiled = self
            .grok
            .compile(pattern_str, false)
            .with_context(|| format!("Failed to compile pattern: {}", pattern.display_name()))?;

        let template = pattern.default_template().to_string();
        let parsed_template = if !template.is_empty() {
            Some(CompiledPattern::parse_template_str(&template))
        } else {
            None
        };

        self.active_pattern = Some(Arc::new(CompiledPattern {
            pattern: compiled,
            display_template: Some(template),
            parsed_template,
        }));
        self.active_pattern_name = Some(pattern.display_name().to_string());

        Ok(())
    }

    /// Set the active pattern from a custom pattern string
    pub fn set_custom_pattern(&mut self, name: &str, pattern_str: &str) -> Result<()> {
        self.set_custom_pattern_with_template(name, pattern_str, None)
    }

    /// Set the active pattern from a custom pattern string with display template
    pub fn set_custom_pattern_with_template(
        &mut self,
        name: &str,
        pattern_str: &str,
        display_template: Option<&str>,
    ) -> Result<()> {
        let compiled = self
            .grok
            .compile(pattern_str, false)
            .with_context(|| format!("Failed to compile pattern: {}", name))?;

        let parsed_template = if let Some(template) = display_template {
            if !template.is_empty() {
                Some(CompiledPattern::parse_template_str(template))
            } else {
                None
            }
        } else {
            None
        };

        self.active_pattern = Some(Arc::new(CompiledPattern {
            pattern: compiled,
            display_template: display_template
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            parsed_template,
        }));
        self.active_pattern_name = Some(name.to_string());

        Ok(())
    }

    /// Test a custom pattern against a line without mutating parser state
    pub fn test_custom_pattern(
        &self,
        name: &str,
        pattern_str: &str,
        display_template: Option<&str>,
        text: &str,
    ) -> Result<Option<ParsedFields>> {
        // Build a fresh Grok instance and include custom definitions
        let mut grok = Grok::default();
        for (def_name, def_pattern) in &self.custom_definitions {
            grok.add_pattern(def_name, def_pattern);
        }

        let compiled = grok
            .compile(pattern_str, false)
            .with_context(|| format!("Failed to compile pattern: {}", name))?;

        let parsed_template = if let Some(template) = display_template {
            if !template.is_empty() {
                Some(CompiledPattern::parse_template_str(template))
            } else {
                None
            }
        } else {
            None
        };

        let compiled_pattern = CompiledPattern {
            pattern: compiled,
            display_template: display_template
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            parsed_template,
        };

        Ok(compiled_pattern.parse(text))
    }

    /// Clear the active pattern
    pub fn clear_pattern(&mut self) {
        self.active_pattern = None;
        self.active_pattern_name = None;
    }

    /// Parse a log line and return formatted segments if available
    pub fn parse_with_format(
        &self,
        text: &str,
    ) -> Option<(
        ParsedFields,
        Option<(String, Vec<crate::log_entry::FormattedSegment>)>,
    )> {
        let (fields, pattern) = self.parse_with_pattern(text)?;
        let formatted = pattern.format_with_style(&fields.fields);
        Some((fields, formatted))
    }

    fn parse_with_pattern(&self, text: &str) -> Option<(ParsedFields, Arc<CompiledPattern>)> {
        let active_pattern = self.active_pattern.as_ref()?;
        let text_to_parse = self.apply_pre_processor(text)?;

        if let Some(result) = active_pattern.parse(&text_to_parse) {
            return Some((result, active_pattern.clone()));
        }

        // Fallback: if no pre-processor is set, try to auto-extract JSON fields
        if self.pre_processor == PreProcessor::None {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                for field in ["log", "message"] {
                    if let Some(value) = json.get(field) {
                        let extracted = match value {
                            serde_json::Value::String(s) => s.trim_end_matches('\n').to_string(),
                            other => other.to_string(),
                        };
                        Self::log_fallback_attempt(field, &extracted);
                        if let Some(fallback) = active_pattern.parse(&extracted) {
                            return Some((fallback, active_pattern.clone()));
                        }
                        if let Some(result) = self.try_access_fallback(&extracted) {
                            return Some(result);
                        }
                    }
                }
            }
        }

        // Fallback: try access-log pattern for mixed logs
        self.try_access_fallback(&text_to_parse)
    }

    fn try_access_fallback(&self, text: &str) -> Option<(ParsedFields, Arc<CompiledPattern>)> {
        let Some(pattern) = self.fallback_access_pattern.as_ref() else {
            return None;
        };
        if !Self::looks_like_access_log(text) {
            return None;
        }
        pattern.parse(text).map(|fields| (fields, pattern.clone()))
    }

    fn looks_like_access_log(text: &str) -> bool {
        let bytes = text.as_bytes();
        if bytes.is_empty() || !bytes[0].is_ascii_digit() {
            return false;
        }
        text.contains(" - - [") || text.contains("\" ")
    }

    fn log_fallback_attempt(field: &str, extracted: &str) {
        // Throttle logs to avoid spamming
        static LAST_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = LAST_LOG.load(std::sync::atomic::Ordering::Relaxed);
        if now > last + 5 {
            LAST_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
            tracing::info!(
                "JSON fallback attempted using field '{}', extracted (first 200 chars): {:?}",
                field,
                &extracted.chars().take(200).collect::<String>()
            );
        }
    }

    /// Apply the pre-processor to extract the actual log content
    fn apply_pre_processor<'a>(&self, text: &'a str) -> Option<Cow<'a, str>> {
        match &self.pre_processor {
            PreProcessor::None => Some(Cow::Borrowed(text)),
            PreProcessor::JsonField(field_name) => {
                // Try to parse the line as JSON and extract the specified field
                match serde_json::from_str::<serde_json::Value>(text) {
                    Ok(json) => {
                        if let Some(value) = json.get(field_name) {
                            match value {
                                serde_json::Value::String(s) => {
                                    // Trim trailing newline that's common in container logs
                                    Some(Cow::Owned(s.trim_end_matches('\n').to_string()))
                                }
                                // For non-string values, convert to string
                                other => Some(Cow::Owned(other.to_string())),
                            }
                        } else {
                            // Field not found, fallback to original text
                            Some(Cow::Borrowed(text))
                        }
                    }
                    Err(_) => {
                        // Not valid JSON, fallback to original text
                        Some(Cow::Borrowed(text))
                    }
                }
            }
        }
    }

    /// Set the pre-processor
    pub fn set_pre_processor(&mut self, pre_processor: PreProcessor) {
        self.pre_processor = pre_processor;
    }

    /// Get the current pre-processor
    pub fn pre_processor(&self) -> &PreProcessor {
        &self.pre_processor
    }

    /// Check if a pattern is currently active
    pub fn has_active_pattern(&self) -> bool {
        self.active_pattern.is_some()
    }

    /// Get the active pattern name
    pub fn active_pattern_name(&self) -> Option<&str> {
        self.active_pattern_name.as_deref()
    }

    /// Get the active compiled pattern (for sharing across threads)
    pub fn active_pattern(&self) -> Option<Arc<CompiledPattern>> {
        self.active_pattern.clone()
    }

    /// Test a pattern against example text
    /// This also applies the current pre-processor if set
    pub fn test_pattern(&self, pattern_str: &str, text: &str) -> Result<ParsedFields> {
        let compiled = self
            .grok
            .compile(pattern_str, false)
            .context("Failed to compile pattern")?;

        let pattern = CompiledPattern {
            pattern: compiled,
            display_template: None,
            parsed_template: None,
        };

        // Apply pre-processor before testing the pattern
        let text_to_parse = self
            .apply_pre_processor(text)
            .unwrap_or(Cow::Borrowed(text));

        Ok(pattern.parse(&text_to_parse).unwrap_or_default())
    }

    /// Add a custom pattern template
    pub fn add_custom_pattern(&mut self, pattern: CustomPattern) {
        self.custom_patterns.push(pattern);
    }

    /// Remove a custom pattern by index
    pub fn remove_custom_pattern(&mut self, index: usize) -> Option<CustomPattern> {
        if index < self.custom_patterns.len() {
            Some(self.custom_patterns.remove(index))
        } else {
            None
        }
    }

    /// Get all custom patterns
    pub fn custom_patterns(&self) -> &[CustomPattern] {
        &self.custom_patterns
    }

    /// Get mutable reference to custom patterns
    pub fn custom_patterns_mut(&mut self) -> &mut Vec<CustomPattern> {
        &mut self.custom_patterns
    }

    /// Export custom patterns for persistence
    pub fn export_custom_patterns(&self) -> Vec<CustomPattern> {
        self.custom_patterns.clone()
    }

    /// Import custom patterns from persistence
    pub fn import_custom_patterns(&mut self, patterns: Vec<CustomPattern>) {
        self.custom_patterns = patterns;
    }
}

/// Grok configuration for persistence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrokConfig {
    /// Whether grok parsing is enabled
    pub enabled: bool,
    /// Currently selected builtin pattern
    pub builtin_pattern: Option<BuiltinPattern>,
    /// Currently selected custom pattern name
    pub custom_pattern_name: Option<String>,
    /// Custom pattern templates
    pub custom_patterns: Vec<CustomPattern>,
    /// Custom pattern definitions (reusable sub-patterns)
    pub custom_definitions: HashMap<String, String>,
    /// Pre-processor to apply before Grok matching
    #[serde(default)]
    pub pre_processor: PreProcessor,
}
