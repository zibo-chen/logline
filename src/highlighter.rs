//! Syntax highlighting for log entries

use crate::log_entry::LogLevel;
use egui::{text::LayoutJob, Color32, TextFormat};
use regex::Regex;
use std::sync::LazyLock;

/// Theme colors for syntax highlighting
#[derive(Debug, Clone)]
pub struct HighlightTheme {
    /// Background color
    #[allow(dead_code)]
    pub background: Color32,
    /// Default text color
    pub text: Color32,
    /// Line number color
    pub line_number: Color32,
    /// Timestamp color
    pub timestamp: Color32,
    /// String literal color
    pub string: Color32,
    /// Number color
    pub number: Color32,
    /// Keyword color
    pub keyword: Color32,
    /// Comment color
    #[allow(dead_code)]
    pub comment: Color32,
    /// Search highlight background
    pub search_highlight: Color32,
    /// Current match highlight background
    pub current_match: Color32,
    /// Selected line background
    pub selection: Color32,
    /// Bookmark indicator color
    pub bookmark: Color32,
}

impl HighlightTheme {
    /// Dark theme (default)
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(30, 30, 30),
            text: Color32::from_rgb(212, 212, 212),
            line_number: Color32::from_rgb(133, 133, 133),
            timestamp: Color32::from_rgb(86, 156, 214),
            string: Color32::from_rgb(206, 145, 120),
            number: Color32::from_rgb(181, 206, 168),
            keyword: Color32::from_rgb(197, 134, 192),
            comment: Color32::from_rgb(106, 153, 85),
            search_highlight: Color32::from_rgba_unmultiplied(255, 235, 59, 80),
            current_match: Color32::from_rgba_unmultiplied(255, 152, 0, 120),
            selection: Color32::from_rgba_unmultiplied(70, 130, 180, 60),
            bookmark: Color32::from_rgb(255, 193, 7),
        }
    }

    /// Light theme
    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(255, 255, 255),
            text: Color32::from_rgb(36, 36, 36),
            line_number: Color32::from_rgb(140, 140, 140),
            timestamp: Color32::from_rgb(0, 102, 204),
            string: Color32::from_rgb(163, 21, 21),
            number: Color32::from_rgb(9, 134, 88),
            keyword: Color32::from_rgb(175, 0, 219),
            comment: Color32::from_rgb(0, 128, 0),
            search_highlight: Color32::from_rgba_unmultiplied(255, 235, 59, 120),
            current_match: Color32::from_rgba_unmultiplied(255, 152, 0, 150),
            selection: Color32::from_rgba_unmultiplied(100, 150, 255, 150),
            bookmark: Color32::from_rgb(255, 160, 0),
        }
    }
}

impl Default for HighlightTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Syntax highlighter for log content
pub struct Highlighter {
    /// Current theme
    pub theme: HighlightTheme,
    /// Whether highlighting is enabled
    pub enabled: bool,
}

impl Highlighter {
    /// Create a new highlighter with default theme
    pub fn new() -> Self {
        Self {
            theme: HighlightTheme::default(),
            enabled: true,
        }
    }

    /// Create with specific theme
    #[allow(dead_code)]
    pub fn with_theme(theme: HighlightTheme) -> Self {
        Self {
            theme,
            enabled: true,
        }
    }

    /// Highlight a log line and return a LayoutJob for egui
    #[allow(dead_code)]
    pub fn highlight_line(
        &self,
        content: &str,
        level: Option<LogLevel>,
        search_query: Option<&str>,
        case_sensitive: bool,
    ) -> LayoutJob {
        self.highlight_line_with_wrap(
            content,
            level,
            search_query,
            case_sensitive,
            f32::INFINITY,
            0.0,
        )
    }

    /// Highlight a log line with optional wrap width and return a LayoutJob for egui
    pub fn highlight_line_with_wrap(
        &self,
        content: &str,
        level: Option<LogLevel>,
        search_query: Option<&str>,
        case_sensitive: bool,
        wrap_width: f32,
        letter_spacing: f32,
    ) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.wrap.max_width = wrap_width;

        if !self.enabled || content.is_empty() {
            job.append(
                content,
                0.0,
                TextFormat {
                    color: level.map(|l| l.color()).unwrap_or(self.theme.text),
                    extra_letter_spacing: letter_spacing,
                    ..Default::default()
                },
            );
            return job;
        }

        // Get base color from log level
        let base_color = level.map(|l| l.color()).unwrap_or(self.theme.text);

        // Find all highlight ranges
        let mut ranges: Vec<(usize, usize, HighlightType)> = Vec::new();

        // Find timestamp ranges
        Self::find_timestamps(content, &mut ranges);

        // Find number ranges
        Self::find_numbers(content, &mut ranges);

        // Find string ranges (quoted text)
        Self::find_strings(content, &mut ranges);

        // Find JSON braces and brackets
        Self::find_json_syntax(content, &mut ranges);

        // Find search matches (highest priority)
        if let Some(query) = search_query {
            if !query.is_empty() {
                Self::find_search_matches(content, query, case_sensitive, &mut ranges);
            }
        }

        // Sort ranges by start position
        ranges.sort_by_key(|r| (r.0, std::cmp::Reverse(r.2.priority())));

        // Remove overlapping ranges (keep higher priority)
        let mut filtered_ranges: Vec<(usize, usize, HighlightType)> = Vec::new();
        for range in ranges {
            let overlaps = filtered_ranges
                .iter()
                .any(|r| range.0 < r.1 && range.1 > r.0);
            if !overlaps || range.2 == HighlightType::SearchMatch {
                // Search matches can overlap
                if range.2 == HighlightType::SearchMatch {
                    // Remove ranges that search match overlaps with
                    filtered_ranges.retain(|r| !(range.0 < r.1 && range.1 > r.0));
                }
                filtered_ranges.push(range);
            }
        }
        filtered_ranges.sort_by_key(|r| r.0);

        // Build the layout job
        let mut pos = 0;
        for (start, end, hl_type) in filtered_ranges {
            // Add text before this range
            if start > pos {
                job.append(
                    &content[pos..start],
                    0.0,
                    TextFormat {
                        color: base_color,
                        extra_letter_spacing: letter_spacing,
                        ..Default::default()
                    },
                );
            }

            // Add highlighted range
            let format = match hl_type {
                HighlightType::Timestamp => TextFormat {
                    color: self.theme.timestamp,
                    extra_letter_spacing: letter_spacing,
                    ..Default::default()
                },
                HighlightType::Number => TextFormat {
                    color: self.theme.number,
                    extra_letter_spacing: letter_spacing,
                    ..Default::default()
                },
                HighlightType::String => TextFormat {
                    color: self.theme.string,
                    extra_letter_spacing: letter_spacing,
                    ..Default::default()
                },
                HighlightType::JsonSyntax => TextFormat {
                    color: self.theme.keyword,
                    extra_letter_spacing: letter_spacing,
                    ..Default::default()
                },
                HighlightType::SearchMatch => TextFormat {
                    color: Color32::BLACK,
                    background: self.theme.search_highlight,
                    extra_letter_spacing: letter_spacing,
                    ..Default::default()
                },
            };

            job.append(&content[start..end], 0.0, format);
            pos = end;
        }

        // Add remaining text
        if pos < content.len() {
            job.append(
                &content[pos..],
                0.0,
                TextFormat {
                    color: base_color,
                    extra_letter_spacing: letter_spacing,
                    ..Default::default()
                },
            );
        }

        job
    }

    /// Find timestamp patterns in content
    fn find_timestamps(content: &str, ranges: &mut Vec<(usize, usize, HighlightType)>) {
        static TIMESTAMP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"\d{4}[-/]\d{2}[-/]\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?(?:Z|[+-]\d{2}:?\d{2})?|\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?"
            ).unwrap()
        });

        for m in TIMESTAMP_REGEX.find_iter(content) {
            ranges.push((m.start(), m.end(), HighlightType::Timestamp));
        }
    }

    /// Find number patterns in content
    fn find_numbers(content: &str, ranges: &mut Vec<(usize, usize, HighlightType)>) {
        static NUMBER_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b").unwrap());

        for m in NUMBER_REGEX.find_iter(content) {
            ranges.push((m.start(), m.end(), HighlightType::Number));
        }
    }

    /// Find quoted string patterns
    fn find_strings(content: &str, ranges: &mut Vec<(usize, usize, HighlightType)>) {
        static STRING_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'"#).unwrap());

        for m in STRING_REGEX.find_iter(content) {
            ranges.push((m.start(), m.end(), HighlightType::String));
        }
    }

    /// Find JSON syntax characters
    fn find_json_syntax(content: &str, ranges: &mut Vec<(usize, usize, HighlightType)>) {
        for (i, c) in content.char_indices() {
            if matches!(c, '{' | '}' | '[' | ']' | ':' | ',') {
                ranges.push((i, i + 1, HighlightType::JsonSyntax));
            }
        }
    }

    /// Find search query matches
    fn find_search_matches(
        content: &str,
        query: &str,
        case_sensitive: bool,
        ranges: &mut Vec<(usize, usize, HighlightType)>,
    ) {
        if case_sensitive {
            for (i, _) in content.match_indices(query) {
                ranges.push((i, i + query.len(), HighlightType::SearchMatch));
            }
        } else {
            let content_lower = content.to_lowercase();
            let query_lower = query.to_lowercase();
            for (i, _) in content_lower.match_indices(&query_lower) {
                ranges.push((i, i + query.len(), HighlightType::SearchMatch));
            }
        }
    }

    /// Format line number
    #[allow(dead_code)]
    pub fn format_line_number(&self, line_number: usize, max_digits: usize) -> LayoutJob {
        let mut job = LayoutJob::default();
        let text = format!("{:>width$}", line_number, width = max_digits);
        job.append(
            &text,
            0.0,
            TextFormat {
                color: self.theme.line_number,
                ..Default::default()
            },
        );
        job
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HighlightType {
    Timestamp,
    Number,
    String,
    JsonSyntax,
    SearchMatch,
}

impl HighlightType {
    /// Get priority (higher = more important)
    fn priority(&self) -> u8 {
        match self {
            HighlightType::SearchMatch => 100,
            HighlightType::String => 50,
            HighlightType::Timestamp => 40,
            HighlightType::Number => 30,
            HighlightType::JsonSyntax => 20,
        }
    }
}

/// Helper to create a simple colored text job
#[allow(dead_code)]
pub fn colored_text(text: &str, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            color,
            ..Default::default()
        },
    );
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_line() {
        let highlighter = Highlighter::new();
        let job = highlighter.highlight_line(
            "2024-01-15 10:30:45 INFO Hello world",
            Some(LogLevel::Info),
            None,
            false,
        );
        assert!(!job.text.is_empty());
    }

    #[test]
    fn test_search_highlight() {
        let highlighter = Highlighter::new();
        let job = highlighter.highlight_line("Hello world test", None, Some("world"), false);
        // Should contain the search term
        assert!(job.text.contains("world"));
    }
}
